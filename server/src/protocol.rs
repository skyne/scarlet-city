// AUTO-GENERATED from shared/protocol/scarlet.spdl — do not edit.
// Protocol scarlet.city v2

#![allow(dead_code)]

use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VehicleKind {
    GroundCar,
    HoverCraft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LootTier {
    Common,
    Rare,
    Epic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Aabb3 {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub depth: f32,
    pub z_min: f32,
    pub z_max: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Road {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub depth: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Obstacle {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub depth: f32,
    pub z_min: f32,
    pub z_max: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleSpawn {
    pub id: uuid::Uuid,
    pub kind: VehicleKind,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub yaw: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LootZone {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub depth: f32,
    pub z_min: f32,
    pub z_max: f32,
    pub tier: LootTier,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExfilZone {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub depth: f32,
    pub z_min: f32,
    pub z_max: f32,
    pub duration_secs: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapBounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub depth: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapData {
    pub name: String,
    pub bounds: MapBounds,
    pub ground_z: f32,
    pub max_altitude: f32,
    pub roads: Vec<Road>,
    pub obstacles: Vec<Obstacle>,
    pub vehicle_spawns: Vec<VehicleSpawn>,
    pub loot_zones: Vec<LootZone>,
    pub exfil_zones: Vec<ExfilZone>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StashItem {
    pub id: uuid::Uuid,
    pub item_type: String,
    pub item_name: String,
    pub quantity: u32,
    pub stats_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub id: uuid::Uuid,
    pub player_id: uuid::Uuid,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub yaw: f32,
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,
    pub health: f32,
    pub vehicle_id: Option<uuid::Uuid>,
    pub extracting: bool,
    pub extract_progress: f32,
    pub last_processed_input_seq: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleState {
    pub id: uuid::Uuid,
    pub kind: VehicleKind,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub yaw: f32,
    pub speed: f32,
    pub driver_id: Option<uuid::Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LootNode {
    pub id: uuid::Uuid,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub tier: LootTier,
    pub taken: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeathCrate {
    pub id: uuid::Uuid,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub items: Vec<StashItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Projectile {
    pub id: uuid::Uuid,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,
    pub owner_id: uuid::Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    Join {
        name: String,
    },
    GetProfile,
    Deploy {
        loadout_item_ids: Vec<uuid::Uuid>,
    },
    Input {
        seq: u32,
        dt: f32,
        move_x: f32,
        move_y: f32,
        ascend_descend: f32,
        aim_angle: f32,
        shoot: bool,
        interact: bool,
        brake: bool,
    },
    Ping {
        client_ms: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    Welcome {
        player_id: uuid::Uuid,
        actor_id: Option<uuid::Uuid>,
        map: MapData,
    },
    ProfileState {
        cash: i32,
        stash: Vec<StashItem>,
        deployed_loadout: Vec<StashItem>,
        actor_id: Option<uuid::Uuid>,
    },
    Snapshot {
        tick: u64,
        last_processed_input_seq: u32,
        players: Vec<PlayerState>,
        vehicles: Vec<VehicleState>,
        loot_nodes: Vec<LootNode>,
        death_crates: Vec<DeathCrate>,
        exfil_zones: Vec<ExfilZone>,
        projectiles: Vec<Projectile>,
    },
    RaidEnded {
        outcome: String,
        cash: i32,
        cash_delta: i32,
        items_banked: Vec<StashItem>,
        message: String,
    },
    Event {
        kind: String,
        message: String,
    },
    Pong {
        client_ms: u64,
        server_tick: u64,
    },
}
