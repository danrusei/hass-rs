use crate::events::HassEvent;

use serde_derive::Deserialize;
use serde_json::Value;

//The tag identifying which variant we are dealing with is inside of the content, 
// next to any other fields of the variant.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    AuthRequired(AuthRequired),
    AuthOk(AuthOk),
    AuthInvalid(AuthInvalid),
    Result(WSResult),
    Pong(WSPong),
    Event(WSEvent),
    Close(String),
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct AuthRequired {
  //  #[serde(rename = "type")]
  //  pub(crate) msg_type: String,
    pub(crate) ha_version: String,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct AuthOk {
  //  #[serde(rename = "type")]
  //  pub(crate) msg_type: String,
    pub(crate) ha_version: String,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct AuthInvalid {
    #[serde(rename = "type")]
    pub(crate) msg_type: String,
    pub(crate) ha_version: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct WSPong {
    pub(crate) id: u32,
    // #[serde(rename = "type")]
    // pub(crate) msg_type: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct WSEvent {
    pub(crate) id: u32,
    #[serde(rename = "type")]
    pub(crate) msg_type: String,
    pub(crate) event: HassEvent,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct WSResult {
    pub(crate) id: u32,
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
