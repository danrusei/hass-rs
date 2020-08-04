use serde_derive::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
pub struct HassEntity {
    entity_id: String,
    state: String,
    last_changed: String,
    last_updated: String,
    attributes: HassEntityAttributeBase,
    context: Context,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct HassEntityAttributeBase {
    #[serde(default)]
    friendly_name: String,
    #[serde(default)]
    unit_of_measurement: String,
    #[serde(default)]
    icon: String,
    #[serde(default)]
    entity_picture: String,
    #[serde(default)]
    supported_features: u32,
    #[serde(default)]
    hidden: bool,
    #[serde(default)]
    assumed_state: bool,
    #[serde(default)]
    device_class: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Context {
    pub(crate) id: String,
    pub(crate) parent_id: Option<String>,
    pub(crate) user_id: Option<String>,
}
