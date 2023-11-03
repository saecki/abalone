use std::sync::Arc;

use abalone_core::dto::{self, ClientMsg, ServerMsg};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::SinkExt;
use futures_util::StreamExt;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::error::Error as TungsteniteError;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

pub enum ConnectionState {
    Disconnected,
    Connected(dto::User),
    InRoom(dto::User, dto::Room),
}

pub fn open_connection(address: &str, username: &str) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();

    runtime.block_on(async {
        let address = address
            .strip_prefix("http://")
            .or_else(|| address.strip_prefix("https://"))
            .or_else(|| address.strip_prefix("ws://"))
            .or_else(|| address.strip_prefix("wss://"))
            .unwrap_or(address);
        let username = urlencoding::Encoded(username);
        let url = format!("ws://{address}/join/{username}");
        let connection = tokio_tungstenite::connect_async(url).await;
        let socket = match connection {
            Ok((socket, resp)) => {
                dbg!(resp);
                socket
            }
            Err(TungsteniteError::ConnectionClosed) => todo!(),
            Err(e) => todo!(),
        };

        let (sender, mut receiver) = socket.split();

        let user = loop {
            let Some(msg) = receiver.next().await else {
                todo!("error");
            };
            let msg = match msg {
                Ok(m) => m,
                Err(_) => todo!(),
            };
            let bytes = match &msg {
                Message::Text(s) => s.as_bytes(),
                Message::Binary(v) => v.as_slice(),
                // ignore
                Message::Ping(_) | Message::Pong(_) => continue,
                Message::Close(_) => todo!(),
                Message::Frame(_) => todo!(),
            };
            match serde_json::from_slice(bytes) {
                Ok(ServerMsg::Welcome(u)) => break u,
                Ok(m) => todo!(),
                Err(_) => todo!(),
            }
        };

        let state = Arc::new(Mutex::new(ConnectionState::Connected(user)));
        tokio::spawn(receiver_task(Arc::clone(&state), receiver));
        tokio::spawn(sender_task(state, sender));
    });
}

async fn receiver_task(
    state: Arc<Mutex<ConnectionState>>,
    receiver: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
) {
    loop {}
}

async fn sender_task(
    state: Arc<Mutex<ConnectionState>>,
    sender: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
) {
    dbg!();
}
