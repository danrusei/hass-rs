use async_tungstenite::tungstenite::Message as TungsteniteMessage;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug)]
pub enum Command {
    AuthInit(Auth),
    Ping(Ask),
    SubscribeEvent(Subscribe),
    Unsubscribe(Unsubscribe),
    GetConfig(Ask),
    GetServices(Ask),
    GetStates(Ask),
    CallService(CallService),
    Close,
}

impl Command {
    /// This function transform a command into a TungsteniteMessage and needs the last
    /// gateway sequence in order to send it correctly
    pub(crate) fn to_tungstenite_message(self) -> TungsteniteMessage {
        match self {
            Self::AuthInit(auth) => {
                let cmd_str = serde_json::to_string(&auth).unwrap();
                TungsteniteMessage::Text(cmd_str)
            }
            Self::Ping(ping) => {
                let cmd_str = serde_json::to_string(&ping).unwrap();
                TungsteniteMessage::Text(cmd_str)
            }
            Self::SubscribeEvent(subscribe) => {
                let cmd_str = serde_json::to_string(&subscribe).unwrap();
                TungsteniteMessage::Text(cmd_str)
            }
            Self::Unsubscribe(unsubscribe) => {
                let cmd_str = serde_json::to_string(&unsubscribe).unwrap();
                TungsteniteMessage::Text(cmd_str)
            }
            Self::GetConfig(getconfig) => {
                let cmd_str = serde_json::to_string(&getconfig).unwrap();
                TungsteniteMessage::Text(cmd_str)
            }
            Self::GetStates(getstates) => {
                let cmd_str = serde_json::to_string(&getstates).unwrap();
                TungsteniteMessage::Text(cmd_str)
            }
            Self::GetServices(getservices) => {
                let cmd_str = serde_json::to_string(&getservices).unwrap();
                TungsteniteMessage::Text(cmd_str)
            }
            Self::CallService(callservice) => {
                let cmd_str = serde_json::to_string(&callservice).unwrap();
                TungsteniteMessage::Text(cmd_str)
            }
            Self::Close => todo!(),
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Auth {
    #[serde(rename = "type")]
    pub(crate) msg_type: String,
    pub(crate) access_token: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Ask {
    pub(crate) id: Option<u64>,
    #[serde(rename = "type")]
    pub(crate) msg_type: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Subscribe {
    pub(crate) id: Option<u64>,
    #[serde(rename = "type")]
    pub(crate) msg_type: String,
    pub(crate) event_type: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Unsubscribe {
    pub(crate) id: Option<u64>,
    #[serde(rename = "type")]
    pub(crate) msg_type: String,
    pub(crate) subscription: u64,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CallService {
    pub(crate) id: Option<u64>,
    #[serde(rename = "type")]
    pub(crate) msg_type: String,
    pub(crate) domain: String,
    pub(crate) service: String,
    pub(crate) service_data: Option<Value>,
}
