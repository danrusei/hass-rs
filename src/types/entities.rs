use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

/// This object represents the Home Assistant Entity
///
/// [Entity](https://developers.home-assistant.io/docs/core/entity/)
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct HassEntity {
    pub entity_id: String,
    pub last_changed: String,
    pub state: String,
    pub attributes: Value,
    pub last_updated: String,
    pub context: Option<Context>, //changed
}

/// General construct used by HassEntity and HassEvent
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Context {
    pub id: String,
    pub parent_id: Option<String>,
    pub user_id: Option<String>,
}

impl fmt::Display for HassEntity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "HassEntity {{")?;
        writeln!(f, "  entity_id: {},", self.entity_id)?;
        writeln!(f, "  state: {},", self.state)?;
        writeln!(f, "  last_changed: {},", self.last_changed)?;
        writeln!(f, "  last_updated: {},", self.last_updated)?;
        writeln!(f, "  attributes: {:?},", self.attributes)?;
        writeln!(f, "  context: {:?},", self.context)?;
        write!(f, "}}")?;
        Ok(())
    }
}
