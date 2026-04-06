mod session;

use axum::{
    extract::{
        State,
        ws::{Message,WebSocket,WebSocketUpgrade},
        Json,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{get,post},
    Router,
};
use session::{
    build_command_frame, run_live_session, run_simulation_session, ActiveSession,
    CommandRequest, LiveConfig, SessionKind,
};
use std::{
    net::SocketAddr,
    sync::{atomic::{AtomicBool, Ordering}, mpsc, Arc},
    time::Instant,
};
use tokio::sync::{broadcast, Mutex};
use tower_http::cors::{Any, CorsLayer};

#[derive(Clone)]
struct AppState {
    tx: broadcast::Sender<String>,
    session: Arc<Mutex<Option<ActiveSession>>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, _) = broadcast::channel::<String>(256);
    let state = AppState {
        tx,
        session: Arc::new(Mutex::new(None)),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/start/sim", post(start_sim_handler))
        .route("/start/live", post(start_live_handler))
        .route("/command", post(command_handler))
        .with_state(state)
        .layer(cors);

    let addr: SocketAddr = std::env::var("ATLAS_BIND_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:3000".to_string())
        .parse()?;
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("ATLAS backend listening on {addr}");
    axum::serve(listener, app).await?;

    Ok(())
}

async fn start_sim_handler(State(state): State<AppState>,) -> impl IntoResponse {
    stop_active_session(&state).await;

    let tx = state.tx.clone();
    let stop_flag = Arc::new(AtomicBool::new(false));

    {
        let mut session = state.session.lock().await;
        *session = Some(ActiveSession {
            kind: SessionKind::Simulation,
            started_at: Instant::now(),
            stop_flag: stop_flag.clone(),
            worker_alive: Arc::new(AtomicBool::new(true)),
            link_connected: Arc::new(AtomicBool::new(true)),
            command_tx: None,
            next_command_sequence: 0,
            label: "simulator".to_string(),
        });
    }

    let worker_alive = {
        let session = state.session.lock().await;
        session
            .as_ref()
            .map(|active_session| active_session.worker_alive.clone())
            .expect("simulation session should exist before spawn")
    };

    tokio::task::spawn_blocking(move || {
        let _ = run_simulation_session(tx, stop_flag);
        worker_alive.store(false, Ordering::Relaxed);
    });

    "simulation started"
}

async fn start_live_handler(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let config = LiveConfig {
        port_name: std::env::var("ATLAS_SERIAL_PORT").unwrap_or_else(|_| "COM7".to_string()),
        baud_rate: std::env::var("ATLAS_SERIAL_BAUD")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(115200),
    };

    {
        let session = state.session.lock().await;
        if let Some(active_session) = session.as_ref() {
            if active_session.kind == SessionKind::Live
                && active_session.label == config.port_name
                && active_session.worker_alive.load(Ordering::Relaxed)
            {
                let status = if active_session.link_connected.load(Ordering::Relaxed) {
                    "live session already running on"
                } else {
                    "live session reconnecting on"
                };
                return Ok(format!("{status} {}", config.port_name));
            }
        }
    }

    stop_active_session(&state).await;

    let tx = state.tx.clone();
    let stop_flag = Arc::new(AtomicBool::new(false));
    let link_connected = Arc::new(AtomicBool::new(false));
    let (command_tx, command_rx) = mpsc::channel::<Vec<u8>>();

    {
        let mut session = state.session.lock().await;
        *session = Some(ActiveSession {
            kind: SessionKind::Live,
            started_at: Instant::now(),
            stop_flag: stop_flag.clone(),
            worker_alive: Arc::new(AtomicBool::new(true)),
            link_connected: link_connected.clone(),
            command_tx: Some(command_tx),
            next_command_sequence: 0,
            label: config.port_name.clone(),
        });
    }

    let worker_alive = {
        let session = state.session.lock().await;
        session
            .as_ref()
            .map(|active_session| active_session.worker_alive.clone())
            .expect("live session should exist before spawn")
    };
    let worker_config = config.clone();

    tokio::task::spawn_blocking(move || {
        let _ = run_live_session(tx, stop_flag, command_rx, worker_config, link_connected.clone());
        link_connected.store(false, Ordering::Relaxed);
        worker_alive.store(false, Ordering::Relaxed);
    });

    Ok(format!("live session started on {}", config.port_name))
}

async fn command_handler(
    State(state): State<AppState>,
    Json(request): Json<CommandRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut session = state.session.lock().await;
    let active_session = session
        .as_mut()
        .ok_or_else(|| (StatusCode::CONFLICT, "no active session".to_string()))?;

    if active_session.kind != SessionKind::Live {
        return Err((StatusCode::CONFLICT, "commands require an active live session".to_string()));
    }

    if !active_session.worker_alive.load(Ordering::Relaxed) {
        *session = None;
        return Err((StatusCode::CONFLICT, "live session is not running; restart the live link".to_string()));
    }

    let link_connected = active_session.link_connected.load(Ordering::Relaxed);
    let command_tx = active_session
        .command_tx
        .clone()
        .ok_or_else(|| (StatusCode::CONFLICT, "live session command channel unavailable".to_string()))?;

    let frame = build_command_frame(
        &request,
        active_session.next_command_sequence,
        active_session.started_at.elapsed().as_millis() as u32,
    )
    .map_err(|error| (StatusCode::BAD_REQUEST, error))?;

    active_session.next_command_sequence = active_session.next_command_sequence.wrapping_add(1);

    command_tx
        .send(frame)
        .map_err(|_| {
            *session = None;
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to queue command; restart the live link".to_string())
        })?;

    let command_name = request.cmd.trim().to_ascii_uppercase();
    if link_connected {
        Ok(format!("queued {command_name}"))
    } else {
        Ok(format!("queued {command_name} while reconnecting"))
    }
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

async fn stop_active_session(state: &AppState) {
    let mut session = state.session.lock().await;
    if let Some(active_session) = session.take() {
        active_session.stop_flag.store(true, Ordering::Relaxed);
        active_session.link_connected.store(false, Ordering::Relaxed);
    }
}
