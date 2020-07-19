//! # Hass error types
use std::fmt;
use async_tungstenite::tungstenite::error::Error as TungsteniteError;

pub type HassResult<T> = std::result::Result<T, HassError>;

/// The error enum for Hass
#[derive(Debug)]
pub enum HassError {
    //Returned when it connection to gateway has failed
    ConnectionFailed,

    //Returned when unable to auathenticate to gateway
    AuthenticationFailed,

    //Returned when connection is unexpected failed
    ConnectionClosed,

    /// tungstenite
    TungsteniteError(TungsteniteError),
} 

impl fmt::Display for HassError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::ConnectionFailed => write!(f, "Cannot connect to gateway"),
            Self::ConnectionClosed => write!(f, "Connection closed unexpectedly"),
            Self::AuthenticationFailed => write!(f, "Authentication has failed"),
            Self::TungsteniteError(e) => write!(f, "Tungstenite Error: {}", e)
        }
        
    }
}

impl From<TungsteniteError> for HassError {
    fn from(error: TungsteniteError) -> Self {
        match error {
            TungsteniteError::ConnectionClosed => HassError::ConnectionClosed,
            _ => HassError::TungsteniteError(error),
        }
    }
}