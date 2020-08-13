use env_logger;
use hass_rs::{client, WSEvent};
use lazy_static::lazy_static;
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

    client.auth_with_longlivedtoken(&*TOKEN).await?;
    println!("WebSocket connection and authethication works");

    //Is the callback definition usefull as it is?..or need to return anything
    // in order to further process the result, like call for a service
    println!("Subscribe to an Event");

    let pet = |item: WSEvent| {
        println!(
        "Closure is executed when the Event with the id: {} has been received, it was fired at {}", item.id,
        item.event.time_fired );
    };

    match client.subscribe_event("state_changed", pet).await {
        Ok(v) => println!("Event subscribed: {}", v),
        Err(err) => println!("Oh no, an error: {}", err),
    }

    async_std::task::sleep(std::time::Duration::from_secs(20)).await;

    println!("Unsubscribe the Event");

    match client.unsubscribe_event(2).await {
        Ok(v) => println!("Succefully unsubscribed: {}", v),
        Err(err) => println!("Oh no, an error: {}", err),
    }

    async_std::task::sleep(std::time::Duration::from_secs(20)).await;

    Ok(())
}
