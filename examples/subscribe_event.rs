use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use hass_rs::client::{check_if_event, HassClient};
use hass_rs::WSEvent;
use lazy_static::lazy_static;
use std::env::var;
use std::{thread, time};
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
    event_sender: Sender<WSEvent>,
) {
    loop {
        while let Some(message) = stream.next().await {
            // check if it is a WSEvent, if so send to the spawned tokio task, that should handle the event
            // otherwise process the message and respond accordingly
            match check_if_event(&message) {
                Ok(event) => {
                    let _ = event_sender.send(event).await;
                    continue;
                }
                _ => {
                    let _ = to_user.send(message).await;
                    continue;
                }
            }
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

    //Channels to recieve the Client Command and send it over to the Websocket server
    let (to_gateway, from_user) = mpsc::channel::<Message>(20);
    //Channels to receive the Response from the Websocket server and send it over to the Client
    let (to_user, from_gateway) = mpsc::channel::<Result<Message, Error>>(20);

    //Channel to receive the Event message from Websocket
    let (event_sender, mut event_receiver) = mpsc::channel::<WSEvent>(20);

    // Handle incoming messages in a separate task
    let read_handle = tokio::spawn(ws_incoming_messages(stream, to_user, event_sender));

    // Read from command line and send messages
    let write_handle = tokio::spawn(ws_outgoing_messages(sink, from_user));

    let mut client = HassClient::new(to_gateway, from_gateway);

    client
        .auth_with_longlivedtoken(&*TOKEN)
        .await
        .expect("Not able to autheticate");

    println!("WebSocket connection and authethication works\n");

    println!("Subscribe to an Event");

    let mut id: u64 = 0;

    match client.subscribe_event("state_changed").await {
        Ok(v) => {
            println!("Event subscribed: {:?}", v);
            id = v.id;
        }

        Err(err) => println!("Oh no, an error: {}", err),
    };

    let subscriptions = client.subscriptions.clone();

    // Spawn a Tokio task to do whatever we want with the received events
    tokio::spawn(async move {
        loop {
            while let Some(message) = event_receiver.recv().await {
                // process only events you have subscribed to
                match subscriptions.get(&message.id) {
                    Some(_) => println!("Event Received: {:?}", message),
                    None => println!("Wrong event received: {:?}", message),
                }
            }
        }
    });

    thread::sleep(time::Duration::from_secs(20));

    println!("Unsubscribe the Event");

    match client.unsubscribe_event(id).await {
        Ok(v) => println!("Succefully unsubscribed: {}", v),
        Err(err) => println!("Oh no, an error: {}", err),
    }

    thread::sleep(time::Duration::from_secs(20));

    // Await both tasks (optional, depending on your use case)
    let _ = tokio::try_join!(read_handle, write_handle);
}

// In order to Test go to Home Assistant --> Developer Tools --> Events , and fire the selected test Event
//
// Subscribe to an Event
// Event subscribed: WSResult { id: 1, success: true, result: None, error: None }
//
// Event Received: WSEvent { id: 1, event: HassEvent { data: EventData { entity_id: None, new_state: None, old_state: None }, event_type: "state_changed", time_fired: "2024-02-16T09:46:45.013050+00:00", origin: "REMOTE", context: Context { id: "01HPRMZAWNXKVVPSP11QFJ53HB", parent_id: None, user_id: Some("f069978dd7964042824cb09287fe7c73") } } }
// Event Received: WSEvent { id: 1, event: HassEvent { data: EventData { entity_id: None, new_state: None, old_state: None }, event_type: "state_changed", time_fired: "2024-02-16T09:46:46.038355+00:00", origin: "REMOTE", context: Context { id: "01HPRMZBWP8E5HQFNV60CJ9HB1", parent_id: None, user_id: Some("f069978dd7964042824cb09287fe7c73") } } }
// Event Received: WSEvent { id: 1, event: HassEvent { data: EventData { entity_id: None, new_state: None, old_state: None }, event_type: "state_changed", time_fired: "2024-02-16T09:46:57.997747+00:00", origin: "REMOTE", context: Context { id: "01HPRMZQJDCEHT1PRQKK6H96AH", parent_id: None, user_id: Some("f069978dd7964042824cb09287fe7c73") } } }
//
// Unsubscribe the Event
// Succefully unsubscribed: Ok
