//! # Async HomeAssistant Websocket Library
//!
//! Hass-rs is a HomeAssistant websokcte api library
//! based on https://developers.home-assistant.io/docs/api/websocket specifications

mod command;
pub mod config;
mod errors;
pub mod events;
pub mod response;
mod runtime;
mod wsconn;

use crate::command::{Auth, Command, Ping};
use crate::config::{Config, ConnectionOptions};
use crate::errors::{HassError, HassResult};
use crate::response::{Response, WSEvent};
use crate::wsconn::WsConn;

use futures::StreamExt;
use url;

// Client defines client connection
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
        let ping_req = Command::Ping(Ping {
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

    pub async fn subscribe_event<F>(&mut self, event_name: &str, callback: F)
    where
        F: Fn(WSEvent) + Send + 'static,
    {
        match self
            .gateway
            .as_mut()
            .expect("no gateway found")
            .subscribe_message(event_name, callback)
            .await
        {
            Ok(v) => println!("Got {} on the event {} subscription request", v, event_name),
            Err(err) => println!(
                "Got this error {} while trying to subscribe to {}",
                err, event_name
            ),
        };

        //TODO this has to be removed, is just for testing purpose
        //    loop {
        //     let event = self
        //         .gateway
        //         .as_mut()
        //         .expect("test")
        //         .from_gateway
        //         .next()
        //         .await
        //         .ok_or_else(|| HassError::ConnectionClosed)
        //         .unwrap();

        //         dbg!(event);
        //    }
    }
}
