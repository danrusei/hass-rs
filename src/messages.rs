use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) enum Response {
    Auth_init(String),
    Close(String),
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct Auth_init {
    #[serde(rename = "type")]
    pub(crate) msg_type: String,
}

// #[derive(Debug, Deserialize)]
// pub struct Response {
//     pub sequence: Uuid,
//     pub result: ResponseResult,
//     pub status: ReponseStatus,
// }

// #[derive(Debug, Deserialize)]
// pub struct ResponseResult {
//     pub data: Value,
// }

// #[derive(Debug, Deserialize)]
// pub struct ReponseStatus {
//     pub code: i16,
//     pub message: String,
// }
