# Hass-Rs

A simple async Rust client library for interacting with Home Assistant **websocket** API.

## Test environment

Connect to your Home Assistant server, or follow the instructions from the [Installation Guide](https://www.home-assistant.io/installation/).  
For development, [docker](https://www.home-assistant.io/installation/linux#install-home-assistant-container) can be used to easily bootstrap a test environment.

Steps to run the provided Examples:

* Clone the hass_rs github repository
* Run the homeassistant server in a docker container

```bash
docker run -d --name="home-assistant" -v /PATH_TO_YOUR_CONFIG:/config -v /etc/localtime:/etc/localtime:ro --net=host homeassistant/home-assistant:stable
```

* Login to the Home Assistant web interface: <http://localhost:8123/>
* Go to `Profile` --> `Long-Lived Access Tokens` and create a token to be used by hass_rs client
* Set the environment variable ***export HASS_TOKEN=<YOUR_TOKEN_HERE>***
* Run the example scripts:
  * `cargo run --example get_cmds`
  * `cargo run --example call_service`
  * `cargo run --example subscribe_event`
  * `cargo run --example get_cmds_async_std --features use-async-std --no-default-features` - example with **async-std** runtime

## Example usage

Check the [Example folder](https://github.com/danrusei/hass-rs/tree/master/examples) for additional details on how to use various hass-rs functions.

```rust
use hass_rs::client::HassClient;
use lazy_static::lazy_static;
use serde_json::json;
use std::env::var;

lazy_static! {
    static ref TOKEN: String =
        var("HASS_TOKEN").expect("please set up the HASS_TOKEN env variable before running this");
}

#[tokio::main]
async fn main() {
    let url = "ws://localhost:8123/api/websocket";

    println!("Connecting to - {}", url);
    let mut client = HassClient::new(url).await.expect("Failed to connect");

    client
        .auth_with_longlivedtoken(&*TOKEN)
        .await
        .expect("Not able to autheticate");

    println!("WebSocket connection and authethication works\n");

    println!("Getting the Config:\n");
    let cmd2 = client
        .get_config()
        .await
        .expect("Unable to retrieve the Config");
    println!("config: {}\n", cmd2);
}
```

## Development status

* [x] Create the client
  * [ ] Automatic reconnection (TBD)
  * [x] Authenticate using long-lived access tokens
  * [ ] Authenticate using OAuth2 (TBD)
* [x] Call a service
* [x] Subscribe
  * [x] Events
  * [ ] Config (you need this?, raise an Issue)
  * [ ] Services (you need this?, raise an Issue)
* [x] UnSubscribe
* [x] Fetch Commands
  * [x] Fetching states
  * [x] Fetching config
  * [x] Fetching services
  * [x] Fetching panels
  * [ ] Fetching media player thumbnails (you need this?, raise an Issue)
* [ ] Ping - Pong
