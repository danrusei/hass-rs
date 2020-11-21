//! Home Assistant client implementation
//!
//! Provides an async connect and methods for issuing the supported commands.

use crate::types::{
    Ask, Auth, CallService, Command, HassConfig, HassEntity, HassServices, Response, WSEvent,
};
use crate::{HassError, HassResult, WsConn};

use futures::StreamExt;
use serde_json::Value;
use url;

/// Established connection with a Home Assistant WebSocket server.
///
/// Backed by async_tungstenite, that provides async Websocket bindings,
/// that can be used with non-blocking/asynchronous TcpStreams.
/// It Supports both "async-std" and "tokio" runtimes.
///
/// Requests are issued using the various methods of `Client`.  
pub struct HassClient {
    pub(crate) gateway: WsConn,
}

/// establish the websocket connection to Home Assistant server
pub async fn connect(host: &str, port: u16) -> HassResult<HassClient> {
    let addr = format!("ws://{}:{}/api/websocket", host, port);
    let url = url::Url::parse(&addr)?;
    let gateway = WsConn::connect(url).await?;
    Ok(HassClient { gateway })
}

impl HassClient {
    /// authenticate the session using a long-lived access token
    ///
    /// When a client connects to the server, the server sends out auth_required.
    /// The first message from the client should be an auth message. You can authorize with an access token.
    /// If the client supplies valid authentication, the authentication phase will complete by the server sending the auth_ok message.
    /// If the data is incorrect, the server will reply with auth_invalid message and disconnect the session.
    ///# Examples
    ///
    /// Demonstrates basic usage.
    ///
    /// ```no_run
    /// use hass_rs::client;
    ///
    /// #[async_std::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>>{
    ///     let mut client = client::connect("localhost", 8123).await?;
    ///     client.auth_with_longlivedtoken("your_token").await?;
    ///     println!("WebSocket connection and authethication works");
    ///     Ok(())
    /// }
    /// ```
    pub async fn auth_with_longlivedtoken(&mut self, token: &str) -> HassResult<()> {
        // Auth Request from Gateway { "type": "auth_required"}
        let _ = self
            .gateway
            .from_gateway
            .next()
            .await
            .ok_or_else(|| HassError::ConnectionClosed)?;

        //Authenticate with Command::AuthInit and payload {"type": "auth", "access_token": "XXXXX"}
        let auth_req = Command::AuthInit(Auth {
            msg_type: "auth".to_owned(),
            access_token: token.to_owned(),
        });
        let response = self.gateway.command(auth_req).await?;

        //Check if the authetication was succefully, should receive {"type": "auth_ok"}
        match response {
            Response::AuthOk(_) => Ok(()),
            Response::AuthInvalid(err) => return Err(HassError::AuthenticationFailed(err.message)),
            _ => return Err(HassError::UnknownPayloadReceived),
        }

        // match value.msg_type.as_str() {
        //     "auth_ok" => return Ok(()),
        //     "auth_invalid" => return Err(HassError::AuthenticationFailed),
        //     _ => return Err(HassError::UnknownPayloadReceived),
        // }
    }

    ///The API supports receiving a ping from the client and returning a pong.
    /// This serves as a heartbeat to ensure the connection is still alive.
    ///# Examples
    ///
    /// Demonstrates basic usage.
    ///
    /// ```no_run
    /// use hass_rs::client;
    ///
    /// #[async_std::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///
    ///     let mut client = client::connect("localhost", 8123).await?;
    ///     client.auth_with_longlivedtoken("your_token").await?;
    ///
    ///     match client.ping().await? {
    ///         pong if pong == String::from("pong") => {
    ///             println!("Great the Hass Websocket Server responds to ping")
    ///         }
    ///         _ => println!("Ooops, I was expecting pong"),
    ///     };
    ///     Ok(())
    /// }
    /// ```
    pub async fn ping(&mut self) -> HassResult<String> {
        //Send Ping command and expect Pong
        let ping_req = Command::Ping(Ask {
            id: Some(0),
            msg_type: "ping".to_owned(),
        });
        let response = self.gateway.command(ping_req).await?;

        //Check the response, if the Pong was received
        match response {
            Response::Pong(_v) => Ok("pong".to_owned()),
            Response::Result(err) => return Err(HassError::ReponseError(err)),
            _ => return Err(HassError::UnknownPayloadReceived),
        }
    }

    ///The command subscribe_event will subscribe your client to the event bus.
    ///
    /// You can either listen to all events or to a specific event type.
    /// If you want to listen to multiple event types, you will have to send multiple subscribe_events commands.
    /// The server will respond with a result message to indicate that the subscription is active.
    /// For each event that matches, the server will send a message of type event.
    /// The id in the message will point at the original id of the listen_event command.
    ///# Examples
    ///
    /// Demonstrates basic usage.
    ///
    /// ```no_run
    /// use hass_rs::client;
    /// use hass_rs::WSEvent;
    ///
    /// #[async_std::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///
    ///     let mut client = client::connect("localhost", 8123).await?;
    ///     client.auth_with_longlivedtoken("your_token").await?;
    ///
    ///     let pet = |item: WSEvent| {
    ///         println!(
    ///         "Closure is executed when the Event with the id: {} has been received, it was fired at {}", item.id,
    ///         item.event.time_fired );
    ///     };
    ///     match client.subscribe_event("state_changed", pet).await {
    ///         Ok(v) => println!("Event subscribed: {}", v),
    ///         Err(err) => println!("Oh no, an error: {}", err),
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn subscribe_event<F>(&mut self, event_name: &str, callback: F) -> HassResult<String>
    where
        F: Fn(WSEvent) + Send + 'static,
    {
        self.gateway.subscribe_message(event_name, callback).await
    }

    ///The command unsubscribe_event will unsubscribe your client from the event bus.
    ///
    /// You can unsubscribe from previously created subscription events.
    /// Pass the id of the original subscription command as value to the subscription field.
    ///# Examples
    ///
    /// Demonstrates basic usage.
    ///
    /// ```no_run
    /// use hass_rs::client;
    ///
    /// #[async_std::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///
    ///     let mut client = client::connect("localhost", 8123).await?;
    ///     client.auth_with_longlivedtoken("your_token").await?;
    ///
    /// //assuming the event subscription is present
    ///     match client.unsubscribe_event(2).await {
    ///         Ok(v) => println!("Succefully unsubscribed: {}", v),
    ///         Err(err) => println!("Oh no, an error: {}", err),
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn unsubscribe_event(&mut self, subscription_id: u64) -> HassResult<String> {
        self.gateway.unsubscribe_message(subscription_id).await
    }

    ///This will get a dump of the current config in Home Assistant.
    ///
    /// The server will respond with a result message containing the config.
    ///# Examples
    ///
    /// Demonstrates basic usage.
    ///
    /// ```no_run
    /// use hass_rs::client;
    ///
    /// #[async_std::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///
    ///     let mut client = client::connect("localhost", 8123).await?;
    ///     client.auth_with_longlivedtoken("your_token").await?;
    ///
    ///     println!("Get Hass Config");
    ///     match client.get_config().await {
    ///         Ok(v) => println!("{:?}", v),
    ///         Err(err) => println!("Oh no, an error: {}", err),
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_config(&mut self) -> HassResult<HassConfig> {
        //Send GetConfig command and expect Pong
        let config_req = Command::GetConfig(Ask {
            id: Some(0),
            msg_type: "get_config".to_owned(),
        });
        let response = self.gateway.command(config_req).await?;

        match response {
            Response::Result(data) => match data.success {
                true => {
                    let config: HassConfig = serde_json::from_value(
                        data.result.expect("Expecting to get the HassConfig"),
                    )?;
                    return Ok(config);
                }
                false => return Err(HassError::ReponseError(data)),
            },
            _ => return Err(HassError::UnknownPayloadReceived),
        }
    }

    ///This will get a dump of all the current states in Home Assistant.
    ///
    /// The server will respond with a result message containing the states.
    ///# Examples
    ///
    /// Demonstrates basic usage.
    ///
    /// ```no_run
    /// use hass_rs::client;
    ///
    /// #[async_std::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>>{
    ///
    ///     let mut client = client::connect("localhost", 8123).await?;
    ///     client.auth_with_longlivedtoken("your_token").await?;
    ///
    ///     println!("Get Hass States");
    ///     match client.get_states().await {
    ///         Ok(v) => println!("{:?}", v),
    ///         Err(err) => println!("Oh no, an error: {}", err),
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_states(&mut self) -> HassResult<Vec<HassEntity>> {
        //Send GetStates command and expect a number of Entities
        let states_req = Command::GetStates(Ask {
            id: Some(0),
            msg_type: "get_states".to_owned(),
        });
        let response = self.gateway.command(states_req).await?;

        match response {
            Response::Result(data) => match data.success {
                true => {
                    let states: Vec<HassEntity> =
                        serde_json::from_value(data.result.expect("Expecting to get the States"))?;
                    return Ok(states);
                }
                false => return Err(HassError::ReponseError(data)),
            },
            _ => return Err(HassError::UnknownPayloadReceived),
        }
    }

    ///This will get a dump of the current services in Home Assistant.
    ///
    /// The server will respond with a result message containing the services.
    ///# Examples
    ///
    /// Demonstrates basic usage.
    ///
    /// ```no_run
    /// use hass_rs::client;
    ///
    /// #[async_std::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///
    ///     let mut client = client::connect("localhost", 8123).await?;
    ///     client.auth_with_longlivedtoken("your_token").await?;
    ///
    ///     println!("Get Hass Services");
    ///     match client.get_services().await {
    ///         Ok(v) => println!("{:?}", v),
    ///         Err(err) => println!("Oh no, an error: {}", err),
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_services(&mut self) -> HassResult<HassServices> {
        //Send GetStates command and expect a number of Entities
        let services_req = Command::GetServices(Ask {
            id: Some(0),
            msg_type: "get_services".to_owned(),
        });
        let response = self.gateway.command(services_req).await?;

        match response {
            Response::Result(data) => match data.success {
                true => {
                    let services: HassServices = serde_json::from_value(
                        data.result.expect("Expecting to get the Services"),
                    )?;
                    return Ok(services);
                }
                false => return Err(HassError::ReponseError(data)),
            },
            _ => return Err(HassError::UnknownPayloadReceived),
        }
    }

    ///This will call a service in Home Assistant. Right now there is no return value.
    ///The client can listen to state_changed events if it is interested in changed entities as a result of a service call.
    ///
    ///The server will indicate with a message indicating that the service is done executing.
    ///
    ///# Examples
    ///
    /// Demonstrates basic usage.
    ///
    /// ```no_run
    /// use hass_rs::client;
    /// use serde_json::json;
    ///
    /// #[async_std::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///
    ///     let mut client = client::connect("localhost", 8123).await?;
    ///     client.auth_with_longlivedtoken("your_token").await?;
    ///
    ///     let value = json!({
    ///         "entity_id": "sun.sun"
    ///     });
    ///
    ///     match client
    ///         .call_service(
    ///             "homeassistant".to_owned(),
    ///             "update_entity".to_owned(),
    ///             Some(value),
    ///         )
    ///         .await
    ///     {
    ///         Ok(done) if done == String::from("command executed successfully") => {
    ///             println!("Good, your command was executed")
    ///         }
    ///         Ok(_) => println!("Ooops, I got strange result"),
    ///         Err(error) => println!("Ooops, I got this error {}", error),
    ///     }
    ///     Ok(())
    /// }
    /// ```
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
        let response = self.gateway.command(services_req).await?;

        match response {
            Response::Result(data) => match data.success {
                true => return Ok("command executed successfully".to_owned()),
                false => return Err(HassError::ReponseError(data)),
            },
            _ => return Err(HassError::UnknownPayloadReceived),
        }
    }
}
