use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

pub type HassPanels = HashMap<String, HassPanel>;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct HassPanel {
    pub component_name: String,
    pub config: Option<HassPanelConfig>,
    pub icon: Option<String>,
    pub require_admin: bool,
    pub title: Option<String>,
    pub url_path: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct HassPanelConfig {
    #[serde(rename = "_panel_custom")]
    pub custom_panel: Option<HassCustomPanelConfig>,
    pub mode: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct HassCustomPanelConfig {
    pub embed_iframe: bool,
    pub module_url: Option<String>,
    pub js_url: Option<String>,
    pub name: String,
    pub trust_external: bool,
}

impl fmt::Display for HassPanel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "HassPanel {{")?;
        writeln!(f, "  component_name: {},", self.component_name)?;
        writeln!(f, "  config: {:?},", self.config)?;
        writeln!(f, "  icon: {:?},", self.icon)?;
        writeln!(f, "  require_admin: {},", self.require_admin)?;
        writeln!(f, "  title: {:?},", self.title)?;
        writeln!(f, "  url_path: {},", self.url_path)?;
        write!(f, "}}")?;
        Ok(())
    }
}

impl fmt::Display for HassPanelConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "HassPanelConfig {{")?;
        writeln!(f, "  custom_panel: {:?},", self.custom_panel)?;
        writeln!(f, "  mode: {:?},", self.mode)?;
        writeln!(f, "  title: {:?},", self.title)?;
        write!(f, "}}")?;
        Ok(())
    }
}

impl fmt::Display for HassCustomPanelConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "HassCustomPanelConfig {{")?;
        writeln!(f, "  embed_iframe: {},", self.embed_iframe)?;
        writeln!(f, "  module_url: {:?},", self.module_url)?;
        writeln!(f, "  js_url: {:?},", self.js_url)?;
        writeln!(f, "  name: {},", self.name)?;
        writeln!(f, "  trust_external: {},", self.trust_external)?;
        write!(f, "}}")?;
        Ok(())
    }
}
