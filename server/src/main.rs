use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use axum_typed_websockets::{Message, WebSocket, WebSocketUpgrade};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::StreamExt;
use serde_derive::{Deserialize, Serialize};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(Clone, Debug)]
struct AppState {}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum ClientMsg {}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum ServerMsg {}

fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from(
            "egui=debug,curvefever=debug,tower_http=debug",
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();

    runtime.block_on(async {
        let state = AppState {};
        let app = Router::new()
            .route("/join", get(ws_handler))
            .with_state(state);

        let listener = axum::Server::bind(&"0.0.0.0:8910".parse().unwrap());
        listener.serve(app.into_make_service()).await.unwrap();
    });
}

async fn ws_handler(
    ws: WebSocketUpgrade<ServerMsg, ClientMsg>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket))
}

async fn handle_socket(socket: WebSocket<ServerMsg, ClientMsg>) {
    let (sender, receiver) = socket.split();
    tokio::spawn(receive_messages(receiver));
    tokio::spawn(send_messages(sender));
}

async fn receive_messages(socket: SplitStream<WebSocket<ServerMsg, ClientMsg>>) {
    loop {
        todo!()
    }
}

async fn send_messages(socket: SplitSink<WebSocket<ServerMsg, ClientMsg>, Message<ServerMsg>>) {
    loop {
        todo!()
    }
}
