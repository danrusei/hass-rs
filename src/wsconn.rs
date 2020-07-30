use crate::command::{Command, Subscribe};
use crate::errors::{HassError, HassResult};
use crate::response::Response;
use crate::runtime::{connect_async, task, WebSocket};

use async_tungstenite::tungstenite::Message as TungsteniteMessage;
use futures::channel::mpsc::{channel, Receiver, Sender};
use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use log::info;
use std::collections::HashMap;

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
    pub(crate) event_listeners: HashMap<u64, Box<dyn FnOnce() + Send>>,
    //TODO hashmap for Commands, is it needed ?
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
        let response = self
            .from_gateway
            .next()
            .await
            .ok_or_else(|| HassError::ConnectionClosed)?;

        response
    }

    pub(crate) async fn subscribe_message<F>(
        &mut self,
        event_name: &str,
        callback: F,
    ) -> HassResult<String>
    where
        F: FnOnce() + Send + 'static,
    {
        //create the Event Subscribe Command
        let cmd = Command::SubscribeEvent(Subscribe {
            id: None,
            msg_type: "subscribe_events".to_owned(),
            event_type: event_name.to_owned(),
        });

        //send command to subscribe to specific event
        let response = self.command(cmd).await.unwrap();

        //Add the callback in the event_listeners hashmap if the Subscription Response is successfull
        match response {
            Response::Result(v) if v.success == true => {
                self.event_listeners
                    .insert(v.id, Box::new(callback));
                return Ok("Ok".to_owned());
            }
            Response::Result(v) if v.success == false => return Err(HassError::ReponseError(v)),
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
                        // NOT GOOD as it is not returned, see above
                        sink.send(cmd)
                            .await
                            .map_err(|_| HassError::ConnectionClosed)
                            .unwrap();
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
                        // NOT GOOD as it is not returned, see above
                        sink.send(cmd)
                            .await
                            .map_err(|_| HassError::ConnectionClosed)
                            .unwrap();
                    }
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
                    TungsteniteMessage::Text(data) => {
                        info!("{}", data);

                        //Serde: The tag identifying which variant we are dealing with is now inside of the content,
                        // next to any other fields of the variant
                        let payload: Result<Response, HassError> = serde_json::from_str(&data)
                            .map_err(|_| HassError::UnknownPayloadReceived);

                        // do I need to check anything here before sending to client?
                        //TODO match on payload Some(x) if x.type == alert
                        //if it is alert verify if we are subscribed to this
                        //if not subscribe, send unsubscribe for that event
                        // if subscribed than execute the callback
                        //else send the response default to user, as below.

                        match payload {
                            Ok(value) =>  match value {
                                Response::Event(event) => {
                                    todo!()
                                },
                                _ => to_client.send(Ok(value)).await.unwrap(),

                            },
                            Err(error) => to_client.send(Err(error)).await.unwrap()
                        };
                        
                    }
                    _ => {}
                },

                Some(Err(error)) => match to_client.send(Err(HassError::from(&error))).await {
                    //send the error to client ("unexpected message format, like a new error")
                    Ok(_r) => {}
                    Err(_e) => {}
                },
                None => {}
            }
        }
    });
    Ok(())
}
