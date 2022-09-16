use crate::types::HassEvent;

use serde::Deserialize;
use serde_json::Value;

///The tag identifying which variant we are dealing with is inside of the content,
/// next to any other fields of the variant.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum Response {
    //request to autheticate
    AuthRequired(AuthRequired),
    //authetication suceeded
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
    Close(String),
}

// this is the first message received from websocket,
// that ask to provide a authetication method
#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) struct AuthRequired {
    //  #[serde(rename = "type")]
    //  pub(crate) msg_type: String,
    pub(crate) ha_version: String,
}

// this is received when the service successfully autheticate
#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) struct AuthOk {
    //  #[serde(rename = "type")]
    //  pub(crate) msg_type: String,
    pub(crate) ha_version: String,
}

// this is received if the authetication failed
#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) struct AuthInvalid {
    // #[serde(rename = "type")]
    // pub(crate) msg_type: String,
    pub(crate) message: String,
}

// this is received as a response to a ping request
#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct WSPong {
    pub(crate) id: u64,
    // #[serde(rename = "type")]
    // pub(crate) msg_type: String,
}

///	This object represents the Home Assistant Event
///
/// received when the client is subscribed to
/// [Subscribe to events](https://developers.home-assistant.io/docs/api/websocket/#subscribe-to-events)
#[derive(Debug, Deserialize, PartialEq)]
pub struct WSEvent {
    pub id: u64,
    // #[serde(rename = "type")]
    // pub(crate) msg_type: String,
    pub event: HassEvent,
}

///this is the general response from the Websocket server when a requesthas been sent
///
/// if "success" is true, then the "result" can be checked
/// if "suceess" is false, then the "error" should be further explored
#[derive(Debug, Deserialize, PartialEq)]
pub struct WSResult {
    pub(crate) id: u64,
    // #[serde(rename = "type")]
    // pub(crate) msg_type: String,
    pub(crate) success: bool,
    pub(crate) result: Option<Value>,
    pub(crate) error: Option<ErrorCode>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct ErrorCode {
    pub(crate) code: String,
    pub(crate) message: String,
}
