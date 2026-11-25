mod collision;
mod map;
mod protocol;
mod sim;

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use futures_util::{SinkExt, StreamExt};
use sim::{InputState, World, DT};
use tokio::sync::{broadcast, mpsc, Mutex};
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

use crate::protocol::{ClientMessage, ServerMessage};

struct AppState {
    world: Mutex<World>,
    snapshots: broadcast::Sender<String>,
    tick: AtomicU64,
}

#[tokio::main]
async fn main() {
    let map = map::load_map(map::default_map_path());
    println!("loaded map `{}`", map.name);

    let (tx, _) = broadcast::channel::<String>(64);
    let state = Arc::new(AppState {
        world: Mutex::new(World::new(map)),
        snapshots: tx.clone(),
        tick: AtomicU64::new(0),
    });

    let tick_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs_f32(DT));
        loop {
            interval.tick().await;
            let json = {
                let mut world = tick_state.world.lock().await;
                world.tick();
                tick_state.tick.store(world.current_tick(), Ordering::Relaxed);
                serde_json::to_string(&world.snapshot()).expect("snapshot json")
            };
            let _ = tick_state.snapshots.send(json);
        }
    });

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/health", get(|| async { "ok" }))
        .with_state(state)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    let addr: SocketAddr = "0.0.0.0:3030".parse().unwrap();
    println!("Scarlet City server listening on ws://{addr}/ws");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    axum::serve(listener, app).await.expect("serve");
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| client_session(socket, state))
}

async fn client_session(socket: WebSocket, state: Arc<AppState>) {
    let (mut sink, mut stream) = socket.split();
    let mut player_id: Option<Uuid> = None;
    let mut rx = state.snapshots.subscribe();
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<String>();

    let writer = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            if sink.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    let out_tx_snap = out_tx.clone();
    let pump = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(json) => {
                    if out_tx_snap.send(json).is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    while let Some(Ok(msg)) = stream.next().await {
        let Message::Text(text) = msg else {
            continue;
        };
        let parsed: ClientMessage = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(e) => {
                let ev = ServerMessage::Event {
                    kind: "error".into(),
                    message: format!("bad client message: {e}"),
                };
                let _ = out_tx.send(serde_json::to_string(&ev).unwrap());
                continue;
            }
        };

        match parsed {
            ClientMessage::Join { name } => {
                if player_id.is_some() {
                    continue;
                }
                let mut world = state.world.lock().await;
                let id = world.spawn_player(sanitize_name(name));
                player_id = Some(id);
                let welcome = ServerMessage::Welcome {
                    player_id: id,
                    map: world.map.clone(),
                };
                let _ = out_tx.send(serde_json::to_string(&welcome).unwrap());
                let joined = ServerMessage::Event {
                    kind: "join".into(),
                    message: format!("{id} entered the city"),
                };
                let _ = out_tx.send(serde_json::to_string(&joined).unwrap());
            }
            ClientMessage::Input {
                seq,
                move_x,
                move_y,
                ascend_descend,
                aim_angle,
                shoot,
                interact,
                brake,
            } => {
                let Some(id) = player_id else { continue };
                let mut world = state.world.lock().await;
                world.set_input(
                    id,
                    InputState {
                        seq,
                        move_x: move_x.clamp(-1.0, 1.0),
                        move_y: move_y.clamp(-1.0, 1.0),
                        ascend_descend: ascend_descend.clamp(-1.0, 1.0),
                        aim_angle,
                        shoot,
                        interact,
                        brake,
                    },
                );
            }
            ClientMessage::Ping { client_ms } => {
                let pong = ServerMessage::Pong {
                    client_ms,
                    server_tick: state.tick.load(Ordering::Relaxed),
                };
                let _ = out_tx.send(serde_json::to_string(&pong).unwrap());
            }
        }
    }

    if let Some(id) = player_id {
        state.world.lock().await.remove_player(id);
    }
    writer.abort();
    pump.abort();
}

fn sanitize_name(name: String) -> String {
    let trimmed: String = name.chars().take(24).collect();
    let trimmed = trimmed.trim();
    if trimmed.is_empty() {
        "Runner".into()
    } else {
        trimmed.into()
    }
}
