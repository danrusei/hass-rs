//! # Async HomeAssistant Websocket Library
//!
//! Hass-rs is a HomeAssistant Websocket API client library.
//! 
//! It is based on the [official API specifications](https://developers.home-assistant.io/docs/api/websocket).
//! 
//! # Configuring async runtime
//! hass_rs supports `async-std` and `tokio` runtimes, by default it uses `async-std`,
//! to use `tokio` change the feature flags in `Cargo.toml`
//!
//! ```toml
//! [dependencies.hass_rs]
//! version = "0.1.0"
//! default-features = false
//! features = ["tokio-runtime"]
//! ```
//!
//!
//! # Example usage
//! It sends a ping and receive a pong
//!
//! ```rust
//! #[async_std::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!
//!   TODO(Soon)
//!
//!     Ok(())
//! }
//! ```

pub mod errors;
mod runtime;
pub mod types;
mod wsconn;

use crate::errors::{HassError, HassResult};
use crate::types::{
    Ask, Auth, CallService, Command, ConnConfig, ConnectionOptions, HassConfig, HassEntity,
    HassServices, Response, WSEvent,
};
use crate::wsconn::WsConn;

use futures::StreamExt;
use serde_json::Value;
use url;

// Client defines client connection
pub struct HassClient {
    pub(crate) opts: ConnectionOptions,
    pub(crate) token: String,
    pub(crate) gateway: Option<WsConn>,
}

impl HassClient {
    //Create a new Hass Client
    pub fn new(config: ConnConfig) -> Self {
        HassClient {
            opts: ConnectionOptions {
                host: config.host,
                port: config.port,
                ssl: false,
            },
            token: config.token,
            gateway: None,
        }
    }

    pub async fn connect(&mut self) -> HassResult<()> {
        let url = url::Url::parse(&self.create_url()).expect("failed to parse the url");
        self.gateway = Some(WsConn::connect(url).await?);
        self.authenticate().await?;
        Ok(())
    }

    async fn authenticate(&mut self) -> HassResult<()> {
        // Auth Request from Gateway { "type": "auth_required"}
        let _ = self
            .gateway
            .as_mut()
            .expect("No gateway provided")
            .from_gateway
            .next()
            .await
            .ok_or_else(|| HassError::ConnectionClosed)?;

        //Authenticate with Command::AuthInit and payload {"type": "auth", "access_token": "XXXXX"}
        let auth_req = Command::AuthInit(Auth {
            msg_type: "auth".to_owned(),
            access_token: self.token.to_owned(),
        });
        let response = self
            .gateway
            .as_mut()
            .expect("No gateway found")
            .command(auth_req)
            .await?;

        //Check if the authetication was succefully, should receive {"type": "auth_ok"}
        match response {
            Response::AuthOk(_) => Ok(()),
            _ => return Err(HassError::UnknownPayloadReceived),
        }

        // match value.msg_type.as_str() {
        //     "auth_ok" => return Ok(()),
        //     "auth_invalid" => return Err(HassError::AuthenticationFailed),
        //     _ => return Err(HassError::UnknownPayloadReceived),
        // }
    }

    pub fn with_ssl(mut self) -> HassClient {
        self.opts.ssl = true;
        self
    }

    fn create_url(&self) -> String {
        let protocol = if self.opts.ssl { "wss" } else { "ws" };
        format!(
            "{}://{}:{}/api/websocket",
            protocol, self.opts.host, self.opts.port
        )
    }

    pub async fn ping(&mut self) -> HassResult<String> {
        //Send Ping command and expect Pong
        let ping_req = Command::Ping(Ask {
            id: Some(0),
            msg_type: "ping".to_owned(),
        });
        let response = self
            .gateway
            .as_mut()
            .expect("no gateway found")
            .command(ping_req)
            .await?;

        //Check the response, if the Pong was received
        match response {
            Response::Pong(_v) => Ok("pong".to_owned()),
            Response::Result(err) => return Err(HassError::ReponseError(err)),
            _ => return Err(HassError::UnknownPayloadReceived),
        }
    }

    pub async fn subscribe_event<F>(&mut self, event_name: &str, callback: F) -> HassResult<String>
    where
        F: Fn(WSEvent) + Send + 'static,
    {
        self.gateway
            .as_mut()
            .expect("no gateway found")
            .subscribe_message(event_name, callback)
            .await
    }
    pub async fn unsubscribe_event(&mut self, subscription_id: u64) -> HassResult<String> {
        self.gateway
            .as_mut()
            .expect("no gateway found")
            .unsubscribe_message(subscription_id)
            .await
    }

    pub async fn get_config(&mut self) -> HassResult<HassConfig> {
        //Send GetConfig command and expect Pong
        let config_req = Command::GetConfig(Ask {
            id: Some(0),
            msg_type: "get_config".to_owned(),
        });
        let response = self
            .gateway
            .as_mut()
            .expect("no gateway found")
            .command(config_req)
            .await?;

        match response {
            Response::Result(data) => {
                match data.success {
                    true => {
                        //TODO handle the error properly
                        let config: HassConfig =
                            serde_json::from_value(data.result.unwrap()).unwrap();
                        return Ok(config);
                    }
                    false => return Err(HassError::ReponseError(data)),
                }
            }
            _ => return Err(HassError::UnknownPayloadReceived),
        }
    }

    pub async fn get_states(&mut self) -> HassResult<Vec<HassEntity>> {
        //Send GetStates command and expect a number of Entities
        let states_req = Command::GetStates(Ask {
            id: Some(0),
            msg_type: "get_states".to_owned(),
        });
        let response = self
            .gateway
            .as_mut()
            .expect("no gateway found")
            .command(states_req)
            .await?;

        // TODO - problem Entity atributes could be different, so this is wrong
        // have to make it Value, and based on entity_id deserialize differently
        // maybe this has to be handled by the user, add to example folder
        match response {
            Response::Result(data) => {
                match data.success {
                    true => {
                        //TODO handle the error properly
                        let states: Vec<HassEntity> =
                            serde_json::from_value(data.result.unwrap()).unwrap();
                        return Ok(states);
                    }
                    false => return Err(HassError::ReponseError(data)),
                }
            }
            _ => return Err(HassError::UnknownPayloadReceived),
        }
    }
    pub async fn get_services(&mut self) -> HassResult<HassServices> {
        //Send GetStates command and expect a number of Entities
        let services_req = Command::GetServices(Ask {
            id: Some(0),
            msg_type: "get_services".to_owned(),
        });
        let response = self
            .gateway
            .as_mut()
            .expect("no gateway found")
            .command(services_req)
            .await?;

        match response {
            Response::Result(data) => {
                match data.success {
                    true => {
                        //TODO handle the error properly
                        let services: HassServices =
                            serde_json::from_value(data.result.unwrap()).unwrap();
                        return Ok(services);
                    }
                    false => return Err(HassError::ReponseError(data)),
                }
            }
            _ => return Err(HassError::UnknownPayloadReceived),
        }
    }
    pub async fn call_service(
        &mut self,
        domain: String,
        service: String,
        service_data: Option<Value>,
    ) -> HassResult<String> {
        //Send GetStates command and expect a number of Entities
        let services_req = Command::CallService(CallService {
            id: Some(0),
            msg_type: "call_service".to_owned(),
            domain,
            service,
            service_data,
        });
        let response = self
            .gateway
            .as_mut()
            .expect("no gateway found")
            .command(services_req)
            .await?;

        match response {
            Response::Result(data) => match data.success {
                true => return Ok("command executed successfully".to_owned()),
                false => return Err(HassError::ReponseError(data)),
            },
            _ => return Err(HassError::UnknownPayloadReceived),
        }
    }
}
