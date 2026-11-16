use crate::types::{Context, HassEntity};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, fmt};

/// This object represents the Home Assistant Event
///
/// received when the client is subscribed to
/// [Subscribe to events](https://developers.home-assistant.io/docs/api/websocket/#subscribe-to-events)
///
/// This is created against StateChangedEvent, may not work with other event types, although
/// extra fields are supported, so with some work it could be used for other events
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
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl fmt::Display for HassEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "HassEvent {{")?;
        writeln!(f, "  event_type: {},", self.event_type)?;
        writeln!(f, "  data: {{")?;
        writeln!(f, "    entity_id: {:?},", self.data.entity_id)?;
        writeln!(f, "    new_state: {:?},", self.data.new_state)?;
        writeln!(f, "    old_state: {:?},", self.data.old_state)?;
        writeln!(f, "  }},")?;
        writeln!(f, "  origin: {},", self.origin)?;
        writeln!(f, "  time_fired: {},", self.time_fired)?;
        writeln!(f, "  context: {:?},", self.context)?;
        write!(f, "}}")?;
        Ok(())
    }
}
