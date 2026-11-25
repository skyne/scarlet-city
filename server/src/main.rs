mod collision;
mod db;
mod map;
mod protocol;
mod sim;

use std::collections::HashMap;
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
use sqlx::SqlitePool;
use tokio::sync::{mpsc, Mutex};
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

use crate::protocol::{ClientMessage, ServerMessage, StashItem};

struct ClientSlot {
    player_id: Uuid,
    tx: mpsc::UnboundedSender<String>,
}

struct AppState {
    world: Mutex<World>,
    clients: Mutex<HashMap<Uuid, mpsc::UnboundedSender<String>>>,
    db: SqlitePool,
    tick: AtomicU64,
}

#[tokio::main]
async fn main() {
    let map = map::load_map(map::default_map_path());
    println!("loaded map `{}`", map.name);

    let db = db::connect("game.db").await.expect("sqlite game.db");
    println!("sqlite ready (game.db)");

    let state = Arc::new(AppState {
        world: Mutex::new(World::new(map)),
        clients: Mutex::new(HashMap::new()),
        db,
        tick: AtomicU64::new(0),
    });

    let tick_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs_f32(DT));
        loop {
            interval.tick().await;
            let events = {
                let mut world = tick_state.world.lock().await;
                world.tick();
                tick_state.tick.store(world.current_tick(), Ordering::Relaxed);
                world.drain_events()
            };
            for ev in events {
                handle_raid_event(&tick_state, ev).await;
            }
            broadcast_snapshots(&tick_state).await;
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

async fn broadcast_snapshots(state: &AppState) {
    let clients: Vec<ClientSlot> = {
        let guard = state.clients.lock().await;
        guard
            .iter()
            .map(|(id, tx)| ClientSlot {
                player_id: *id,
                tx: tx.clone(),
            })
            .collect()
    };
    if clients.is_empty() {
        return;
    }
    let world = state.world.lock().await;
    for client in clients {
        if world.squad(client.player_id).and_then(|s| s.controlled_actor_id).is_none() {
            continue;
        }
        let snap = world.snapshot_for(client.player_id);
        if let Ok(json) = serde_json::to_string(&snap) {
            let _ = client.tx.send(json);
        }
    }
}

async fn handle_raid_event(state: &AppState, ev: sim::RaidEvent) {
    let profile_id = {
        let world = state.world.lock().await;
        world.profile_id(ev.player_id)
    };
    let mut cash = 0;
    let mut items_banked = ev.items_banked.clone();
    if let Some(pid) = profile_id {
        if ev.outcome == "extracted" {
            if !items_banked.is_empty() {
                let _ = db::insert_items(&state.db, pid, &items_banked).await;
            }
            cash = db::add_cash(&state.db, pid, ev.cash_delta)
                .await
                .unwrap_or(0);
        } else {
            items_banked.clear();
            if let Ok(p) = db::load_profile(&state.db, pid).await {
                cash = p.cash;
            }
        }
    }
    let ended = ServerMessage::RaidEnded {
        outcome: ev.outcome,
        cash,
        cash_delta: ev.cash_delta,
        items_banked: items_banked.clone(),
        message: ev.message,
    };
    send_to_player(state, ev.player_id, &ended).await;
    if let Some(pid) = profile_id {
        if let Ok(profile) = db::load_profile(&state.db, pid).await {
            send_profile(state, ev.player_id, &profile.stash, profile.cash, Vec::new(), None)
                .await;
        }
    }
}

async fn send_to_player(state: &AppState, player_id: Uuid, msg: &ServerMessage) {
    let json = serde_json::to_string(msg).unwrap();
    let tx = state.clients.lock().await.get(&player_id).cloned();
    if let Some(tx) = tx {
        let _ = tx.send(json);
    }
}

async fn send_profile(
    state: &AppState,
    player_id: Uuid,
    stash: &[StashItem],
    cash: i32,
    deployed: Vec<StashItem>,
    actor_id: Option<Uuid>,
) {
    let msg = ServerMessage::ProfileState {
        cash,
        stash: stash.to_vec(),
        deployed_loadout: deployed,
        actor_id,
    };
    send_to_player(state, player_id, &msg).await;
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| client_session(socket, state))
}

async fn client_session(socket: WebSocket, state: Arc<AppState>) {
    let (mut sink, mut stream) = socket.split();
    let mut player_id: Option<Uuid> = None;
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<String>();

    let writer = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            if sink.send(Message::Text(msg)).await.is_err() {
                break;
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
                let name = sanitize_name(name);
                let profile = match db::ensure_profile(&state.db, &name).await {
                    Ok(p) => p,
                    Err(e) => {
                        let ev = ServerMessage::Event {
                            kind: "error".into(),
                            message: format!("profile error: {e}"),
                        };
                        let _ = out_tx.send(serde_json::to_string(&ev).unwrap());
                        continue;
                    }
                };
                let mut world = state.world.lock().await;
                let id = world.register_squad(name.clone(), profile.id);
                drop(world);
                player_id = Some(id);
                state.clients.lock().await.insert(id, out_tx.clone());
                let map = {
                    let world = state.world.lock().await;
                    world.map.clone()
                };
                let welcome = ServerMessage::Welcome {
                    player_id: id,
                    actor_id: None,
                    map,
                };
                let _ = out_tx.send(serde_json::to_string(&welcome).unwrap());
                let ps = ServerMessage::ProfileState {
                    cash: profile.cash,
                    stash: profile.stash,
                    deployed_loadout: Vec::new(),
                    actor_id: None,
                };
                let _ = out_tx.send(serde_json::to_string(&ps).unwrap());
            }
            ClientMessage::GetProfile => {
                let Some(id) = player_id else { continue };
                let profile_id = state.world.lock().await.profile_id(id);
                if let Some(pid) = profile_id {
                    if let Ok(profile) = db::load_profile(&state.db, pid).await {
                        let deployed = state.world.lock().await.deployed_loadout(id);
                        let actor_id = state
                            .world
                            .lock()
                            .await
                            .squad(id)
                            .and_then(|s| s.controlled_actor_id);
                        send_profile(&state, id, &profile.stash, profile.cash, deployed, actor_id)
                            .await;
                    }
                }
            }
            ClientMessage::Deploy { loadout_item_ids } => {
                let Some(id) = player_id else { continue };
                let profile_id = match state.world.lock().await.profile_id(id) {
                    Some(p) => p,
                    None => continue,
                };
                let taken = match db::take_loadout(&state.db, profile_id, &loadout_item_ids).await {
                    Ok(t) => t,
                    Err(e) => {
                        let ev = ServerMessage::Event {
                            kind: "error".into(),
                            message: format!("loadout error: {e}"),
                        };
                        let _ = out_tx.send(serde_json::to_string(&ev).unwrap());
                        continue;
                    }
                };
                let deploy_result = {
                    let mut world = state.world.lock().await;
                    world.deploy(id, taken.clone())
                };
                match deploy_result {
                    Ok(actor_id) => {
                        let profile = db::load_profile(&state.db, profile_id).await.ok();
                        if let Some(p) = profile {
                            send_profile(&state, id, &p.stash, p.cash, taken, Some(actor_id)).await;
                        }
                        let ev = ServerMessage::Event {
                            kind: "deploy".into(),
                            message: format!("{actor_id}"),
                        };
                        let _ = out_tx.send(serde_json::to_string(&ev).unwrap());
                    }
                    Err(msg) => {
                        if !taken.is_empty() {
                            let _ = db::insert_items(&state.db, profile_id, &taken).await;
                        }
                        let ev = ServerMessage::Event {
                            kind: "error".into(),
                            message: msg,
                        };
                        let _ = out_tx.send(serde_json::to_string(&ev).unwrap());
                    }
                }
            }
            ClientMessage::Input {
                seq,
                dt,
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
                        dt,
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
        state.clients.lock().await.remove(&id);
        state.world.lock().await.remove_session(id);
    }
    writer.abort();
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
