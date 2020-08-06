use env_logger;
use hass_rs::{ConnConfig, HassClient};

static TOKEN: &str = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJpc3MiOiI0YzcyOGFjNDQ4MTc0NWIwODUxY2ZjMGE5YTc2ZWE1NSIsImlhdCI6MTU5NTIzNDYwMiwiZXhwIjoxOTEwNTk0NjAyfQ.Ow-mSTKNUSyqcJJrSBMYy6ftKMiTEwhMl-uhtBxln80";

#[cfg_attr(feature = "async-std-runtime", async_std::main)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("Creating the Websocket Client and Authenticate the session");
    let mut client = HassClient::new(ConnConfig {
        host: "localhost".to_owned(),
        port: 8123,
        token: TOKEN.to_owned(),
    });

    client.connect().await?;
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
