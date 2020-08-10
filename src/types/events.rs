use crate::types::{Context, HassEntity};
use serde_derive::Deserialize;

///	This object represents the Home Assistant Event
/// 
/// received when the client is subscribed to
/// [Subscribe to events](https://developers.home-assistant.io/docs/api/websocket/#subscribe-to-events)
//This constructed against StateChangedEvent, may not be compatible with other event types
//TODO try out other type of events
#[derive(Debug, Deserialize, PartialEq)]
pub struct HassEvent {
    pub event_type: String,
    pub data: EventData,
    pub origin: String,
    pub time_fired: String,
    pub context: Context,
}

///	This is part of HassEvent 
#[derive(Debug, Deserialize, PartialEq)]
pub struct EventData {
    entity_id: String,
    new_state: Option<HassEntity>,
    old_state: Option<HassEntity>,
}
