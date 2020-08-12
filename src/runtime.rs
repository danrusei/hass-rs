use crate::errors::HassResult;

use async_tungstenite::{stream::Stream, WebSocketStream};
use url::Url;

// ******************************
// ASYNC-STD
// *****************************
#[cfg(feature = "async-std-runtime")]
pub(crate) type WebSocket = WebSocketStream<
    Stream<async_std::net::TcpStream, async_tls::client::TlsStream<async_std::net::TcpStream>>,
>;

#[cfg(feature = "async-std-runtime")]
pub use async_std::task;

#[cfg(feature = "async-std-runtime")]
pub(crate) async fn connect_async(url: Url) -> HassResult<WebSocket> {
    let (client, _) = async_tungstenite::async_std::connect_async(url).await?;
    Ok(client)
}

// ******************************
// TOKIO
// ******************************

#[cfg(feature = "tokio-runtime")]
use async_tungstenite::tokio::TokioAdapter;

#[cfg(feature = "tokio-runtime")]
pub(crate) type WebSocket = WebSocketStream<
    Stream<
        TokioAdapter<tokio::net::TcpStream>,
        TokioAdapter<tokio_native_tls::TlsStream<tokio::net::TcpStream>>,
    >,
>;

#[cfg(feature = "tokio-runtime")]
pub use tokio::task;

#[cfg(feature = "tokio-runtime")]
pub(crate) async fn connect_async(url: Url) -> HassResult<WebSocket> {
    let (client, _) = async_tungstenite::tokio::connect_async(url).await?;
    Ok(client)
}
