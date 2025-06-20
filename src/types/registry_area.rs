use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct HassRegistryArea {
    pub aliases: Vec<String>,
    pub area_id: String,
    pub floor_id: Option<String>,
    pub humidity_entity_id: Option<String>,
    pub icon: Option<String>,
    pub labels: Vec<String>,
    pub name: String,
    pub picture: Option<String>,
    pub temperature_entity_id: Option<String>,
    pub created_at: f64,
    pub modified_at: f64,
}
