//! # Async HomeAssistant Websocket Library
//!
//! Hass-rs is a HomeAssistant Websocket API client library.
//!
//! It is based on the [official API specifications](https://developers.home-assistant.io/docs/api/websocket).
//!

pub mod errors;
pub use errors::{HassError, HassResult};

pub mod types;
pub use types::*;

pub mod client;
pub use client::HassClient;
