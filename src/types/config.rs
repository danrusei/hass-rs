use serde_derive::Deserialize;

/// This object represents the Home Assistant Config
/// 
/// [Fetch Config](https://developers.home-assistant.io/docs/api/websocket/#fetching-config)
#[derive(Debug, Deserialize, PartialEq)]
pub struct HassConfig {
    latitude: f32,
    longitude: f32,
    elevation: u32,
    unit_system: UnitSystem,
    location_name: String,
    time_zone: String,
    components: Vec<String>,
    config_dir: String,
    whitelist_external_dirs: Vec<String>,
    version: String,
    config_source: String,
    safe_mode: bool,
    external_url: Option<String>,
    internal_url: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct UnitSystem {
    length: String,
    mass: String,
    pressure: String,
    temperature: String,
    volume: String,
}
