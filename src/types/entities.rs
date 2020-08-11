use serde_derive::Deserialize;

/// This object represents the Home Assistant Entity
///
/// [Entity](https://developers.home-assistant.io/docs/core/entity/)
#[derive(Debug, Deserialize, PartialEq)]
pub struct HassEntity {
    pub entity_id: String,
    pub state: String,
    pub last_changed: String,
    pub last_updated: String,
    pub attributes: HassEntityAttributeBase,
    pub context: Context,
}

///	This is part of HassEntity
#[derive(Debug, Deserialize, PartialEq)]
pub struct HassEntityAttributeBase {
    #[serde(default)]
    pub friendly_name: String,
    #[serde(default)]
    pub unit_of_measurement: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub entity_picture: String,
    #[serde(default)]
    pub supported_features: u32,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub assumed_state: bool,
    #[serde(default)]
    pub device_class: String,
}

/// General construct used by HassEntity and HassEvent
#[derive(Debug, Deserialize, PartialEq)]
pub struct Context {
    pub id: String,
    pub parent_id: Option<String>,
    pub user_id: Option<String>,
}
