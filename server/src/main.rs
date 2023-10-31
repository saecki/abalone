use std::sync::Arc;

use abalone_core::{dto, Move};
use async_channel::{Receiver, Sender};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use axum_typed_websockets::{Message, WebSocket, WebSocketUpgrade};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::Mutex;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use abalone_core::dto::{ClientMsg, ServerMsg};

#[derive(Clone, Debug)]
struct AppState {
    next_id: u64,
    rooms: Vec<Room>,
}

impl AppState {
    fn new() -> Self {
        Self {
            next_id: 1,
            rooms: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
struct Room {
    id: u64,
    name: String,
    player_a: (Sender<RoomMsg>, Receiver<RoomMsg>),
    player_b: (Sender<RoomMsg>, Receiver<RoomMsg>),
}

enum RoomMsg {
    OpenRooms(Vec<dto::Room>),
    Sync(dto::Sync),
    SyncEmpty,
    UpdateMove(Move),
    UndoRequested,
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
        let state = Arc::new(Mutex::new(AppState::new()));
        let app = Router::new()
            .route("/join", get(ws_handler))
            .with_state(state);

        let listener = axum::Server::bind(&"0.0.0.0:8910".parse().unwrap());
        listener.serve(app.into_make_service()).await.unwrap();
    });
}

async fn ws_handler(
    ws: WebSocketUpgrade<ServerMsg, ClientMsg>,
    State(state): State<Arc<Mutex<AppState>>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(state, socket))
}

async fn handle_socket(state: Arc<Mutex<AppState>>, socket: WebSocket<ServerMsg, ClientMsg>) {
    let (socket_sender, socket_receiver) = socket.split();
    let (session_sender, session_receiver) = async_channel::unbounded();
    tokio::spawn(receive_messages(state, socket_receiver, session_sender));
    tokio::spawn(send_messages(socket_sender, session_receiver));
}

async fn receive_messages(
    state: Arc<Mutex<AppState>>,
    mut socket: SplitStream<WebSocket<ServerMsg, ClientMsg>>,
    session: Sender<RoomMsg>,
) {
    let mut room = None;

    loop {
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
            Message::Ping(_) | Message::Pong(_) => continue,
            // TODO: if the client was in a room, notify the opponent
            Message::Close(_) => return,
        };

        match msg {
            ClientMsg::CreateRoom(name) => {}
            ClientMsg::ListRooms => todo!(),
            ClientMsg::Sync => todo!("send a sync message back"),
            ClientMsg::Join(room_id) => {
                let lock = state.lock().await;
            }
            ClientMsg::Leave => todo!(),
            ClientMsg::MakeMove { first, last, dir } => todo!(),
            ClientMsg::RequestUndo => todo!(),
            ClientMsg::AllowUndo => todo!(),
        }
    }
}

async fn send_messages(
    mut socket: SplitSink<WebSocket<ServerMsg, ClientMsg>, Message<ServerMsg>>,
    session: Receiver<RoomMsg>,
) {
    loop {
        let Ok(msg) = session.recv().await else {
            return;
        };

        let server_msg = match msg {
            RoomMsg::OpenRooms(rooms) => ServerMsg::OpenRooms(rooms),
            RoomMsg::Sync(sync) => ServerMsg::Sync(sync),
            RoomMsg::SyncEmpty => ServerMsg::SyncEmpty,
            RoomMsg::UpdateMove(mov) => ServerMsg::UpdateMove(mov),
            RoomMsg::UndoRequested => ServerMsg::UndoRequested,
        };

        let res = socket.send(Message::Item(server_msg)).await;
        match res {
            Ok(()) => todo!(),
            Err(_) => todo!(),
        }
    }
}
