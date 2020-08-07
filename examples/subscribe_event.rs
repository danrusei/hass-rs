use env_logger;
use hass_rs::{client, WSEvent};

static TOKEN: &str = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJpc3MiOiI0YzcyOGFjNDQ4MTc0NWIwODUxY2ZjMGE5YTc2ZWE1NSIsImlhdCI6MTU5NTIzNDYwMiwiZXhwIjoxOTEwNTk0NjAyfQ.Ow-mSTKNUSyqcJJrSBMYy6ftKMiTEwhMl-uhtBxln80";

#[cfg_attr(feature = "async-std-runtime", async_std::main)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("Creating the Websocket Client and Authenticate the session");
    let mut client = client::connect("localhost", 8123).await?;

    client.auth(TOKEN).await?;
    println!("WebSocket connection and authethication works");

    //TODO do something with the event, send a command back , Call a service
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

    match client.unsubscribe_event(2).await {
        Ok(v) => println!("Succefully unsubscribed: {}", v),
        Err(err) => println!("Oh no, an error: {}", err),
    }

    async_std::task::sleep(std::time::Duration::from_secs(20)).await;

    Ok(())
}
