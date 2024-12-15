use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use hass_rs::client::HassClient;
use lazy_static::lazy_static;
use serde_json::json;
use std::env::var;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{mpsc, mpsc::Receiver, mpsc::Sender};
use tokio_tungstenite::tungstenite::{Error, Message};
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
    let (wsclient, _) = connect_async(url).await.expect("Failed to connect");
    let (sink, stream) = wsclient.split();

    //Channels to recieve the Client Command and send it over to Websocket server
    let (to_gateway, from_user) = mpsc::channel::<Message>(20);
    //Channels to receive the Response from the Websocket server and send it over to Client
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

    let domain = "homeassistant";
    let service = "turn_on";
    let entity_id = "media_player.bravia_4k_gb";

    println!("Getting the Services:\n");
    let cmd1 = client
        .get_services()
        .await
        .expect("Unable to retrieve the Services");

    // Validate if the selected **domain** and **service** exist
    if let Some(service_names) = cmd1.list_services(domain) {
        for (name, hass_service) in service_names {
            if name == service.to_string() {
                println!("Name: {}", name);
                println!("hass_service: {}", hass_service);
            }
        }
    } else {
        println!(
            "Domain {}, or the service {} for provided domain, not found",
            domain, service
        );
    }

    println!("Getting the States (Entities):\n");
    let cmd2 = client
        .get_states()
        .await
        .expect("Unable to retrieve the States");

    // Validate if the selected **entity_id** exist
    let entity_found = cmd2.iter().find(|e| e.entity_id == entity_id);
    if let Some(entity) = entity_found {
        println!("{}", entity);
    }

    let value = json!({
        "entity_id": entity_id,
    });

    println!("Calling a service:, in this specific case to turn ON the TV\n");
    let cmd3 = client
        .call_service(domain.to_owned(), service.to_owned(), Some(value))
        .await
        .expect("Unable to call the targeted service");
    println!("service: {:?}\n", cmd3);

    //check the new Entity state
    println!("Getting again the States (Entities):\n");
    let cmd4 = client
        .get_states()
        .await
        .expect("Unable to retrieve the States");

    let entity_found = cmd4.iter().find(|e| e.entity_id == entity_id);
    if let Some(entity) = entity_found {
        println!("{}", entity);
    }

    let _ = tokio::try_join!(read_handle, write_handle);
}

// Running it:
//
// Getting the States (Entities):
//
// HassEntity {
//     entity_id: media_player.bravia_4k_gb,
//     state: off,
//     last_changed: 2024-02-15T11:13:02.291378+00:00,
//     last_updated: 2024-02-15T11:13:27.686327+00:00,
//
// Calling a service:, in this specific case to turn ON the TV
//
// service: "command executed successfully"
//
// Getting the States (Entities):
//
// HassEntity {
//     entity_id: media_player.bravia_4k_gb,
//     state: paused,
//     last_changed: 2024-02-15T11:14:05.370810+00:00,
//     last_updated: 2024-02-15T11:14:05.370810+00:00
