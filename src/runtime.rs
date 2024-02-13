use crate::errors::HassResult;

//use async_tungstenite::{stream::Stream, WebSocketStream};
use tokio_tungstenite::WebSocketStream;
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

#[cfg(feature = "async-std-runtime")]
pub use async_std::channel::{bounded, Receiver, Sender};

// ******************************
// TOKIO
// ******************************

//#[cfg(feature = "tokio-runtime")]
//use async_tungstenite::tokio::TokioAdapter;

#[cfg(feature = "tokio-runtime")]
pub(crate) type WebSocket =
    WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

#[cfg(feature = "tokio-runtime")]
pub use tokio::task;

#[allow(unused_imports)]
#[cfg(feature = "tokio-runtime")]
use tokio::sync::mpsc::{channel, Receiver, Sender};

#[cfg(feature = "tokio-runtime")]
pub(crate) async fn connect_async(url: Url) -> HassResult<WebSocket> {
    let (client, _) = tokio_tungstenite::connect_async(url).await?;
    Ok(client)
}
