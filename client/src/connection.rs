use std::sync::Arc;

use abalone_core::dto::{self, ClientMsg, ServerMsg};
use async_channel::Receiver;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::SinkExt;
use futures_util::StreamExt;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::error::Error as TungsteniteError;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::Userdata;

#[derive(Default)]
pub enum ConnectionState {
    #[default]
    Disconnected,
    Connecting,
    Connected(dto::User),
    JoiningRoom(dto::User),
    LeavingRoom(dto::User, dto::Room),
    InRoom(dto::User, dto::Room),
}

pub(crate) fn open_connection(
    state: Arc<Mutex<ConnectionState>>,
    userdata: Userdata,
    receiver: Receiver<ClientMsg>,
) {
    std::thread::spawn(move || {
        start_websocket(state, userdata, receiver);
    });
}

fn start_websocket(
    state: Arc<Mutex<ConnectionState>>,
    userdata: Userdata,
    session_receiver: Receiver<ClientMsg>,
) {
    *state.blocking_lock() = ConnectionState::Connecting;

    let address = userdata.address.as_str();
    let address = address
        .strip_prefix("http://")
        .or_else(|| address.strip_prefix("https://"))
        .or_else(|| address.strip_prefix("ws://"))
        .or_else(|| address.strip_prefix("wss://"))
        .unwrap_or(address);
    let username = urlencoding::Encoded(userdata.username);
    let url = format!("ws://{address}/join/{username}");

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();

    runtime.block_on(async {
        let connection = tokio_tungstenite::connect_async(url).await;
        let socket = match connection {
            Ok((socket, resp)) => {
                dbg!(resp);
                socket
            }
            Err(TungsteniteError::ConnectionClosed) => todo!(),
            Err(e) => todo!(),
        };

        let (socket_sender, mut socket_receiver) = socket.split();

        let user = loop {
            let Some(msg) = socket_receiver.next().await else {
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

        *state.lock().await = ConnectionState::Connected(user);

        tokio::spawn(receiver_task(Arc::clone(&state), socket_receiver));
        tokio::spawn(sender_task(socket_sender, session_receiver));
    });
}

async fn receiver_task(
    state: Arc<Mutex<ConnectionState>>,
    mut socket: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
) {
    'session: loop {
        let Some(msg) = socket.next().await else {
            break 'session;
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
        let msg: ServerMsg = match serde_json::from_slice(bytes) {
            Ok(m) => m,
            Err(_) => todo!(),
        };

        match msg {
            ServerMsg::Welcome(_) => todo!("error"),
            ServerMsg::OpenRooms(_) => todo!(),
            ServerMsg::JoinRoomRequested(_) => todo!(),
            ServerMsg::JoinRoomAllowed(_, _) => todo!(),
            ServerMsg::Sync(_) => todo!(),
            ServerMsg::SyncEmpty => todo!(),
            ServerMsg::AppliedMove(_) => todo!(),
            ServerMsg::UndoRequested => todo!(),
            ServerMsg::Error(_) => todo!(),
        }
    }

    *state.lock().await = ConnectionState::Disconnected;
}

async fn sender_task(
    mut socket: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    session: Receiver<ClientMsg>,
) {
    loop {
        let Ok(msg) = session.recv().await else {
            return;
        };

        let string = serde_json::to_string(&msg).expect("message should be valid");
        let res = socket.send(Message::Text(string)).await;
        if let Err(_e) = res {
            todo!();
        }
    }
}
