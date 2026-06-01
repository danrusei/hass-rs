use hass_rs::client::HassClient;
use std::env::var;
use std::sync::OnceLock;

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
    for (key, panel) in cmd5 {
        println!("panel_key: {}\n", key);
        println!("panel: {}\n", panel);
    }

    // println!("Getting the Services:\n");
    // let cmd4 = client
    //     .get_services()
    //     .await
    //     .expect("Unable to retrieve the Services");
    // println!("services: {}\n", cmd4);
}
