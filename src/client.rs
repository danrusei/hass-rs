//! Home Assistant client implementation
//!
//! Provides an async connect and methods for issuing the supported commands.

use crate::types::{
    Ask, Auth, CallService, Command, HassConfig, HassEntity, HassPanels, HassServices, Response,
    Subscribe, Unsubscribe, WSEvent,
};
use crate::{HassError, HassResult};
use async_tungstenite::tungstenite::Error;
use async_tungstenite::tungstenite::Message as TungsteniteMessage;

//use futures_util::StreamExt;
use futures_util::lock::Mutex;
use log::info;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use tokio::sync::mpsc::{Receiver, Sender};

//use url;
//use url::Url;

/// Established connection with a Home Assistant WebSocket server.
///
/// Backed by async_tungstenite, that provides async Websocket bindings,
/// that can be used with non-blocking/asynchronous TcpStreams.
/// It Supports both "async-std" and "tokio" runtimes.
///
/// Requests are issued using the various methods of `Client`.  
#[derive(Debug)]
pub struct HassClient {
    //pub(crate) gateway: WsConn,
    last_sequence: Arc<AtomicU64>,
    event_listeners: Arc<Mutex<HashMap<u64, Box<dyn Fn(WSEvent) + Send>>>>,

    //Client --> Gateway (send "Commands" msg to the Gateway)
    pub(crate) to_gateway: Sender<TungsteniteMessage>,

    //Gateway --> Client (receive "Response" msg from the Gateway)
    pub(crate) from_gateway: Receiver<Result<TungsteniteMessage, Error>>,
}

/// establish the websocket connection to Home Assistant server
// pub async fn connect(host: &str, port: u16) -> HassResult<HassClient> {
//     let addr = format!("ws://{}:{}/api/websocket", host, port);
//     let url = url::Url::parse(&addr)?;
//     connect_to_url(url).await
// }

// establish the websocker connection to Home Assistant server, providing the complete WS URL
// pub async fn connect_to_url(url: Url) -> HassResult<HassClient> {
//     let gateway = WsConn::connect(url).await?;
//     Ok(HassClient { gateway })
// }

impl HassClient {
    /// authenticate the session using a long-lived access token
    ///
    /// When a client connects to the server, the server sends out auth_required.
    /// The first message from the client should be an auth message. You can authorize with an access token.
    /// If the client supplies valid authentication, the authentication phase will complete by the server sending the auth_ok message.
    /// If the data is incorrect, the server will reply with auth_invalid message and disconnect the session.

    pub fn new(
        tx: Sender<TungsteniteMessage>,
        rx: Receiver<Result<TungsteniteMessage, Error>>,
    ) -> Self {
        let last_sequence = Arc::new(AtomicU64::new(1));
        //let last_sequence_clone_receiver = Arc::clone(&last_sequence);

        let event_listeners = Arc::new(Mutex::new(HashMap::new()));
        //let event_listeners_clone_receiver = Arc::clone(&event_listeners);

        HassClient {
            last_sequence,
            event_listeners,
            to_gateway: tx,
            from_gateway: rx,
        }
    }

    pub async fn auth_with_longlivedtoken(&mut self, token: &str) -> HassResult<()> {
        // Auth Request from Gateway { "type": "auth_required"}
        let _ = self
            .from_gateway
            .recv()
            .await
            .ok_or_else(|| HassError::ConnectionClosed)?;

        //Authenticate with Command::AuthInit and payload {"type": "auth", "access_token": "XXXXX"}
        let auth_req = Command::AuthInit(Auth {
            msg_type: "auth".to_owned(),
            access_token: token.to_owned(),
        });

        info!("{:?}", auth_req);

        let response = self.command(auth_req).await?;

        //Check if the authetication was succefully, should receive {"type": "auth_ok"}
        match response {
            Response::AuthOk(_) => Ok(()),
            Response::AuthInvalid(err) => return Err(HassError::AuthenticationFailed(err.message)),
            _ => return Err(HassError::UnknownPayloadReceived),
        }

        // match value.msg_type.as_str() {
        //     "auth_ok" => return Ok(()),
        //     "auth_invalid" => return Err(HassError::AuthenticationFailed),
        //     _ => return Err(HassError::UnknownPayloadReceived),
        // }
    }

    ///The API supports receiving a ping from the client and returning a pong.
    /// This serves as a heartbeat to ensure the connection is still alive.

    pub async fn ping(&mut self) -> HassResult<String> {
        let id = get_last_seq(&self.last_sequence).expect("could not read the Atomic value");

        //Send Ping command and expect Pong
        let ping_req = Command::Ping(Ask {
            id: Some(id),
            msg_type: "ping".to_owned(),
        });

        let response = self.command(ping_req).await?;

        //Check the response, if the Pong was received
        match response {
            Response::Pong(_v) => Ok("pong".to_owned()),
            Response::Result(err) => return Err(HassError::ReponseError(err)),
            _ => return Err(HassError::UnknownPayloadReceived),
        }
    }

    ///The command subscribe_event will subscribe your client to the event bus.
    ///
    /// You can either listen to all events or to a specific event type.
    /// If you want to listen to multiple event types, you will have to send multiple subscribe_events commands.
    /// The server will respond with a result message to indicate that the subscription is active.
    /// For each event that matches, the server will send a message of type event.
    /// The id in the message will point at the original id of the listen_event command.

    pub async fn subscribe_event<F>(&mut self, event_name: &str, callback: F) -> HassResult<String>
    where
        F: Fn(WSEvent) + Send + 'static,
    {
        self.subscribe_message(event_name, callback).await
    }

    ///The command unsubscribe_event will unsubscribe your client from the event bus.
    ///
    /// You can unsubscribe from previously created subscription events.
    /// Pass the id of the original subscription command as value to the subscription field.

    pub async fn unsubscribe_event(&mut self, subscription_id: u64) -> HassResult<String> {
        self.unsubscribe_message(subscription_id).await
    }

    ///This will get a dump of the current config in Home Assistant.
    ///
    /// The server will respond with a result message containing the config.

    pub async fn get_config(&mut self) -> HassResult<HassConfig> {
        let id = get_last_seq(&self.last_sequence).expect("could not read the Atomic value");

        //Send GetConfig command and expect Pong
        let config_req = Command::GetConfig(Ask {
            id: Some(id),
            msg_type: "get_config".to_owned(),
        });
        let response = self.command(config_req).await?;

        match response {
            Response::Result(data) => match data.success {
                true => {
                    let config: HassConfig = serde_json::from_value(
                        data.result.expect("Expecting to get the HassConfig"),
                    )?;
                    return Ok(config);
                }
                false => return Err(HassError::ReponseError(data)),
            },
            _ => return Err(HassError::UnknownPayloadReceived),
        }
    }

    ///This will get a dump of all the current states in Home Assistant.
    ///
    /// The server will respond with a result message containing the states.

    pub async fn get_states(&mut self) -> HassResult<Vec<HassEntity>> {
        let id = get_last_seq(&self.last_sequence).expect("could not read the Atomic value");

        //Send GetStates command and expect a number of Entities
        let states_req = Command::GetStates(Ask {
            id: Some(id),
            msg_type: "get_states".to_owned(),
        });
        let response = self.command(states_req).await?;

        match response {
            Response::Result(data) => match data.success {
                true => {
                    let states: Vec<HassEntity> =
                        serde_json::from_value(data.result.expect("Expecting to get the States"))?;
                    return Ok(states);
                }
                false => return Err(HassError::ReponseError(data)),
            },
            _ => return Err(HassError::UnknownPayloadReceived),
        }
    }

    ///This will get a dump of the current services in Home Assistant.
    ///
    /// The server will respond with a result message containing the services.

    pub async fn get_services(&mut self) -> HassResult<HassServices> {
        let id = get_last_seq(&self.last_sequence).expect("could not read the Atomic value");
        //Send GetStates command and expect a number of Entities
        let services_req = Command::GetServices(Ask {
            id: Some(id),
            msg_type: "get_services".to_owned(),
        });
        let response = self.command(services_req).await?;

        match response {
            Response::Result(data) => match data.success {
                true => {
                    let services: HassServices = serde_json::from_value(
                        data.result.expect("Expecting to get the Services"),
                    )?;
                    return Ok(services);
                }
                false => return Err(HassError::ReponseError(data)),
            },
            _ => return Err(HassError::UnknownPayloadReceived),
        }
    }

    ///This will get a dump of the current registered panels in Home Assistant.
    ///
    /// The server will respond with a result message containing the current registered panels.

    pub async fn get_panels(&mut self) -> HassResult<HassPanels> {
        let id = get_last_seq(&self.last_sequence).expect("could not read the Atomic value");

        //Send GetStates command and expect a number of Entities
        let services_req = Command::GetPanels(Ask {
            id: Some(id),
            msg_type: "get_panels".to_owned(),
        });
        let response = self.command(services_req).await?;

        match response {
            Response::Result(data) => match data.success {
                true => {
                    let services: HassPanels =
                        serde_json::from_value(data.result.expect("Expecting panels"))?;
                    return Ok(services);
                }
                false => return Err(HassError::ReponseError(data)),
            },
            _ => return Err(HassError::UnknownPayloadReceived),
        }
    }

    ///This will call a service in Home Assistant. Right now there is no return value.
    ///The client can listen to state_changed events if it is interested in changed entities as a result of a service call.
    ///
    ///The server will indicate with a message indicating that the service is done executing.
    ///

    pub async fn call_service(
        &mut self,
        domain: String,
        service: String,
        service_data: Option<Value>,
    ) -> HassResult<String> {
        let id = get_last_seq(&self.last_sequence).expect("could not read the Atomic value");

        //Send GetStates command and expect a number of Entities
        let services_req = Command::CallService(CallService {
            id: Some(id),
            msg_type: "call_service".to_owned(),
            domain,
            service,
            service_data,
        });
        let response = self.command(services_req).await?;

        match response {
            Response::Result(data) => match data.success {
                true => return Ok("command executed successfully".to_owned()),
                false => return Err(HassError::ReponseError(data)),
            },
            _ => return Err(HassError::UnknownPayloadReceived),
        }
    }

    //used to send commands and receive responses from the gasteway
    pub(crate) async fn command(&mut self, cmd: Command) -> HassResult<Response> {
        //transform to TungsteniteMessage to be sent to WebSocket
        let cmd_tungstenite = cmd.to_tungstenite_message();

        // Send the auth command to gateway
        self.to_gateway
            .send(cmd_tungstenite)
            .await
            .map_err(|_| HassError::ConnectionClosed)?;

        // Receive auth response event from the gateway
        // self.from_gateway
        //     .recv()
        //     .await
        //     .ok_or_else(|| HassError::ConnectionClosed)?
        match self.from_gateway.recv().await {
            Some(Ok(item)) => match item {
                TungsteniteMessage::Text(data) => {
                    // info!("{}", data);

                    //Serde: The tag identifying which variant we are dealing with is now inside of the content,
                    // next to any other fields of the variant
                    let payload: Result<Response, HassError> =
                        serde_json::from_str(&data).map_err(|_| HassError::UnknownPayloadReceived);

                    //Match on payload, and act accordingly, like execute the client defined closure if any Event received
                    match payload {
                        Ok(value) => match value {
                            Response::Event(event) => {
                                let mut table = self.event_listeners.lock().await;

                                match table.get_mut(&event.id) {
                                    Some(client_func) => {
                                        //execute client closure
                                        client_func(event);
                                    }
                                    None => todo!("send unsubscribe request"),
                                }
                                // FIXME  this should not return Error, as it found event and is executing customer closure
                                return Err(HassError::Generic("TO DO".to_string()));
                            }
                            _ => return Ok(value), //todo!("to_client.send(Ok(value)).await.unwrap(),"),
                        },
                        Err(error) => return Err(error), //todo!("to_client.send(Err(error)).await.unwrap(),"),
                    };
                }
                _ => todo!(),
            },
            Some(Err(error)) => {
                let err = Err(HassError::from(&error));
                err
            }

            None => {
                todo!()
            }
        }
    }

    //used to subscribe to the event and if the subscribtion succeded the callback is registered
    pub(crate) async fn subscribe_message<F>(
        &mut self,
        event_name: &str,
        callback: F,
    ) -> HassResult<String>
    where
        F: Fn(WSEvent) + Send + 'static,
    {
        let id = get_last_seq(&self.last_sequence).expect("could not read the Atomic value");

        //create the Event Subscribe Command
        let cmd = Command::SubscribeEvent(Subscribe {
            id: Some(id),
            msg_type: "subscribe_events".to_owned(),
            event_type: event_name.to_owned(),
        });

        //send command to subscribe to specific event
        let response = self.command(cmd).await.unwrap();

        //Add the callback in the event_listeners hashmap if the Subscription Response is successfull
        match response {
            Response::Result(v) if v.success == true => {
                let mut table = self.event_listeners.lock().await;
                table.insert(v.id, Box::new(callback));
                return Ok("Ok".to_owned());
            }
            Response::Result(v) if v.success == false => return Err(HassError::ReponseError(v)),
            _ => return Err(HassError::UnknownPayloadReceived),
        }
    }

    //used to unsubscribe the event and remove the registered callback
    pub(crate) async fn unsubscribe_message(&mut self, subscription_id: u64) -> HassResult<String> {
        let id = get_last_seq(&self.last_sequence).expect("could not read the Atomic value");

        //Unsubscribe the Event
        let unsubscribe_req = Command::Unsubscribe(Unsubscribe {
            id: Some(id),
            msg_type: "unsubscribe_events".to_owned(),
            subscription: subscription_id,
        });

        //send command to unsubscribe from specific event
        let response = self.command(unsubscribe_req).await.unwrap();

        //Remove the event_type and the callback fromthe event_listeners hashmap
        match response {
            Response::Result(v) if v.success == true => {
                let mut table = self.event_listeners.lock().await;
                if let Some(_) = table.remove(&subscription_id) {
                    return Ok("Ok".to_owned());
                }
                return Err(HassError::Generic("Wrong subscription ID".to_owned()));
            }
            Response::Result(v) if v.success == false => return Err(HassError::ReponseError(v)),
            _ => return Err(HassError::UnknownPayloadReceived),
        }
    }
}

// message sequence required by the Websocket server
fn get_last_seq(last_sequence: &Arc<AtomicU64>) -> Option<u64> {
    // Increase the last sequence and use the previous value in the request
    match last_sequence.fetch_add(1, Ordering::Relaxed) {
        0 => None,
        v => Some(v),
    }
}
