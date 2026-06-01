use hass_rs::client::HassClient;
use std::env::var;
use std::sync::OnceLock;
use std::{thread, time};

static TOKEN: OnceLock<String> = OnceLock::new();

#[tokio::main]
async fn main() {
    let url = "ws://localhost:8123/api/websocket";

    println!("Connecting to - {}", url);
    let mut client = HassClient::new(url).await.expect("Failed to connect");

    let token = TOKEN.get_or_init(|| {
        var("HASS_TOKEN").expect("please set up the HASS_TOKEN env variable before running this")
    });

    client
        .auth_with_longlivedtoken(token)
        .await
        .expect("Not able to authenticate");

    println!("WebSocket connection and authentication works\n");

    println!("Subscribe to an Event");

    let mut event_receiver = client
        .subscribe_event("state_changed")
        .await
        .expect("Failed to subscribe");

    // Spawn a Tokio task to do whatever we want with the received events
    tokio::spawn(async move {
        while let Some(message) = event_receiver.recv().await {
            println!("Event Received: {:?}", message);
        }
    });

    thread::sleep(time::Duration::from_secs(20));
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
// Successfully unsubscribed: Ok
