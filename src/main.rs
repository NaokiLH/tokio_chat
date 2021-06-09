use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::accept_async;
use tungstenite::{Message, Result};
#[tokio::main]
async fn main() {
    let tcp_listenner = TcpListener::bind("127.0.0.1:12356").await.unwrap();
    let (sender, receiver) = broadcast::channel::<ChatMessage>(5000);
    while let Ok((stream, _addr)) = tcp_listenner.accept().await {
        tokio::spawn(accept_connection(stream, sender.clone()));
    }
}
extern "C" {
    fn time(output: *mut i64) -> i64;
}

async fn accept_connection(
    stream: TcpStream,
    sender: broadcast::Sender<ChatMessage>,
) -> Result<()> {
    let ws_stream = accept_async(stream).await.expect("Failed to accept");
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let mut receiver = sender.subscribe();
    let mut interval = tokio::time::interval(Duration::from_millis(1000));
    const TIMEOUT: i64 = 15;
    let mut last_pong_recv_timestamp: i64 = unsafe { time(std::ptr::null_mut()) };

    loop {
        tokio::select! {
            msg = ws_receiver.next() => {
                match msg {
                    Some(msg) => {
                        let msg = msg?;
                        if msg.is_text()  {
                            if let Message::Text(msg)=msg{
                                let (cmd,arg) = msg.split_once(' ').unwrap();
                                match cmd {
                                    "send"=>{
                                        let chat_message = ChatMessage {
                                            message: arg.to_string(),
                                        };
                                        sender.send(chat_message).unwrap();
                                    }
                                    _=>todo!()
                                }
                            }
                        }
                        else if msg.is_ping(){
                            ws_sender.send(Message::Pong(Vec::new())).await?;
                        }else if msg.is_pong(){
                            //TODO
                            last_pong_recv_timestamp = unsafe { time(std::ptr::null_mut()) };
                        }
                        else if msg.is_close() {
                            break;
                        }
                    }
                    None => break,
                }
            }
            Ok(msg) = receiver.recv()=>{
                ws_sender.send(Message::Text(msg.message)).await?;
            }
            _ = interval.tick() => {
                if unsafe{time(std::ptr::null_mut())} - last_pong_recv_timestamp > TIMEOUT {
                    //资源回收,在各个聊天室内踢掉这个用户
                    break;
                }
                ws_sender.send(Message::Ping(Vec::new())).await?;
            }
        }
    }

    Ok(())
}
#[derive(Clone, Debug)]
struct ChatMessage {
    message: String,
}
