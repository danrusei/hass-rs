use hass_rs::{config::Config, HassClient};

static TOKEN: &str = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJpc3MiOiI0YzcyOGFjNDQ4MTc0NWIwODUxY2ZjMGE5YTc2ZWE1NSIsImlhdCI6MTU5NTIzNDYwMiwiZXhwIjoxOTEwNTk0NjAyfQ.Ow-mSTKNUSyqcJJrSBMYy6ftKMiTEwhMl-uhtBxln80";

#[cfg_attr(feature = "async-std-runtime", async_std::main)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating the Websocket Client and Authenticate the session");
    let mut client = HassClient::new(Config {
        host: "localhost".to_owned(),
        port: 8123,
        token: TOKEN.to_owned(),
    });

    client.connect().await?;
    println!("WebSocket connection and authethication works");

    println!("Sending a Ping command and waiting for Pong");

    match client.ping().await? {
        pong if pong == String::from("pong") => println!("Great the Hass Websocket Server responds to ping"),
        _ => println!("I was expecting pong") 
    }

    println!("Subscribe to an Event");

    client.subscribe_event("state_changed", || {
        println!("Closure is executed when the event received")
    }).await;

    async_std::task::sleep(std::time::Duration::from_secs(2)).await;

    Ok(())
}
