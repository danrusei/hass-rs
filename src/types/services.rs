use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;

/// This object represents the collection of Home Assistant Services
///
/// This will get a dump of the current services in Home Assistant.
/// [Fetch Services](https://developers.home-assistant.io/docs/api/websocket/#fetching-services)
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct HassServices(pub Domain);

/// This is part of HassServices
pub type Domain = HashMap<String, ServiceName>;

/// This is part of HassServices
pub type ServiceName = HashMap<String, HassService>;

/// This object represents the Home Assistant Service
///
/// This will get a dump of the current services in Home Assistant.
/// [Fetch Services](https://developers.home-assistant.io/docs/api/websocket/#fetching-services)
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct HassService {
    pub name: Option<String>,
    pub description: Option<String>,
    pub fields: FieldName,
    //pub response: Option<bool>,
}

/// This is part of HassService
pub type FieldName = HashMap<String, Field>;

///This is part of HassService
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Field {
    pub name: Option<String>,
    pub description: Option<String>,
    pub example: Option<Value>,
}

impl fmt::Display for HassServices {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "HassServices {{")?;
        writeln!(f, "  domain: {{")?;
        for (domain_name, service_name) in &self.0 {
            writeln!(f, "    {}: {{", domain_name)?;
            for (service_name, hass_service) in service_name {
                writeln!(f, "      {}: {{", service_name)?;
                writeln!(f, "        name: {:?},", hass_service.name)?;
                writeln!(f, "        description: {:?},", hass_service.description)?;
                writeln!(f, "        fields: {{")?;
                for (field_name, field) in &hass_service.fields {
                    writeln!(f, "          {}: {{", field_name)?;
                    writeln!(f, "            name: {:?},", field.name)?;
                    writeln!(f, "            description: {:?},", field.description)?;
                    writeln!(f, "            example: {:?},", field.example)?;
                    writeln!(f, "          }},")?;
                }
                writeln!(f, "        }},")?;
                writeln!(f, "      }},")?;
            }
            writeln!(f, "    }},")?;
        }
        writeln!(f, "  }},")?;
        write!(f, "}}")?;
        Ok(())
    }
}

impl fmt::Display for HassService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "    name: {:?},", self.name)?;
        writeln!(f, "    description: {:?},", self.description)?;
        writeln!(f, "    fields: {{")?;
        for (field_name, field) in &self.fields {
            writeln!(f, "      {}: {{", field_name)?;
            writeln!(f, "          name: {:?},", field.name)?;
            writeln!(f, "          description: {:?},", field.description)?;
            writeln!(f, "          example: {:?},", field.example)?;
            writeln!(f, "          }},")?;
        }
        Ok(())
    }
}

impl HassServices {
    pub fn list_domains(&self) -> Vec<String> {
        self.0.keys().cloned().collect()
    }

    pub fn list_services(&self, user_provided_domain: &str) -> Option<Vec<(String, &HassService)>> {
        self.0.get(user_provided_domain).map(|service_name| {
            service_name
                .iter()
                .map(|(name, hass_service)| (name.clone(), hass_service))
                .collect()
        })
    }
}
