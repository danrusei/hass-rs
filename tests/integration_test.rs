use futures_util::{SinkExt, StreamExt};
use hass_rs::client::HassClient;
use hass_rs::errors::HassError;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

async fn setup_mock_server() -> (TcpListener, String) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("ws://{}", addr);
    (listener, url)
}

#[tokio::test]
async fn test_auth_success() {
    let (listener, url) = setup_mock_server().await;

    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = accept_async(stream).await.unwrap();

        // 1. Send auth_required
        ws.send(Message::Text(
            r#"{"type":"auth_required","ha_version":"2021.3.0"}"#.into(),
        ))
        .await
        .unwrap();

        // 2. Receive auth command
        let msg = ws.next().await.unwrap().unwrap();
        let text = msg.to_text().unwrap();
        assert!(text.contains(r#""type":"auth""#));
        assert!(text.contains(r#""access_token":"valid_token""#));

        // 3. Send auth_ok
        ws.send(Message::Text(
            r#"{"type":"auth_ok","ha_version":"2021.3.0"}"#.into(),
        ))
        .await
        .unwrap();
    });

    let mut client = HassClient::new(&url).await.unwrap();
    let auth_res = client.auth_with_longlivedtoken("valid_token").await;
    assert!(auth_res.is_ok());

    server_task.await.unwrap();
}

#[tokio::test]
async fn test_auth_failure() {
    let (listener, url) = setup_mock_server().await;

    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = accept_async(stream).await.unwrap();

        // 1. Send auth_required
        ws.send(Message::Text(
            r#"{"type":"auth_required","ha_version":"2021.3.0"}"#.into(),
        ))
        .await
        .unwrap();

        // 2. Receive auth command
        let msg = ws.next().await.unwrap().unwrap();
        let text = msg.to_text().unwrap();
        assert!(text.contains(r#""type":"auth""#));
        assert!(text.contains(r#""access_token":"invalid_token""#));

        // 3. Send auth_invalid
        ws.send(Message::Text(
            r#"{"type":"auth_invalid","message":"Invalid token"}"#.into(),
        ))
        .await
        .unwrap();
    });

    let mut client = HassClient::new(&url).await.unwrap();
    let auth_res = client.auth_with_longlivedtoken("invalid_token").await;
    assert!(auth_res.is_err());
    if let Err(HassError::AuthenticationFailed(msg)) = auth_res {
        assert_eq!(msg, "Invalid token");
    } else {
        panic!("Expected AuthenticationFailed error");
    }

    server_task.await.unwrap();
}

#[tokio::test]
async fn test_ping_pong() {
    let (listener, url) = setup_mock_server().await;

    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = accept_async(stream).await.unwrap();

        ws.send(Message::Text(
            r#"{"type":"auth_required","ha_version":"2021.3.0"}"#.into(),
        ))
        .await
        .unwrap();

        let msg = ws.next().await.unwrap().unwrap();
        assert!(msg.to_text().unwrap().contains(r#""type":"auth""#));
        ws.send(Message::Text(
            r#"{"type":"auth_ok","ha_version":"2021.3.0"}"#.into(),
        ))
        .await
        .unwrap();

        let msg = ws.next().await.unwrap().unwrap();
        let text = msg.to_text().unwrap();
        assert!(text.contains(r#""type":"ping""#));
        assert!(text.contains(r#""id":1"#));

        ws.send(Message::Text(r#"{"id":1,"type":"pong"}"#.into()))
            .await
            .unwrap();
    });

    let mut client = HassClient::new(&url).await.unwrap();
    client.auth_with_longlivedtoken("token").await.unwrap();

    let ping_res = client.ping().await;
    assert!(ping_res.is_ok());

    server_task.await.unwrap();
}

#[tokio::test]
async fn test_get_config() {
    let (listener, url) = setup_mock_server().await;

    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = accept_async(stream).await.unwrap();

        ws.send(Message::Text(
            r#"{"type":"auth_required","ha_version":"2021.3.0"}"#.into(),
        ))
        .await
        .unwrap();

        let msg = ws.next().await.unwrap().unwrap();
        assert!(msg.to_text().unwrap().contains(r#""type":"auth""#));
        ws.send(Message::Text(
            r#"{"type":"auth_ok","ha_version":"2021.3.0"}"#.into(),
        ))
        .await
        .unwrap();

        let msg = ws.next().await.unwrap().unwrap();
        let text = msg.to_text().unwrap();
        assert!(text.contains(r#""type":"get_config""#));
        assert!(text.contains(r#""id":1"#));

        let response_json = r#"{
            "id": 1,
            "type": "result",
            "success": true,
            "result": {
                "latitude": 52.379189,
                "longitude": 4.899431,
                "elevation": 12,
                "unit_system": {
                    "length": "km",
                    "mass": "g",
                    "pressure": "Pa",
                    "temperature": "C",
                    "volume": "L"
                },
                "location_name": "Home",
                "time_zone": "Europe/Amsterdam",
                "components": ["frontend", "config"],
                "config_dir": "/config",
                "whitelist_external_dirs": [],
                "version": "2021.3.0",
                "config_source": "storage",
                "safe_mode": false,
                "external_url": "https://example.com",
                "internal_url": "http://192.168.1.2:8123"
            }
        }"#;
        ws.send(Message::Text(response_json.into())).await.unwrap();
    });

    let mut client = HassClient::new(&url).await.unwrap();
    client.auth_with_longlivedtoken("token").await.unwrap();

    let config = client.get_config().await.unwrap();
    assert_eq!(config.location_name, "Home");
    assert_eq!(config.version, "2021.3.0");
    assert_eq!(config.unit_system.length, "km");

    server_task.await.unwrap();
}

#[tokio::test]
async fn test_subscribe_events() {
    let (listener, url) = setup_mock_server().await;

    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = accept_async(stream).await.unwrap();

        ws.send(Message::Text(
            r#"{"type":"auth_required","ha_version":"2021.3.0"}"#.into(),
        ))
        .await
        .unwrap();

        let msg = ws.next().await.unwrap().unwrap();
        assert!(msg.to_text().unwrap().contains(r#""type":"auth""#));
        ws.send(Message::Text(
            r#"{"type":"auth_ok","ha_version":"2021.3.0"}"#.into(),
        ))
        .await
        .unwrap();

        // Expect subscribe_events command
        let msg = ws.next().await.unwrap().unwrap();
        let text = msg.to_text().unwrap();
        assert!(text.contains(r#""type":"subscribe_events""#));
        assert!(text.contains(r#""id":1"#));

        // Respond with success
        ws.send(Message::Text(
            r#"{"id":1,"type":"result","success":true,"result":null}"#.into(),
        ))
        .await
        .unwrap();

        // Send an event after a brief sleep to ensure client has registered the subscription channel
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let event_json = r#"{
            "id": 1,
            "type": "event",
            "event": {
                "event_type": "state_changed",
                "data": {
                    "entity_id": "light.kitchen",
                    "new_state": {
                        "entity_id": "light.kitchen",
                        "state": "on",
                        "attributes": {},
                        "last_changed": "2024-02-15T11:13:02.291378+00:00",
                        "last_updated": "2024-02-15T11:13:02.291378+00:00",
                        "context": {
                            "id": "01HPRMZAWNXKVVPSP11QFJ53HB",
                            "parent_id": null,
                            "user_id": null
                        }
                    },
                    "old_state": null
                },
                "origin": "LOCAL",
                "time_fired": "2024-02-15T11:13:02.291378+00:00",
                "context": {
                    "id": "01HPRMZAWNXKVVPSP11QFJ53HB",
                    "parent_id": null,
                    "user_id": null
                }
            }
        }"#;
        ws.send(Message::Text(event_json.into())).await.unwrap();
    });

    let mut client = HassClient::new(&url).await.unwrap();
    client.auth_with_longlivedtoken("token").await.unwrap();

    let mut rx = client.subscribe_event("state_changed").await.unwrap();
    let event = rx.recv().await.unwrap();
    assert_eq!(event.event.event_type, "state_changed");
    assert_eq!(event.event.data.entity_id.unwrap(), "light.kitchen");
    assert_eq!(event.event.data.new_state.unwrap().state, "on");

    server_task.await.unwrap();
}

#[tokio::test]
async fn test_close_connection() {
    let (listener, url) = setup_mock_server().await;

    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = accept_async(stream).await.unwrap();

        ws.send(Message::Text(
            r#"{"type":"auth_required","ha_version":"2021.3.0"}"#.into(),
        ))
        .await
        .unwrap();

        let msg = ws.next().await.unwrap().unwrap();
        assert!(msg.to_text().unwrap().contains(r#""type":"auth""#));
        ws.send(Message::Text(
            r#"{"type":"auth_ok","ha_version":"2021.3.0"}"#.into(),
        ))
        .await
        .unwrap();

        // Expect get_config command
        let msg = ws.next().await.unwrap().unwrap();
        assert!(msg.to_text().unwrap().contains(r#""type":"get_config""#));

        // Send Close frame
        ws.send(Message::Close(None)).await.unwrap();
    });

    let mut client = HassClient::new(&url).await.unwrap();
    client.auth_with_longlivedtoken("token").await.unwrap();

    let res = client.get_config().await;
    assert!(res.is_err());
    if let Err(HassError::UnknownPayloadReceived(hass_rs::types::Response::Close(reason))) = res {
        assert_eq!(reason, "");
    } else {
        panic!(
            "Expected UnknownPayloadReceived(Response::Close) error, got {:?}",
            res
        );
    }

    server_task.await.unwrap();
}

#[tokio::test]
async fn test_unsubscribe_event_manual() {
    let (listener, url) = setup_mock_server().await;

    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = accept_async(stream).await.unwrap();

        ws.send(Message::Text(
            r#"{"type":"auth_required","ha_version":"2021.3.0"}"#.into(),
        ))
        .await
        .unwrap();

        let msg = ws.next().await.unwrap().unwrap();
        assert!(msg.to_text().unwrap().contains(r#""type":"auth""#));
        ws.send(Message::Text(
            r#"{"type":"auth_ok","ha_version":"2021.3.0"}"#.into(),
        ))
        .await
        .unwrap();

        // 1. Expect subscribe_events command (ID = 1)
        let msg = ws.next().await.unwrap().unwrap();
        assert!(msg.to_text().unwrap().contains(r#""type":"subscribe_events""#));
        ws.send(Message::Text(
            r#"{"id":1,"type":"result","success":true,"result":null}"#.into(),
        ))
        .await
        .unwrap();

        // 2. Expect unsubscribe_events command (ID = 2) for subscription 1
        let msg = ws.next().await.unwrap().unwrap();
        let text = msg.to_text().unwrap();
        assert!(text.contains(r#""type":"unsubscribe_events""#));
        assert!(text.contains(r#""subscription":1"#));
        assert!(text.contains(r#""id":2"#));

        // Respond with success to unsubscribe command
        ws.send(Message::Text(
            r#"{"id":2,"type":"result","success":true,"result":null}"#.into(),
        ))
        .await
        .unwrap();
    });

    let mut client = HassClient::new(&url).await.unwrap();
    client.auth_with_longlivedtoken("token").await.unwrap();

    let _rx = client.subscribe_event("state_changed").await.unwrap();

    let unsub_res = client.unsubscribe_event(1).await;
    assert!(unsub_res.is_ok());

    server_task.await.unwrap();
}

#[tokio::test]
async fn test_unsubscribe_event_auto() {
    let (listener, url) = setup_mock_server().await;

    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = accept_async(stream).await.unwrap();

        ws.send(Message::Text(
            r#"{"type":"auth_required","ha_version":"2021.3.0"}"#.into(),
        ))
        .await
        .unwrap();

        let msg = ws.next().await.unwrap().unwrap();
        assert!(msg.to_text().unwrap().contains(r#""type":"auth""#));
        ws.send(Message::Text(
            r#"{"type":"auth_ok","ha_version":"2021.3.0"}"#.into(),
        ))
        .await
        .unwrap();

        // 1. Expect subscribe_events command (ID = 1)
        let msg = ws.next().await.unwrap().unwrap();
        assert!(msg.to_text().unwrap().contains(r#""type":"subscribe_events""#));
        ws.send(Message::Text(
            r#"{"id":1,"type":"result","success":true,"result":null}"#.into(),
        ))
        .await
        .unwrap();

        // 2. Wait a bit for subscription setup on client side, then send an event
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let event_json = r#"{
            "id": 1,
            "type": "event",
            "event": {
                "event_type": "state_changed",
                "data": {
                    "entity_id": "light.kitchen",
                    "new_state": null,
                    "old_state": null
                },
                "origin": "LOCAL",
                "time_fired": "2024-02-15T11:13:02.291378+00:00",
                "context": {
                    "id": "01HPRMZAWNXKVVPSP11QFJ53HB",
                    "parent_id": null,
                    "user_id": null
                }
            }
        }"#;
        ws.send(Message::Text(event_json.into())).await.unwrap();

        // 3. Expect auto unsubscribe command from client background loop
        let msg = ws.next().await.unwrap().unwrap();
        let text = msg.to_text().unwrap();
        assert!(text.contains(r#""type":"unsubscribe_events""#));
        assert!(text.contains(r#""subscription":1"#));
    });

    let mut client = HassClient::new(&url).await.unwrap();
    client.auth_with_longlivedtoken("token").await.unwrap();

    let rx = client.subscribe_event("state_changed").await.unwrap();

    // Explicitly drop rx to trigger auto-unsubscribe on next received event
    drop(rx);

    server_task.await.unwrap();
}


#[test]
fn test_deserialize_event() {
    let event_json = r#"{
        "id": 1,
        "type": "event",
        "event": {
            "event_type": "state_changed",
            "data": {
                "entity_id": "light.kitchen",
                "new_state": {
                    "entity_id": "light.kitchen",
                    "state": "on",
                    "attributes": {},
                    "last_changed": "2024-02-15T11:13:02.291378+00:00",
                    "last_updated": "2024-02-15T11:13:02.291378+00:00",
                    "context": {
                        "id": "01HPRMZAWNXKVVPSP11QFJ53HB",
                        "parent_id": null,
                        "user_id": null
                    }
                },
                "old_state": null
            },
            "origin": "LOCAL",
            "time_fired": "2024-02-15T11:13:02.291378+00:00",
            "context": {
                "id": "01HPRMZAWNXKVVPSP11QFJ53HB",
                "parent_id": null,
                "user_id": null
            }
        }
    }"#;
    let res: Result<hass_rs::types::Response, _> = serde_json::from_str(event_json);
    println!("DESERIALIZATION RESULT: {:?}", res);
    let res = res.unwrap();
    assert!(matches!(res, hass_rs::types::Response::Event(_)));
}

