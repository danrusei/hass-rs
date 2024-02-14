use serde::Deserialize;
use serde_json::Value;
use std::fmt;

/// This object represents the Home Assistant Entity
///
/// [Entity](https://developers.home-assistant.io/docs/core/entity/)
#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct HassEntity {
    pub entity_id: String,
    pub state: String,
    pub last_changed: String,
    pub last_updated: String,
    pub attributes: Value,
    pub context: Context,
}

/// General construct used by HassEntity and HassEvent
#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct Context {
    pub id: String,
    pub parent_id: Option<String>,
    pub user_id: Option<String>,
}

impl fmt::Display for HassEntity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HassEntity {{\n")?;
        write!(f, "  entity_id: {},\n", self.entity_id)?;
        write!(f, "  state: {},\n", self.state)?;
        write!(f, "  last_changed: {},\n", self.last_changed)?;
        write!(f, "  last_updated: {},\n", self.last_updated)?;
        write!(f, "  attributes: {:?},\n", self.attributes)?;
        write!(f, "  context: {:?},\n", self.context)?;
        write!(f, "}}")?;
        Ok(())
    }
}
