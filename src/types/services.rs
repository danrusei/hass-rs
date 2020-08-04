use serde_derive::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

//This is the HassService
#[derive(Debug, Deserialize, PartialEq)]
pub struct HassServices(Domain);

type Domain = HashMap<String, ServiceName>;
type ServiceName = HashMap<String, HassService>;

#[derive(Debug, Deserialize, PartialEq)]
pub struct HassService {
    description: String,
    fields: FieldName,
}

type FieldName = HashMap<String, Field>;

#[derive(Debug, Deserialize, PartialEq)]
pub struct Field {
    description: String,
    example: Value,
}
