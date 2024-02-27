use crate::types::{Context, HassEntity};
use serde::{Deserialize, Serialize};
use std::fmt;

/// This object represents the Home Assistant Event
///
/// received when the client is subscribed to
/// [Subscribe to events](https://developers.home-assistant.io/docs/api/websocket/#subscribe-to-events)
///
///This is created against StateChangedEvent, may not work with other event types
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct HassEvent {
    pub data: EventData,
    pub event_type: String,
    pub time_fired: String,
    pub origin: String,
    pub context: Context,
}

/// This is part of HassEvent
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct EventData {
    pub entity_id: Option<String>,
    pub new_state: Option<HassEntity>,
    pub old_state: Option<HassEntity>,
}

impl fmt::Display for HassEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HassEvent {{\n")?;
        write!(f, "  event_type: {},\n", self.event_type)?;
        write!(f, "  data: {{\n")?;
        write!(f, "    entity_id: {:?},\n", self.data.entity_id)?;
        write!(f, "    new_state: {:?},\n", self.data.new_state)?;
        write!(f, "    old_state: {:?},\n", self.data.old_state)?;
        write!(f, "  }},\n")?;
        write!(f, "  origin: {},\n", self.origin)?;
        write!(f, "  time_fired: {},\n", self.time_fired)?;
        write!(f, "  context: {:?},\n", self.context)?;
        write!(f, "}}")?;
        Ok(())
    }
}
