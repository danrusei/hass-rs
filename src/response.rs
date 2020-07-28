use crate::events::HassEvent;

use serde_derive::Deserialize;

//TODO!!!!! -- this is wrong due to mismatches between types, when data is deserialize on gateway-recieve
//There is no explicit tag identifying which variant the data contains. 
//Serde will try to match the data against each variant in order and the first one that deserializes successfully is the one returned.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Response {
    AuthInit(AuthInit),
    Result(WSResult),
    Pong(WSPong),
    Event(WSEvent),
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
    pub(crate) event: HassEvent,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct WSResult {
    pub(crate) id: u32,
    #[serde(rename = "type")]
    pub(crate) msg_type: String,
    pub(crate) success: bool,
    //TODO, not sure if below is correct
    pub(crate) result: Option<String>,
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
