use async_tungstenite::tungstenite::Message as TungsteniteMessage;
use serde::Serialize;

#[derive(Debug)]
pub(crate) enum Command {
    AuthInit(Auth),
    Msg(u64, String),
    // maybe -> Heartbeat(Option<u64>),
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
            Self::Close => todo!(),
            _ => todo!(),
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub(crate) struct Auth {
    #[serde(rename = "type")]
    pub(crate) msg_type: String,
    pub(crate) access_token: String,
}
