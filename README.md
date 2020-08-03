# Hass-Rs

This is a websocket client Library written in Rust that can be used to integrate Home Assistant into your application. It communicates with the Home Assistant websocket API.


## Usage:

cargo run --example example_1

## Development status

- [x] Create the client
    - [ ] Automatic reconnection
    - [x] Authenticate using long-lived access tokens
    - [ ] Authenticate using OAuth2
- [ ] Call a service
- [x] Subscribe
    - [x] Events
    - [ ] Config
    - [ ] Services
- [x] UnSubscribe
- [x] Fetch Commands
    - [ ] Fetching states
    - [x] Fetching config
    - [ ] Fetching services
    - [ ] Fetching pannels
    - [ ] Fetching media player thumbnails
- [x] Pings - Pongs
