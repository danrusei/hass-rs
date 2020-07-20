//! # Async HomeAssistant Websocket Library
//!
//! Hass-rs is a HomeAssistant websokcte api library
//! based on https://developers.home-assistant.io/docs/api/websocket specifications

mod errors;
mod messages;
mod runtime;
mod wsconn;

use crate::errors::{HassError, HassResult};
use crate::messages::Response;
use crate::wsconn::WsConn;

use url;

// Client defines client connection
#[derive(Debug)]
pub struct HassClient {
    pub(crate) opts: ConnectionOptions,
    pub(crate) token: String,
    pub(crate) conn: Option<WsConn>,
}

impl HassClient {
    //Create a new Hass Client
    pub fn new(host: &str, port: u16) -> HassClient {
        HassClient {
            opts: ConnectionOptions {
                host: host.to_owned(),
                port: port,
                ..Default::default()
            },
            token: String::new(),
            conn: None
        }
    }

    pub fn with_ssl(mut self) -> HassClient {
        self.opts.ssl = true;
        self
    }

    pub fn create_url(&self) -> String {
        let protocol = if self.opts.ssl { "wss" } else { "ws" };
        format!("{}://{}:{}/api/websocket", protocol, self.opts.host, self.opts.port)
    }

    pub async fn connect(&mut self, token: &str) -> HassResult<HassClient> {
        self.token = token.to_owned();
        let url = url::Url::parse(&self.create_url()).expect("failed to parse the url");
        let conn = WsConn::connect(url).await?;
        // use token to authenthicate
        //example use run to authenthicate
        todo!()
    }

    pub fn auth(&self) -> HassClient {
        todo!()
    
    }

    pub fn command(&self) -> HassResult<()> {
        //public usage, to send commands over Hass websocket
        // use run to send commands
        todo!()
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

