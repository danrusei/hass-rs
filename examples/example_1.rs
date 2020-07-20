use hass_rs::HassClient;

static TOKEN: &str = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJpc3MiOiI0YzcyOGFjNDQ4MTc0NWIwODUxY2ZjMGE5YTc2ZWE1NSIsImlhdCI6MTU5NTIzNDYwMiwiZXhwIjoxOTEwNTk0NjAyfQ.Ow-mSTKNUSyqcJJrSBMYy6ftKMiTEwhMl-uhtBxln80";

#[cfg_attr(feature = "async-std-runtime", async_std::main)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = HassClient::new("localhost", 8123).connect(TOKEN).await?;

    Ok(())
}