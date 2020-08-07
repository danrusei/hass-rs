use crate::types::{
    Ask, Auth, CallService, Command, HassConfig, HassEntity, HassServices, Response,
    WSEvent,
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

/// create websocket connection to Home Assistant server
pub async fn connect(host: &str, port: u16) -> HassResult<HassClient> {
    let addr = format!("ws://{}:{}/api/websocket", host, port);
    let url = url::Url::parse(&addr)?;
    let gateway = WsConn::connect(url).await?;
    Ok(HassClient { gateway })
}

impl HassClient {
    /// client.auth(TOKEN) try to authenticate the session with a long-lived access token
    pub async fn auth(&mut self, token: &str) -> HassResult<()> {
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
            _ => return Err(HassError::UnknownPayloadReceived),
        }

        // match value.msg_type.as_str() {
        //     "auth_ok" => return Ok(()),
        //     "auth_invalid" => return Err(HassError::AuthenticationFailed),
        //     _ => return Err(HassError::UnknownPayloadReceived),
        // }
    }

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

    pub async fn subscribe_event<F>(&mut self, event_name: &str, callback: F) -> HassResult<String>
    where
        F: Fn(WSEvent) + Send + 'static,
    {
        self.gateway.subscribe_message(event_name, callback).await
    }
    pub async fn unsubscribe_event(&mut self, subscription_id: u64) -> HassResult<String> {
        self.gateway.unsubscribe_message(subscription_id).await
    }

    pub async fn get_config(&mut self) -> HassResult<HassConfig> {
        //Send GetConfig command and expect Pong
        let config_req = Command::GetConfig(Ask {
            id: Some(0),
            msg_type: "get_config".to_owned(),
        });
        let response = self.gateway.command(config_req).await?;

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
        let response = self.gateway.command(states_req).await?;

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
        let response = self.gateway.command(services_req).await?;

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
