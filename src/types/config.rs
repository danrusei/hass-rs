use serde::Deserialize;

/// This object represents the Home Assistant Config
///
/// This will get a dump of the current config in Home Assistant.
/// [Fetch Config](https://developers.home-assistant.io/docs/api/websocket/#fetching-config)
#[derive(Debug, Deserialize, PartialEq)]
pub struct HassConfig {
    pub latitude: f32,
    pub longitude: f32,
    pub elevation: u32,
    pub unit_system: UnitSystem,
    pub location_name: String,
    pub time_zone: String,
    pub components: Vec<String>,
    pub config_dir: String,
    pub whitelist_external_dirs: Vec<String>,
    pub version: String,
    pub config_source: String,
    pub safe_mode: bool,
    pub external_url: Option<String>,
    pub internal_url: Option<String>,
}

/// This is part of HassConfig
#[derive(Debug, Deserialize, PartialEq)]
pub struct UnitSystem {
    pub length: String,
    pub mass: String,
    pub pressure: String,
    pub temperature: String,
    pub volume: String,
}
