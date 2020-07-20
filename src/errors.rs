//! # Hass error types
use async_tungstenite::tungstenite;
use std::fmt;

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
    TungsteniteError(tungstenite::error::Error),

    /// others
    Generic(String),
}

impl std::error::Error for HassError {}

impl fmt::Display for HassError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::ConnectionFailed => write!(f, "Cannot connect to gateway"),
            Self::ConnectionClosed => write!(f, "Connection closed unexpectedly"),
            Self::AuthenticationFailed => write!(f, "Authentication has failed"),
            Self::TungsteniteError(e) => write!(f, "Tungstenite Error: {}", e),
            Self::Generic(detail) => write!(f, "Generic Error: {}", detail),
        }
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
