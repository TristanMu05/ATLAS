use axum::{
    extract::{
        State,
        ws::{Message,WebSocket,WebSocketUpgrade},
    },
    response::IntoResponse,
    routing::{get,post},
    Router,
};
use std::net::SocketAddr;
use tokio::sync::broadcast;
use tower_http::cors::{Any, CorsLayer};

#[derive(Clone)]
struct AppState {
    tx: broadcast::Sender<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, _) = broadcast::channel::<String>(256);
    let state = AppState { tx };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/start/sim", post(start_sim_handler))
        .with_state(state)
        .layer(cors);

    let addr: SocketAddr = "127.0.0.1:3000".parse()?;
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn start_sim_handler(State(state): State<AppState>,) -> impl IntoResponse {
    let tx = state.tx.clone();

    tokio::task::spawn_blocking(move || {
        let _ = start_simulation_logging(tx);
    });
    "simulation started"
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| websocket_client(socket, state.tx.subscribe()))
}

async fn websocket_client(
    mut socket: WebSocket,
    mut rx: broadcast::Receiver<String>,
) {
    while let Ok(msg) = rx.recv().await {
        if socket.send(Message::Text(msg.into())).await.is_err() {
            break;
        }
    }
}

fn start_live_logging() -> Result<(), Box<dyn std::error::Error>> {
    atlas_logger::live()
}

fn start_simulation_logging(
    tx: broadcast::Sender<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    atlas_logger::simulation(|update| {
        // Send to UI
        let msg = serde_json::to_string(&update)
            .map_err(std::io::Error::other)?;
        let _ = tx.send(msg);
        Ok(())
    })
}

fn start_live_replay() -> Result<(), Box<dyn std::error::Error>> {
    atlas_replay::replay()
}

fn start_sim_replay() -> Result<(), Box<dyn std::error::Error>> {
    atlas_replay::replay()
}

