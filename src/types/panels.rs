use std::collections::HashMap;

use serde::Deserialize;

pub type HassPanels = HashMap<String, HassPanel>;

#[derive(Debug, Deserialize, PartialEq)]
pub struct HassPanel {
    pub component_name: String,
    pub config: Option<HassPanelConfig>,
    pub icon: Option<String>,
    pub require_admin: bool,
    pub title: Option<String>,
    pub url_path: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct HassPanelConfig {
    #[serde(rename = "_panel_custom")]
    pub custom_panel: Option<HassCustomPanelConfig>,
    pub mode: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct HassCustomPanelConfig {
    pub embed_iframe: bool,
    pub module_url: Option<String>,
    pub js_url: Option<String>,
    pub name: String,
    pub trust_external: bool,
}
