use hass_rs::client::HassClient;
use lazy_static::lazy_static;
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

    println!("Getting the Config:\n");
    let cmd2 = client
        .get_config()
        .await
        .expect("Unable to retrieve the Config");
    println!("config: {}\n", cmd2);

    println!("Getting the States:\n");
    let cmd3 = client
        .get_states()
        .await
        .expect("Unable to retrieve the States");
    for entity in cmd3 {
        println!("entities: {}\n", entity);
    }

    println!("Getting the Panels:\n");
    let cmd5 = client
        .get_panels()
        .await
        .expect("Unable to retrieve the Panels");
    for (key, pannel) in cmd5 {
        println!("pannel_key: {}\n", key);
        println!("pannel: {}\n", pannel);
    }

    // println!("Getting the Services:\n");
    // let cmd4 = client
    //     .get_services()
    //     .await
    //     .expect("Unable to retrieve the Services");
    // println!("services: {}\n", cmd4);
}
