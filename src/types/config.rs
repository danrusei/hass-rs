use serde::{Deserialize, Serialize};
use std::fmt;

/// This object represents the Home Assistant Config
///
/// This will get a dump of the current config in Home Assistant.
/// [Fetch Config](https://developers.home-assistant.io/docs/api/websocket/#fetching-config)
#[derive(Debug, Serialize, Deserialize, PartialEq)]
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
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct UnitSystem {
    pub length: String,
    pub mass: String,
    pub pressure: String,
    pub temperature: String,
    pub volume: String,
}

impl fmt::Display for HassConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "HassConfig {{")?;
        writeln!(f, "  latitude: {},", self.latitude)?;
        writeln!(f, "  longitude: {},", self.longitude)?;
        writeln!(f, "  elevation: {},", self.elevation)?;
        writeln!(f, "  unit_system: {:?},", self.unit_system)?;
        writeln!(f, "  location_name: {},", self.location_name)?;
        writeln!(f, "  time_zone: {},", self.time_zone)?;
        writeln!(f, "  components: {:?},", self.components)?;
        writeln!(f, "  config_dir: {},", self.config_dir)?;
        writeln!(
            f,
            "  whitelist_external_dirs: {:?},",
            self.whitelist_external_dirs
        )?;
        writeln!(f, "  version: {},", self.version)?;
        writeln!(f, "  config_source: {},", self.config_source)?;
        writeln!(f, "  safe_mode: {},", self.safe_mode)?;
        writeln!(f, "  external_url: {:?},", self.external_url)?;
        writeln!(f, "  internal_url: {:?},", self.internal_url)?;
        write!(f, "}}")?;
        Ok(())
    }
}

impl fmt::Display for UnitSystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "UnitSystem {{")?;
        writeln!(f, "  length: {},", self.length)?;
        writeln!(f, "  mass: {},", self.mass)?;
        writeln!(f, "  pressure: {},", self.pressure)?;
        writeln!(f, "  temperature: {},", self.temperature)?;
        writeln!(f, "  volume: {},", self.volume)?;
        write!(f, "}}")?;
        Ok(())
    }
}
