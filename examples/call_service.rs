use hass_rs::client::HassClient;
use lazy_static::lazy_static;
use serde_json::json;
use std::env::var;

lazy_static! {
    static ref TOKEN: String =
        var("HASS_TOKEN").expect("please set up the HASS_TOKEN env variable before running this");
}

#[tokio::main]
async fn main() {
    let url = "ws://localhost:8123/api/websocket";

    println!("Connecting to - {}", url);
    let mut client = HassClient::new(url).await.expect("Failed to connect");

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
