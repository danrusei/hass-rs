use serde::Deserialize;
use serde_json::Value;

/// This object represents the Home Assistant Entity
///
/// [Entity](https://developers.home-assistant.io/docs/core/entity/)
#[derive(Debug, Deserialize, PartialEq)]
pub struct HassEntity {
    pub entity_id: String,
    pub state: String,
    pub last_changed: String,
    pub last_updated: String,
    pub attributes: Value,
    pub context: Context,
}

/// General construct used by HassEntity and HassEvent
#[derive(Debug, Deserialize, PartialEq)]
pub struct Context {
    pub id: String,
    pub parent_id: Option<String>,
    pub user_id: Option<String>,
}
