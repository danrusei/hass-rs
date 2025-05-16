use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct HassRegistryDevice {
    pub area_id: Option<String>,
    pub configuration_url: Option<String>,
    pub config_entries: Vec<String>,
    pub config_entries_subentries: HashMap<String, Vec<Value>>,
    pub connections: Vec<Vec<String>>,
    pub created_at: f64,
    pub disabled_by: Option<String>,
    pub entry_type: Option<String>,
    pub hw_version: Option<String>,
    pub id: String,
    pub identifiers: Vec<Vec<String>>,
    pub labels: Vec<String>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub model_id: Option<String>,
    pub modified_at: f64,
    pub name_by_user: Option<String>,
    pub name: String,
    pub primary_config_entry: Option<String>,
    pub serial_number: Option<String>,
    pub sw_version: Option<String>,
    pub via_device_id: Option<String>,
}