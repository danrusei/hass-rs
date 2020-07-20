use hass_rs::HassClient;
use uuid::Uuid;

static TOKEN: &str = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJpc3MiOiI0YzcyOGFjNDQ4MTc0NWIwODUxY2ZjMGE5YTc2ZWE1NSIsImlhdCI6MTU5NTIzNDYwMiwiZXhwIjoxOTEwNTk0NjAyfQ.Ow-mSTKNUSyqcJJrSBMYy6ftKMiTEwhMl-uhtBxln80";

#[cfg_attr(feature = "async-std-runtime", async_std::main)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = HassClient::new("localhost", 8123);
    dbg!("before connection");
    client.connect(TOKEN).await?;
    dbg!("connected");
    let num = Uuid::new_v4();
    let payload = vec![1u8, 2, 3, 4];
    let result = client.run(num, payload).await?;
    dbg!(result);
   // println!("{:?}", result.0);

    Ok(())
}