use serde_derive::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

/// This object represents the collection of Home Assistant Services
///
/// This will get a dump of the current services in Home Assistant.
/// [Fetch Services](https://developers.home-assistant.io/docs/api/websocket/#fetching-services)
#[derive(Debug, Deserialize, PartialEq)]
pub struct HassServices(Domain);

type Domain = HashMap<String, ServiceName>;
type ServiceName = HashMap<String, HassService>;

/// This object represents the Home Assistant Service
///
/// This will get a dump of the current services in Home Assistant.
/// [Fetch Services](https://developers.home-assistant.io/docs/api/websocket/#fetching-services)
#[derive(Debug, Deserialize, PartialEq)]
pub struct HassService {
    description: String,
    fields: FieldName,
}

type FieldName = HashMap<String, Field>;

///This is part of HassService
#[derive(Debug, Deserialize, PartialEq)]
pub struct Field {
    description: String,
    example: Value,
}
