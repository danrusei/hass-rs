use crate::types::HassEvent;
use crate::HassResult;

use serde::Deserialize;
use serde_json::Value;

///The tag identifying which variant we are dealing with is inside of the content,
/// next to any other fields of the variant.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    //request to autheticate
    AuthRequired(AuthRequired),
    //authetication suceeded
    #[allow(unused)]
    AuthOk(AuthOk),
    //authetication failed
    AuthInvalid(AuthInvalid),
    //general response from server
    Result(WSResult),
    //response to ping request
    Pong(WSPong),
    //received when subscribed to event
    Event(WSEvent),
    //when the server close the websocket connection
    #[allow(unused)]
    Close(String),
}

impl Response {
    pub fn id(&self) -> Option<u64> {
        match self {
            Self::AuthRequired(_) | Self::AuthOk(_) | Self::AuthInvalid(_) | Self::Close(_) => None,
            Self::Pong(pong) => Some(pong.id),
            Self::Result(result) => Some(result.id),
            Self::Event(event) => Some(event.id),
        }
    }
}

// this is the first message received from websocket,
// that ask to provide a authetication method
#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct AuthRequired {
    pub ha_version: String,
}

// this is received when the service successfully autheticate
#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct AuthOk {
    pub ha_version: String,
}

// this is received if the authetication failed
#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct AuthInvalid {
    pub message: String,
}

// this is received as a response to a ping request
#[derive(Debug, Deserialize, PartialEq)]
pub struct WSPong {
    pub id: u64,
}

///	This object represents the Home Assistant Event
///
/// received when the client is subscribed to
/// [Subscribe to events](https://developers.home-assistant.io/docs/api/websocket/#subscribe-to-events)
#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct WSEvent {
    pub id: u64,
    pub event: HassEvent,
}

///this is the general response from the Websocket server when a requesthas been sent
///
/// if "success" is true, then the "result" can be checked
/// if "suceess" is false, then the "error" should be further explored
#[derive(Debug, Deserialize, PartialEq)]
pub struct WSResult {
    pub id: u64,
    success: bool,
    result: Option<Value>,
    error: Option<ErrorCode>,
}

impl WSResult {
    pub fn is_ok(&self) -> bool {
        self.success
    }

    pub fn is_err(&self) -> bool {
        !self.success
    }

    pub fn result(self) -> HassResult<Value> {
        if self.success {
            if let Some(result) = self.result {
                return Ok(result);
            }
        }
        Err(crate::HassError::ResponseError(self))
    }
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct ErrorCode {
    pub code: String,
    pub message: String,
}
