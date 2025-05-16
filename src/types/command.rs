use serde::Serialize;
use serde_json::Value;
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;

/// This enum defines the type of commands that the client is allowed to send to the Websocket server
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub(crate) enum Command {
    AuthInit(Auth),
    Ping(Ask),
    SubscribeEvent(Subscribe),
    #[allow(unused)]
    Unsubscribe(Unsubscribe),
    GetConfig(Ask),
    GetServices(Ask),
    GetStates(Ask),
    GetPanels(Ask),
    GetAreaRegistryList(Ask),
    GetDeviceRegistryList(Ask),
    GetEntityRegistryList(Ask),
    CallService(CallService),
    #[allow(dead_code)]
    Close,
}

impl Command {
    /// This function transform a command into a TungsteniteMessage and needs the last
    /// gateway sequence in order to send it correctly
    pub(crate) fn to_tungstenite_message(self) -> TungsteniteMessage {
        let cmd_str = serde_json::to_string(&self).unwrap();
        TungsteniteMessage::text(cmd_str)
    }
}

//used to authenticate the session
#[derive(Debug, Serialize, PartialEq)]
pub(crate) struct Auth {
    #[serde(rename = "type")]
    pub(crate) msg_type: String,
    pub(crate) access_token: String,
}

//used to fetch from server
#[derive(Debug, Serialize, PartialEq)]
pub(crate) struct Ask {
    pub(crate) id: u64,
    #[serde(rename = "type")]
    pub(crate) msg_type: String,
}

//used for Event subscribtion
#[derive(Debug, Serialize, PartialEq)]
pub(crate) struct Subscribe {
    pub(crate) id: u64,
    #[serde(rename = "type")]
    pub(crate) msg_type: String,
    pub(crate) event_type: String,
}

//used for Event Unsubscribe
#[derive(Debug, Serialize, PartialEq)]
pub(crate) struct Unsubscribe {
    pub(crate) id: u64,
    #[serde(rename = "type")]
    pub(crate) msg_type: String,
    pub(crate) subscription: u64,
}

//used to call a service
#[derive(Debug, Serialize, PartialEq)]
pub(crate) struct CallService {
    pub(crate) id: u64,
    #[serde(rename = "type")]
    pub(crate) msg_type: String,
    pub(crate) domain: String,
    pub(crate) service: String,
    pub(crate) service_data: Option<Value>,
}
