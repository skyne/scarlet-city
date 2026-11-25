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
use sim::{CrewSpawn, InputState, World, DT};
use sqlx::SqlitePool;
use tokio::sync::{mpsc, Mutex};
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

use crate::protocol::{ClientMessage, CrewMember, ServerMessage, StashItem};

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
            let (events, notices) = {
                let mut world = tick_state.world.lock().await;
                world.tick();
                tick_state.tick.store(world.current_tick(), Ordering::Relaxed);
                (world.drain_events(), world.drain_notices())
            };
            for ev in events {
                handle_raid_event(&tick_state, ev).await;
            }
            broadcast_notices(&tick_state, notices).await;
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

async fn broadcast_notices(state: &AppState, notices: Vec<sim::CityNotice>) {
    if notices.is_empty() {
        return;
    }
    let listeners = {
        let world = state.world.lock().await;
        world.listener_positions()
    };
    for notice in notices {
        for (player_id, x, y) in &listeners {
            let dx = x - notice.x;
            let dy = y - notice.y;
            if dx * dx + dy * dy > notice.hear * notice.hear {
                continue;
            }
            let ev = ServerMessage::Event {
                kind: notice.kind.clone(),
                message: notice.message.clone(),
            };
            send_to_player(state, *player_id, &ev).await;
        }
    }
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
            send_profile(
                state,
                ev.player_id,
                &profile.stash,
                profile.cash,
                Vec::new(),
                None,
                None,
                None,
                &roster_members(&profile.crew),
            )
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
    squad_id: Option<Uuid>,
    controlled_actor_id: Option<Uuid>,
    crew: &[CrewMember],
) {
    let msg = ServerMessage::ProfileState {
        cash,
        stash: stash.to_vec(),
        deployed_loadout: deployed,
        actor_id,
        squad_id,
        controlled_actor_id,
        crew: crew.to_vec(),
    };
    send_to_player(state, player_id, &msg).await;
}

async fn send_live_profile(state: &AppState, player_id: Uuid) {
    let Some(profile_id) = state.world.lock().await.profile_id(player_id) else {
        return;
    };
    let Ok(profile) = db::load_profile(&state.db, profile_id).await else {
        return;
    };
    let (deployed, actor_id, squad_id, controlled) = {
        let world = state.world.lock().await;
        let squad = world.squad(player_id);
        (
            world.deployed_loadout(player_id),
            squad.and_then(|s| s.controlled_actor_id),
            squad.and_then(|s| s.squad_id),
            squad.and_then(|s| s.controlled_actor_id),
        )
    };
    send_profile(
        state,
        player_id,
        &profile.stash,
        profile.cash,
        deployed,
        actor_id,
        squad_id,
        controlled,
        &roster_members(&profile.crew),
    )
    .await;
}

fn roster_members(crew: &[db::CrewRecord]) -> Vec<CrewMember> {
    crew.iter()
        .map(|c| CrewMember {
            actor_id: c.id,
            name: c.name.clone(),
            role: c.role.clone(),
            hp: c.max_hp,
            max_hp: c.max_hp,
            is_ai: true,
            order_label: String::new(),
            order_x: 0.0,
            order_y: 0.0,
            order_z: 0.0,
            appearance: c.appearance.clone(),
        })
        .collect()
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
                    squad_id: None,
                    controlled_actor_id: None,
                    crew: roster_members(&profile.crew),
                };
                let _ = out_tx.send(serde_json::to_string(&ps).unwrap());
            }
            ClientMessage::GetProfile => {
                let Some(id) = player_id else { continue };
                send_live_profile(&state, id).await;
            }
            ClientMessage::Deploy {
                loadout_item_ids,
                crew_ids,
                join_squad_id,
            } => {
                let Some(id) = player_id else { continue };
                let profile_id = match state.world.lock().await.profile_id(id) {
                    Some(p) => p,
                    None => continue,
                };
                if crew_ids.is_empty() || crew_ids.len() > 4 {
                    let ev = ServerMessage::Event {
                        kind: "error".into(),
                        message: "select 1 to 4 crew".into(),
                    };
                    let _ = out_tx.send(serde_json::to_string(&ev).unwrap());
                    continue;
                }
                let roster = match db::load_crew(&state.db, profile_id).await {
                    Ok(c) => c,
                    Err(e) => {
                        let ev = ServerMessage::Event {
                            kind: "error".into(),
                            message: format!("crew error: {e}"),
                        };
                        let _ = out_tx.send(serde_json::to_string(&ev).unwrap());
                        continue;
                    }
                };
                let mut spawns = Vec::new();
                let mut missing = false;
                for cid in &crew_ids {
                    if let Some(c) = roster.iter().find(|c| c.id == *cid) {
                        spawns.push(CrewSpawn {
                            specialist_id: c.id,
                            name: c.name.clone(),
                            role: c.role.clone(),
                            max_hp: c.max_hp,
                            appearance: c.appearance.clone(),
                        });
                    } else {
                        missing = true;
                        break;
                    }
                }
                if missing || spawns.len() != crew_ids.len() {
                    let ev = ServerMessage::Event {
                        kind: "error".into(),
                        message: "unknown crew member".into(),
                    };
                    let _ = out_tx.send(serde_json::to_string(&ev).unwrap());
                    continue;
                }
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
                    world.deploy_crew(id, taken.clone(), spawns, join_squad_id)
                };
                match deploy_result {
                    Ok(actor_id) => {
                        let profile = db::load_profile(&state.db, profile_id).await.ok();
                        let squad_id = state.world.lock().await.squad(id).and_then(|s| s.squad_id);
                        if let Some(p) = profile {
                            send_profile(
                                &state,
                                id,
                                &p.stash,
                                p.cash,
                                taken,
                                Some(actor_id),
                                squad_id,
                                Some(actor_id),
                                &roster_members(&p.crew),
                            )
                            .await;
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
            ClientMessage::SwapActor { target_actor_id } => {
                let Some(id) = player_id else { continue };
                let result = {
                    let mut world = state.world.lock().await;
                    world.swap_actor(id, target_actor_id)
                };
                match result {
                    Ok(()) => {
                        let ev = ServerMessage::Event {
                            kind: "swap".into(),
                            message: target_actor_id.to_string(),
                        };
                        let _ = out_tx.send(serde_json::to_string(&ev).unwrap());
                    }
                    Err(msg) => {
                        let ev = ServerMessage::Event {
                            kind: "error".into(),
                            message: msg,
                        };
                        let _ = out_tx.send(serde_json::to_string(&ev).unwrap());
                    }
                }
            }
            ClientMessage::SetCrewAppearance { crew_id, appearance } => {
                let Some(id) = player_id else { continue };
                let Some(profile_id) = state.world.lock().await.profile_id(id) else {
                    continue;
                };
                let crew = match db::load_crew(&state.db, profile_id).await {
                    Ok(c) => c,
                    Err(e) => {
                        let ev = ServerMessage::Event {
                            kind: "error".into(),
                            message: format!("crew error: {e}"),
                        };
                        let _ = out_tx.send(serde_json::to_string(&ev).unwrap());
                        continue;
                    }
                };
                let appearance = db::normalize_appearance(&appearance);
                if !crew.iter().any(|c| c.id == crew_id)
                    || db::update_crew_appearance(&state.db, profile_id, crew_id, &appearance)
                        .await
                        .is_err()
                {
                    let ev = ServerMessage::Event {
                        kind: "error".into(),
                        message: "could not save appearance".into(),
                    };
                    let _ = out_tx.send(serde_json::to_string(&ev).unwrap());
                    continue;
                }
                send_live_profile(&state, id).await;
            }
            ClientMessage::SetCrewName { crew_id, name } => {
                let Some(id) = player_id else { continue };
                let Some(profile_id) = state.world.lock().await.profile_id(id) else {
                    continue;
                };
                let crew = match db::load_crew(&state.db, profile_id).await {
                    Ok(c) => c,
                    Err(e) => {
                        let ev = ServerMessage::Event {
                            kind: "error".into(),
                            message: format!("crew error: {e}"),
                        };
                        let _ = out_tx.send(serde_json::to_string(&ev).unwrap());
                        continue;
                    }
                };
                let name = sanitize_name(name);
                if !crew.iter().any(|c| c.id == crew_id)
                    || db::update_crew_name(&state.db, profile_id, crew_id, &name)
                        .await
                        .is_err()
                {
                    let ev = ServerMessage::Event {
                        kind: "error".into(),
                        message: "could not save name".into(),
                    };
                    let _ = out_tx.send(serde_json::to_string(&ev).unwrap());
                    continue;
                }
                send_live_profile(&state, id).await;
            }
            ClientMessage::SquadOrder {
                order_type,
                target_actor_id,
                x,
                y,
                z,
            } => {
                let Some(id) = player_id else { continue };
                let result = {
                    let mut world = state.world.lock().await;
                    world.squad_order(id, order_type, target_actor_id, x, y, z)
                };
                if let Err(msg) = result {
                    let ev = ServerMessage::Event {
                        kind: "error".into(),
                        message: msg,
                    };
                    let _ = out_tx.send(serde_json::to_string(&ev).unwrap());
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
