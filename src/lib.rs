//! # Async HomeAssistant Websocket Library
//!
//! Hass-rs is a HomeAssistant websokcte api library
//! based on https://developers.home-assistant.io/docs/api/websocket specifications

pub mod client;
mod errors;
mod messages;
mod runtime;

pub use errors::{HassError, HassResult};
