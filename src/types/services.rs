use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

/// This object represents the collection of Home Assistant Services
///
/// This will get a dump of the current services in Home Assistant.
/// [Fetch Services](https://developers.home-assistant.io/docs/api/websocket/#fetching-services)
#[derive(Debug, Deserialize, PartialEq)]
pub struct HassServices(pub Domain);

/// This is part of HassServices
pub type Domain = HashMap<String, ServiceName>;

/// This is part of HassServices
pub type ServiceName = HashMap<String, HassService>;

/// This object represents the Home Assistant Service
///
/// This will get a dump of the current services in Home Assistant.
/// [Fetch Services](https://developers.home-assistant.io/docs/api/websocket/#fetching-services)
#[derive(Debug, Deserialize, PartialEq)]
pub struct HassService {
    pub description: String,
    pub fields: FieldName,
}

/// This is part of HassService
pub type FieldName = HashMap<String, Field>;

///This is part of HassService
#[derive(Debug, Deserialize, PartialEq)]
pub struct Field {
    pub description: String,
    pub example: Option<Value>,
}
