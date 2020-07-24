//! # Async HomeAssistant Websocket Library
//!
//! Hass-rs is a HomeAssistant websokcte api library
//! based on https://developers.home-assistant.io/docs/api/websocket specifications

mod command;
pub mod config;
mod errors;
mod response;
mod runtime;
mod wsconn;

use crate::command::{Auth, Command};
use crate::config::Config;
use crate::errors::{HassError, HassResult};
use crate::response::Response;
use crate::wsconn::WsConn;

use futures::{SinkExt, StreamExt};
use url;

// Client defines client connection
#[derive(Debug)]
pub struct HassClient {
    pub(crate) opts: ConnectionOptions,
    pub(crate) token: String,
    pub(crate) gateway: Option<WsConn>,
}

impl HassClient {
    //Create a new Hass Client
    pub fn new(config: Config) -> Self {
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
        // Auth Request
        let _ = self
            .gateway
            .as_mut()
            .expect("No gateway provided")
            .from_gateway
            .next()
            .await
            .ok_or_else(|| HassError::ConnectionClosed)?;

        //Authenticate with auth {"type": "auth", "access_token": "XXXXX"}
        let auth_req = Command::AuthInit(Auth {
            msg_type: "auth".to_owned(),
            access_token: self.token.to_owned(),
        });

        // Send the auth command to gateway
        self.gateway
            .as_mut()
            .expect("No gateway provided")
            .to_gateway
            .send(auth_req)
            .await
            .map_err(|_| HassError::ConnectionClosed)?;

        // Receive auth response event from the gatewat
        let auth_response = self
            .gateway
            .as_mut()
            .expect("No gateway provided")
            .from_gateway
            .next()
            .await
            .ok_or_else(|| HassError::ConnectionClosed)?;

        let value = match auth_response {
            Ok(Response::AuthInit(v)) => v,
            _ => return Err(HassError::UnknownPayloadReceived.into()),
        };

        dbg!(value);
        Ok(())
    }

    pub fn with_ssl(mut self) -> HassClient {
        self.opts.ssl = true;
        self
    }

    pub fn create_url(&self) -> String {
        let protocol = if self.opts.ssl { "wss" } else { "ws" };
        format!(
            "{}://{}:{}/api/websocket",
            protocol, self.opts.host, self.opts.port
        )
    }

    pub async fn command(&mut self, payload: &str) -> HassResult<()> {
        let cmd = Command::Msg(10, payload.into());

        self.gateway
            .as_mut()
            .expect("no connection to gateway ")
            .to_gateway
            .send(cmd)
            .await
            .expect("Could not send the command");

        Ok(())
    }
}

#[derive(Debug)]
pub struct ConnectionOptions {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) ssl: bool,
}

impl Default for ConnectionOptions {
    fn default() -> ConnectionOptions {
        ConnectionOptions {
            host: String::from("localhost"),
            port: 8123,
            ssl: false,
        }
    }
}
