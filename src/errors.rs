//! Convenient error handling

use crate::types::WSResult;

use async_tungstenite::tungstenite;
use futures::channel::mpsc::SendError;
use std::fmt;

pub type HassResult<T> = std::result::Result<T, HassError>;

/// The error enum for Hass
#[derive(Debug)]
pub enum HassError {
    /// Returned when the connection to gateway has failed
    CantConnectToGateway,

    /// Returned when it is unable to authenticate
    AuthenticationFailed,

    /// Returned when unable to parse the websocket server address
    WrongAddressProvided(url::ParseError),

    /// Returned when connection has unexpected failed
    ConnectionClosed,

    /// Tungstenite error
    TungsteniteError(tungstenite::error::Error),

    /// Returned mpsc send channel error
    ChannelSend(SendError),

    /// Returned when an unknown message format is received
    UnknownPayloadReceived,

    /// Returned the error received from the Home Assistant Gateway
    ReponseError(WSResult),

    /// Returned for errors which do not fit any of the above criterias
    Generic(String),
}

impl std::error::Error for HassError {}

impl fmt::Display for HassError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::CantConnectToGateway => write!(f, "Cannot connect to gateway"),
            Self::ConnectionClosed => write!(f, "Connection closed unexpectedly"),
            Self::AuthenticationFailed => write!(f, "Authentication has failed"),
            Self::WrongAddressProvided(e) => write!(f, "Could not parse the provided address: {}", e),
            Self::TungsteniteError(e) => write!(f, "Tungstenite Error: {}", e),
            Self::ChannelSend(e) => write!(f, "Channel Send Error: {}", e),
            Self::UnknownPayloadReceived => write!(f, "The received payload is unknown"),
            Self::ReponseError(e) => write!(
                f,
                "The error code reveiced is {} and the error message {}",
                e.error.as_ref().unwrap().code,
                e.error.as_ref().unwrap().message
            ),
            Self::Generic(detail) => write!(f, "Generic Error: {}", detail),
        }
    }
}

impl From<SendError> for HassError {
    fn from(error: SendError) -> Self {
        match error {
            _ => HassError::ChannelSend(error),
        }
    }
}

impl From<url::ParseError> for HassError {
    fn from(error: url::ParseError) -> Self {
        HassError::WrongAddressProvided(error)
    }
}

impl From<tungstenite::error::Error> for HassError {
    fn from(error: tungstenite::error::Error) -> Self {
        match error {
            tungstenite::error::Error::ConnectionClosed => HassError::ConnectionClosed,
            _ => HassError::TungsteniteError(error),
        }
    }
}

impl From<&tungstenite::error::Error> for HassError {
    fn from(error: &tungstenite::error::Error) -> Self {
        let e = match error {
            tungstenite::error::Error::ConnectionClosed => {
                tungstenite::error::Error::ConnectionClosed
            }
            tungstenite::error::Error::AlreadyClosed => tungstenite::error::Error::AlreadyClosed,
            _ => return HassError::Generic(format!("Error from ws {}", error)),
        };
        HassError::TungsteniteError(e)
    }
}
