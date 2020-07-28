use crate::command::{Command, Subscribe};
use crate::errors::{HassError, HassResult};
use crate::response::Response;
use crate::runtime::{connect_async, task, WebSocket};

use std::collections::HashMap;
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

pub struct WsConn {
    //message sequence required by the Websocket server
    last_sequence: Arc<AtomicU64>,

    //Client --> Gateway (send "Commands" msg to the Gateway)
    pub(crate) to_gateway: Sender<Command>,

    //Gateway --> Client (receive "Response" msg from the Gateway)
    pub(crate) from_gateway: Receiver<HassResult<Response>>,

    //Register all the events to be listen and its callback
    //TODO intial form, but I can send a result like Box<dyn Fn(String) -> BoxFuture<'static, EventResult>
    pub(crate) event_listeners: HashMap<String, Box<dyn FnOnce() + Send>>,
    //TODO
    // I may have to create an hashmap for Commands and another one for Events
    // so when I receive an response I can search both hashmap and know the type of event to json Deserialize 
}

impl WsConn {
    pub(crate) async fn connect(url: url::Url) -> HassResult<WsConn> {
        let wsclient = connect_async(url).await.expect("Can't connect to gateway");
        let (sink, stream) = wsclient.split();

        //Channels to recieve the Client Command and send it over to the Websocket server
        let (to_gateway, from_client) = channel::<Command>(20);

        //Channels to receive the Response from the Websocket server and send it over to the Client
        let (to_client, from_gateway) = channel::<HassResult<Response>>(20);

        let last_sequence = Arc::new(AtomicU64::new(1));
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
            event_listeners: HashMap::new(),
        })
    }

    pub(crate) async fn command(&mut self, cmd: Command) -> HassResult<Response> {

        // Send the auth command to gateway
        self.to_gateway
            .send(cmd)
            .await
            .map_err(|_| HassError::ConnectionClosed)?;

        // Receive auth response event from the gateway
        let response = self.from_gateway
            .next()
            .await
            .ok_or_else(|| HassError::ConnectionClosed)?;
        
        response
    }

    pub(crate) async fn subscribe_message<F>(&mut self, event_name: &str, callback: F ) -> HassResult<String> 
    where
        F: FnOnce() + Send + 'static,
        {
            

            //create the Event Subscribe Command
            let cmd = Command::SubscribeEvent(Subscribe{
                id: None,
                msg_type: "subscribe_events".to_owned(),
                event_type: event_name.to_owned(),
                
            });

            //send command to subscribe to specific event
            let response = self.command(cmd).await.unwrap();

            //this function will be executed on the Event received from Stream
            //Check the response
         match response {
            Response::Result(v) => {
                // if the response is with suceess then the callback is registered to Event
                if v.success == true {
                    self.event_listeners.insert(event_name.to_owned(), Box::new(callback));
                    return Ok("Ok".to_owned())
                }

                return Ok("NOT OK".to_owned())
            },
            Response::ResultError(err) =>  return Err(HassError::ReponseError(err)),
            _ => return Err(HassError::UnknownPayloadReceived),
        }
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
                    Command::Ping(mut ping) => {
                        
                        // Increase the last sequence and use the previous value in the request
                         let seq = match last_sequence.fetch_add(1, Ordering::Relaxed) {
                                 0 => None,
                                  v => Some(v),
                         };

                        ping.id = seq;
                         
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
                    Command::SubscribeEvent(mut subscribe) => {
                         
                        // Increase the last sequence and use the previous value in the request
                         let seq = match last_sequence.fetch_add(1, Ordering::Relaxed) {
                            0 => None,
                             v => Some(v),
                    };

                        subscribe.id = seq;

                        // Transform command to TungsteniteMessage
                        let cmd = Command::SubscribeEvent(subscribe).to_tungstenite_message();

                         // Send command to gateway
                        // NOT GOOD as it is not returned
                        sink.send(cmd)
                            .await
                            .map_err(|_| HassError::ConnectionClosed)
                            .unwrap();
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
                
                Some(Ok(item)) => match item {
                    //Authentication has no id compared with all the other messages(Response)
                    TungsteniteMessage::Text(data) => {

                        //There is no explicit tag identifying which variant the data contains. 
                        //Serde will try to match the data against each variant in order and the first one that deserializes successfully is the one returned.
                       let payload: Result<Response, HassError> = serde_json::from_str(&data).map_err(|_| HassError::UnknownPayloadReceived);
                       
                       // do I need to check anything here before sending to client?
                       to_client.send(payload).await.unwrap();

                    }
                    _ => {}
                },

                Some(Err(error)) => match to_client.send(Err(HassError::from(&error))).await {
                    //send the error to client ("unexpected message format, like a new error")
                    Ok(_r) => {}
                    Err(_e) => {}
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
