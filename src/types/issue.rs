use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct HassIssue {
    pub breaks_in_ha_version: Option<String>,
    pub created: String,
    pub dismissed_version: Option<String>,
    pub domain: String,
    pub ignored: bool,
    pub is_fixable: bool,
    pub issue_domain: Option<String>,
    pub issue_id: String,
    pub learn_more_url: Option<String>,
    pub severity: Option<String>,
    pub translation_key: Option<String>,
    pub translation_placeholders: Option<HashMap<String, Value>>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct HassIssues {
    pub issues: Vec<HassIssue>,
}
