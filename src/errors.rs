//! Convenient error handling

use crate::types::WSResult;
use std::fmt;
use tokio_tungstenite::tungstenite;

pub type HassResult<T> = std::result::Result<T, HassError>;

/// The error enum for Hass
#[derive(Debug)]
pub enum HassError {
    /// Returned when it is unable to authenticate
    AuthenticationFailed(String),

    /// Returned when serde was unable to deserialize the values
    UnableToDeserialize(serde_json::error::Error),

    /// Returned when connection has unexpected failed
    ConnectionClosed,

    /// Mpsc channel SendError<T> message
    SendError(String),

    /// Tungstenite error
    TungsteniteError(tungstenite::error::Error),

    ///Tokio Tungstenite error
    //TokioTungsteniteError(tokio_tungstenite::tungstenite::Error),

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
            // Self::CantConnectToGateway => write!(f, "Cannot connect to gateway"),
            Self::ConnectionClosed => write!(f, "Connection closed unexpectedly"),
            Self::SendError(e) => write!(f, "Unable to send the message on channel: {}", e),
            Self::AuthenticationFailed(e) => write!(f, "Authentication has failed: {}", e),
            Self::UnableToDeserialize(e) => {
                write!(f, "Unable to deserialize the received value: {}", e)
            }
            Self::TungsteniteError(e) => write!(f, "Tungstenite Error: {}", e),
            //Self::TokioTungsteniteError(e) => write!(f, "Tokio Tungstenite Error: {}", e),
            Self::UnknownPayloadReceived => write!(f, "The received payload is unknown"),
            Self::ReponseError(e) => match &e.error {
                Some(e) => write!(
                    f,
                    "The error code:{} with the error message: {}",
                    e.code, e.message
                ),
                None => write!(f, "ResponseError({e:?})"),
            },
            Self::Generic(detail) => write!(f, "Generic Error: {}", detail),
        }
    }
}

impl From<serde_json::error::Error> for HassError {
    fn from(error: serde_json::error::Error) -> Self {
        HassError::UnableToDeserialize(error)
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
