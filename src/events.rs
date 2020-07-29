use serde_derive::Deserialize;
use serde_json::Value;

//TODO -- need to validate the Event Structure
#[derive(Debug, Deserialize, PartialEq)]
pub struct HassEvent {
    pub(crate) origin: String,
    pub(crate) time_fired: String,
    pub(crate) context: Context,
    pub(crate) event_type: String,
    pub(crate) data: Option<Value>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Context {
    pub(crate) id: String,
    pub(crate) user_id: String,
}
