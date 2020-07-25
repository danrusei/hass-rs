use crate::command::Command;
use crate::errors::{HassError, HassResult};
use crate::response::Response;
use crate::runtime::{connect_async, task, WebSocket};

use async_tungstenite::tungstenite::Message as TungsteniteMessage;
use futures::channel::mpsc::{channel, Receiver, Sender};
use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use url;

#[derive(Debug)]
pub struct WsConn {
    //message sequence required by the Websocket server
    last_sequence: Arc<AtomicU64>,

    //Client --> Gateway (send "Commands" msg to the Gateway)
    pub(crate) to_gateway: Sender<Command>,

    //Gateway --> Client (receive "Response" msg from the Gateway)
    pub(crate) from_gateway: Receiver<HassResult<Response>>,

    //TODO
    // I may have to create an hashmap for Commands and another one for Events
    // so when I receive an response I can search both hashmap and know the type of event to json Deserialize 
}

impl WsConn {
    pub async fn connect(url: url::Url) -> HassResult<WsConn> {
        let wsclient = connect_async(url).await.expect("Can't connect to gateway");
        let (sink, stream) = wsclient.split();

        //Channels to recieve the Client Command and send it over to the Websocket server
        let (to_gateway, from_client) = channel::<Command>(20);

        //Channels to receive the Response from the Websocket server and send it over to the Client
        let (to_client, from_gateway) = channel::<HassResult<Response>>(20);

        let last_sequence = Arc::new(AtomicU64::default());
        let last_sequence_clone_sender = Arc::clone(&last_sequence);
        let last_sequence_clone_receiver = Arc::clone(&last_sequence);

        // Client --> Gateway
        if let Err(e) = sender_loop(last_sequence_clone_sender, sink, from_client).await {
            //TODO - properly handle the Errors
            return Err(e);
        }

        //Gateway --> Client
        if let Err(e) = receiver_loop(last_sequence_clone_receiver, stream, to_client).await {
            match e {
                HassError::AuthenticationFailed | HassError::ConnectionClosed => {
                    //TODO - to_client.send(Response::Close(e).await.expect("Messahe closed"));
                    return Err(e);
                }
                _ => {}
            }
        };

        Ok(WsConn {
            last_sequence,
            to_gateway,
            from_gateway,
        })
    }
}

async fn sender_loop(
    last_sequence: Arc<AtomicU64>,
    mut sink: SplitSink<WebSocket, TungsteniteMessage>,
    mut from_client: Receiver<Command>,
) -> HassResult<()> {
    task::spawn(async move {
        loop {
            match from_client.next().await {
                Some(item) => match item {
                    Command::Close => {
                        return sink
                            .send(TungsteniteMessage::Close(None))
                            .await
                            .map_err(|_| HassError::ConnectionClosed);
                    }
                    Command::AuthInit(auth) => {
                        // Get the last sequence
                        // let seq = match last_sequence.load(Ordering::Relaxed) {
                        //         0 => None,
                        //          v => Some(v),
                        // };

                        // Transform command to TungsteniteMessage
                        let cmd = Command::AuthInit(auth).to_tungstenite_message();

                        // Send command to gateway
                        // NOT GOOD as it is not returned
                        sink.send(cmd)
                            .await
                            .map_err(|_| HassError::ConnectionClosed)
                            .unwrap();

                        // Send command to gateway
                        // if let Err(e) = sink.send(TungsteniteMessage::Text(item)).await {
                        //     let mut sender = guard.remove(&msg.0).unwrap();
                        //     sender
                        //         .send(Err(HassError::from(e)))
                        //         .await
                        //         .expect("Failed to send error");
                        // };
                    }
                    Command::Ping(ping) => {
                        // Get the last sequence
                        // let seq = match last_sequence.load(Ordering::Relaxed) {
                        //         0 => None,
                        //          v => Some(v),
                        // };

                        // Transform command to TungsteniteMessage
                        let cmd = Command::Ping(ping).to_tungstenite_message();

                         // Send command to gateway
                        // NOT GOOD as it is not returned
                        sink.send(cmd)
                            .await
                            .map_err(|_| HassError::ConnectionClosed)
                            .unwrap();

                        // Send command to gateway
                        // if let Err(e) = sink.send(TungsteniteMessage::Text(item)).await {
                        //     let mut sender = guard.remove(&msg.0).unwrap();
                        //     sender
                        //         .send(Err(HassError::from(e)))
                        //         .await
                        //         .expect("Failed to send error");
                        // };


                    }
                    // Command::Msg(msg) => {
                    //     let mut guard = requests.lock().await;
                    //     guard.insert(msg.0, msg.1);
                    //     if let Err(e) = sink.send(TungsteniteMessage::Binary(msg.2)).await {
                    //         let mut sender = guard.remove(&msg.0).unwrap();
                    //         sender
                    //             .send(Err(HassError::from(e)))
                    //             .await
                    //             .expect("Failed to send error");
                    //     }
                    //     drop(guard);
                    // }
                    // Command::Shutdown => {
                    //     let mut guard = requests.lock().await;
                    //     guard.clear();
                    // }
                    _ => todo!("sender_loop, other options"),
                },
                None => {}
            }
        }
    });

    Ok(())
}

async fn receiver_loop(
    last_sequence: Arc<AtomicU64>,
    mut stream: SplitStream<WebSocket>,
    mut to_client: Sender<HassResult<Response>>,
) -> HassResult<()> {
    task::spawn(async move {
        loop {
            match stream.next().await {
                Some(Err(error)) => match to_client.send(Err(HassError::from(&error))).await {
                    Ok(_r) => {}
                    Err(e) => {}
                },
                Some(Ok(item)) => match item {
                    //Authentication has no id compared with all the other messages(Response)
                    TungsteniteMessage::Text(data) => {

                        //There is no explicit tag identifying which variant the data contains. 
                        //Serde will try to match the data against each variant in order and the first one that deserializes successfully is the one returned.
                       let payload: Result<Response, HassError> = serde_json::from_str(&data).map_err(|_| HassError::UnknownPayloadReceived);
                       to_client.send(payload).await.unwrap();

                    }
                    TungsteniteMessage::Ping(data) => {
                        todo!("receiver_loop, I should not get this")
                    }
                    _ => {}
                },
                _ => {} // Some(Err(error)) => {
                        //     let mut guard = requests.lock().await;
                        //     for s in guard.values_mut() {
                        //         match s.send(Err(HassError::from(&error))).await {
                        //             Ok(_r) => {}
                        //             Err(_e) => {}
                        //         }
                        //     }
                        //     guard.clear();
                        // }
                        // Some(Ok(item)) => match item {
                        //     TungsteniteMessage::Text(data) => {
                        //         let response: Response = serde_json::from_str(&data)
                        //             .map_err(|_| HassError::UnknownPayloadReceived)
                        //             .unwrap();
                        //         let mut guard = requests.lock().await;
                        //         if response.status.code != 206 {
                        //             let item = guard.remove(&response.sequence);
                        //             drop(guard);
                        //             if let Some(mut s) = item {
                        //                 match s.send(Ok(response)).await {
                        //                     Ok(_r) => {}
                        //                     Err(_e) => {}
                        //                 };
                        //             }
                        //         } else {
                        //             let item = guard.get_mut(&response.sequence);
                        //             if let Some(s) = item {
                        //                 match s.send(Ok(response)).await {
                        //                     Ok(_r) => {}
                        //                     Err(_e) => {}
                        //                 };
                        //             }
                        //             drop(guard);
                        //         }
                        //     }
            }
        }
    });
    Ok(())
}
