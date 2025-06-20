//! Home Assistant client implementation

use crate::types::{
    Ask, Auth, CallService, Command, HassConfig, HassEntity, HassPanels, HassServices, Response,
    HassRegistryArea, HassRegistryDevice, HassRegistryEntity,
    Subscribe, WSEvent,
};
use crate::{HassError, HassResult};

use futures_util::{stream::SplitStream, SinkExt, StreamExt};
use parking_lot::Mutex;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::oneshot::{channel as oneshot, Sender as OneShotSender};
use tokio_tungstenite::tungstenite::{Error, Message};
use tokio_tungstenite::{connect_async, WebSocketStream};

/// HassClient is a library that is meant to simplify the conversation with HomeAssistant Web Socket Server
/// it provides a number of convenient functions that creates the requests and read the messages from server
pub struct HassClient {
    // holds the id of the WS message
    last_sequence: AtomicU64,

    rx_state: Arc<ReceiverState>,

    /// Client --> Gateway (send "Commands" msg to the Gateway)
    message_tx: Arc<Sender<Message>>,
}

#[derive(Default)]
struct ReceiverState {
    subscriptions: Mutex<HashMap<u64, Sender<WSEvent>>>,
    pending_requests: Mutex<HashMap<u64, OneShotSender<Response>>>,
    untagged_request: Mutex<Option<OneShotSender<Response>>>,
}

impl ReceiverState {
    fn get_tx(self: &Arc<Self>, id: u64) -> Option<Sender<WSEvent>> {
        self.subscriptions.lock().get(&id).map(|tx| tx.clone())
    }

    fn rm_subscription(self: &Arc<Self>, id: u64) {
        self.subscriptions.lock().remove(&id);
    }

    fn take_responder(self: &Arc<Self>, id: u64) -> Option<OneShotSender<Response>> {
        self.pending_requests.lock().remove(&id)
    }

    fn take_untagged(self: &Arc<Self>) -> Option<OneShotSender<Response>> {
        self.untagged_request.lock().take()
    }
}

async fn ws_incoming_messages(
    mut stream: SplitStream<WebSocketStream<impl AsyncRead + AsyncWrite + Unpin>>,
    rx_state: Arc<ReceiverState>,
    message_tx: Arc<Sender<Message>>,
) {
    while let Some(message) = stream.next().await {
        log::trace!("incoming: {message:#?}");
        match check_if_event(message) {
            Ok(event) => {
                // Dispatch to subscriber
                let id = event.id;
                if let Some(tx) = rx_state.get_tx(id) {
                    if tx.send(event).await.is_err() {
                        rx_state.rm_subscription(id);
                        // TODO: send unsub request here
                    }
                }
            }
            Err(message) => match message {
                Ok(Message::Text(data)) => {
                    let payload: Result<Response, HassError> = serde_json::from_str(data.as_str())
                        .map_err(|err| HassError::UnableToDeserialize(err));

                    match payload {
                        Ok(response) => match response.id() {
                            Some(id) => {
                                if let Some(tx) = rx_state.take_responder(id) {
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

                                if let Some(tx) = rx_state.take_untagged() {
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
                    if let Err(err) = message_tx.send(Message::Pong(data)).await {
                        log::error!("Error responding to ping: {err:#}");
                        break;
                    }
                }
                unexpected => log::error!("Unexpected message: {unexpected:#?}"),
            },
        }
    }
}

impl HassClient {
    pub async fn new(url: &str) -> HassResult<Self> {
        let (wsclient, _) = connect_async(url).await?;
        let (mut sink, stream) = wsclient.split();
        let (message_tx, mut message_rx) = channel(20);

        let message_tx = Arc::new(message_tx);

        let rx_state = Arc::new(ReceiverState::default());

        tokio::spawn(async move {
            while let Some(msg) = message_rx.recv().await {
                if let Err(err) = sink.send(msg).await {
                    log::error!("sink error: {err:#}");
                    break;
                }
            }
        });
        tokio::spawn(ws_incoming_messages(
            stream,
            rx_state.clone(),
            message_tx.clone(),
        ));

        let last_sequence = AtomicU64::new(1);

        Ok(Self {
            last_sequence,
            rx_state,
            message_tx,
        })
    }

    /// authenticate the session using a long-lived access token
    ///
    /// When a client connects to the server, the server sends out auth_required.
    /// The first message from the client should be an auth message. You can authorize with an access token.
    /// If the client supplies valid authentication, the authentication phase will complete by the server sending the auth_ok message.
    /// If the data is incorrect, the server will reply with auth_invalid message and disconnect the session.

    pub async fn auth_with_longlivedtoken(&mut self, token: &str) -> HassResult<()> {
        let auth_message = Command::AuthInit(Auth {
            msg_type: "auth".to_owned(),
            access_token: token.to_owned(),
        });

        let response = self.command(auth_message, None).await?;

        // Check if the authetication was succefully, should receive {"type": "auth_ok"}
        match response {
            Response::AuthOk(_) => Ok(()),
            Response::AuthInvalid(err) => Err(HassError::AuthenticationFailed(err.message)),
            unknown => Err(HassError::UnknownPayloadReceived(unknown)),
        }
    }

    /// The API supports receiving a ping from the client and returning a pong.
    /// This serves as a heartbeat to ensure the connection is still alive.
    pub async fn ping(&mut self) -> HassResult<()> {
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
    pub async fn get_config(&mut self) -> HassResult<HassConfig> {
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

    pub async fn get_states(&mut self) -> HassResult<Vec<HassEntity>> {
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

    pub async fn get_services(&mut self) -> HassResult<HassServices> {
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

    pub async fn get_panels(&mut self) -> HassResult<HassPanels> {
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
    pub async fn get_area_registry_list(&mut self) -> HassResult<Vec<HassRegistryArea>> {
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
    pub async fn get_device_registry_list(&mut self) -> HassResult<Vec<HassRegistryDevice>> {
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
    pub async fn get_entity_registry_list(&mut self) -> HassResult<Vec<HassRegistryEntity>> {
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
        &mut self,
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
            unknown => Err(HassError::UnknownPayloadReceived(unknown)),
        }
    }

    /// The command subscribe_event will subscribe your client to the event bus.
    ///
    /// Returns a channel that will receive the subscription messages.
    pub async fn subscribe_event(&mut self, event_name: &str) -> HassResult<Receiver<WSEvent>> {
        let id = self.next_seq();

        let cmd = Command::SubscribeEvent(Subscribe {
            id,
            msg_type: "subscribe_events".to_owned(),
            event_type: event_name.to_owned(),
        });

        let response = self.command(cmd, Some(id)).await?;

        match response {
            Response::Result(v) if v.is_ok() => {
                let (tx, rx) = channel(20);
                self.rx_state.subscriptions.lock().insert(v.id, tx);
                return Ok(rx);
            }
            Response::Result(v) => Err(HassError::ResponseError(v)),
            unknown => Err(HassError::UnknownPayloadReceived(unknown)),
        }
    }

    /// send commands and receive responses from the gateway
    pub(crate) async fn command(&mut self, cmd: Command, id: Option<u64>) -> HassResult<Response> {
        let cmd_tungstenite = cmd.to_tungstenite_message();

        let (tx, rx) = oneshot();

        match id {
            Some(id) => {
                self.rx_state.pending_requests.lock().insert(id, tx);
            }
            None => {
                self.rx_state.untagged_request.lock().replace(tx);
            }
        }

        // Send the auth command to gateway
        self.message_tx
            .send(cmd_tungstenite)
            .await
            .map_err(|err| HassError::SendError(err.to_string()))?;

        rx.await
            .map_err(|err| HassError::RecvError(err.to_string()))
    }

    /// get message sequence required by the Websocket server
    fn next_seq(&self) -> u64 {
        self.last_sequence.fetch_add(1, Ordering::Relaxed)
    }
}

/// convenient function that validates if the message received is an Event
/// the Events should be processed by used in a separate async task
fn check_if_event(result: Result<Message, Error>) -> Result<WSEvent, Result<Message, Error>> {
    match result {
        Ok(Message::Text(data)) => {
            let payload: Result<Response, HassError> =
                serde_json::from_str(data.as_str()).map_err(|err| HassError::from(err));

            if let Ok(Response::Event(event)) = payload {
                Ok(event)
            } else {
                Err(Ok(Message::Text(data)))
            }
        }
        result => Err(result),
    }
}
