use std::collections::HashMap;
use std::sync::Arc;

use abalone_core::{dto, Abalone};
use async_channel::{Receiver, Sender};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use axum_typed_websockets::{Message, WebSocket, WebSocketUpgrade};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::RwLock;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use abalone_core::dto::{ClientMsg, RoomId, ServerMsg, TransactionId};

#[derive(Clone, Debug)]
struct AppState {
    next_id: RoomId,
    rooms: HashMap<RoomId, Arc<RwLock<Room>>>,
}

impl AppState {
    fn new() -> Self {
        Self {
            next_id: RoomId(1),
            rooms: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug)]
struct Room {
    id: RoomId,
    name: String,
    game: Abalone,
    players: [Option<PlayerSession>; 2],
    transactions: HashMap<TransactionId, JoinRoomTransaction>,
}

#[derive(Clone, Debug)]
struct PlayerSession {
    sender: Sender<ServerMsg>,
    receiver: Receiver<ServerMsg>,
}

#[derive(Clone, Debug)]
struct JoinRoomTransaction {
    transaction_id: TransactionId,
    /// The player that requested to join the room.
    player: PlayerSession,
}

fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from(
            "abalone_server=debug,tower_http=debug",
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();

    runtime.block_on(async {
        let state = Arc::new(RwLock::new(AppState::new()));
        let app = Router::new()
            .route("/join", get(ws_handler))
            .with_state(state);

        let listener = axum::Server::bind(&"0.0.0.0:8910".parse().unwrap());
        listener.serve(app.into_make_service()).await.unwrap();
    });
}

// TODO: user accounts and login
async fn ws_handler(
    ws: WebSocketUpgrade<ServerMsg, ClientMsg>,
    State(state): State<Arc<RwLock<AppState>>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(state, socket))
}

async fn handle_socket(state: Arc<RwLock<AppState>>, socket: WebSocket<ServerMsg, ClientMsg>) {
    let (socket_sender, socket_receiver) = socket.split();
    let (session_sender, session_receiver) = async_channel::unbounded();
    tokio::spawn(send_messages(socket_sender, session_receiver.clone()));
    let session = PlayerSession {
        sender: session_sender,
        receiver: session_receiver,
    };
    tokio::spawn(receive_messages(state, socket_receiver, session));
}

struct RoomState {
    /// The index of the player inside the room.
    player_idx: usize,
    /// The room the player is inside.
    room: Arc<RwLock<Room>>,
}

async fn receive_messages(
    state: Arc<RwLock<AppState>>,
    mut socket: SplitStream<WebSocket<ServerMsg, ClientMsg>>,
    session: PlayerSession,
) {
    let mut room = None;

    'session: loop {
        let Some(msg) = socket.next().await else {
            return;
        };

        let msg = match msg {
            Ok(m) => m,
            Err(axum_typed_websockets::Error::Ws(_)) => return,
            // TODO: maybe try to recover from codec, either by requesting the client to resend the
            // message, or resynchronizing
            Err(axum_typed_websockets::Error::Codec(_)) => return,
        };

        let msg = match msg {
            Message::Item(m) => m,
            // ignore
            Message::Ping(_) | Message::Pong(_) => continue 'session,
            // TODO: if the client was in a room, notify the opponent
            Message::Close(_) => return,
        };

        match msg {
            ClientMsg::CreateRoom(name) => {
                let mut state_lock = state.write().await;
                let id = state_lock.next_id;
                state_lock.next_id.0 += 1;

                let player_idx = 0;
                let new_room = Room {
                    id,
                    name,
                    game: Abalone::new(),
                    players: [Some(session.clone()), None],
                    transactions: HashMap::new(),
                };
                let dto = dto::Room::from(&new_room);
                let new_room = Arc::new(RwLock::new(new_room));

                state_lock.rooms.insert(id, Arc::clone(&new_room));
                room = Some(RoomState {
                    player_idx,
                    room: new_room,
                });

                let res = session.sender.send(ServerMsg::Sync(dto)).await;
                if let Err(e) = res {
                    tracing::error!("Error sending message {}", e);
                }
            }
            ClientMsg::ListRooms => {
                let state_lock = state.read().await;
                let mut open_rooms = Vec::new();
                for (_, r) in state_lock.rooms.iter() {
                    let room_lock = r.read().await;
                    let num_players = room_lock.players.iter().filter_map(|p| p.as_ref()).count();
                    if num_players < 2 {
                        open_rooms.push(dto::OpenRoom::from(&*room_lock));
                    }
                }

                let res = session.sender.send(ServerMsg::OpenRooms(open_rooms)).await;
                if let Err(e) = res {
                    tracing::error!("Error sending message {}", e);
                }
            }
            ClientMsg::Sync => todo!("send a sync message back"),
            ClientMsg::RequestJoinRoom(room_id) => {
                if let Some(r) = &room {
                    let id = r.room.read().await.id;
                    let error = format!("Already inside room with id {id:?}");
                    let res = session.sender.send(ServerMsg::Error(error)).await;
                    if let Err(e) = res {
                        tracing::error!("Error sending message {}", e);
                    }
                    continue 'session;
                }
            }
            ClientMsg::AllowJoinRoom(transaction_id) => {
                let Some(r) = &mut room else {
                    let error = format!("Not inside a room");
                    let res = session.sender.send(ServerMsg::Error(error)).await;
                    if let Err(e) = res {
                        tracing::error!("Error sending message {}", e);
                    }
                    continue 'session;
                };
                let mut room_lock = r.room.write().await;

                // TODO: check if other player is still connected
                let transaction = room_lock.transactions.get(&transaction_id) else {
                    let error = format!("Transaction with id {transaction_id} not found");
                    let res = session.sender.send(ServerMsg::Error(error)).await;
                    if let Err(e) = res {
                        tracing::error!("Error sending message {}", e);
                    }
                    continue 'session;
                };

                let player_idx = if room_lock.players[0].is_none() {
                    0
                } else if room_lock.players[1].is_none() {
                    1
                } else {
                    let error = format!("Room with id {:?} is full", room_lock.id);
                    let res = session.sender.send(ServerMsg::Error(error)).await;
                    if let Err(e) = res {
                        tracing::error!("Error sending message {}", e);
                    }
                    continue 'session;
                };
                room_lock.players[player_idx] = Some(session.clone());
                room = Some(RoomState {
                    player_idx,
                    room: Arc::clone(r),
                });

                for p in room_lock.players.iter().filter_map(|p| p.as_ref()) {
                    let dto = dto::Room::from(&*room_lock);
                    let res = p.sender.send(ServerMsg::Sync(dto)).await;
                    if let Err(e) = res {
                        tracing::error!("Error sending message {}", e);
                    }
                }
            }
            ClientMsg::LeaveRoom => {
                let Some(r) = &room else {
                    let error = format!("Not inside a room");
                    let res = session.sender.send(ServerMsg::Error(error)).await;
                    if let Err(e) = res {
                        tracing::error!("Error sending message {}", e);
                    }
                    continue 'session;
                };

                {
                    let mut room_lock = r.room.write().await;
                    room_lock.players[r.player_idx] = None;

                    // notify other players
                    let mut num_others = 0;
                    for (i, p) in room_lock.players.iter().enumerate() {
                        if i == r.player_idx {
                            continue;
                        }
                        let Some(p) = p else { continue };

                        let dto = dto::Room::from(&*room_lock);
                        let res = p.sender.send(ServerMsg::Sync(dto)).await;
                        if let Err(e) = res {
                            tracing::error!("Error sending message {}", e);
                        }

                        num_others += 1;
                    }

                    // delete room if it's empty
                    if num_others == 0 {
                        let mut state_lock = state.write().await;
                        state_lock.rooms.remove(&room_lock.id);
                    }
                }

                room = None;
                let res = session.sender.send(ServerMsg::SyncEmpty).await;
                if let Err(e) = res {
                    tracing::error!("Error sending message {}", e);
                }
            }
            ClientMsg::MakeMove { first, last, dir } => {
                let Some(r) = &room else {
                    let error = format!("Not inside a room");
                    let res = session.sender.send(ServerMsg::Error(error)).await;
                    if let Err(e) = res {
                        tracing::error!("Error sending message {}", e);
                    }
                    continue 'session;
                };

                // TODO: check whose turn it is, and if two players are present
                let mut room_lock = r.room.write().await;
                match room_lock.game.check_move([first, last], dir) {
                    Ok(m) => {
                        room_lock.game.submit_move(m);

                        for p in room_lock.players.iter() {
                            let Some(p) = p else { continue };

                            let res = p.sender.send(ServerMsg::AppliedMove(m)).await;
                            if let Err(e) = res {
                                tracing::error!("Error sending message {}", e);
                            }
                        }
                    }
                    Err(_) => todo!(),
                }
            }
            ClientMsg::RequestUndo => todo!(),
            ClientMsg::AllowUndo => todo!(),
        }
    }
}

async fn send_messages(
    mut socket: SplitSink<WebSocket<ServerMsg, ClientMsg>, Message<ServerMsg>>,
    session: Receiver<ServerMsg>,
) {
    loop {
        let Ok(msg) = session.recv().await else {
            return;
        };

        let res = socket.send(Message::Item(msg)).await;
        if let Err(e) = res {
            todo!();
        }
    }
}

impl From<&Room> for dto::Room {
    fn from(room: &Room) -> Self {
        dto::Room {
            id: room.id,
            name: room.name.clone(),
            game: room.game.clone(),
        }
    }
}

impl From<&Room> for dto::OpenRoom {
    fn from(room: &Room) -> Self {
        dto::OpenRoom {
            id: room.id,
            name: room.name.clone(),
        }
    }
}
