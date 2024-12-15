//! Home Assistant client implementation

use crate::types::{
    Ask, Auth, CallService, Command, HassConfig, HassEntity, HassPanels, HassServices, Response,
    Subscribe, Unsubscribe, WSEvent,
};
use crate::{HassError, HassResult, WSResult};
use crate::{Receiver, Sender};

use serde_json::Value;
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use tokio_tungstenite::tungstenite::Error;
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;

/// HassClient is a library that is meant to simplify the conversation with HomeAssistant Web Socket Server
/// it provides a number of convenient functions that creates the requests and read the messages from server
#[derive(Debug)]
pub struct HassClient {
    // holds the id of the WS message
    last_sequence: Arc<AtomicU64>,

    // holds the Events Subscriptions
    pub subscriptions: HashMap<u64, String>,

    //Client --> Gateway (send "Commands" msg to the Gateway)
    pub(crate) to_gateway: Sender<TungsteniteMessage>,

    //Gateway --> Client (receive "Response" msg from the Gateway)
    pub(crate) from_gateway: Receiver<Result<TungsteniteMessage, Error>>,
}

impl HassClient {
    pub fn new(
        tx: Sender<TungsteniteMessage>,
        rx: Receiver<Result<TungsteniteMessage, Error>>,
    ) -> Self {
        let last_sequence = Arc::new(AtomicU64::new(1));
        let subscriptions = HashMap::new();

        HassClient {
            last_sequence,
            subscriptions,
            to_gateway: tx,
            from_gateway: rx,
        }
    }

    /// authenticate the session using a long-lived access token
    ///
    /// When a client connects to the server, the server sends out auth_required.
    /// The first message from the client should be an auth message. You can authorize with an access token.
    /// If the client supplies valid authentication, the authentication phase will complete by the server sending the auth_ok message.
    /// If the data is incorrect, the server will reply with auth_invalid message and disconnect the session.

    pub async fn auth_with_longlivedtoken(&mut self, token: &str) -> HassResult<()> {
        // Auth Request from Gateway { "type": "auth_required"}
        if let Ok(Response::AuthRequired(msg)) = self.ws_receive().await {
            if msg.msg_type != "auth_required".to_string() {
                return Err(HassError::Generic(
                    "Expecting the first message from server to be auth_required".to_string(),
                ));
            }
        }

        //Authenticate with Command::AuthInit and payload {"type": "auth", "access_token": "XXXXX"}
        let auth_message = Command::AuthInit(Auth {
            msg_type: "auth".to_owned(),
            access_token: token.to_owned(),
        });

        let response = self.command(auth_message).await?;

        //Check if the authetication was succefully, should receive {"type": "auth_ok"}
        match response {
            Response::AuthOk(_) => Ok(()),
            Response::AuthInvalid(err) => return Err(HassError::AuthenticationFailed(err.message)),
            unknown => return Err(HassError::UnknownPayloadReceived(unknown)),
        }
    }

    /// The API supports receiving a ping from the client and returning a pong.
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
            Response::Result(err) => return Err(HassError::ResponseError(err)),
            unknown => return Err(HassError::UnknownPayloadReceived(unknown)),
        }
    }

    /// This will get the current config of the Home Assistant.
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
                false => return Err(HassError::ResponseError(data)),
            },
            unknown => return Err(HassError::UnknownPayloadReceived(unknown)),
        }
    }

    /// This will get all the current states from Home Assistant.
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
                false => return Err(HassError::ResponseError(data)),
            },
            unknown => return Err(HassError::UnknownPayloadReceived(unknown)),
        }
    }

    /// This will get all the services from Home Assistant.
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
                false => return Err(HassError::ResponseError(data)),
            },
            unknown => return Err(HassError::UnknownPayloadReceived(unknown)),
        }
    }

    /// This will get all the registered panels from Home Assistant.
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
                false => return Err(HassError::ResponseError(data)),
            },
            unknown => return Err(HassError::UnknownPayloadReceived(unknown)),
        }
    }

    ///This will call a service in Home Assistant. Right now there is no return value.
    ///The client can listen to state_changed events if it is interested in changed entities as a result of a service call.
    ///
    ///The server will indicate with a message indicating that the service is done executing.
    /// https://developers.home-assistant.io/docs/api/websocket#calling-a-service
    /// additional info : https://developers.home-assistant.io/docs/api/rest ==> Post /api/services/<domain>/<service>

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
                false => return Err(HassError::ResponseError(data)),
            },
            unknown => return Err(HassError::UnknownPayloadReceived(unknown)),
        }
    }

    /// The command subscribe_event will subscribe your client to the event bus.
    ///
    /// You can either listen to all events or to a specific event type.
    /// If you want to listen to multiple event types, you will have to send multiple subscribe_events commands.
    /// The server will respond with a result message to indicate that the subscription is active.
    /// For each event that matches, the server will send a message of type event.
    /// The id in the message will point at the original id of the listen_event command.

    pub async fn subscribe_event(&mut self, event_name: &str) -> HassResult<WSResult> {
        let id = get_last_seq(&self.last_sequence).expect("could not read the Atomic value");

        //create the Event Subscribe Command
        let cmd = Command::SubscribeEvent(Subscribe {
            id: Some(id),
            msg_type: "subscribe_events".to_owned(),
            event_type: event_name.to_owned(),
        });

        //send command to subscribe to specific event
        let response = self.command(cmd).await?;

        //Add the callback in the event_listeners hashmap if the Subscription Response is successfull
        match response {
            Response::Result(v) if v.success == true => {
                self.subscriptions.insert(v.id, event_name.to_owned());
                return Ok(v);
            }
            Response::Result(v) if v.success == false => return Err(HassError::ResponseError(v)),
            unknown => return Err(HassError::UnknownPayloadReceived(unknown)),
        }
    }

    ///The command unsubscribe_event will unsubscribe your client from the event bus.
    ///
    /// You can unsubscribe from previously created subscription events.
    /// Pass the id of the original subscription command as value to the subscription field.

    pub async fn unsubscribe_event(&mut self, subscription_id: u64) -> HassResult<String> {
        let id = get_last_seq(&self.last_sequence).expect("could not read the Atomic value");

        //Unsubscribe the Event
        let unsubscribe_req = Command::Unsubscribe(Unsubscribe {
            id: Some(id),
            msg_type: "unsubscribe_events".to_owned(),
            subscription: subscription_id,
        });

        //send command to unsubscribe from specific event
        let response = self.command(unsubscribe_req).await?;

        //Remove the event_type and the callback from the event_listeners hashmap
        match response {
            Response::Result(v) if v.success == true => {
                if let Some(_) = self.subscriptions.remove(&subscription_id) {
                    return Ok("Ok".to_owned());
                }
                return Err(HassError::Generic("Wrong subscription ID".to_owned()));
            }
            Response::Result(v) if v.success == false => return Err(HassError::ResponseError(v)),
            unknown => return Err(HassError::UnknownPayloadReceived(unknown)),
        }
    }

    //used to send commands and receive responses from the gateway
    pub(crate) async fn command(&mut self, cmd: Command) -> HassResult<Response> {
        //transform to TungsteniteMessage to be sent to WebSocket
        let cmd_tungstenite = cmd.to_tungstenite_message();

        // Send the auth command to gateway
        self.to_gateway
            .send(cmd_tungstenite)
            .await
            .map_err(|err| HassError::SendError(err.to_string()))?;

        self.ws_receive().await
    }

    //read the messages from the Websocket connection
    pub(crate) async fn ws_receive(&mut self) -> HassResult<Response> {
        match self.from_gateway.recv().await {
            Some(Ok(item)) => match item {
                TungsteniteMessage::Text(data) => {
                    //Serde: The tag identifying which variant we are dealing with is now inside of the content,
                    // next to any other fields of the variant

                    let payload: Result<Response, HassError> = serde_json::from_str(data.as_str())
                        .map_err(|err| HassError::UnableToDeserialize(err));

                    payload
                }
                msg => Err(HassError::UnexpectedMessage(msg)),
            },
            Some(Err(error)) => Err(HassError::from(error)),

            None => Err(HassError::ConnectionClosed),
        }
    }
}

/// convenient function that validates if the message received is an Event
/// the Events should be processed by used in a separate async task
pub fn check_if_event(
    result: Result<TungsteniteMessage, Error>,
) -> Result<WSEvent, Result<TungsteniteMessage, Error>> {
    match result {
        Ok(TungsteniteMessage::Text(data)) => {
            //Serde: The tag identifying which variant we are dealing with is now inside of the content,
            // next to any other fields of the variant

            let payload: Result<Response, HassError> =
                serde_json::from_str(data.as_str()).map_err(|err| HassError::from(err));

            if let Ok(Response::Event(event)) = payload {
                Ok(event)
            } else {
                Err(Ok(TungsteniteMessage::Text(data)))
            }
        }
        result => Err(result),
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
