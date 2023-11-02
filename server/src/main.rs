use std::collections::HashMap;
use std::sync::Arc;

use abalone_core::{dto, Abalone, Color};
use async_channel::{Receiver, Sender};
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use axum_typed_websockets::{Message, WebSocket, WebSocketUpgrade};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::RwLock;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use abalone_core::dto::{ClientMsg, RoomId, ServerMsg, TransactionId, UserId};
use uuid::Uuid;

#[cfg(test)]
mod test;

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
    undo_requested: bool,
}

#[derive(Clone, Debug)]
struct PlayerSession {
    user_id: UserId,
    username: String,
    sender: Sender<ServerMsg>,
}

#[derive(Clone, Debug)]
struct JoinRoomTransaction {
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
            .route("/join/:username", get(ws_handler))
            .with_state(state);

        let listener = axum::Server::bind(&"0.0.0.0:8910".parse().unwrap());
        listener.serve(app.into_make_service()).await.unwrap();
    });
}

// TODO: user accounts and login
async fn ws_handler(
    ws: WebSocketUpgrade<ServerMsg, ClientMsg>,
    State(state): State<Arc<RwLock<AppState>>>,
    Path(username): Path<String>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(state, socket, username))
}

async fn handle_socket(
    state: Arc<RwLock<AppState>>,
    socket: WebSocket<ServerMsg, ClientMsg>,
    username: String,
) {
    let (socket_sender, socket_receiver) = socket.split();
    let (session_sender, session_receiver) = async_channel::unbounded();
    tokio::spawn(sender_task(socket_sender, session_receiver));
    let session = PlayerSession {
        user_id: UserId(Uuid::new_v4()),
        username,
        sender: session_sender,
    };
    tokio::spawn(receiver_task(state, socket_receiver, session));
}

struct RoomState {
    /// The index of the player inside the room.
    player_idx: usize,
    /// The room the player is inside.
    room: Arc<RwLock<Room>>,
}

async fn receiver_task(
    state: Arc<RwLock<AppState>>,
    mut socket: SplitStream<WebSocket<ServerMsg, ClientMsg>>,
    session: PlayerSession,
) {
    let mut room: Option<RoomState> = None;

    {
        let dto = dto::User::from(&session);
        send_msg(&session.sender, ServerMsg::Welcome(dto)).await;
    }

    'session: loop {
        let Some(msg) = socket.next().await else {
            if let Some(r) = room {
                tracing::debug!(
                    "Connection closed by user \"{}\" with id {}",
                    session.username,
                    session.user_id
                );
                leave_room(&state, &r).await;
            }
            return;
        };

        let msg = match msg {
            Ok(m) => m,
            Err(axum_typed_websockets::Error::Ws(_)) => {
                if let Some(r) = room {
                    tracing::debug!(
                        "Connection error: user \"{}\" with id {}",
                        session.username,
                        session.user_id
                    );
                    leave_room(&state, &r).await;
                }
                return;
            }
            Err(axum_typed_websockets::Error::Codec(e)) => {
                let error = format!("Invalid message format: {e}");
                send_msg(&session.sender, ServerMsg::Error(error)).await;
                continue 'session;
            }
        };

        let msg = match msg {
            Message::Item(m) => m,
            // ignore
            Message::Ping(_) | Message::Pong(_) => continue 'session,
            Message::Close(_) => {
                if let Some(r) = room {
                    tracing::debug!(
                        "Connection closed by user \"{}\" with id {}",
                        session.username,
                        session.user_id
                    );
                    leave_room(&state, &r).await;
                }
                return;
            }
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
                    undo_requested: false,
                };
                let dto = dto::Room::from(&new_room);
                let new_room = Arc::new(RwLock::new(new_room));

                state_lock.rooms.insert(id, Arc::clone(&new_room));
                room = Some(RoomState {
                    player_idx,
                    room: new_room,
                });

                send_msg(&session.sender, ServerMsg::Sync(dto)).await;
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

                send_msg(&session.sender, ServerMsg::OpenRooms(open_rooms)).await;
            }
            ClientMsg::Sync => {
                let msg = match &room {
                    Some(r) => {
                        let room_lock = r.room.read().await;
                        let dto = dto::Room::from(&*room_lock);
                        ServerMsg::Sync(dto)
                    }
                    None => ServerMsg::SyncEmpty,
                };
                send_msg(&session.sender, msg).await;
            }
            ClientMsg::RequestJoinRoom(room_id) => {
                if let Some(r) = &room {
                    let id = r.room.read().await.id;
                    let error = format!("Already inside room with id {id}");
                    send_msg(&session.sender, ServerMsg::Error(error)).await;
                    continue 'session;
                }

                let state_lock = state.read().await;
                let Some(open_room) = state_lock.rooms.get(&room_id) else {
                    let error = format!("Room with id {room_id} not found");
                    send_msg(&session.sender, ServerMsg::Error(error)).await;
                    continue 'session;
                };

                let mut room_lock = open_room.write().await;
                let mut other_player = None;
                for p in room_lock.players.iter().filter_map(|p| p.as_ref()) {
                    if other_player.is_some() {
                        let error = format!("Room with id {room_id} is full");
                        send_msg(&session.sender, ServerMsg::Error(error)).await;
                        continue 'session;
                    }

                    other_player = Some(p.clone());
                }
                let Some(other_player) = other_player else {
                    let error = format!("Room with id {room_id} is empty");
                    send_msg(&session.sender, ServerMsg::Error(error)).await;
                    continue 'session;
                };

                let transaction_id = TransactionId(Uuid::new_v4());
                let transaction = JoinRoomTransaction {
                    player: session.clone(),
                };
                room_lock.transactions.insert(transaction_id, transaction);

                let msg = ServerMsg::JoinRoomRequested(transaction_id);
                send_msg(&other_player.sender, msg).await;
            }
            ClientMsg::AllowJoinRoom(transaction_id) => {
                let Some(r) = &mut room else {
                    let error = format!("Not inside a room");
                    send_msg(&session.sender, ServerMsg::Error(error)).await;
                    continue 'session;
                };
                let room_lock = r.room.read().await;

                let Some(transaction) = room_lock.transactions.get(&transaction_id) else {
                    let error = format!("Transaction with id {transaction_id} not found");
                    send_msg(&session.sender, ServerMsg::Error(error)).await;
                    continue 'session;
                };

                if transaction.player.sender.is_closed() {
                    let error = format!("Player already disconnected");
                    send_msg(&session.sender, ServerMsg::Error(error)).await;
                    continue 'session;
                }

                let dto = dto::OpenRoom::from(&*room_lock);
                let msg = ServerMsg::JoinRoomAllowed(dto, transaction_id);
                send_msg(&transaction.player.sender, msg).await;
            }
            ClientMsg::JoinRoom(room_id, transaction_id) => {
                if let Some(r) = &room {
                    let id = r.room.read().await.id;
                    let error = format!("Already inside room with id {id}");
                    send_msg(&session.sender, ServerMsg::Error(error)).await;
                    continue 'session;
                }

                let state_lock = state.read().await;
                let Some(open_room) = state_lock.rooms.get(&room_id) else {
                    let error = format!("Room with id {room_id} not found");
                    send_msg(&session.sender, ServerMsg::Error(error)).await;
                    continue 'session;
                };
                let mut room_lock = open_room.write().await;

                if room_lock.transactions.remove(&transaction_id).is_none() {
                    let error = format!("Transaction with id {transaction_id} not found");
                    send_msg(&session.sender, ServerMsg::Error(error)).await;
                    continue 'session;
                };

                let player_idx = if room_lock.players[0].is_none() {
                    0
                } else if room_lock.players[1].is_none() {
                    1
                } else {
                    let error = format!("Room with id {room_id} is full");
                    send_msg(&session.sender, ServerMsg::Error(error)).await;
                    continue 'session;
                };
                room_lock.players[player_idx] = Some(session.clone());
                room = Some(RoomState {
                    player_idx,
                    room: Arc::clone(open_room),
                });

                for p in room_lock.players.iter().filter_map(|p| p.as_ref()) {
                    let dto = dto::Room::from(&*room_lock);
                    send_msg(&p.sender, ServerMsg::Sync(dto)).await;
                }
            }
            ClientMsg::LeaveRoom => {
                let Some(r) = &room else {
                    let error = format!("Not inside a room");
                    send_msg(&session.sender, ServerMsg::Error(error)).await;
                    continue 'session;
                };

                leave_room(&state, &r).await;

                room = None;
                send_msg(&session.sender, ServerMsg::SyncEmpty).await;
            }
            ClientMsg::MakeMove { first, last, dir } => {
                let Some(r) = &room else {
                    let error = format!("Not inside a room");
                    send_msg(&session.sender, ServerMsg::Error(error)).await;
                    continue 'session;
                };

                let mut room_lock = r.room.write().await;
                let player_color = Color::try_from(r.player_idx as u8)
                    .expect("player_idx should always be 0 or 1");
                if room_lock.game.turn != player_color {
                    let error = format!("It's not {player_color}s turn");
                    send_msg(&session.sender, ServerMsg::Error(error)).await;
                    continue 'session;
                }

                match room_lock.game.check_move([first, last], dir) {
                    Ok(m) => {
                        room_lock.game.submit_move(m);
                        room_lock.undo_requested = false;

                        for p in room_lock.players.iter().filter_map(|p| p.as_ref()) {
                            send_msg(&p.sender, ServerMsg::AppliedMove(m)).await;
                        }
                    }
                    Err(e) => {
                        let error = format!("Invalid move: {e}");
                        send_msg(&session.sender, ServerMsg::Error(error)).await;
                    }
                }
            }
            ClientMsg::RequestUndo => {
                let Some(r) = &room else {
                    let error = format!("Not inside a room");
                    send_msg(&session.sender, ServerMsg::Error(error)).await;
                    continue 'session;
                };

                let mut room_lock = r.room.write().await;
                let player_color = Color::try_from(r.player_idx as u8)
                    .expect("player_idx should always be 0 or 1");
                if room_lock.game.turn == player_color {
                    let error = format!("Can't request undo for opponents move");
                    send_msg(&session.sender, ServerMsg::Error(error)).await;
                    continue 'session;
                }
                if !room_lock.game.can_undo() {
                    let error = format!("No move to undo");
                    send_msg(&session.sender, ServerMsg::Error(error)).await;
                    continue 'session;
                }

                room_lock.undo_requested = true;

                let mut sent_request = false;
                for (i, p) in room_lock.players.iter().enumerate() {
                    if i == r.player_idx {
                        continue;
                    }
                    if let Some(p) = p {
                        send_msg(&p.sender, ServerMsg::UndoRequested).await;
                        sent_request = true;
                    }
                }

                if !sent_request {
                    let error = format!("No other player in room");
                    send_msg(&session.sender, ServerMsg::Error(error)).await;
                    continue 'session;
                }
            }
            ClientMsg::AllowUndo => {
                let Some(r) = &room else {
                    let error = format!("Not inside a room");
                    send_msg(&session.sender, ServerMsg::Error(error)).await;
                    continue 'session;
                };

                let mut room_lock = r.room.write().await;
                let player_color = Color::try_from(r.player_idx as u8)
                    .expect("player_idx should always be 0 or 1");
                if room_lock.game.turn != player_color {
                    let error = format!("Can't allow undo for own move");
                    send_msg(&session.sender, ServerMsg::Error(error)).await;
                    continue 'session;
                }
                if !room_lock.game.can_undo() {
                    let error = format!("No move to undo");
                    send_msg(&session.sender, ServerMsg::Error(error)).await;
                    continue 'session;
                }

                room_lock.game.undo_move();
                room_lock.undo_requested = false;

                for p in room_lock.players.iter().filter_map(|p| p.as_ref()) {
                    let dto = dto::Room::from(&*room_lock);
                    send_msg(&p.sender, ServerMsg::Sync(dto)).await;
                }
            }
        }
    }
}

async fn sender_task(
    mut socket: SplitSink<WebSocket<ServerMsg, ClientMsg>, Message<ServerMsg>>,
    session: Receiver<ServerMsg>,
) {
    loop {
        let Ok(msg) = session.recv().await else {
            return;
        };

        let res = socket.send(Message::Item(msg)).await;
        if let Err(_e) = res {
            todo!();
        }
    }
}

async fn leave_room(state: &Arc<RwLock<AppState>>, room: &RoomState) {
    let mut room_lock = room.room.write().await;
    room_lock.players[room.player_idx] = None;

    // notify other players
    let mut num_others = 0;
    for (i, p) in room_lock.players.iter().enumerate() {
        if i == room.player_idx {
            continue;
        }
        let Some(p) = p else { continue };

        let dto = dto::Room::from(&*room_lock);
        send_msg(&p.sender, ServerMsg::Sync(dto)).await;

        num_others += 1;
    }

    // delete room if it's empty
    if num_others == 0 {
        let mut state_lock = state.write().await;
        state_lock.rooms.remove(&room_lock.id);
    }
}

async fn send_msg(sender: &Sender<ServerMsg>, msg: ServerMsg) {
    let res = sender.send(msg).await;
    if let Err(e) = res {
        tracing::error!("Error sending message {}", e);
    }
}

impl From<&Room> for dto::Room {
    fn from(room: &Room) -> Self {
        Self {
            id: room.id,
            name: room.name.clone(),
            game: room.game.clone(),
            players: [
                room.players[0].as_ref().map(|p| dto::User::from(p)),
                room.players[1].as_ref().map(|p| dto::User::from(p)),
            ],
        }
    }
}

impl From<&Room> for dto::OpenRoom {
    fn from(room: &Room) -> Self {
        Self {
            id: room.id,
            name: room.name.clone(),
            players: [
                room.players[0].as_ref().map(|p| dto::User::from(p)),
                room.players[1].as_ref().map(|p| dto::User::from(p)),
            ],
        }
    }
}

impl From<&PlayerSession> for dto::User {
    fn from(value: &PlayerSession) -> Self {
        Self {
            id: value.user_id,
            name: value.username.clone(),
        }
    }
}
