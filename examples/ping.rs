use env_logger;
use hass_rs::client;
use lazy_static::lazy_static;
use serde_json::json;
use std::env::var;

lazy_static! {
    static ref TOKEN: String =
        var("HASS_TOKEN").expect("please set up the HASS_TOKEN env variable before running this");
}

#[cfg_attr(feature = "async-std-runtime", async_std::main)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("Creating the Websocket Client and Authenticate the session");
    let mut client = client::connect("localhost", 8123).await?;

    client.auth(&*TOKEN).await?;
    println!("WebSocket connection and authethication works");

    println!("Sending a Ping command and waiting for Pong");

    match client.ping().await? {
        pong if pong == String::from("pong") => {
            println!("Great the Hass Websocket Server responds to ping")
        }
        _ => println!("Ooops, I was expecting pong"),
    }

    async_std::task::sleep(std::time::Duration::from_secs(2)).await;

    let value = json!({
        "entity_id": "sun.sun"
    });

    match client
        .call_service(
            "homeassistant".to_owned(),
            "update_entity".to_owned(),
            Some(value),
        )
        .await
    {
        Ok(done) if done == String::from("command executed successfully") => {
            println!("Good, your command was executed")
        }
        Ok(_) => println!("Ooops, I got strange result"),
        Err(error) => println!("Ooops, I got this error {}", error),
    }

    Ok(())
}
