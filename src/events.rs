use serde_derive::Deserialize;
use serde_json::Value;

//TODO -- Structure is working but I have to create the **data** field
#[derive(Debug, Deserialize, PartialEq)]
pub struct HassEvent {
    pub(crate) event_type: String,
    pub(crate) data: Option<Value>,
    pub(crate) origin: String,
    pub(crate) time_fired: String,
    pub(crate) context: Context,
    
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Context {
    pub(crate) id: String,
    pub(crate) parent_id: Option<String>,
    pub(crate) user_id: Option<String>,
}
