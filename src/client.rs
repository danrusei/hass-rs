//! Home Assistant client implementation
//!
//! A [`HassClient`] owns a background supervisor task that keeps the websocket
//! connection alive: when the connection is lost it reconnects with an
//! exponential backoff, re-authenticates with the cached long lived access
//! token, restores the event subscriptions of the caller and resumes
//! delivering events on the very same channels.

use crate::types::{
    Ask, Auth, CallService, Command, HassConfig, HassEntity, HassPanels, HassRegistryArea,
    HassRegistryDevice, HassRegistryEntity, HassServices, Response, Subscribe, Unsubscribe,
    WSEvent,
};
use crate::{HassError, HassIssues, HassResult};

use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use parking_lot::Mutex;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::oneshot::{
    channel as oneshot, Receiver as OneShotReceiver, Sender as OneShotSender,
};
use tokio::sync::{watch, Mutex as AsyncMutex};
use tokio::time::{interval, sleep, timeout, Instant, MissedTickBehavior};
use tokio_tungstenite::tungstenite::{Error, Message};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

/// The websocket stream produced by [`connect_async`].
type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
type WsSink = SplitSink<WsStream, Message>;
type WsSource = SplitStream<WsStream>;

/// Capacity of the client -> supervisor command queue.
const OUTBOUND_QUEUE: usize = 20;
/// Capacity of the supervisor -> socket writer queue.
const WRITE_QUEUE: usize = 20;
/// Capacity of the event channel handed out by
/// [`HassClient::subscribe_event`]; a consumer that lags further behind loses
/// events instead of stalling the connection.
const EVENT_BUFFER: usize = 20;
/// Reported to callers whose request was in flight when a session died
/// without a close frame.
const CLOSE_REASON_LOST: &str = "connection lost";

/// Options controlling reconnection and connection liveness detection.
///
/// The defaults target a long running client: reconnect forever with an
/// exponential backoff between 1s and 60s, and a ping heartbeat every 30s.
#[derive(Debug, Clone)]
pub struct ReconnectOptions {
    /// Reconnect automatically when the connection is lost.
    ///
    /// When false the client behaves like a plain one-shot connection: the
    /// request in flight fails with [`HassError::ConnectionClosed`] and the
    /// state becomes [`ConnectionState::Closed`].
    pub enabled: bool,
    /// Delay before the first reconnection attempt. Doubles on every
    /// consecutive failure and is capped by [`Self::max_delay`].
    pub initial_delay: Duration,
    /// Upper bound of the reconnect delay.
    pub max_delay: Duration,
    /// Maximum number of consecutive *failed* attempts, `None` retries forever.
    ///
    /// The counter is reset as soon as a connection is established.
    pub max_attempts: Option<u32>,
    /// How long to wait for `auth_ok` when re-authenticating, and for the
    /// confirmation of re-subscriptions. A timeout is treated as a dead
    /// connection.
    pub handshake_timeout: Duration,
    /// Interval of the ping heartbeat, `None` disables it.
    ///
    /// The heartbeat is what detects a connection that looks alive but stopped
    /// answering (sleeping machine, NAT timeout, dead peer). Without it a peer
    /// that vanishes without closing the socket is only noticed when a write
    /// fails, so in-flight requests can then wait indefinitely.
    pub heartbeat_interval: Option<Duration>,
    /// How long to wait for a `pong` before declaring the connection dead.
    pub heartbeat_timeout: Duration,
    /// Maximum time a single frame write may take before the connection is
    /// considered dead.
    pub write_timeout: Duration,
}

impl Default for ReconnectOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            max_attempts: None,
            handshake_timeout: Duration::from_secs(10),
            heartbeat_interval: Some(Duration::from_secs(30)),
            heartbeat_timeout: Duration::from_secs(10),
            write_timeout: Duration::from_secs(10),
        }
    }
}

impl ReconnectOptions {
    /// Options for a client that never reconnects.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            heartbeat_interval: None,
            ..Self::default()
        }
    }
}

/// State of the underlying websocket connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Dialing the server, or restoring the session (auth / subscriptions).
    Connecting,
    /// The connection is up and commands are accepted.
    Connected,
    /// The connection was lost and a reconnection attempt is pending.
    Disconnected,
    /// The client stopped trying: reconnection is disabled, the attempts ran
    /// out, the access token was rejected, or the client was dropped.
    Closed,
}

/// A live event subscription.
///
/// Events are routed by the id the *current* session uses for the
/// subscription, while callers keep addressing it by the id of the command that
/// created it.
struct Subscription {
    event_type: String,
    session_id: u64,
    sender: Sender<WSEvent>,
}

#[derive(Default)]
struct SubscriptionRegistry {
    by_handle: HashMap<u64, Subscription>,
    by_session_id: HashMap<u64, u64>,
}

impl SubscriptionRegistry {
    fn insert(&mut self, handle: u64, subscription: Subscription) {
        self.by_session_id.insert(subscription.session_id, handle);
        self.by_handle.insert(handle, subscription);
    }

    /// Point an existing subscription at the id it got in a new session.
    fn retarget(&mut self, handle: u64, session_id: u64) {
        if let Some(subscription) = self.by_handle.get_mut(&handle) {
            self.by_session_id.remove(&subscription.session_id);
            subscription.session_id = session_id;
            self.by_session_id.insert(session_id, handle);
        }
    }

    fn remove(&mut self, handle: u64) -> Option<Subscription> {
        let subscription = self.by_handle.remove(&handle);
        if let Some(subscription) = &subscription {
            self.by_session_id.remove(&subscription.session_id);
        }
        subscription
    }

    fn handle_for_session(&self, session_id: u64) -> Option<u64> {
        self.by_session_id.get(&session_id).copied()
    }

    fn remove_closed(&mut self) {
        self.by_handle
            .retain(|_, subscription| !subscription.sender.is_closed());
        let alive = &self.by_handle;
        self.by_session_id
            .retain(|_, handle| alive.contains_key(handle));
    }
}

/// State shared between the [`HassClient`] handle and the connection supervisor.
#[derive(Default)]
struct ReceiverState {
    /// Whether the socket of the current session is up. Turned off *before*
    /// pending requests are drained, so a command can never register into a
    /// map that nobody will drain (which is what used to make callers hang
    /// forever).
    session_open: AtomicBool,
    subscriptions: Mutex<SubscriptionRegistry>,
    pending_requests: Mutex<HashMap<u64, OneShotSender<Response>>>,
    untagged_request: Mutex<Option<OneShotSender<Response>>>,
}

impl ReceiverState {
    fn set_session_open(&self, open: bool) {
        self.session_open.store(open, Ordering::SeqCst);
    }

    /// Register a responder, refusing to do so when the session is already gone.
    fn register(&self, id: Option<u64>, tx: OneShotSender<Response>) -> Result<(), ()> {
        match id {
            Some(id) => {
                let mut pending = self.pending_requests.lock();
                if !self.session_open.load(Ordering::SeqCst) {
                    return Err(());
                }
                pending.insert(id, tx);
                Ok(())
            }
            None => {
                let mut slot = self.untagged_request.lock();
                if !self.session_open.load(Ordering::SeqCst) {
                    return Err(());
                }
                slot.replace(tx);
                Ok(())
            }
        }
    }

    fn take_responder(&self, id: u64) -> Option<OneShotSender<Response>> {
        self.pending_requests.lock().remove(&id)
    }

    fn take_untagged(&self) -> Option<OneShotSender<Response>> {
        self.untagged_request.lock().take()
    }

    /// Close the session for writes and resolve every waiter with a
    /// [`Response::Close`] carrying `reason`.
    fn fail_pending(&self, reason: &str) {
        self.session_open.store(false, Ordering::SeqCst);

        let mut pending = self.pending_requests.lock();
        for (_, tx) in pending.drain() {
            let _ = tx.send(Response::Close(reason.to_owned()));
        }
        drop(pending);

        if let Some(tx) = self.untagged_request.lock().take() {
            let _ = tx.send(Response::Close(reason.to_owned()));
        }
    }

    fn insert_subscription(&self, handle: u64, subscription: Subscription) {
        self.subscriptions.lock().insert(handle, subscription);
    }

    fn subscription_for_session(&self, session_id: u64) -> Option<(u64, Sender<WSEvent>)> {
        let registry = self.subscriptions.lock();
        let handle = registry.handle_for_session(session_id)?;
        registry
            .by_handle
            .get(&handle)
            .map(|subscription| (handle, subscription.sender.clone()))
    }

    fn session_id_of(&self, handle: u64) -> Option<u64> {
        self.subscriptions
            .lock()
            .by_handle
            .get(&handle)
            .map(|subscription| subscription.session_id)
    }

    fn remove_subscription(&self, handle: u64) -> Option<Subscription> {
        self.subscriptions.lock().remove(handle)
    }

    /// Subscriptions that still have a live receiver, as (handle, event type).
    fn live_subscriptions(&self) -> Vec<(u64, String)> {
        let mut registry = self.subscriptions.lock();
        registry.remove_closed();
        registry
            .by_handle
            .iter()
            .map(|(handle, subscription)| (*handle, subscription.event_type.clone()))
            .collect()
    }

    fn retarget_subscription(&self, handle: u64, session_id: u64) {
        self.subscriptions.lock().retarget(handle, session_id);
    }

    fn remove_closed_subscriptions(&self) {
        self.subscriptions.lock().remove_closed();
    }

    /// Drop every subscription sender, which ends the receivers handed to callers.
    fn close_subscriptions(&self) {
        let mut registry = self.subscriptions.lock();
        registry.by_handle.clear();
        registry.by_session_id.clear();
    }
}

/// Everything the connection supervisor and the public handle share.
struct Shared {
    url: String,
    options: ReconnectOptions,
    /// Long lived access token, cached so the supervisor can re-authenticate.
    token: Mutex<Option<String>>,
    /// Serialises authentication so a session is never authenticated twice.
    auth_lock: AsyncMutex<()>,
    /// Whether the current session completed authentication.
    authenticated: AtomicBool,
    /// Bumped for every session; stamps the frames in [`Shared::message_tx`].
    session_generation: AtomicU64,
    last_sequence: AtomicU64,
    rx_state: ReceiverState,
    /// Command queue from the client handle to the supervisor. Each frame is
    /// stamped with the session generation it was created for, so a command
    /// that was queued as a session died is not replayed on the next one.
    message_tx: Sender<(u64, Message)>,
    state_tx: watch::Sender<ConnectionState>,
}

impl Shared {
    fn next_seq(&self) -> u64 {
        self.last_sequence.fetch_add(1, Ordering::Relaxed)
    }

    fn set_state(&self, state: ConnectionState) {
        self.state_tx.send_replace(state);
    }
}

/// Signals the supervisor to stop when the [`HassClient`] handle is dropped.
struct ShutdownGuard(Option<OneShotSender<()>>);

impl Drop for ShutdownGuard {
    fn drop(&mut self) {
        if let Some(tx) = self.0.take() {
            let _ = tx.send(());
        }
    }
}

/// How a websocket session ended.
enum ConnectionEnd {
    /// The session died, reconnecting is worthwhile.
    Lost {
        /// Whether this session ever reached [`ConnectionState::Connected`].
        was_connected: bool,
    },
    /// No point in reconnecting, e.g. the access token was rejected.
    Fatal(String),
    /// The client handle was dropped.
    Shutdown,
}

/// HassClient is a library that is meant to simplify the conversation with HomeAssistant Web Socket Server
/// it provides a number of convenient functions that creates the requests and read the messages from server
///
/// The client keeps the connection alive on its own. See [`ReconnectOptions`]
/// for the reconnection policy and [`HassClient::watch_connection_state`] to
/// observe it.
pub struct HassClient {
    shared: Arc<Shared>,
    state_rx: watch::Receiver<ConnectionState>,
    _shutdown: ShutdownGuard,
}

/// Completes as soon as the session is marked dead.
///
/// The signal is level triggered on purpose: a plain `Notify` permit could be
/// consumed by a waiter that is dropped together with a finished handshake, and
/// the supervisor would then never notice that the socket is gone.
async fn wait_dead(dead: &mut watch::Receiver<bool>) {
    loop {
        if *dead.borrow_and_update() {
            return;
        }
        if dead.changed().await.is_err() {
            return;
        }
    }
}

/// Writes commands to one session's socket.
///
/// A write that takes longer than `write_timeout` marks the connection dead:
/// otherwise a wedged socket would block the supervisor forever.
async fn writer_task(
    mut sink: WsSink,
    mut queue: Receiver<Message>,
    dead: watch::Sender<bool>,
    write_timeout: Duration,
) {
    while let Some(message) = queue.recv().await {
        match timeout(write_timeout, sink.send(message)).await {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                log::warn!("websocket write failed: {err:#}");
                break;
            }
            Err(_elapsed) => {
                log::warn!("websocket write timed out after {write_timeout:?}");
                break;
            }
        }
    }
    let _ = timeout(write_timeout, sink.close()).await;
    dead.send_replace(true);
}

/// Reads and dispatches the messages of one session.
async fn reader_task(
    mut source: WsSource,
    shared: Arc<Shared>,
    writer_tx: Sender<Message>,
    dead: watch::Sender<bool>,
) {
    while let Some(message) = source.next().await {
        log::trace!("incoming: {message:#?}");
        match check_if_event(message) {
            Ok(event) => {
                let Some((handle, tx)) = shared.rx_state.subscription_for_session(event.id) else {
                    log::debug!("no subscription for event id={}", event.id);
                    continue;
                };
                match tx.try_send(event) {
                    Ok(()) => {}
                    Err(TrySendError::Full(event)) => {
                        // Never let a slow consumer stall the socket: a blocked
                        // reader would stop answering pongs, and the heartbeat
                        // would then tear down a perfectly healthy connection.
                        log::warn!(
                            "subscriber of subscription {} is not keeping up, dropping an event",
                            event.id
                        );
                    }
                    Err(TrySendError::Closed(_)) => {
                        // The caller dropped the receiver, tell the server to stop.
                        if let Some(subscription) = shared.rx_state.remove_subscription(handle) {
                            let unsub_id = shared.next_seq();
                            let cmd = Command::Unsubscribe(Unsubscribe {
                                id: unsub_id,
                                msg_type: "unsubscribe_events".to_owned(),
                                subscription: subscription.session_id,
                            });
                            let _ = writer_tx.send(cmd.into_tungstenite_message()).await;
                        }
                    }
                }
            }
            Err(message) => match message {
                Ok(Message::Text(data)) => {
                    let payload: Result<Response, HassError> =
                        serde_json::from_str(data.as_str()).map_err(HassError::UnableToDeserialize);

                    match payload {
                        Ok(response) => match response.id() {
                            Some(id) => {
                                if let Some(tx) = shared.rx_state.take_responder(id) {
                                    tx.send(response).ok();
                                } else {
                                    log::error!("no responder for id={id} {response:#?}");
                                }
                            }
                            None => {
                                if matches!(&response, Response::AuthRequired(_)) {
                                    // AuthRequired is always sent unilaterally at connect time.
                                    // It is never a response to one of our commands, so the
                                    // simplest way to deal with it is to ignore it.
                                    log::trace!("Ignoring {response:?}");
                                    continue;
                                }

                                if let Some(tx) = shared.rx_state.take_untagged() {
                                    tx.send(response).ok();
                                } else {
                                    log::error!("no untagged responder for {response:#?}");
                                }
                            }
                        },
                        Err(err) => {
                            log::error!("Error deserializing response: {err:#} {data}");
                        }
                    }
                }
                Ok(Message::Ping(data)) => {
                    if let Err(err) = writer_tx.send(Message::Pong(data)).await {
                        log::error!("Error responding to ping: {err:#}");
                        break;
                    }
                }
                Ok(Message::Close(frame)) => {
                    let reason = frame.map_or_else(String::new, |f| f.reason.to_string());
                    log::info!("Close message received: {reason:?}");
                    shared.rx_state.fail_pending(&reason);
                    break;
                }
                Ok(Message::Pong(_)) => {}
                Ok(unexpected) => log::error!("Unexpected message: {unexpected:#?}"),
                Err(err) => {
                    log::error!("Error reading from websocket: {err:#}");
                    break;
                }
            },
        }
    }

    shared.rx_state.set_session_open(false);
    dead.send_replace(true);
}

/// Restores authentication and event subscriptions on a freshly connected socket.
async fn handshake(
    shared: &Arc<Shared>,
    writer_tx: &Sender<Message>,
    dead: &watch::Receiver<bool>,
) -> Result<(), ConnectionEnd> {
    let mut dead = dead.clone();
    let _guard = shared.auth_lock.lock().await;

    if shared.authenticated.load(Ordering::SeqCst) {
        // A concurrent `auth_with_longlivedtoken` already authenticated this
        // session, so there is nothing left to restore.
        return resubscribe(shared, writer_tx, &dead).await;
    }

    let Some(token) = shared.token.lock().clone() else {
        // Nothing to authenticate with; subscriptions can only exist after a
        // successful authentication, so there is nothing to restore either.
        return Ok(());
    };

    let (tx, rx) = oneshot();
    if shared.rx_state.register(None, tx).is_err() {
        return Err(ConnectionEnd::Lost {
            was_connected: false,
        });
    }

    let cmd = Command::AuthInit(Auth {
        msg_type: "auth".to_owned(),
        access_token: token,
    });
    if writer_tx
        .send(cmd.into_tungstenite_message())
        .await
        .is_err()
    {
        return Err(ConnectionEnd::Lost {
            was_connected: false,
        });
    }

    let response = tokio::select! {
        res = timeout(shared.options.handshake_timeout, rx) => match res {
            Ok(Ok(response)) => response,
            Ok(Err(_dropped)) => {
                return Err(ConnectionEnd::Lost {
                    was_connected: false,
                });
            }
            Err(_elapsed) => {
                log::warn!(
                    "authentication timed out after {:?}",
                    shared.options.handshake_timeout
                );
                return Err(ConnectionEnd::Lost {
                    was_connected: false,
                });
            }
        },
        _ = wait_dead(&mut dead) => {
            return Err(ConnectionEnd::Lost {
                was_connected: false,
            });
        }
    };

    match response {
        Response::AuthOk(_) => {
            shared.authenticated.store(true, Ordering::SeqCst);
            log::debug!("re-authenticated with the cached access token");
        }
        Response::AuthInvalid(err) => {
            log::error!("the cached access token was rejected: {}", err.message);
            *shared.token.lock() = None;
            return Err(ConnectionEnd::Fatal("access token rejected".to_owned()));
        }
        unknown => {
            log::error!("unexpected response to auth: {unknown:?}");
            return Err(ConnectionEnd::Lost {
                was_connected: false,
            });
        }
    }

    resubscribe(shared, writer_tx, &dead).await
}

/// Re-creates every live subscription in the new session.
async fn resubscribe(
    shared: &Arc<Shared>,
    writer_tx: &Sender<Message>,
    dead: &watch::Receiver<bool>,
) -> Result<(), ConnectionEnd> {
    let mut dead = dead.clone();
    for (handle, event_type) in shared.rx_state.live_subscriptions() {
        let id = shared.next_seq();
        let (tx, rx) = oneshot();
        if shared.rx_state.register(Some(id), tx).is_err() {
            return Err(ConnectionEnd::Lost {
                was_connected: false,
            });
        }

        // Point the subscription at its new session id before asking for it, so
        // events that follow the acknowledgement are routed right away.
        shared.rx_state.retarget_subscription(handle, id);

        let cmd = Command::SubscribeEvent(Subscribe {
            id,
            msg_type: "subscribe_events".to_owned(),
            event_type: event_type.clone(),
        });
        if writer_tx
            .send(cmd.into_tungstenite_message())
            .await
            .is_err()
        {
            return Err(ConnectionEnd::Lost {
                was_connected: false,
            });
        }

        let response = tokio::select! {
            res = timeout(shared.options.handshake_timeout, rx) => match res {
                Ok(Ok(response)) => response,
                Ok(Err(_dropped)) => {
                    return Err(ConnectionEnd::Lost {
                        was_connected: false,
                    });
                }
                Err(_elapsed) => {
                    log::warn!("re-subscribing to '{event_type}' timed out");
                    return Err(ConnectionEnd::Lost {
                        was_connected: false,
                    });
                }
            },
            _ = wait_dead(&mut dead) => {
                return Err(ConnectionEnd::Lost {
                    was_connected: false,
                });
            }
        };

        match response {
            Response::Result(result) if result.is_ok() => {
                shared.rx_state.retarget_subscription(handle, result.id);
                log::debug!(
                    "restored subscription to '{event_type}' as session id {}",
                    result.id
                );
            }
            other => {
                log::error!("re-subscribing to '{event_type}' failed: {other:?}");
                shared.rx_state.remove_subscription(handle);
            }
        }
    }

    Ok(())
}

/// Pings the server and declares the connection dead when no pong comes back.
async fn heartbeat_task(
    shared: Arc<Shared>,
    writer_tx: Sender<Message>,
    dead: watch::Sender<bool>,
    every: Duration,
    pong_timeout: Duration,
) {
    let mut ticker = interval(every);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    // The first tick of an interval completes immediately.
    ticker.tick().await;

    loop {
        ticker.tick().await;

        if !shared.authenticated.load(Ordering::SeqCst) {
            // Home Assistant only answers pings of an authenticated session.
            continue;
        }

        let id = shared.next_seq();
        let (tx, rx) = oneshot();
        if shared.rx_state.register(Some(id), tx).is_err() {
            return;
        }

        let cmd = Command::Ping(Ask {
            id,
            msg_type: "ping".to_owned(),
        });
        if writer_tx
            .send(cmd.into_tungstenite_message())
            .await
            .is_err()
        {
            return;
        }

        match timeout(pong_timeout, rx).await {
            Ok(Ok(Response::Pong(_))) => {}
            Ok(Ok(other)) => log::warn!("unexpected response to heartbeat ping: {other:?}"),
            Ok(Err(_dropped)) => {
                log::warn!("heartbeat interrupted");
                return;
            }
            Err(_elapsed) => {
                log::warn!("no pong within {pong_timeout:?}, treating the connection as dead");
                dead.send_replace(true);
                return;
            }
        }
    }
}

/// Delay before reconnection attempt number `attempt` (1-based).
fn backoff_delay(options: &ReconnectOptions, attempt: u32) -> Duration {
    let exponent = attempt.saturating_sub(1).min(16);
    let delay = options
        .initial_delay
        .saturating_mul(1u32 << exponent)
        .min(options.max_delay);

    // Spread reconnections of many clients so they do not hit the server in
    // lockstep after a restart. No RNG dependency, subsecond nanos are enough.
    let jitter_ceiling = (delay.as_millis() / 4) as u64;
    if jitter_ceiling == 0 {
        return delay;
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.subsec_nanos() as u64)
        .unwrap_or_default();
    delay + Duration::from_millis(nanos % jitter_ceiling)
}

/// Runs one websocket session and reports how it ended.
///
/// `restore` asks the supervisor to re-authenticate and re-subscribe on this
/// socket. It is false for the first connection, whose session the caller sets
/// up itself through [`HassClient::auth_with_longlivedtoken`] — restoring it
/// here as well would race with that call into a double authentication.
async fn run_connection(
    shared: &Arc<Shared>,
    outbound: &mut Receiver<(u64, Message)>,
    ws: WsStream,
    shutdown: &mut OneShotReceiver<()>,
    restore: bool,
) -> ConnectionEnd {
    let (sink, source) = ws.split();
    let (writer_tx, writer_rx) = channel::<Message>(WRITE_QUEUE);
    let (dead_tx, mut dead_rx) = watch::channel(false);

    shared.authenticated.store(false, Ordering::SeqCst);
    let generation = shared.session_generation.fetch_add(1, Ordering::SeqCst) + 1;
    shared.rx_state.set_session_open(true);

    let writer = tokio::spawn(writer_task(
        sink,
        writer_rx,
        dead_tx.clone(),
        shared.options.write_timeout,
    ));
    let reader = tokio::spawn(reader_task(
        source,
        shared.clone(),
        writer_tx.clone(),
        dead_tx.clone(),
    ));

    // Restore the session before exposing it to callers. A restored session must
    // be authenticated: publishing `Connected` without a token would let
    // commands flow into a socket Home Assistant will not serve, and nothing
    // would ever detect it because the heartbeat only pings authenticated
    // sessions.
    let handshake_end = if restore {
        if shared.token.lock().is_some() {
            handshake(shared, &writer_tx, &dead_rx).await.err()
        } else {
            Some(ConnectionEnd::Fatal(
                "no cached access token to restore the session".to_owned(),
            ))
        }
    } else {
        None
    };

    let mut was_connected = false;
    let mut heartbeat = None;

    if handshake_end.is_none() {
        was_connected = true;
        shared.set_state(ConnectionState::Connected);
        heartbeat = shared.options.heartbeat_interval.map(|every| {
            tokio::spawn(heartbeat_task(
                shared.clone(),
                writer_tx.clone(),
                dead_tx.clone(),
                every,
                shared.options.heartbeat_timeout,
            ))
        });
    }

    let end = match handshake_end {
        Some(end) => end,
        None => loop {
            tokio::select! {
                _ = wait_dead(&mut dead_rx) => {
                    break ConnectionEnd::Lost { was_connected };
                }
                _ = &mut *shutdown => {
                    break ConnectionEnd::Shutdown;
                }
                message = outbound.recv() => match message {
                    Some((frame_generation, message)) => {
                        if frame_generation != generation {
                            // Queued for a session that died before it was sent;
                            // its caller has already been told it failed.
                            log::debug!("dropping a command queued for a previous session");
                        } else if writer_tx.send(message).await.is_err() {
                            break ConnectionEnd::Lost { was_connected };
                        }
                    }
                    None => break ConnectionEnd::Shutdown,
                },
            }
        },
    };

    writer.abort();
    reader.abort();
    if let Some(task) = heartbeat {
        task.abort();
    }
    let _ = writer.await;
    let _ = reader.await;

    end
}

/// Keeps the connection alive: reconnects with a backoff and restores the
/// session every time the socket is lost.
async fn supervise(
    shared: Arc<Shared>,
    mut outbound: Receiver<(u64, Message)>,
    initial: WsStream,
    mut shutdown: OneShotReceiver<()>,
) {
    let mut connection = Some(initial);
    let mut attempt: u32 = 0;

    loop {
        let (ws, restore) = match connection.take() {
            Some(ws) => (ws, false),
            None => {
                if !shared.options.enabled {
                    log::info!("connection lost, automatic reconnection is disabled");
                    shared.rx_state.close_subscriptions();
                    shared.set_state(ConnectionState::Closed);
                    return;
                }

                attempt += 1;
                if let Some(max) = shared.options.max_attempts {
                    if attempt > max {
                        log::error!("giving up after {max} failed reconnection attempts");
                        shared.rx_state.close_subscriptions();
                        shared.set_state(ConnectionState::Closed);
                        return;
                    }
                }

                let delay = backoff_delay(&shared.options, attempt);
                log::warn!("reconnecting in {delay:?} (attempt {attempt})");
                shared.set_state(ConnectionState::Disconnected);

                tokio::select! {
                    _ = sleep(delay) => {}
                    _ = &mut shutdown => {
                        shared.set_state(ConnectionState::Closed);
                        return;
                    }
                }

                shared.set_state(ConnectionState::Connecting);
                match connect_async(&shared.url).await {
                    Ok((ws, _)) => (ws, true),
                    Err(err) => {
                        log::warn!("reconnect attempt {attempt} failed: {err:#}");
                        continue;
                    }
                }
            }
        };

        let end = run_connection(&shared, &mut outbound, ws, &mut shutdown, restore).await;

        // The session is gone: refuse new commands, resolve every waiter and
        // drop the frames that were queued for the dead socket.
        shared.authenticated.store(false, Ordering::SeqCst);
        shared.rx_state.fail_pending(CLOSE_REASON_LOST);
        shared.rx_state.remove_closed_subscriptions();
        while outbound.try_recv().is_ok() {}
        shared.set_state(ConnectionState::Disconnected);

        match end {
            ConnectionEnd::Shutdown => {
                shared.rx_state.close_subscriptions();
                shared.set_state(ConnectionState::Closed);
                return;
            }
            ConnectionEnd::Fatal(reason) => {
                log::error!("the session cannot be restored: {reason}");
                shared.rx_state.close_subscriptions();
                shared.set_state(ConnectionState::Closed);
                return;
            }
            ConnectionEnd::Lost { was_connected } => {
                if was_connected {
                    attempt = 0;
                }
                connection = None;
            }
        }
    }
}

impl HassClient {
    /// Connect to the Home Assistant websocket API.
    ///
    /// The connection is supervised: when it is lost the client reconnects and
    /// restores authentication and subscriptions on its own.
    pub async fn new(url: &str) -> HassResult<Self> {
        Self::new_with_options(url, ReconnectOptions::default()).await
    }

    /// Connect with an explicit reconnection policy.
    pub async fn new_with_options(url: &str, options: ReconnectOptions) -> HassResult<Self> {
        let (ws, _) = connect_async(url).await?;

        let (message_tx, message_rx) = channel(OUTBOUND_QUEUE);
        let (state_tx, state_rx) = watch::channel(ConnectionState::Connecting);
        let (shutdown_tx, shutdown_rx) = oneshot();

        let shared = Arc::new(Shared {
            url: url.to_owned(),
            options,
            token: Mutex::new(None),
            auth_lock: AsyncMutex::new(()),
            authenticated: AtomicBool::new(false),
            session_generation: AtomicU64::new(0),
            last_sequence: AtomicU64::new(1),
            rx_state: ReceiverState::default(),
            message_tx,
            state_tx,
        });

        tokio::spawn(supervise(shared.clone(), message_rx, ws, shutdown_rx));

        Ok(Self {
            shared,
            state_rx,
            _shutdown: ShutdownGuard(Some(shutdown_tx)),
        })
    }

    /// The current state of the underlying connection.
    pub fn connection_state(&self) -> ConnectionState {
        *self.state_rx.borrow()
    }

    /// Watch the connection state, e.g.
    ///
    /// ```no_run
    /// # use hass_rs::client::{ConnectionState, HassClient};
    /// # async fn example(client: &HassClient) -> Result<(), Box<dyn std::error::Error>> {
    /// let mut state = client.watch_connection_state();
    /// while state.changed().await.is_ok() {
    ///     println!("connection state: {:?}", *state.borrow_and_update());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn watch_connection_state(&self) -> watch::Receiver<ConnectionState> {
        self.state_rx.clone()
    }

    /// authenticate the session using a long-lived access token
    ///
    /// When a client connects to the server, the server sends out auth_required.
    /// The first message from the client should be an auth message. You can authorize with an access token.
    /// If the client supplies valid authentication, the authentication phase will complete by the server sending the auth_ok message.
    /// If the data is incorrect, the server will reply with auth_invalid message and disconnect the session.
    ///
    /// The token is cached, so the client can re-authenticate by itself after a
    /// reconnection. If the session is lost while this call runs, it waits for
    /// the supervisor to restore it and tries again, up to
    /// [`ReconnectOptions::handshake_timeout`].
    pub async fn auth_with_longlivedtoken(&self, token: &str) -> HassResult<()> {
        // Cache the token first: the supervisor restores the session with it
        // after a reconnection.
        *self.shared.token.lock() = Some(token.to_owned());

        let authenticate = || {
            Command::AuthInit(Auth {
                msg_type: "auth".to_owned(),
                access_token: token.to_owned(),
            })
        };
        let deadline = Instant::now() + self.shared.options.handshake_timeout;

        loop {
            if self.shared.authenticated.load(Ordering::SeqCst) {
                // The supervisor restored the session for us.
                return Ok(());
            }

            match self.connection_state() {
                ConnectionState::Closed => return Err(HassError::ConnectionClosed),
                ConnectionState::Connected => {
                    let _guard = self.shared.auth_lock.lock().await;
                    if self.shared.authenticated.load(Ordering::SeqCst) {
                        return Ok(());
                    }

                    // Bound the exchange: a half-open socket would otherwise
                    // keep this call (and the auth lock) forever.
                    let remaining = deadline.saturating_duration_since(Instant::now());
                    let attempt = timeout(remaining, self.command(authenticate(), None)).await;

                    match attempt {
                        Ok(Ok(Response::AuthOk(_))) => {
                            self.shared.authenticated.store(true, Ordering::SeqCst);
                            return Ok(());
                        }
                        Ok(Ok(Response::AuthInvalid(err))) => {
                            // Do not retry a rejected token on every reconnect.
                            *self.shared.token.lock() = None;
                            return Err(HassError::AuthenticationFailed(err.message));
                        }
                        // The session died before it could authenticate: a close
                        // frame, a failed send, or the deadline. The supervisor
                        // brings up a fresh session and we retry against it.
                        Ok(Ok(Response::Close(_)))
                        | Ok(Err(HassError::ConnectionClosed))
                        | Ok(Err(HassError::SendError(_)))
                        | Err(_) => {}
                        Ok(Ok(unknown)) => return Err(HassError::UnknownPayloadReceived(unknown)),
                        Ok(Err(err)) => return Err(err),
                    }
                }
                ConnectionState::Connecting | ConnectionState::Disconnected => {}
            }

            if Instant::now() >= deadline {
                return Err(HassError::ConnectionClosed);
            }
            sleep(Duration::from_millis(25)).await;
        }
    }

    /// The API supports receiving a ping from the client and returning a pong.
    /// This serves as a heartbeat to ensure the connection is still alive.
    ///
    /// [`ReconnectOptions::heartbeat_interval`] sends these automatically.
    pub async fn ping(&self) -> HassResult<()> {
        let id = self.next_seq();

        let ping_req = Command::Ping(Ask {
            id,
            msg_type: "ping".to_owned(),
        });

        let response = self.command(ping_req, Some(id)).await?;

        match response {
            Response::Pong(_v) => Ok(()),
            Response::Result(err) => Err(HassError::ResponseError(err)),
            unknown => Err(HassError::UnknownPayloadReceived(unknown)),
        }
    }

    /// This will get the current config of the Home Assistant.
    ///
    /// The server will respond with a result message containing the config.
    pub async fn get_config(&self) -> HassResult<HassConfig> {
        let id = self.next_seq();

        let config_req = Command::GetConfig(Ask {
            id,
            msg_type: "get_config".to_owned(),
        });
        let response = self.command(config_req, Some(id)).await?;

        match response {
            Response::Result(data) => {
                let value = data.result()?;
                let config: HassConfig = serde_json::from_value(value)?;
                Ok(config)
            }
            unknown => Err(HassError::UnknownPayloadReceived(unknown)),
        }
    }

    /// This will get all the current states from Home Assistant.
    ///
    /// The server will respond with a result message containing the states.
    pub async fn get_states(&self) -> HassResult<Vec<HassEntity>> {
        let id = self.next_seq();

        let states_req = Command::GetStates(Ask {
            id,
            msg_type: "get_states".to_owned(),
        });
        let response = self.command(states_req, Some(id)).await?;

        match response {
            Response::Result(data) => {
                let value = data.result()?;
                let states: Vec<HassEntity> = serde_json::from_value(value)?;
                Ok(states)
            }
            unknown => Err(HassError::UnknownPayloadReceived(unknown)),
        }
    }

    /// This will get all the services from Home Assistant.
    ///
    /// The server will respond with a result message containing the services.
    pub async fn get_services(&self) -> HassResult<HassServices> {
        let id = self.next_seq();
        let services_req = Command::GetServices(Ask {
            id,
            msg_type: "get_services".to_owned(),
        });
        let response = self.command(services_req, Some(id)).await?;

        match response {
            Response::Result(data) => {
                let value = data.result()?;
                let services: HassServices = serde_json::from_value(value)?;
                Ok(services)
            }
            unknown => Err(HassError::UnknownPayloadReceived(unknown)),
        }
    }

    /// This will get all the registered panels from Home Assistant.
    ///
    /// The server will respond with a result message containing the current registered panels.
    pub async fn get_panels(&self) -> HassResult<HassPanels> {
        let id = self.next_seq();

        let services_req = Command::GetPanels(Ask {
            id,
            msg_type: "get_panels".to_owned(),
        });
        let response = self.command(services_req, Some(id)).await?;

        match response {
            Response::Result(data) => {
                let value = data.result()?;
                let services: HassPanels = serde_json::from_value(value)?;
                Ok(services)
            }
            unknown => Err(HassError::UnknownPayloadReceived(unknown)),
        }
    }

    /// This will get the current area registry list from Home Assistant.
    ///
    /// The server will respond with a result message containing the area registry list.
    pub async fn get_area_registry_list(&self) -> HassResult<Vec<HassRegistryArea>> {
        let id = self.next_seq();

        let area_req = Command::GetAreaRegistryList(Ask {
            id,
            msg_type: "config/area_registry/list".to_owned(),
        });
        let response = self.command(area_req, Some(id)).await?;

        match response {
            Response::Result(data) => {
                let value = data.result()?;
                let areas: Vec<HassRegistryArea> = serde_json::from_value(value)?;
                Ok(areas)
            }
            unknown => Err(HassError::UnknownPayloadReceived(unknown)),
        }
    }

    /// This will get the current device registry list from Home Assistant.
    ///
    /// The server will respond with a result message containing the device registry list.
    pub async fn get_device_registry_list(&self) -> HassResult<Vec<HassRegistryDevice>> {
        let id = self.next_seq();

        let device_req = Command::GetDeviceRegistryList(Ask {
            id,
            msg_type: "config/device_registry/list".to_owned(),
        });
        let response = self.command(device_req, Some(id)).await?;

        match response {
            Response::Result(data) => {
                let value = data.result()?;
                let devices: Vec<HassRegistryDevice> = serde_json::from_value(value)?;
                Ok(devices)
            }
            unknown => Err(HassError::UnknownPayloadReceived(unknown)),
        }
    }

    /// This will get the current entity registry list from Home Assistant.
    ///
    /// The server will respond with a result message containing the entity registry list.
    pub async fn get_entity_registry_list(&self) -> HassResult<Vec<HassRegistryEntity>> {
        let id = self.next_seq();

        let entity_req = Command::GetEntityRegistryList(Ask {
            id,
            msg_type: "config/entity_registry/list".to_owned(),
        });
        let response = self.command(entity_req, Some(id)).await?;

        match response {
            Response::Result(data) => {
                let value = data.result()?;
                let entities: Vec<HassRegistryEntity> = serde_json::from_value(value)?;
                Ok(entities)
            }
            unknown => Err(HassError::UnknownPayloadReceived(unknown)),
        }
    }

    ///This will call a service in Home Assistant. Right now there is no return value.
    ///The client can listen to state_changed events if it is interested in changed entities as a result of a service call.
    ///
    /// The server will indicate with a message indicating that the service is done executing.
    /// <https://developers.home-assistant.io/docs/api/websocket#calling-a-service>
    /// additional info : <https://developers.home-assistant.io/docs/api/rest> ==> Post `/api/services/<domain>/<service>`
    pub async fn call_service(
        &self,
        domain: String,
        service: String,
        service_data: Option<Value>,
    ) -> HassResult<()> {
        let id = self.next_seq();

        let services_req = Command::CallService(CallService {
            id,
            msg_type: "call_service".to_owned(),
            domain,
            service,
            service_data,
        });
        let response = self.command(services_req, Some(id)).await?;

        match response {
            Response::Result(data) => {
                data.result()?;
                Ok(())
            }
            // Usually happens when service triggers Home Assistant reboot, e.g. hassio.host_reboot or update.home_assistant_core_update.
            Response::Close(_reason) => Err(HassError::ConnectionClosed),
            unknown => Err(HassError::UnknownPayloadReceived(unknown)),
        }
    }

    /// The command subscribe_event will subscribe your client to the event bus.
    ///
    /// Returns a channel that will receive the subscription messages. The
    /// receiver keeps working across reconnections: the subscription is
    /// restored automatically and its events keep arriving on this channel.
    ///
    /// The channel buffers 20 events. A consumer that falls further behind
    /// loses events (a warning is logged) instead of stalling the connection for
    /// every other command and subscription.
    pub async fn subscribe_event(&self, event_name: &str) -> HassResult<Receiver<WSEvent>> {
        let id = self.next_seq();

        // Home Assistant reuses the id of the subscribe command as the id of the
        // subscription, so the subscription can be registered *before* the
        // command is sent. Events that arrive right after the acknowledgement
        // then already have a route, and a reconnection that happens while this
        // call is in flight finds the subscription to restore it.
        let (tx, rx) = channel(EVENT_BUFFER);
        self.shared.rx_state.insert_subscription(
            id,
            Subscription {
                event_type: event_name.to_owned(),
                session_id: id,
                sender: tx,
            },
        );

        let cmd = Command::SubscribeEvent(Subscribe {
            id,
            msg_type: "subscribe_events".to_owned(),
            event_type: event_name.to_owned(),
        });

        match self.command(cmd, Some(id)).await {
            Ok(Response::Result(v)) if v.is_ok() => {
                // Defensive: should the server ever answer with another id, route
                // the events by the id it actually uses.
                self.shared.rx_state.retarget_subscription(id, v.id);
                Ok(rx)
            }
            Ok(Response::Result(v)) => {
                self.shared.rx_state.remove_subscription(id);
                Err(HassError::ResponseError(v))
            }
            Ok(unknown) => {
                self.shared.rx_state.remove_subscription(id);
                Err(HassError::UnknownPayloadReceived(unknown))
            }
            Err(err) => {
                self.shared.rx_state.remove_subscription(id);
                Err(err)
            }
        }
    }

    /// Lists pending issues of the Home Assistant instance.
    pub async fn list_issues(&self) -> HassResult<HassIssues> {
        let id = self.next_seq();

        let cmd = Command::ListRepairs(Ask {
            id,
            msg_type: "repairs/list_issues".to_owned(),
        });

        let response = self.command(cmd, Some(id)).await?;

        match response {
            Response::Result(data) => {
                let value = data.result()?;
                let issues: HassIssues = serde_json::from_value(value)?;
                Ok(issues)
            }
            unknown => Err(HassError::UnknownPayloadReceived(unknown)),
        }
    }

    /// send commands and receive responses from the gateway
    pub(crate) async fn command(&self, cmd: Command, id: Option<u64>) -> HassResult<Response> {
        let cmd_tungstenite = cmd.into_tungstenite_message();

        let (tx, rx) = oneshot();

        // Refuse fast while the session is down or still being restored:
        // a request registered now would wait for a response that cannot come.
        if !matches!(*self.state_rx.borrow(), ConnectionState::Connected) {
            return Err(HassError::ConnectionClosed);
        }

        // Stamp the frame with the session it belongs to, so the supervisor
        // drops it instead of replaying it on the next session.
        let generation = self.shared.session_generation.load(Ordering::SeqCst);

        if self.shared.rx_state.register(id, tx).is_err() {
            return Err(HassError::ConnectionClosed);
        }

        if self
            .shared
            .message_tx
            .send((generation, cmd_tungstenite))
            .await
            .is_err()
        {
            return Err(HassError::SendError(
                "the connection supervisor stopped".to_owned(),
            ));
        }

        rx.await.map_err(|_| HassError::ConnectionClosed)
    }

    /// get message sequence required by the Websocket server
    fn next_seq(&self) -> u64 {
        self.shared.next_seq()
    }

    /// This will unsubscribe from an event subscription.
    pub async fn unsubscribe_event(&self, subscription_id: u64) -> HassResult<()> {
        let id = self.next_seq();

        // The subscription may have been restored under a new id after a
        // reconnection; `subscription_id` stays the handle the caller knows.
        let session_id = self
            .shared
            .rx_state
            .session_id_of(subscription_id)
            .unwrap_or(subscription_id);

        let cmd = Command::Unsubscribe(Unsubscribe {
            id,
            msg_type: "unsubscribe_events".to_owned(),
            subscription: session_id,
        });

        let response = self.command(cmd, Some(id)).await?;

        match response {
            Response::Result(v) if v.is_ok() => {
                self.shared.rx_state.remove_subscription(subscription_id);
                Ok(())
            }
            Response::Result(v) => Err(HassError::ResponseError(v)),
            unknown => Err(HassError::UnknownPayloadReceived(unknown)),
        }
    }
}

/// convenient function that validates if the message received is an Event
/// the Events should be processed by used in a separate async task
fn check_if_event(result: Result<Message, Error>) -> Result<WSEvent, Result<Message, Error>> {
    match result {
        Ok(Message::Text(data)) => {
            let payload: Result<Response, HassError> =
                serde_json::from_str(data.as_str()).map_err(HassError::from);

            if let Ok(Response::Event(event)) = payload {
                Ok(*event)
            } else {
                Err(Ok(Message::Text(data)))
            }
        }
        result => Err(result),
    }
}
