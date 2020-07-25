use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Response {
    //below was String instead of AuthInit when auth worked --- TO Remove
    AuthInit(AuthInit),
    Pong(WSPong),
    Event(WSEvent),
    Result(WSResult),
    ResultError(WSResultError),
    Close(String),
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct AuthInit {
    #[serde(rename = "type")]
    pub(crate) msg_type: String,
    pub(crate) ha_version: String,
}


#[derive(Debug, Deserialize, PartialEq)]
pub struct WSPong {
    pub(crate) id: u32,
    #[serde(rename = "type")]
    pub(crate) msg_type: String,
}


#[derive(Debug, Deserialize, PartialEq)]
pub struct WSEvent {
    pub(crate) id: u32,
    #[serde(rename = "type")]
    pub(crate) msg_type: String,
    // TODO define HassEvent
//    pub(crate) event: HassEvent,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct WSResult {
    pub(crate) id: u32,
    #[serde(rename = "type")]
    pub(crate) msg_type: String,
    pub(crate) success: bool,
    //TODO, add an Option with Enum of structs for results
    //like config and others ?, not sure if it is working
    // pub(crate) result: Option<>,
}


#[derive(Debug, Deserialize, PartialEq)]
pub struct WSResultError {
    pub(crate) id: u32,
    #[serde(rename = "type")]
    pub(crate) msg_type: String,
    pub(crate) success: bool,
    pub(crate) error: ErrorCode,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct ErrorCode {
    pub(crate) code: String,
    pub(crate) message: String,
}

  
//   type WebSocketResponse =
//     | WebSocketPongResponse
//     | WebSocketEventResponse
//     | WebSocketResultResponse
//     | WebSocketResultErrorResponse;

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
