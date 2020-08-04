use env_logger;
use hass_rs::types::ConnConfig;
use hass_rs::HassClient;

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

    println!("Sending a Ping command and waiting for Pong");

    match client.ping().await? {
        pong if pong == String::from("pong") => {
            println!("Great the Hass Websocket Server responds to ping")
        }
        _ => println!("Ooops, I was expecting pong"),
    }

    async_std::task::sleep(std::time::Duration::from_secs(2)).await;

    let service_data = String::from(r#""{ "entity_id": "group.kitchen" }""#);

    match client.call_service("homeassistant".to_owned(), "update_entity".to_owned(), Some(service_data)).await {
        Ok(done) if done == String::from("command executed successfully") => {
            println!("Good, your command was executed")
        }
        Ok(_) => println!("Ooops, I got strange result"),
        Err(error) => println!("Ooops, I got this error {}", error)

    }

    Ok(())
}
