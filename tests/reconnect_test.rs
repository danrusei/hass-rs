//! Tests for the automatic reconnection of the websocket connection.

use futures_util::{SinkExt, StreamExt};
use hass_rs::client::{ConnectionState, HassClient, ReconnectOptions};
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio::time::{sleep, timeout, Instant};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{accept_async, WebSocketStream};

const AUTH_REQUIRED: &str = r#"{"type":"auth_required","ha_version":"2021.3.0"}"#;
const AUTH_OK: &str = r#"{"type":"auth_ok","ha_version":"2021.3.0"}"#;

const STATES: &str = r#"[{
    "entity_id": "light.kitchen",
    "state": "on",
    "attributes": {},
    "last_changed": "2024-02-15T11:13:02.291378+00:00",
    "last_updated": "2024-02-15T11:13:02.291378+00:00",
    "context": null
}]"#;

/// Reconnection tuned for a test: fast retries, no heartbeat unless asked for.
fn fast_options() -> ReconnectOptions {
    ReconnectOptions {
        initial_delay: Duration::from_millis(50),
        max_delay: Duration::from_millis(200),
        heartbeat_interval: None,
        handshake_timeout: Duration::from_secs(2),
        write_timeout: Duration::from_secs(2),
        ..ReconnectOptions::default()
    }
}

async fn bind() -> (TcpListener, String) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    (listener, url)
}

/// Server side of the auth handshake, returns the raw auth message.
async fn handshake(ws: &mut WebSocketStream<TcpStream>) -> String {
    ws.send(Message::Text(AUTH_REQUIRED.into())).await.unwrap();
    let auth = ws
        .next()
        .await
        .unwrap()
        .unwrap()
        .to_text()
        .unwrap()
        .to_owned();
    ws.send(Message::Text(AUTH_OK.into())).await.unwrap();
    auth
}

fn id_of(text: &str) -> u64 {
    serde_json::from_str::<serde_json::Value>(text).unwrap()["id"]
        .as_u64()
        .unwrap_or_else(|| panic!("no id in {text}"))
}

fn event_json(id: u64) -> String {
    format!(
        r#"{{"id":{id},"type":"event","event":{{
            "event_type":"state_changed",
            "data":{{"entity_id":"light.kitchen","new_state":null,"old_state":null}},
            "origin":"LOCAL",
            "time_fired":"2024-02-15T11:13:02.291378+00:00",
            "context":{{"id":"01HPRMZAWNXKVVPSP11QFJ53HB","parent_id":null,"user_id":null}}
        }}}}"#
    )
}

async fn ack(ws: &mut WebSocketStream<TcpStream>, id: u64) {
    ws.send(Message::Text(
        format!(r#"{{"id":{id},"type":"result","success":true,"result":null}}"#).into(),
    ))
    .await
    .unwrap();
}

/// The client must notice the lost socket, fail the request that was in flight
/// instead of hanging, reconnect and re-authenticate with the cached token.
#[tokio::test]
async fn reconnects_and_reauthenticates_after_abrupt_disconnect() {
    let (listener, url) = bind().await;

    let server = tokio::spawn(async move {
        // First session: answer auth, then die without a close frame while a
        // request is in flight.
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = accept_async(stream).await.unwrap();
        let auth = handshake(&mut ws).await;
        assert!(auth.contains(r#""access_token":"test_token""#), "{auth}");

        let msg = ws.next().await.unwrap().unwrap();
        assert!(msg.to_text().unwrap().contains(r#""type":"get_states""#));
        drop(ws);

        // Second session: the client has to authenticate by itself.
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = accept_async(stream).await.unwrap();
        let auth = handshake(&mut ws).await;
        assert!(
            auth.contains(r#""access_token":"test_token""#),
            "client did not re-authenticate: {auth}"
        );

        while let Some(Ok(msg)) = ws.next().await {
            let text = msg.to_text().unwrap_or_default().to_owned();
            if text.contains(r#""type":"get_states""#) {
                let id = id_of(&text);
                ws.send(Message::Text(
                    format!(r#"{{"id":{id},"type":"result","success":true,"result":{STATES}}}"#)
                        .into(),
                ))
                .await
                .unwrap();
            }
        }
    });

    let client = HassClient::new_with_options(&url, fast_options())
        .await
        .unwrap();
    client.auth_with_longlivedtoken("test_token").await.unwrap();

    // The request that was in flight when the socket vanished must fail...
    let failed = timeout(Duration::from_secs(3), client.get_states())
        .await
        .expect("the in-flight request hung instead of failing");
    assert!(
        failed.is_err(),
        "expected the in-flight request to fail, got {failed:?}"
    );

    // ...and the supervisor has to recover the connection on its own.
    let mut recovered = None;
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Ok(states) = client.get_states().await {
            recovered = Some(states);
            break;
        }
        sleep(Duration::from_millis(50)).await;
    }

    let states = recovered.expect("the client did not recover from the lost connection");
    assert_eq!(states.len(), 1);
    assert_eq!(states[0].entity_id, "light.kitchen");
    assert_eq!(client.connection_state(), ConnectionState::Connected);

    drop(client);
    let _ = timeout(Duration::from_secs(5), server).await;
}

/// Event subscriptions must survive a reconnection: the receiver the caller
/// already holds keeps receiving events from the restored subscription.
#[tokio::test]
async fn restores_event_subscriptions_after_reconnect() {
    let (listener, url) = bind().await;
    let (drop_tx, drop_rx) = oneshot::channel::<()>();

    let server = tokio::spawn(async move {
        // First session.
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = accept_async(stream).await.unwrap();
        handshake(&mut ws).await;

        let msg = ws.next().await.unwrap().unwrap();
        let text = msg.to_text().unwrap().to_owned();
        assert!(text.contains(r#""type":"subscribe_events""#), "{text}");
        let first_session_id = id_of(&text);
        ack(&mut ws, first_session_id).await;

        // Give the client time to register the subscription, then prove it is
        // live by sending an event.
        sleep(Duration::from_millis(100)).await;
        ws.send(Message::Text(event_json(first_session_id).into()))
            .await
            .unwrap();

        drop_rx.await.unwrap();
        drop(ws);

        // Second session: the subscription must be restored with a new id.
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = accept_async(stream).await.unwrap();
        handshake(&mut ws).await;

        let msg = ws.next().await.unwrap().unwrap();
        let text = msg.to_text().unwrap().to_owned();
        assert!(
            text.contains(r#""type":"subscribe_events""#),
            "the subscription was not restored: {text}"
        );
        let second_session_id = id_of(&text);
        assert_ne!(
            second_session_id, first_session_id,
            "the restored subscription must use an id of the new session"
        );
        ack(&mut ws, second_session_id).await;

        sleep(Duration::from_millis(100)).await;
        ws.send(Message::Text(event_json(second_session_id).into()))
            .await
            .unwrap();

        while let Some(Ok(_)) = ws.next().await {}
    });

    let client = HassClient::new_with_options(&url, fast_options())
        .await
        .unwrap();
    client.auth_with_longlivedtoken("test_token").await.unwrap();

    let mut events = client.subscribe_event("state_changed").await.unwrap();
    let first = timeout(Duration::from_secs(3), events.recv())
        .await
        .expect("no event on the first connection")
        .expect("subscription receiver closed");
    assert_eq!(first.event.event_type, "state_changed");

    // Now kill the socket and expect the same receiver to keep working.
    drop_tx.send(()).unwrap();

    let second = timeout(Duration::from_secs(5), events.recv())
        .await
        .expect("no event after the reconnection on the same receiver")
        .expect("subscription receiver closed");
    assert_eq!(second.event.event_type, "state_changed");
    assert_ne!(
        second.id, first.id,
        "expected an event of the restored subscription"
    );

    drop(client);
    let _ = timeout(Duration::from_secs(5), server).await;
}

/// A connection that stays open but never answers must be detected by the
/// heartbeat and replaced.
#[tokio::test]
async fn heartbeat_recovers_from_unresponsive_connection() {
    let (listener, url) = bind().await;

    let server = tokio::spawn(async move {
        // First session: authenticates, then goes silent (socket stays open).
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = accept_async(stream).await.unwrap();
        handshake(&mut ws).await;
        while let Some(Ok(_)) = ws.next().await {}

        // Second session: answers again.
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = accept_async(stream).await.unwrap();
        handshake(&mut ws).await;
        while let Some(Ok(msg)) = ws.next().await {
            let text = msg.to_text().unwrap_or_default().to_owned();
            if text.contains(r#""type":"get_states""#) {
                let id = id_of(&text);
                ws.send(Message::Text(
                    format!(r#"{{"id":{id},"type":"result","success":true,"result":{STATES}}}"#)
                        .into(),
                ))
                .await
                .unwrap();
            }
        }
    });

    let options = ReconnectOptions {
        initial_delay: Duration::from_millis(50),
        max_delay: Duration::from_millis(200),
        heartbeat_interval: Some(Duration::from_millis(100)),
        heartbeat_timeout: Duration::from_millis(100),
        handshake_timeout: Duration::from_secs(2),
        write_timeout: Duration::from_secs(2),
        ..ReconnectOptions::default()
    };

    let client = HassClient::new_with_options(&url, options).await.unwrap();
    client.auth_with_longlivedtoken("test_token").await.unwrap();

    // Nothing is answered on the first session, so this only succeeds if the
    // heartbeat declared that session dead and the client reconnected.
    let mut recovered = None;
    let deadline = Instant::now() + Duration::from_secs(6);
    while Instant::now() < deadline {
        if let Ok(states) = client.get_states().await {
            recovered = Some(states);
            break;
        }
        sleep(Duration::from_millis(50)).await;
    }

    let states = recovered.expect("the heartbeat did not recover the connection");
    assert_eq!(states.len(), 1);
    assert_eq!(client.connection_state(), ConnectionState::Connected);

    drop(client);
    let _ = timeout(Duration::from_secs(5), server).await;
}

/// With reconnection disabled the client must end the session instead of
/// hanging: the state becomes `Closed` and the subscription receivers end.
#[tokio::test]
async fn disabled_reconnection_closes_the_session() {
    let (listener, url) = bind().await;

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = accept_async(stream).await.unwrap();
        handshake(&mut ws).await;

        let msg = ws.next().await.unwrap().unwrap();
        let text = msg.to_text().unwrap().to_owned();
        let id = id_of(&text);
        ack(&mut ws, id).await;

        sleep(Duration::from_millis(100)).await;
        ws.send(Message::Text(event_json(id).into())).await.unwrap();

        sleep(Duration::from_millis(100)).await;
        drop(ws);
    });

    let client = HassClient::new_with_options(&url, ReconnectOptions::disabled())
        .await
        .unwrap();
    client.auth_with_longlivedtoken("test_token").await.unwrap();

    let mut events = client.subscribe_event("state_changed").await.unwrap();
    assert!(timeout(Duration::from_secs(3), events.recv())
        .await
        .expect("no event before the connection was lost")
        .is_some());

    let closed = timeout(Duration::from_secs(5), async {
        while client.connection_state() != ConnectionState::Closed {
            sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
    assert!(
        closed.is_ok(),
        "the client stayed in state {:?} instead of closing",
        client.connection_state()
    );

    assert!(
        timeout(Duration::from_secs(2), events.recv())
            .await
            .expect("the subscription receiver was not closed")
            .is_none(),
        "expected the subscription channel to end"
    );

    let _ = timeout(Duration::from_secs(5), server).await;
}

/// When the server stays unreachable the client must give up after
/// `max_attempts`, end the session and refuse commands instead of retrying
/// forever or hanging.
#[tokio::test]
async fn gives_up_after_max_attempts() {
    let (listener, url) = bind().await;

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = accept_async(stream).await.unwrap();
        handshake(&mut ws).await;
        // Drop the socket *and* the listener: nothing accepts the next dial.
        drop(ws);
    });

    let options = ReconnectOptions {
        initial_delay: Duration::from_millis(50),
        max_delay: Duration::from_millis(100),
        max_attempts: Some(1),
        heartbeat_interval: None,
        handshake_timeout: Duration::from_secs(2),
        write_timeout: Duration::from_secs(2),
        ..ReconnectOptions::default()
    };

    let client = HassClient::new_with_options(&url, options).await.unwrap();
    client.auth_with_longlivedtoken("test_token").await.unwrap();

    let closed = timeout(Duration::from_secs(5), async {
        while client.connection_state() != ConnectionState::Closed {
            sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
    assert!(
        closed.is_ok(),
        "the client kept retrying instead of giving up, state {:?}",
        client.connection_state()
    );

    // Commands must fail fast, not hang, once the client gave up.
    let refused = timeout(Duration::from_secs(2), client.get_states())
        .await
        .expect("a command hung after the client gave up");
    assert!(
        refused.is_err(),
        "expected an error after giving up, got {refused:?}"
    );

    let _ = timeout(Duration::from_secs(5), server).await;
}

/// While the server is unreachable commands fail fast, and the client keeps
/// retrying until it is back, then re-authenticates by itself.
#[tokio::test]
async fn reconnects_when_the_server_comes_back() {
    let (listener, url) = bind().await;
    let addr = listener.local_addr().unwrap();

    let first = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = accept_async(stream).await.unwrap();
        handshake(&mut ws).await;
        drop(ws);
        // The listener is dropped here too, so the port becomes free again.
    });

    let client = HassClient::new_with_options(&url, fast_options())
        .await
        .unwrap();
    client.auth_with_longlivedtoken("test_token").await.unwrap();
    first.await.unwrap();

    // The server is gone: a command must fail instead of hanging.
    let down = timeout(Duration::from_secs(2), client.get_states())
        .await
        .expect("a command hung while the server was unreachable");
    assert!(
        down.is_err(),
        "expected an error while the server is down, got {down:?}"
    );

    // The server comes back on the same port.
    let listener = TcpListener::bind(addr)
        .await
        .expect("could not re-bind the port");
    let second = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = accept_async(stream).await.unwrap();
        let auth = handshake(&mut ws).await;
        assert!(
            auth.contains(r#""access_token":"test_token""#),
            "client did not re-authenticate: {auth}"
        );

        while let Some(Ok(msg)) = ws.next().await {
            let text = msg.to_text().unwrap_or_default().to_owned();
            if text.contains(r#""type":"get_states""#) {
                let id = id_of(&text);
                ws.send(Message::Text(
                    format!(r#"{{"id":{id},"type":"result","success":true,"result":{STATES}}}"#)
                        .into(),
                ))
                .await
                .unwrap();
            }
        }
    });

    let mut recovered = None;
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Ok(states) = client.get_states().await {
            recovered = Some(states);
            break;
        }
        sleep(Duration::from_millis(50)).await;
    }

    let states = recovered.expect("the client did not reconnect once the server was back");
    assert_eq!(states.len(), 1);
    assert_eq!(states[0].entity_id, "light.kitchen");

    drop(client);
    let _ = timeout(Duration::from_secs(5), second).await;
}

/// Losing the connection while the initial auth is in flight must not fail the
/// call: the supervisor restores the session with the cached token and the call
/// retries against it.
#[tokio::test]
async fn auth_survives_a_connection_lost_during_the_handshake() {
    let (listener, url) = bind().await;

    let server = tokio::spawn(async move {
        // First session: swallows the auth message and dies without answering.
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = accept_async(stream).await.unwrap();
        ws.send(Message::Text(AUTH_REQUIRED.into())).await.unwrap();
        let _ = ws.next().await.unwrap().unwrap();
        drop(ws);

        // Second session: completes the handshake and serves commands.
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = accept_async(stream).await.unwrap();
        handshake(&mut ws).await;

        while let Some(Ok(msg)) = ws.next().await {
            let text = msg.to_text().unwrap_or_default().to_owned();
            if text.contains(r#""type":"get_states""#) {
                let id = id_of(&text);
                ws.send(Message::Text(
                    format!(r#"{{"id":{id},"type":"result","success":true,"result":{STATES}}}"#)
                        .into(),
                ))
                .await
                .unwrap();
            }
        }
    });

    let client = HassClient::new_with_options(&url, fast_options())
        .await
        .unwrap();

    let auth = timeout(
        Duration::from_secs(5),
        client.auth_with_longlivedtoken("test_token"),
    )
    .await
    .expect("authentication hung instead of recovering");
    assert!(
        auth.is_ok(),
        "authentication failed instead of retrying on the restored session: {auth:?}"
    );

    let mut recovered = None;
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Ok(states) = client.get_states().await {
            recovered = Some(states);
            break;
        }
        sleep(Duration::from_millis(50)).await;
    }
    let states = recovered.expect("the session was not usable after the retried auth");
    assert_eq!(states[0].entity_id, "light.kitchen");

    drop(client);
    let _ = timeout(Duration::from_secs(5), server).await;
}

/// A client that never supplied a token has no session to restore: it ends up
/// `Closed` instead of pretending to be connected or reconnecting forever.
#[tokio::test]
async fn unauthenticated_client_closes_when_the_connection_drops() {
    let (listener, url) = bind().await;

    let server = tokio::spawn(async move {
        // First session: accepted, then dropped.
        let (stream, _) = listener.accept().await.unwrap();
        drop(accept_async(stream).await.unwrap());

        // Second session: establishes without a token. The client cannot restore
        // an unauthenticated session, so it must close instead of exposing this
        // socket as usable.
        let (stream, _) = listener.accept().await.unwrap();
        let ws = accept_async(stream).await.unwrap();
        sleep(Duration::from_secs(2)).await;
        drop(ws);
    });

    let client = HassClient::new_with_options(&url, fast_options())
        .await
        .unwrap();

    let closed = timeout(Duration::from_secs(5), async {
        while client.connection_state() != ConnectionState::Closed {
            sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
    assert!(
        closed.is_ok(),
        "state stuck at {:?} instead of closing",
        client.connection_state()
    );

    let refused = timeout(Duration::from_secs(2), client.get_states())
        .await
        .expect("a command hung after the client closed");
    assert!(refused.is_err(), "expected an error, got {refused:?}");

    drop(client);
    let _ = timeout(Duration::from_secs(5), server).await;
}

/// An event that follows the subscribe acknowledgement with no delay must still
/// be delivered: the subscription is registered before the command is sent, so
/// the acknowledgement cannot arrive before the route exists.
#[tokio::test]
async fn events_arriving_with_the_acknowledgement_are_delivered() {
    let (listener, url) = bind().await;

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = accept_async(stream).await.unwrap();
        handshake(&mut ws).await;

        let msg = ws.next().await.unwrap().unwrap();
        let id = id_of(msg.to_text().unwrap());
        ack(&mut ws, id).await;
        // No sleep: the event races the acknowledgement on purpose.
        ws.send(Message::Text(event_json(id).into())).await.unwrap();

        while let Some(Ok(_)) = ws.next().await {}
    });

    let client = HassClient::new_with_options(&url, fast_options())
        .await
        .unwrap();
    client.auth_with_longlivedtoken("test_token").await.unwrap();

    let mut events = client.subscribe_event("state_changed").await.unwrap();
    let event = timeout(Duration::from_secs(3), events.recv())
        .await
        .expect("the event sent with the acknowledgement was lost")
        .expect("subscription receiver closed");
    assert_eq!(event.event.event_type, "state_changed");

    drop(client);
    let _ = timeout(Duration::from_secs(5), server).await;
}
