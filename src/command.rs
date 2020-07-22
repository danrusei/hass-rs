use async_tungstenite::tungstenite::Message as TungsteniteMessage;

#[derive(Debug)]
pub(crate) enum Command {
    Auth(Auth),
    Msg(u64, String),
    // maybe -> Heartbeat(Option<u64>),
    Close,
}

impl Command {
    /// This function transform a command into a TungsteniteMessage and needs the last
    /// gateway sequence in order to send it correctly
    pub(crate) fn to_tungstenite_message(self, sequence: Option<u64>) -> TungsteniteMessage {
        match self {
            Self::Auth(auth) => {
                let cmd_str = serde_json::to_string(&auth).unwrap();
                TungsteniteMessage::Text(cmd_str)
            }
            Self::Close => todo!(),
            _ => todo!(),
        }
    }
}

use serde::Serialize;

#[derive(Debug, Serialize, PartialEq)]
pub(crate) struct Auth {
    pub(crate) msg_type: String,
    pub(crate) access_token: String,
}
