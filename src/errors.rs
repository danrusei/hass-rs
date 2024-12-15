//! Convenient error handling

use crate::types::Response;
use crate::types::WSResult;
use thiserror::Error;
use tokio_tungstenite::tungstenite;

pub type HassResult<T> = std::result::Result<T, HassError>;

/// The error enum for Hass
#[derive(Error, Debug)]
pub enum HassError {
    /// Returned when it is unable to authenticate
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    /// Returned when serde was unable to deserialize the values
    #[error("Unable to deserialize received value: {0}")]
    UnableToDeserialize(#[from] serde_json::error::Error),

    /// Returned when connection has unexpected failed
    #[error("Connection closed unexpectedly")]
    ConnectionClosed,

    /// Mpsc channel SendError<T> message
    #[error("Unable to send the message on channel: {0}")]
    SendError(String),

    /// Tungstenite error
    #[error("Tungstenite error: {0}")]
    TungsteniteError(#[from] tungstenite::error::Error),

    /// Returned when an unknown message format is received
    #[error("The received payload is unknown {0:?}")]
    UnknownPayloadReceived(Response),

    /// Returned when an unknown message format is received
    #[error("Received an unexpected message: {0:?}")]
    UnexpectedMessage(tungstenite::Message),

    /// Returned the error received from the Home Assistant Gateway
    #[error("ResponseError: {0:?}")]
    ResponseError(WSResult),

    /// Returned for errors which do not fit any of the above criterias
    #[error("Generic Error: {0}")]
    Generic(String),
}
