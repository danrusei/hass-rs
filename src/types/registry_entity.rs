use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct HassRegistryEntity {
    pub area_id: Option<String>,
    pub categories: HashMap<String, Value>,
    pub config_entry_id: Option<String>,
    pub config_subentry_id: Option<String>,
    pub created_at: f64,
    pub device_id: Option<String>,
    pub disabled_by: Option<String>,
    pub entity_category: Option<String>,
    pub entity_id: String,
    pub has_entity_name: bool,
    pub hidden_by: Option<String>,
    pub icon: Option<String>,
    pub id: String,
    pub labels: Vec<String>,
    pub modified_at: f64,
    pub name: Option<String>,
    pub options: HashMap<String, Value>,
    pub original_name: Option<String>,
    pub platform: String,
    pub translation_key: Option<String>,
    pub unique_id: String,
}