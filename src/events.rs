use serde_derive::Deserialize;

//TODO -- need to validate the Event Structure
#[derive(Debug, Deserialize, PartialEq)]
pub struct HassEvent {
    pub(crate) origin: String,
    pub(crate) time_fired: String,
    pub(crate) context: Context,
    pub(crate) event_type: String,
    //TODO it is not string, there are alot of values inside, specific to event
    pub(crate) data: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Context {
    pub(crate) id: String,
    pub(crate) user_id: String,
}
