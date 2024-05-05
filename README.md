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

use async_tungstenite::tungstenite::{Error, Message};
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use lazy_static::lazy_static;
use std::env::var;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{mpsc, mpsc::Receiver, mpsc::Sender};
use tokio_tungstenite::{connect_async, WebSocketStream};

lazy_static! {
    static ref TOKEN: String =
        var("HASS_TOKEN").expect("please set up the HASS_TOKEN env variable before running this");
}

async fn ws_incoming_messages(
    mut stream: SplitStream<WebSocketStream<impl AsyncRead + AsyncWrite + Unpin>>,
    to_user: Sender<Result<Message, Error>>,
) {
    loop {
        while let Some(message) = stream.next().await {
            let _ = to_user.send(message).await;
        }
    }
}

async fn ws_outgoing_messages(
    mut sink: SplitSink<WebSocketStream<impl AsyncRead + AsyncWrite + Unpin>, Message>,
    mut from_user: Receiver<Message>,
) {
    loop {
        match from_user.recv().await {
            Some(msg) => sink.send(msg).await.expect("Failed to send message"),
            None => continue,
        }
    }
}

#[tokio::main]
async fn main() {
    let url = "ws://localhost:8123/api/websocket";

    println!("Connecting to - {}", url);
    let (wsclient, _) = connect_async(url).await.expect("Failed to connect");
    let (sink, stream) = wsclient.split();

    //Channels to recieve the Client Command and send it over to Websocket server
    let (to_gateway, from_user) = mpsc::channel::<Message>(20);
    //Channels to receive the Response from the Websocket server and send it over to Client
    let (to_user, from_gateway) = mpsc::channel::<Result<Message, Error>>(20);

    // Handle incoming messages in a separate task
    let read_handle = tokio::spawn(ws_incoming_messages(stream, to_user));

    // Read from command line and send messages
    let write_handle = tokio::spawn(ws_outgoing_messages(sink, from_user));

    let mut client = HassClient::new(to_gateway, from_gateway);

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

    // Await both tasks (optional, depending on your use case)
    let _ = tokio::try_join!(read_handle, write_handle);
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
