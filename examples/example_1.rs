use hass_rs::{config::Config, HassClient};

static TOKEN: &str = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJpc3MiOiI0YzcyOGFjNDQ4MTc0NWIwODUxY2ZjMGE5YTc2ZWE1NSIsImlhdCI6MTU5NTIzNDYwMiwiZXhwIjoxOTEwNTk0NjAyfQ.Ow-mSTKNUSyqcJJrSBMYy6ftKMiTEwhMl-uhtBxln80";

#[cfg_attr(feature = "async-std-runtime", async_std::main)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = HassClient::new(Config {
        host: "localhost".to_owned(),
        port: 8123,
        token: TOKEN.to_owned(),
    });

    client.connect().await?;
    dbg!("Connected");

    let payload = "Trying again";
    let result = client.command(payload).await?;
    println!("{:?}", result);
    dbg!("Command was sent");

    Ok(())
}
