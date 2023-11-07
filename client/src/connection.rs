use std::sync::Arc;

use abalone_core::dto::{self, ClientMsg, ServerMsg};
use async_channel::{Receiver, Sender};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::StreamExt;
use futures_util::{join, SinkExt};
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
    Connected(Connection),
}

pub struct Connection {
    pub user: dto::User,
    pub open_rooms: Vec<dto::OpenRoom>,
    pub join_allowed: Vec<(dto::OpenRoom, dto::TransactionId)>,
    pub state: RoomState,
}

pub enum RoomState {
    Connected {
        joining: bool,
    },
    InRoom {
        room: dto::Room,
        join_requests: Vec<dto::TransactionId>,
        undo_requested: bool,
        leaving: bool,
    },
}

pub(crate) fn open_connection(
    state: Arc<Mutex<ConnectionState>>,
    userdata: Userdata,
    sender: Sender<ClientMsg>,
    receiver: Receiver<ClientMsg>,
) {
    std::thread::spawn(move || {
        start_websocket(state, userdata, sender, receiver);
    });
}

fn start_websocket(
    state: Arc<Mutex<ConnectionState>>,
    userdata: Userdata,
    session_sender: Sender<ClientMsg>,
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
            Ok((socket, _resp)) => socket,
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

        *state.lock().await = ConnectionState::Connected(Connection {
            user,
            open_rooms: Vec::new(),
            join_allowed: Vec::new(),
            state: RoomState::Connected { joining: false },
        });

        let receiver_task = tokio::spawn(receiver_task(
            Arc::clone(&state),
            socket_receiver,
            session_sender,
        ));
        let sender_task = tokio::spawn(sender_task(socket_sender, session_receiver));

        let (a, b) = join!(receiver_task, sender_task);
        a.unwrap();
        b.unwrap();
    });
}

async fn receiver_task(
    state: Arc<Mutex<ConnectionState>>,
    mut socket: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    session: Sender<ClientMsg>,
) {
    session.send(ClientMsg::ListRooms).await.unwrap();

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

        let mut state_lock = state.lock().await;
        let connection = match &mut *state_lock {
            ConnectionState::Disconnected | ConnectionState::Connecting => break 'session,
            ConnectionState::Connected(c) => c,
        };

        match msg {
            ServerMsg::Welcome(_) => todo!(),
            ServerMsg::OpenRooms(rooms) => {
                connection.open_rooms = rooms;
            }
            ServerMsg::JoinRoomRequested(transaction) => match &mut connection.state {
                RoomState::Connected { .. } => todo!(),
                RoomState::InRoom { join_requests, .. } => {
                    join_requests.push(transaction);
                }
            },
            ServerMsg::JoinRoomAllowed(room, transaction) => {
                connection.join_allowed.push((room, transaction));
            }
            ServerMsg::JoinRoomNoLongerAllowed(transaction) => {
                connection.join_allowed.retain(|(_, t)| *t != transaction);
            }
            ServerMsg::Sync(room) => match &mut connection.state {
                RoomState::Connected { .. } => {
                    connection.state = RoomState::InRoom {
                        room,
                        join_requests: Vec::new(),
                        undo_requested: false,
                        leaving: false,
                    };
                }
                RoomState::InRoom {
                    room: r,
                    undo_requested,
                    ..
                } => {
                    *r = room;
                    *undo_requested = false;
                }
            },
            ServerMsg::SyncEmpty => {
                connection.state = RoomState::Connected { joining: false };
                session.send(ClientMsg::ListRooms).await.unwrap();
            }
            ServerMsg::AppliedMove(m) => match &mut connection.state {
                RoomState::Connected { .. } => todo!(),
                RoomState::InRoom {
                    room,
                    undo_requested,
                    ..
                } => {
                    room.game.submit_move(m);
                    *undo_requested = false;
                }
            },
            ServerMsg::UndoRequested => match &mut connection.state {
                RoomState::Connected { .. } => todo!(),
                RoomState::InRoom { undo_requested, .. } => {
                    *undo_requested = true;
                }
            },
            ServerMsg::Error(e) => println!("Error: {e}"),
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
        if let Err(e) = res {
            todo!("{e}");
        }
    }
}
