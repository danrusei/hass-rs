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
        write!(f, "HassPanel {{\n")?;
        write!(f, "  component_name: {},\n", self.component_name)?;
        write!(f, "  config: {:?},\n", self.config)?;
        write!(f, "  icon: {:?},\n", self.icon)?;
        write!(f, "  require_admin: {},\n", self.require_admin)?;
        write!(f, "  title: {:?},\n", self.title)?;
        write!(f, "  url_path: {},\n", self.url_path)?;
        write!(f, "}}")?;
        Ok(())
    }
}

impl fmt::Display for HassPanelConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HassPanelConfig {{\n")?;
        write!(f, "  custom_panel: {:?},\n", self.custom_panel)?;
        write!(f, "  mode: {:?},\n", self.mode)?;
        write!(f, "  title: {:?},\n", self.title)?;
        write!(f, "}}")?;
        Ok(())
    }
}

impl fmt::Display for HassCustomPanelConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HassCustomPanelConfig {{\n")?;
        write!(f, "  embed_iframe: {},\n", self.embed_iframe)?;
        write!(f, "  module_url: {:?},\n", self.module_url)?;
        write!(f, "  js_url: {:?},\n", self.js_url)?;
        write!(f, "  name: {},\n", self.name)?;
        write!(f, "  trust_external: {},\n", self.trust_external)?;
        write!(f, "}}")?;
        Ok(())
    }
}
