use env_logger;
use hass_rs::client;
use std::env::var;
use lazy_static::lazy_static;

lazy_static! {
    static ref TOKEN: String = var("HASS_TOKEN").expect("please set up the HASS_TOKEN env variable before running this");
}

#[cfg_attr(feature = "async-std-runtime", async_std::main)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("Creating the Websocket Client and Authenticate the session");
    let mut client = client::connect("localhost", 8123).await?;

    client.auth(&*TOKEN).await?;
    println!("WebSocket connection and authethication works");

    println!("Get Hass Config");
    match client.get_config().await {
        Ok(v) => println!("{:?}", v),
        Err(err) => println!("Oh no, an error: {}", err),
    }

    async_std::task::sleep(std::time::Duration::from_secs(2)).await;

    //TODO  iterate and find specific fields
    println!("Get Hass States");
    match client.get_states().await {
        Ok(v) => println!("{:?}", v),
        Err(err) => println!("Oh no, an error: {}", err),
    }

    async_std::task::sleep(std::time::Duration::from_secs(2)).await;

    //TODO  iterate and find specific fields
    println!("Get Hass Services");
    match client.get_services().await {
        Ok(v) => println!("{:?}", v),
        Err(err) => println!("Oh no, an error: {}", err),
    }

    Ok(())
}
