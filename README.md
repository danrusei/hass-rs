# Hass-Rs

This is a websocket client Library written in Rust that can be used to integrate Home Assistant into your application. It communicates with the Home Assistant websocket API.


## Usage:

cargo run --example example_1

## Development status

- [x] Create the client
- [ ] Automatic reconnection
- [x] Authenticate using long-lived access tokens
- [ ] Authenticate using OAuth2
- [x] Ping - Pong
- [ ] Fetch Commands
    - [ ] Fetch State
    - [ ] Fetch Config
    - [ ] Fetch Services
- [ ] Subscribe Events
    - [ ] Events
    - [ ] Config
    - [ ] Services
- [ ] Call a service
