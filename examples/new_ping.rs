use async_tungstenite::tungstenite::{Error, Message};
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use hass_rs::client::HassClient;
use lazy_static::lazy_static;
use std::env::var;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{mpsc, mpsc::Receiver, mpsc::Sender};
use tokio_tungstenite::{connect_async, WebSocketStream};

lazy_static! {
    static ref TOKEN: String =
        var("HASS_TOKEN").expect("please set up the HASS_TOKEN env variable before running this");
}

async fn ws_incoming_messages(
    mut stream: SplitStream<WebSocketStream<impl AsyncRead + AsyncWrite + Unpin>>,
    to_user: Sender<Result<Message, Error>>,
) {
    loop {
        while let Some(message) = stream.next().await {
            let _ = to_user.send(message).await;
        }
    }
}

async fn ws_outgoing_messages(
    mut sink: SplitSink<WebSocketStream<impl AsyncRead + AsyncWrite + Unpin>, Message>,
    mut from_user: Receiver<Message>,
) {
    loop {
        match from_user.recv().await {
            Some(msg) => sink.send(msg).await.expect("Failed to send message"),
            None => todo!(),
        }
    }
}

#[tokio::main]
async fn main() {
    let url = "ws://localhost:8123/api/websocket";

    println!("Connecting to - {}", url);
    //let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    let (wsclient, _) = connect_async(url).await.expect("Failed to connect");
    let (sink, stream) = wsclient.split();

    //Channels to recieve the Client Command and send it over to the Websocket server
    // MAYBE migrate to multiple producers single consumer, instead of 2 distinct channels
    let (to_gateway, from_user) = mpsc::channel::<Message>(20);
    //Channels to receive the Response from the Websocket server and send it over to the Client
    let (to_user, from_gateway) = mpsc::channel::<Result<Message, Error>>(20);

    // Handle incoming messages in a separate task
    let read_handle = tokio::spawn(ws_incoming_messages(stream, to_user));

    // Read from command line and send messages
    let write_handle = tokio::spawn(ws_outgoing_messages(sink, from_user));

    let mut client = HassClient::new(to_gateway, from_gateway);

    client
        .auth_with_longlivedtoken(&*TOKEN)
        .await
        .expect("Not able to autheticate");

    println!("WebSocket connection and authethication works\n");

    println!("Getting the Config\n");
    let cmd2 = client.get_config().await.expect("Not getting the config");
    println!("config: {:?}", cmd2);

    // Await both tasks (optional, depending on your use case)
    let _ = tokio::try_join!(read_handle, write_handle);
}
