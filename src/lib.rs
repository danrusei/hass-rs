//! # Async HomeAssistant Websocket Library
//!
//! Hass-rs is a HomeAssistant Websocket API client library.
//!
//! It is based on the [official API specifications](https://developers.home-assistant.io/docs/api/websocket).
//!
//! # Configuring async runtime
//! hass_rs supports `async-std` and `tokio` runtimes, by default it uses `async-std`,
//! to use `tokio` change the feature flags in `Cargo.toml`
//!
//! ```toml
//! [dependencies.hass_rs]
//! version = "0.1.0"
//! default-features = false
//! features = ["tokio-runtime"]
//! ```
//!
//!
//! # Example usage
//! It sends a ping and receive a pong
//!
//! ```rust
//! #[async_std::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!
//!   TODO(Soon)
//!
//!     Ok(())
//! }
//! ```

pub mod errors;
pub use errors::{HassError, HassResult};

pub mod types;
pub use types::*;

pub mod client;
pub use client::{connect, HassClient};

mod runtime;
use runtime::{connect_async, task, WebSocket};

mod wsconn;
use wsconn::WsConn;
