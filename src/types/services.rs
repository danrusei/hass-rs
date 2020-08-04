use serde_json::Value;
use serde_derive::Deserialize;
use std::collections::HashMap;

//This is the HassService
#[derive(Debug, Deserialize, PartialEq)]
pub struct HassService {
    domain: HashMap<String, Value>,
}
