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
use std::env::var;
use std::sync::OnceLock;

static TOKEN: OnceLock<String> = OnceLock::new();

#[tokio::main]
async fn main() {
    let url = "ws://localhost:8123/api/websocket";

    println!("Connecting to - {}", url);
    let client = HassClient::new(url).await.expect("Failed to connect");

    let token = TOKEN.get_or_init(|| {
        var("HASS_TOKEN").expect("please set up the HASS_TOKEN env variable before running this")
    });

    client
        .auth_with_longlivedtoken(token)
        .await
        .expect("Not able to authenticate");

    println!("WebSocket connection and authentication works\n");

    println!("Getting the Config:\n");
    let cmd2 = client
        .get_config()
        .await
        .expect("Unable to retrieve the Config");
    println!("config: {}\n", cmd2);
}
```

## Reconnection

The client supervises its websocket connection. When the connection is lost it
reconnects on its own, re-authenticates with the cached long lived access token
and restores the event subscriptions, so the `Receiver` you got from
`subscribe_event` keeps delivering events across reconnections.

* A request that is in flight when the connection dies fails with
  `HassError::ConnectionClosed` (or `UnknownPayloadReceived(Response::Close(_))`);
  it never blocks forever. If you need an answer, retry it — a retry is accepted
  again as soon as the session is back up.
* Reconnection uses an exponential backoff between 1s and 60s, with jitter.
* Subscriptions are restored after every reconnection, so the `Receiver` you
  already hold keeps working. Its channel buffers 20 events: a consumer that
  falls further behind loses events (a warning is logged) instead of stalling the
  connection for every other command and subscription.
* Commands are only accepted while the session is `Connected`; before
  authentication (and while a session is being restored) they fail at once with
  `HassError::ConnectionClosed` instead of being written to a socket that cannot
  answer.
* A client that never authenticated has no session to restore, so it ends up
  `Closed` when its connection drops — create a new client in that case.
* A ping heartbeat every 30s detects connections that look alive but stopped
  answering (sleeping machine, NAT timeout, dead peer).
* The state of the connection is observable through
  `client.connection_state()` and `client.watch_connection_state()`.

```rust
use hass_rs::client::{ConnectionState, HassClient, ReconnectOptions};
use std::time::Duration;

let options = ReconnectOptions {
    initial_delay: Duration::from_secs(2),
    max_attempts: Some(20),
    heartbeat_interval: Some(Duration::from_secs(15)),
    ..ReconnectOptions::default()
};
let client = HassClient::new_with_options(url, options).await?;
client.auth_with_longlivedtoken(token).await?;

let mut state = client.watch_connection_state();
loop {
    state.changed().await?;
    match *state.borrow_and_update() {
        ConnectionState::Connected => println!("connected"),
        ConnectionState::Disconnected => println!("reconnecting"),
        ConnectionState::Closed => break,
        ConnectionState::Connecting => println!("connecting"),
    }
}
```

Use `ReconnectOptions::disabled()` for a plain one-shot connection: the client
then ends the session (state `Closed`, subscription channels closed) instead of
reconnecting.

## Development status

* [x] Create the client
  * [x] Automatic reconnection
    * [x] Exponential backoff with jitter
    * [x] Re-authenticate with the cached access token
    * [x] Restore event subscriptions
    * [x] Ping heartbeat to detect half-open connections
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
* [x] Ping - Pong
