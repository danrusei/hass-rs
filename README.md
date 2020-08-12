# Hass-Rs

A simple async Rust client library for interacting with Home Assistant websocket API.

## Configure:

hass-rs supports tokio and async-std runtimes, by default it uses async-std, to use tokio change the feature flags in Cargo.toml

with [async-std](https://async.rs/) support 

```toml
[dependencies]
hass-rs = { version = "0.1", features = ["async-std-runtime"] }
```

with [tokio](https://tokio.rs/) support 

```toml
[dependencies]
hass-rs = { version = "0.1", features = ["tokio-runtime"] }
```

## API reference

* **client::connect(host, port)** -- establish the websocket connection to Home Assistant server.
* **client.auth_with_longlivedtoken(token)** - authenticate the session using a long-lived access token.
* **client.ping()** - can serve as a heartbeat, to ensure the connection is still alive.
* **client.subscribe_event(event_name, callback)** - subscribe the client to the event bus. the callback is a closure, of type Fn(WSEvent), which is executed every time when a specific event is received.
* **client.unsubscribe_event(subscription_id)** - unsubscribe the client from the event bus.
* **client.get_config()** - it gets a dump of the current config in Home Assistant.
* **client.get_states()** - it gets a dump of all the current states in Home Assistant.
* **client.get_services()** - it gets a dump of the current services in Home Assistant. 
* **client.call_service(domain, service, service_data)** - it calls call a service in Home Assistant.

## Example usage

```rust
use env_logger;
use hass_rs::client;
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

    println!("Get Hass Config");
    match client.get_config().await {
        Ok(v) => println!("{:?}", v),
        Err(err) => println!("Oh no, an error: {}", err),
    }
    Ok(())
}
```

## Development status

- [x] Create the client
    - [ ] Automatic reconnection (in progress)
    - [x] Authenticate using long-lived access tokens
    - [ ] Authenticate using OAuth2 (in progress)
- [x] Call a service
- [x] Subscribe
    - [x] Events
    - [ ] Config (you need this?, raise an Issue)
    - [ ] Services (you need this?, raise an Issue)
- [x] UnSubscribe
- [x] Fetch Commands
    - [x] Fetching states
    - [x] Fetching config
    - [x] Fetching services
    - [ ] Fetching pannels (you need this?, raise an Issue)
    - [ ] Fetching media player thumbnails (you need this?, raise an Issue)
- [x] Ping - Pong
