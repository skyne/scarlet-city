// AUTO-GENERATED from shared/protocol/scarlet.spdl — do not edit.
// Protocol scarlet.city v7

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeistKind {
    Vault,
    Terminal,
    Cargo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeistState {
    Locked,
    Ready,
    InProgress,
    Breached,
    Looted,
    Cooldown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HitKind {
    Wall,
    Vehicle,
    Actor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    Follow,
    Hold,
    MoveOrDriveTo,
    Engage,
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
pub struct HeistSite {
    pub id: String,
    pub name: String,
    pub kind: HeistKind,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub depth: f32,
    pub z_min: f32,
    pub z_max: f32,
    pub quiet_x: f32,
    pub quiet_y: f32,
    pub quiet_z: f32,
    pub quiet_secs: f32,
    pub loud_x: f32,
    pub loud_y: f32,
    pub loud_z: f32,
    pub loud_secs: f32,
    pub loot_tier: LootTier,
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
    pub heist_sites: Vec<HeistSite>,
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
pub struct CharacterAppearance {
    pub body_set: String,
    pub hair: String,
    pub outfit: String,
    pub beard: bool,
    pub brows: bool,
    pub hair_color: String,
    pub beard_color: String,
    pub eye_color: String,
    pub skin_color: String,
    pub outfit_color: String,
    pub height: f32,
    pub head: f32,
    pub neck: f32,
    pub shoulders: f32,
    pub bust: f32,
    pub waist: f32,
    pub hips: f32,
    pub bum: f32,
    pub muscle: f32,
    pub arms: f32,
    pub legs: f32,
    pub leg_length: f32,
    pub torso_length: f32,
    pub slot_top: String,
    pub slot_bottom: String,
    pub slot_shoes: String,
    pub slot_gloves: String,
    pub slot_head: String,
    pub slot_shoulder: String,
    pub slot_belt: String,
    pub weapon: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrewMember {
    pub actor_id: uuid::Uuid,
    pub name: String,
    pub role: String,
    pub hp: f32,
    pub max_hp: f32,
    pub is_ai: bool,
    pub order_label: String,
    pub order_x: f32,
    pub order_y: f32,
    pub order_z: f32,
    pub appearance: CharacterAppearance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub id: uuid::Uuid,
    pub player_id: uuid::Uuid,
    pub squad_id: uuid::Uuid,
    pub name: String,
    pub role: String,
    pub is_ai: bool,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub yaw: f32,
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,
    pub health: f32,
    pub armor: f32,
    pub backpack: Vec<StashItem>,
    pub vehicle_id: Option<uuid::Uuid>,
    pub extracting: bool,
    pub extract_progress: f32,
    pub last_processed_input_seq: u32,
    pub appearance: CharacterAppearance,
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
    pub hp: f32,
    pub driver_id: Option<uuid::Uuid>,
    pub law: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeistMarker {
    pub id: String,
    pub name: String,
    pub kind: HeistKind,
    pub state: HeistState,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub quiet_x: f32,
    pub quiet_y: f32,
    pub quiet_z: f32,
    pub loud_x: f32,
    pub loud_y: f32,
    pub loud_z: f32,
    pub progress: f32,
    pub approach: String,
    pub alarm: bool,
    pub owner_squad: Option<uuid::Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlarmPing {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub radius: f32,
    pub kind: String,
    pub intensity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcState {
    pub id: uuid::Uuid,
    pub kind: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub yaw: f32,
    pub health: f32,
    pub alert: bool,
    pub vehicle_id: Option<uuid::Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Impact {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub from_x: f32,
    pub from_y: f32,
    pub from_z: f32,
    pub hit_kind: HitKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LootNode {
    pub id: uuid::Uuid,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub tier: LootTier,
    pub taken: bool,
    pub respawn_secs: f32,
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
        crew_ids: Vec<uuid::Uuid>,
        join_squad_id: Option<uuid::Uuid>,
    },
    SwapActor {
        target_actor_id: uuid::Uuid,
    },
    SetCrewAppearance {
        crew_id: uuid::Uuid,
        appearance: CharacterAppearance,
    },
    SetCrewName {
        crew_id: uuid::Uuid,
        name: String,
    },
    SquadOrder {
        order_type: OrderType,
        target_actor_id: Option<uuid::Uuid>,
        x: f32,
        y: f32,
        z: f32,
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
        squad_id: Option<uuid::Uuid>,
        controlled_actor_id: Option<uuid::Uuid>,
        crew: Vec<CrewMember>,
    },
    Snapshot {
        tick: u64,
        last_processed_input_seq: u32,
        squad_id: Option<uuid::Uuid>,
        controlled_actor_id: Option<uuid::Uuid>,
        crew: Vec<CrewMember>,
        players: Vec<PlayerState>,
        vehicles: Vec<VehicleState>,
        loot_nodes: Vec<LootNode>,
        death_crates: Vec<DeathCrate>,
        exfil_zones: Vec<ExfilZone>,
        projectiles: Vec<Projectile>,
        impacts: Vec<Impact>,
        heists: Vec<HeistMarker>,
        heat: f32,
        stars: u8,
        alarms: Vec<AlarmPing>,
        npcs: Vec<NpcState>,
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
