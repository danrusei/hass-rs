use async_tungstenite::tungstenite::Message as TungsteniteMessage;
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use hass_rs::client::HassClient;
use hass_rs::Command;
use lazy_static::lazy_static;
use std::env::var;
use tokio::io::{self, AsyncBufReadExt};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream};

lazy_static! {
    static ref TOKEN: String =
        var("HASS_TOKEN").expect("please set up the HASS_TOKEN env variable before running this");
}

// async fn authenticate(
//     write: &mut SplitSink<WebSocketStream<impl AsyncRead + AsyncWrite + Unpin>, Message>,
//     bot_name: &str,
// ) {
//     let registration_message = Message::Text(format!("register as {}", bot_name));
//     write
//         .send(registration_message)
//         .await
//         .expect("Failed to send registration message");
// }

async fn ws_incoming_messages(
    mut read: SplitStream<WebSocketStream<impl AsyncRead + AsyncWrite + Unpin>>,
) {
    while let Some(message) = read.next().await {
        client.read_message(message)
        // match message {
        //     Ok(msg) => println!("Received a message: {}", msg),
        //     Err(e) => eprintln!("Error receiving message: {}", e),
        // }
    }
}

async fn ws_outgoing_messages(
    mut write: SplitSink<WebSocketStream<impl AsyncRead + AsyncWrite + Unpin>, Message>,
) {
    let mut reader = io::BufReader::new(io::stdin()).lines();
    while let Some(line) = reader.next_line().await.expect("Failed to read line") {
        if !line.trim().is_empty() {
            write
                .send(Message::Text(line))
                .await
                .expect("Failed to send message");
        }
    }
}

#[tokio::main]
async fn main() {
    let url = "ws://localhost:3000";

    println!("Connecting to - {}", url);
    //let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    let (wsclient, _) = connect_async(url).await.expect("Failed to connect");
    let (sink, stream) = wsclient.split();

    let (to_ws, from_ws) = mspc::channel::<TungsteniteMessage>(40);

    let client = HassClient::new(to_ws, from_ws);

    println!("WebSocket connection and authethication works");
    client.auth_with_longlivedtoken(&*TOKEN).await?;

    println!("Getting the Config");
    let cmd2 = client.get_config();

    // let (mut write, read) = ws_stream.split();
    // register the timebot
    //register_bot(&mut write, "RustClient").await;

    // Handle incoming messages in a separate task
    let read_handle = tokio::spawn(ws_incoming_messages(read));

    // Read from command line and send messages
    let write_handle = tokio::spawn(ws_outgoing_messages(write));

    // Await both tasks (optional, depending on your use case)
    let _ = tokio::try_join!(read_handle, write_handle);
}
