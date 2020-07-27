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

use crate::command::{Command, Auth, Ping};
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
        let response = self.command(auth_req).await?; 

        //Check if the authetication was succefully, should receive {"type": "auth_ok"}
        let value = match response {
            Response::AuthInit(v) => v,
            _ => return Err(HassError::UnknownPayloadReceived),
        };

        match value.msg_type.as_str() {
            "auth_ok" => return Ok(()),
            "auth_invalid" => return Err(HassError::AuthenticationFailed),
            _ => return Err(HassError::UnknownPayloadReceived),
        }

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
        let ping_req = Command::Ping(Ping {
            id: Some(0),
            msg_type: "ping".to_owned(),
        });
        let response = self.command(ping_req).await?; 

        //Check the response, if the Pong was received
         let pong = match response {
            Response::Pong(v) => v,
            Response::ResultError(err) => return Err(HassError::HassGateway(err)),
            _ => return Err(HassError::UnknownPayloadReceived),
        };

        Ok(pong.msg_type)
    }

    async fn command(&mut self, cmd: Command) -> HassResult<Response> {

        // Send the auth command to gateway
        self.gateway
            .as_mut()
            .expect("No gateway provided")
            .to_gateway
            .send(cmd)
            .await
            .map_err(|_| HassError::ConnectionClosed)?;

        // Receive auth response event from the gateway
        let response = self
            .gateway
            .as_mut()
            .expect("No gateway provided")
            .from_gateway
            .next()
            .await
            .ok_or_else(|| HassError::ConnectionClosed)?;
        
        response
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
