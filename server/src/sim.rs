use std::collections::{HashMap, VecDeque};

use uuid::Uuid;

use crate::collision::{
    blocked_by_map, in_vision_radius, intervals_overlap, point_in_rect, segment_cylinder_t,
    segment_obstacle_t, sightline_blocked,
};
use crate::protocol::{
    CrewMember, DeathCrate, ExfilZone, HitKind, Impact, LootNode, LootTier, MapData, OrderType,
    PlayerState, Projectile, ServerMessage, StashItem, VehicleKind, VehicleState,
};

pub const TICK_HZ: u32 = 20;
pub const DT: f32 = 1.0 / TICK_HZ as f32;

const PLAYER_RADIUS: f32 = 0.45;
const PLAYER_HEIGHT: f32 = 1.8;
const PLAYER_SPEED: f32 = 8.0;

const CAR_RADIUS: f32 = 1.1;
const CAR_HEIGHT: f32 = 1.4;
const CAR_ACCEL: f32 = 18.0;
const CAR_MAX_SPEED: f32 = 22.0;
const CAR_TURN: f32 = 2.6;
const CAR_GRIP: f32 = 6.0;

const HOVER_RADIUS: f32 = 1.2;
const HOVER_HEIGHT: f32 = 1.2;
const HOVER_ACCEL: f32 = 14.0;
const HOVER_MAX_SPEED: f32 = 16.0;
const HOVER_CLIMB: f32 = 7.0;
const HOVER_TURN: f32 = 2.2;

const PROJECTILE_SPEED: f32 = 48.0;
const PROJECTILE_LIFE: f32 = 0.9;
const SHOOT_COOLDOWN: f32 = 0.22;
const INTERACT_RANGE: f32 = 2.5;
const INTERACT_Z: f32 = 2.0;
const BACKPACK_SLOTS: usize = 8;
const BACKPACK_MAX_WEIGHT: f32 = 12.0;
const DEATH_CRATE_TTL_SECS: f32 = 90.0;
const LOOT_RESPAWN_COMMON_SECS: f32 = 45.0;
const LOOT_RESPAWN_RARE_SECS: f32 = 90.0;
const LOOT_RESPAWN_EPIC_SECS: f32 = 180.0;
const CAR_HP: f32 = 80.0;
const HOVER_HP: f32 = 60.0;
const FRIENDLY_FIRE: bool = false;
pub const VISION_RADIUS: f32 = 32.0;

#[derive(Clone, Debug, Default)]
pub struct InputState {
    pub seq: u32,
    pub dt: f32,
    pub move_x: f32,
    pub move_y: f32,
    pub ascend_descend: f32,
    pub aim_angle: f32,
    pub shoot: bool,
    pub interact: bool,
    pub brake: bool,
}

/// Session identity is `player_id`. The body in the raid is `controlled_actor_id`.
/// `squad_id` is the shared crew (1–4 actors) this session is deployed with.
pub struct Squad {
    #[allow(dead_code)]
    pub player_id: Uuid,
    pub squad_id: Option<Uuid>,
    pub controlled_actor_id: Option<Uuid>,
    #[allow(dead_code)]
    pub name: String,
    pub profile_id: i64,
}

#[derive(Clone, Debug)]
pub struct CrewSpawn {
    pub specialist_id: Uuid,
    pub name: String,
    pub role: String,
    pub max_hp: f32,
}

#[derive(Clone, Debug)]
enum Order {
    Follow,
    Hold { x: f32, y: f32, z: f32 },
    MoveOrDriveTo { x: f32, y: f32, z: f32 },
    Engage { x: f32, y: f32, z: f32 },
}

impl Order {
    fn point(&self) -> Option<(f32, f32, f32)> {
        match *self {
            Order::Follow => None,
            Order::Hold { x, y, z }
            | Order::MoveOrDriveTo { x, y, z }
            | Order::Engage { x, y, z } => Some((x, y, z)),
        }
    }
}

struct Actor {
    id: Uuid,
    player_id: Uuid,
    squad_id: Uuid,
    #[allow(dead_code)]
    specialist_id: Uuid,
    name: String,
    role: String,
    max_hp: f32,
    order: Order,
    x: f32,
    y: f32,
    z: f32,
    yaw: f32,
    vx: f32,
    vy: f32,
    vz: f32,
    health: f32,
    armor: f32,
    vehicle_id: Option<Uuid>,
    extract_time: f32,
    input: InputState,
    pending_inputs: VecDeque<InputState>,
    shoot_cd: f32,
    interact_edge: bool,
    last_seq: u32,
    backpack: Vec<StashItem>,
}

struct Vehicle {
    id: Uuid,
    kind: VehicleKind,
    x: f32,
    y: f32,
    z: f32,
    yaw: f32,
    vel_yaw: f32,
    speed: f32,
    hp: f32,
    driver_id: Option<Uuid>,
}

struct FlyingShot {
    proj: Projectile,
    life: f32,
    damage: f32,
}

struct TrackedCrate {
    data: DeathCrate,
    age: f32,
}

enum LootPick {
    Node(usize),
    Crate(usize),
}

pub struct RaidEvent {
    pub player_id: Uuid,
    pub outcome: String,
    pub items_banked: Vec<StashItem>,
    pub cash_delta: i32,
    pub message: String,
}

struct TrackedLoot {
    node: LootNode,
    respawn_at: Option<u64>,
}

pub struct World {
    pub map: MapData,
    tick: u64,
    squads: HashMap<Uuid, Squad>,
    crews: HashMap<Uuid, Vec<Uuid>>,
    actors: HashMap<Uuid, Actor>,
    vehicles: HashMap<Uuid, Vehicle>,
    loot: Vec<TrackedLoot>,
    death_crates: Vec<TrackedCrate>,
    shots: Vec<FlyingShot>,
    impacts: Vec<Impact>,
    events: Vec<RaidEvent>,
}

impl World {
    pub fn new(map: MapData) -> Self {
        let loot = map
            .loot_zones
            .iter()
            .map(|z| TrackedLoot {
                node: LootNode {
                    id: Uuid::new_v4(),
                    x: z.x + z.width * 0.5,
                    y: z.y + z.depth * 0.5,
                    z: z.z_min,
                    tier: z.tier,
                    taken: false,
                    respawn_secs: 0.0,
                },
                respawn_at: None,
            })
            .collect();
        let vehicles = map
            .vehicle_spawns
            .iter()
            .map(|s| {
                (
                    s.id,
                    Vehicle {
                        id: s.id,
                        kind: s.kind,
                        x: s.x,
                        y: s.y,
                        z: s.z,
                        yaw: s.yaw,
                        vel_yaw: s.yaw,
                        speed: 0.0,
                        hp: vehicle_max_hp(s.kind),
                        driver_id: None,
                    },
                )
            })
            .collect();
        Self {
            map,
            tick: 0,
            squads: HashMap::new(),
            crews: HashMap::new(),
            actors: HashMap::new(),
            vehicles,
            loot,
            death_crates: Vec::new(),
            shots: Vec::new(),
            impacts: Vec::new(),
            events: Vec::new(),
        }
    }

    pub fn register_squad(&mut self, name: String, profile_id: i64) -> Uuid {
        let player_id = Uuid::new_v4();
        self.squads.insert(
            player_id,
            Squad {
                player_id,
                squad_id: None,
                controlled_actor_id: None,
                name,
                profile_id,
            },
        );
        player_id
    }

    pub fn squad(&self, player_id: Uuid) -> Option<&Squad> {
        self.squads.get(&player_id)
    }

    pub fn profile_id(&self, player_id: Uuid) -> Option<i64> {
        self.squads.get(&player_id).map(|s| s.profile_id)
    }

    pub fn deployed_loadout(&self, player_id: Uuid) -> Vec<StashItem> {
        self.squads
            .get(&player_id)
            .and_then(|s| s.controlled_actor_id)
            .and_then(|aid| self.actors.get(&aid))
            .map(|a| a.backpack.clone())
            .unwrap_or_default()
    }

    pub fn remove_session(&mut self, player_id: Uuid) {
        let Some(session) = self.squads.remove(&player_id) else {
            return;
        };
        let squad_id = session.squad_id;
        let owned: Vec<Uuid> = self
            .actors
            .values()
            .filter(|a| a.player_id == player_id)
            .map(|a| a.id)
            .collect();
        for id in owned {
            if let Some(sid) = self.actors.get(&id).map(|a| a.squad_id) {
                self.crew_remove(sid, id);
            }
            self.drop_actor(id, true);
        }
        if let Some(sid) = squad_id {
            self.prune_crew(sid);
        }
    }

    #[allow(dead_code)]
    pub fn deploy(&mut self, player_id: Uuid, loadout: Vec<StashItem>) -> Result<Uuid, String> {
        self.deploy_crew(
            player_id,
            loadout,
            vec![CrewSpawn {
                specialist_id: Uuid::new_v4(),
                name: "Operative".into(),
                role: "Point".into(),
                max_hp: 100.0,
            }],
            None,
        )
    }

    /// Spawn 1–4 crew bodies. `join_squad` attaches them to an existing co-op squad.
    /// The first specialist is controlled by `player_id`; the rest start on Follow.
    pub fn deploy_crew(
        &mut self,
        player_id: Uuid,
        loadout: Vec<StashItem>,
        crew: Vec<CrewSpawn>,
        join_squad: Option<Uuid>,
    ) -> Result<Uuid, String> {
        if crew.is_empty() || crew.len() > 4 {
            return Err("select 1 to 4 crew".into());
        }
        let session = self
            .squads
            .get(&player_id)
            .ok_or_else(|| "unknown player".to_string())?;
        if session.controlled_actor_id.is_some() || session.squad_id.is_some() {
            return Err("already in raid".into());
        }
        let squad_id = if let Some(id) = join_squad {
            let existing = self.crews.get(&id).map(|c| c.len()).unwrap_or(0);
            if existing == 0 {
                return Err("unknown squad".into());
            }
            if existing + crew.len() > 4 {
                return Err("that squad only has room for fewer crew".into());
            }
            id
        } else {
            let id = Uuid::new_v4();
            self.crews.insert(id, Vec::new());
            id
        };

        let (ax, ay, _az) = self.spawn_anchor(squad_id);
        let mut first = None;
        for (i, member) in crew.iter().enumerate() {
            let actor_id = Uuid::new_v4();
            let ang = i as f32 * std::f32::consts::TAU / 4.0;
            let ring = if crew.len() == 1 { 0.0 } else { 1.7 };
            let mut pack = if i == 0 { loadout.clone() } else { Vec::new() };
            if !pack
                .iter()
                .any(|it| it.item_type.eq_ignore_ascii_case("weapon"))
            {
                pack.insert(0, issued_sidearm());
            }
            let order = if i == 0 {
                Order::Hold {
                    x: ax,
                    y: ay,
                    z: self.map.ground_z,
                }
            } else {
                Order::Follow
            };
            self.actors.insert(
                actor_id,
                Actor {
                    id: actor_id,
                    player_id,
                    squad_id,
                    specialist_id: member.specialist_id,
                    name: member.name.clone(),
                    role: member.role.clone(),
                    max_hp: member.max_hp.max(1.0),
                    order,
                    x: ax + ang.cos() * ring,
                    y: ay + ang.sin() * ring,
                    z: self.map.ground_z,
                    yaw: 0.0,
                    vx: 0.0,
                    vy: 0.0,
                    vz: 0.0,
                    health: member.max_hp.max(1.0),
                    armor: if i == 0 {
                        armor_from_items(&pack)
                    } else {
                        0.0
                    },
                    vehicle_id: None,
                    extract_time: 0.0,
                    input: InputState::default(),
                    pending_inputs: VecDeque::new(),
                    shoot_cd: 0.0,
                    interact_edge: false,
                    last_seq: 0,
                    backpack: pack,
                },
            );
            self.crews.entry(squad_id).or_default().push(actor_id);
            if first.is_none() {
                first = Some(actor_id);
            }
        }
        let actor_id = first.ok_or_else(|| "empty crew".to_string())?;
        if let Some(session) = self.squads.get_mut(&player_id) {
            session.squad_id = Some(squad_id);
            session.controlled_actor_id = Some(actor_id);
        }
        Ok(actor_id)
    }

    pub fn swap_actor(&mut self, player_id: Uuid, target: Uuid) -> Result<(), String> {
        let session = self
            .squads
            .get(&player_id)
            .ok_or_else(|| "unknown player".to_string())?;
        let squad_id = session.squad_id.ok_or_else(|| "not in a squad".to_string())?;
        if session.controlled_actor_id == Some(target) {
            return Ok(());
        }
        let actor = self
            .actors
            .get(&target)
            .ok_or_else(|| "unknown actor".to_string())?;
        if actor.squad_id != squad_id {
            return Err("not in your squad".into());
        }
        if actor.health <= 0.0 {
            return Err("that crewmate is down".into());
        }
        if let Some(other) = self.controller_of(target) {
            if other != player_id {
                return Err("already controlled".into());
            }
        }
        let prev = session.controlled_actor_id;
        if let Some(prev_id) = prev {
            if let Some(body) = self.actors.get_mut(&prev_id) {
                body.order = if body.vehicle_id.is_some() {
                    Order::MoveOrDriveTo {
                        x: body.x,
                        y: body.y,
                        z: body.z,
                    }
                } else {
                    Order::Hold {
                        x: body.x,
                        y: body.y,
                        z: body.z,
                    }
                };
                body.pending_inputs.clear();
                body.input.move_x = 0.0;
                body.input.move_y = 0.0;
                body.input.shoot = false;
                body.input.interact = false;
            }
        }
        if let Some(body) = self.actors.get_mut(&target) {
            body.pending_inputs.clear();
            body.input.shoot = false;
        }
        if let Some(session) = self.squads.get_mut(&player_id) {
            session.controlled_actor_id = Some(target);
        }
        Ok(())
    }

    pub fn squad_order(
        &mut self,
        player_id: Uuid,
        kind: OrderType,
        target: Option<Uuid>,
        x: f32,
        y: f32,
        z: f32,
    ) -> Result<(), String> {
        let session = self
            .squads
            .get(&player_id)
            .ok_or_else(|| "unknown player".to_string())?;
        let squad_id = session.squad_id.ok_or_else(|| "not in a squad".to_string())?;
        let order = match kind {
            OrderType::Follow => Order::Follow,
            OrderType::Hold => Order::Hold { x, y, z },
            OrderType::MoveOrDriveTo => Order::MoveOrDriveTo { x, y, z },
            OrderType::Engage => Order::Engage { x, y, z },
        };
        let ids: Vec<Uuid> = if let Some(tid) = target {
            let actor = self
                .actors
                .get(&tid)
                .ok_or_else(|| "unknown actor".to_string())?;
            if actor.squad_id != squad_id {
                return Err("not in your squad".into());
            }
            if actor.health <= 0.0 {
                return Err("that crewmate is down".into());
            }
            if self.is_human_controlled(tid) {
                return Err("that crewmate is player-controlled".into());
            }
            vec![tid]
        } else {
            self.crews
                .get(&squad_id)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter(|id| !self.is_human_controlled(*id))
                .collect()
        };
        for id in ids {
            if let Some(actor) = self.actors.get_mut(&id) {
                actor.order = order.clone();
            }
        }
        Ok(())
    }

    pub fn set_input(&mut self, player_id: Uuid, input: InputState) {
        let Some(squad) = self.squads.get(&player_id) else {
            return;
        };
        let Some(aid) = squad.controlled_actor_id else {
            return;
        };
        if let Some(actor) = self.actors.get_mut(&aid) {
            actor.pending_inputs.push_back(input);
            if actor.pending_inputs.len() > 64 {
                actor.pending_inputs.pop_front();
            }
        }
    }

    pub fn tick(&mut self) {
        self.tick += 1;
        self.impacts.clear();
        self.tick_loot();
        self.tick_crates();
        let ids: Vec<Uuid> = self.actors.keys().copied().collect();
        for id in ids {
            self.tick_actor(id);
        }
        self.tick_projectiles();
    }

    fn tick_loot(&mut self) {
        for node in &mut self.loot {
            let Some(at) = node.respawn_at else {
                continue;
            };
            if node.node.taken && self.tick >= at {
                node.node.taken = false;
                node.respawn_at = None;
            }
        }
    }

    fn tick_crates(&mut self) {
        for c in &mut self.death_crates {
            c.age += DT;
        }
        self.death_crates
            .retain(|c| !c.data.items.is_empty() && c.age < DEATH_CRATE_TTL_SECS);
    }

    pub fn drain_events(&mut self) -> Vec<RaidEvent> {
        std::mem::take(&mut self.events)
    }

    fn tick_actor(&mut self, id: Uuid) {
        if !self.is_human_controlled(id) {
            let input = self.bot_think(id);
            self.apply_input_step(id, input);
            return;
        }
        let Some(actor) = self.actors.get_mut(&id) else {
            return;
        };
        // One fixed step per server tick. Packet rate and client dt do not add time.
        let mut steps: Vec<InputState> = Vec::new();
        while let Some(inp) = actor.pending_inputs.pop_front() {
            steps.push(inp);
        }
        let input = if let Some(latest) = steps.last() {
            let mut merged = latest.clone();
            merged.dt = DT;
            merged.shoot = steps.iter().any(|s| s.shoot);
            merged.interact = steps.iter().any(|s| s.interact);
            merged
        } else {
            let mut held = actor.input.clone();
            held.dt = DT;
            held.shoot = false;
            held.interact = false;
            held
        };
        self.apply_input_step(id, input);
    }

    fn apply_input_step(&mut self, id: Uuid, input: InputState) {
        let dt = DT;
        let Some(actor) = self.actors.get_mut(&id) else {
            return;
        };
        actor.last_seq = input.seq;
        actor.input = input.clone();
        let interact = input.interact && !actor.interact_edge;
        let vehicle_id = actor.vehicle_id;

        if let Some(vid) = vehicle_id {
            self.drive_vehicle(id, vid, &input, dt);
            self.follow_vehicle(id, vid);
        } else {
            self.walk_actor(id, &input, dt);
        }

        if interact {
            self.try_interact(id);
        }
        if let Some(p) = self.actors.get_mut(&id) {
            p.interact_edge = input.interact;
            p.yaw = input.aim_angle;
            p.shoot_cd = (p.shoot_cd - dt).max(0.0);
        }
        self.update_extraction(id, dt);
        self.try_shoot(id);
    }

    fn walk_actor(&mut self, id: Uuid, input: &InputState, dt: f32) {
        let Some(p) = self.actors.get(&id) else {
            return;
        };
        let mut mx = input.move_x;
        let mut my = input.move_y;
        let mag = (mx * mx + my * my).sqrt();
        if mag > 1.0 {
            mx /= mag;
            my /= mag;
        }
        let ox = p.x;
        let oy = p.y;
        let oz = p.z;
        let nx = ox + mx * PLAYER_SPEED * dt;
        let ny = oy + my * PLAYER_SPEED * dt;
        let z = self.map.ground_z;
        let z1 = z + PLAYER_HEIGHT;
        let (x, y) = self.try_move(ox, oy, nx, ny, PLAYER_RADIUS, z, z1);
        let (x, y) = self.clamp_bounds(x, y, PLAYER_RADIUS);
        if let Some(p) = self.actors.get_mut(&id) {
            p.vx = (x - ox) / dt;
            p.vy = (y - oy) / dt;
            p.vz = (z - oz) / dt;
            p.x = x;
            p.y = y;
            p.z = z;
        }
    }

    fn drive_vehicle(&mut self, actor_id: Uuid, vid: Uuid, input: &InputState, dt: f32) {
        let Some(v) = self.vehicles.get(&vid) else {
            return;
        };
        if v.hp <= 0.0 {
            return;
        }
        let kind = v.kind;
        let (radius, height, accel, max_speed, turn) = match kind {
            VehicleKind::GroundCar => (CAR_RADIUS, CAR_HEIGHT, CAR_ACCEL, CAR_MAX_SPEED, CAR_TURN),
            VehicleKind::HoverCraft => (
                HOVER_RADIUS,
                HOVER_HEIGHT,
                HOVER_ACCEL,
                HOVER_MAX_SPEED,
                HOVER_TURN,
            ),
        };

        let throttle = input.move_y;
        let steer = input.move_x;
        let mut speed = v.speed;
        speed += throttle * accel * dt;
        if input.brake {
            speed *= 0.82;
        }
        speed = speed.clamp(-max_speed * 0.4, max_speed);

        let mut yaw = v.yaw;
        yaw += steer * turn * dt * (speed.abs() / max_speed).max(0.25);
        let mut vel_yaw = v.vel_yaw;
        let grip = match kind {
            VehicleKind::GroundCar => CAR_GRIP,
            VehicleKind::HoverCraft => 10.0,
        };
        vel_yaw = lerp_angle(vel_yaw, yaw, (grip * dt).min(1.0));

        let ox = v.x;
        let oy = v.y;
        let oz = v.z;
        let nx = ox + vel_yaw.cos() * speed * dt;
        let ny = oy + vel_yaw.sin() * speed * dt;

        let mut z = v.z;
        match kind {
            VehicleKind::GroundCar => z = self.map.ground_z,
            VehicleKind::HoverCraft => {
                z += input.ascend_descend.clamp(-1.0, 1.0) * HOVER_CLIMB * dt;
                z = z.clamp(self.map.ground_z, self.map.max_altitude);
            }
        }

        let z1 = z + height;
        let (x, y) = self.try_move(ox, oy, nx, ny, radius, z, z1);
        let (x, y) = self.clamp_bounds(x, y, radius);

        let z = if blocked_by_map(x, y, radius, z, z1, &self.map.obstacles) {
            oz
        } else {
            z
        };

        if let Some(v) = self.vehicles.get_mut(&vid) {
            v.x = x;
            v.y = y;
            v.z = z;
            v.yaw = yaw;
            v.vel_yaw = vel_yaw;
            v.speed = speed;
            v.driver_id = Some(actor_id);
        }
        if let Some(p) = self.actors.get_mut(&actor_id) {
            p.vx = (x - ox) / dt;
            p.vy = (y - oy) / dt;
            p.vz = (z - oz) / dt;
        }
    }

    fn follow_vehicle(&mut self, actor_id: Uuid, vid: Uuid) {
        let Some(v) = self.vehicles.get(&vid) else {
            return;
        };
        let (x, y, z, yaw) = (v.x, v.y, v.z, v.yaw);
        if let Some(p) = self.actors.get_mut(&actor_id) {
            p.x = x;
            p.y = y;
            p.z = z;
            p.yaw = yaw;
        }
    }

    fn try_interact(&mut self, id: Uuid) {
        let Some(p) = self.actors.get(&id) else {
            return;
        };
        let (px, py, pz) = (p.x, p.y, p.z);
        let in_vehicle = p.vehicle_id;

        let mut best: Option<(f32, LootPick)> = None;
        for (idx, node) in self.loot.iter().enumerate() {
            if node.node.taken {
                continue;
            }
            if let Some(dist) = interact_dist(px, py, pz, node.node.x, node.node.y, node.node.z) {
                if best.as_ref().map(|(d, _)| dist < *d).unwrap_or(true) {
                    best = Some((dist, LootPick::Node(idx)));
                }
            }
        }
        for (idx, c) in self.death_crates.iter().enumerate() {
            if c.data.items.is_empty() {
                continue;
            }
            if let Some(dist) = interact_dist(px, py, pz, c.data.x, c.data.y, c.data.z) {
                if best.as_ref().map(|(d, _)| dist < *d).unwrap_or(true) {
                    best = Some((dist, LootPick::Crate(idx)));
                }
            }
        }
        if let Some((_, pick)) = best {
            self.take_one(id, pick);
            return;
        }

        if let Some(vid) = in_vehicle {
            if let Some(v) = self.vehicles.get_mut(&vid) {
                v.driver_id = None;
            }
            if let Some(p) = self.actors.get_mut(&id) {
                p.vehicle_id = None;
                p.z = self.map.ground_z;
            }
            return;
        }

        let mut nearest: Option<(f32, Uuid)> = None;
        for v in self.vehicles.values() {
            if v.driver_id.is_some() || v.hp <= 0.0 {
                continue;
            }
            if let Some(dist) = interact_dist(px, py, pz, v.x, v.y, v.z) {
                if nearest.as_ref().map(|(d, _)| dist < *d).unwrap_or(true) {
                    nearest = Some((dist, v.id));
                }
            }
        }
        if let Some((_, vid)) = nearest {
            if let Some(v) = self.vehicles.get_mut(&vid) {
                v.driver_id = Some(id);
            }
            if let Some(p) = self.actors.get_mut(&id) {
                p.vehicle_id = Some(vid);
            }
        }
    }

    fn take_one(&mut self, id: Uuid, pick: LootPick) {
        let item = match pick {
            LootPick::Crate(idx) => {
                let Some(c) = self.death_crates.get(idx) else {
                    return;
                };
                let Some(item) = c.data.items.first().cloned() else {
                    return;
                };
                item
            }
            LootPick::Node(idx) => {
                let Some(node) = self.loot.get(idx) else {
                    return;
                };
                if node.node.taken {
                    return;
                }
                loot_to_item(node.node.tier)
            }
        };
        let Some(p) = self.actors.get(&id) else {
            return;
        };
        if !can_stow(&p.backpack, &item) {
            return;
        }
        match pick {
            LootPick::Crate(idx) => {
                if let Some(c) = self.death_crates.get_mut(idx) {
                    if !c.data.items.is_empty() {
                        c.data.items.remove(0);
                    }
                }
                if self
                    .death_crates
                    .get(idx)
                    .map(|c| c.data.items.is_empty())
                    .unwrap_or(false)
                {
                    self.death_crates.remove(idx);
                }
            }
            LootPick::Node(idx) => {
                if let Some(node) = self.loot.get_mut(idx) {
                    node.node.taken = true;
                    node.respawn_at =
                        Some(self.tick.saturating_add(loot_respawn_ticks(node.node.tier)));
                }
            }
        }
        if let Some(p) = self.actors.get_mut(&id) {
            if item.item_type.eq_ignore_ascii_case("armor") {
                if let Some(bonus) = stat_f32(&item, "armor") {
                    p.armor += bonus.max(0.0);
                }
            }
            p.backpack.push(item);
        }
    }

    fn update_extraction(&mut self, id: Uuid, dt: f32) {
        if !self.is_human_controlled(id) {
            if let Some(p) = self.actors.get_mut(&id) {
                p.extract_time = 0.0;
            }
            return;
        }
        let Some(p) = self.actors.get(&id) else {
            return;
        };
        let (x, y, z) = (p.x, p.y, p.z);
        let mut in_zone: Option<&ExfilZone> = None;
        for zoned in &self.map.exfil_zones {
            if point_in_rect(x, y, zoned.x, zoned.y, zoned.width, zoned.depth)
                && intervals_overlap(z, z + PLAYER_HEIGHT, zoned.z_min, zoned.z_max)
            {
                in_zone = Some(zoned);
                break;
            }
        }
        if let Some(zone) = in_zone {
            let dur = zone.duration_secs.max(0.1);
            let extract_now = {
                if let Some(p) = self.actors.get_mut(&id) {
                    p.extract_time += dt;
                    p.extract_time >= dur
                } else {
                    false
                }
            };
            if extract_now {
                self.complete_extract(id);
            }
        } else if let Some(p) = self.actors.get_mut(&id) {
            p.extract_time = 0.0;
        }
    }

    fn complete_extract(&mut self, actor_id: Uuid) {
        let Some(actor) = self.actors.get(&actor_id) else {
            return;
        };
        let player_id = self.controller_of(actor_id).unwrap_or(actor.player_id);
        let squad_id = actor.squad_id;
        let owned: Vec<Uuid> = self
            .actors
            .values()
            .filter(|a| a.player_id == player_id && a.squad_id == squad_id)
            .map(|a| a.id)
            .collect();
        let mut items = Vec::new();
        for id in &owned {
            if let Some(body) = self.actors.get(id) {
                items.extend(body.backpack.iter().filter(|i| !is_issued(i)).cloned());
            }
        }
        let cash_delta: i32 = items.iter().map(crate::db::loot_cash_value).sum();
        self.events.push(RaidEvent {
            player_id,
            outcome: "extracted".into(),
            items_banked: items,
            cash_delta,
            message: "You extracted. Gear is in the stash.".into(),
        });
        for id in owned {
            self.crew_remove(squad_id, id);
            self.drop_actor(id, false);
        }
        if let Some(session) = self.squads.get_mut(&player_id) {
            session.controlled_actor_id = None;
            session.squad_id = None;
        }
        self.prune_crew(squad_id);
    }

    fn drop_actor(&mut self, actor_id: Uuid, as_crate: bool) {
        let Some(actor) = self.actors.remove(&actor_id) else {
            return;
        };
        if let Some(vid) = actor.vehicle_id {
            if let Some(v) = self.vehicles.get_mut(&vid) {
                v.driver_id = None;
            }
        }
        if as_crate && !actor.backpack.is_empty() {
            self.death_crates.push(TrackedCrate {
                data: DeathCrate {
                    id: Uuid::new_v4(),
                    x: actor.x,
                    y: actor.y,
                    z: actor.z,
                    items: actor.backpack,
                },
                age: 0.0,
            });
        }
    }

    fn try_shoot(&mut self, id: Uuid) {
        let Some(p) = self.actors.get(&id) else {
            return;
        };
        if !p.input.shoot || p.shoot_cd > 0.0 {
            return;
        }
        let has_weapon = p
            .backpack
            .iter()
            .any(|i| i.item_type.eq_ignore_ascii_case("weapon"));
        if !has_weapon {
            return;
        }
        let aim = p.input.aim_angle;
        let ox = p.x + aim.cos() * 0.8;
        let oy = p.y + aim.sin() * 0.8;
        let oz = p.z + 0.9;
        let damage = weapon_damage(&p.backpack);
        self.shots.push(FlyingShot {
            proj: Projectile {
                id: Uuid::new_v4(),
                x: ox,
                y: oy,
                z: oz,
                vx: aim.cos() * PROJECTILE_SPEED,
                vy: aim.sin() * PROJECTILE_SPEED,
                vz: 0.0,
                owner_id: id,
            },
            life: PROJECTILE_LIFE,
            damage,
        });
        if let Some(p) = self.actors.get_mut(&id) {
            p.shoot_cd = SHOOT_COOLDOWN;
        }
    }

    fn tick_projectiles(&mut self) {
        struct ShotHit {
            t: f32,
            kind: HitKind,
            target: Option<Uuid>,
        }

        let actors: Vec<(Uuid, Uuid, Uuid, f32, f32, f32, Option<Uuid>)> = self
            .actors
            .values()
            .map(|a| (a.id, a.player_id, a.squad_id, a.x, a.y, a.z, a.vehicle_id))
            .collect();
        let vehicles: Vec<(Uuid, f32, f32, f32, f32, f32)> = self
            .vehicles
            .values()
            .map(|v| {
                let (radius, height) = vehicle_size(v.kind);
                (v.id, v.x, v.y, v.z, radius, height)
            })
            .collect();
        let obstacles = self.map.obstacles.clone();

        let mut hits: Vec<Option<ShotHit>> = Vec::with_capacity(self.shots.len());
        for shot in &self.shots {
            let x0 = shot.proj.x;
            let y0 = shot.proj.y;
            let z0 = shot.proj.z;
            let x1 = x0 + shot.proj.vx * DT;
            let y1 = y0 + shot.proj.vy * DT;
            let z1 = z0 + shot.proj.vz * DT;
            let owner = shot.proj.owner_id;
            let owner_player = actors.iter().find(|a| a.0 == owner).map(|a| a.1);
            let owner_squad = actors.iter().find(|a| a.0 == owner).map(|a| a.2);
            let owner_vehicle = actors.iter().find(|a| a.0 == owner).and_then(|a| a.6);

            let mut best_t = f32::MAX;
            let mut best_rank = u8::MAX;
            let mut best: Option<ShotHit> = None;

            let consider = |best_t: &mut f32,
                            best_rank: &mut u8,
                            best: &mut Option<ShotHit>,
                            t: f32,
                            rank: u8,
                            kind: HitKind,
                            target: Option<Uuid>| {
                let closer = t < *best_t - 1.0e-4;
                let tie = (t - *best_t).abs() <= 1.0e-4 && rank < *best_rank;
                if closer || tie {
                    *best_t = t;
                    *best_rank = rank;
                    *best = Some(ShotHit { t, kind, target });
                }
            };

            for obs in &obstacles {
                if let Some(t) = segment_obstacle_t(x0, y0, z0, x1, y1, z1, obs) {
                    consider(&mut best_t, &mut best_rank, &mut best, t, 2, HitKind::Wall, None);
                }
            }
            for (id, vx, vy, vz, radius, height) in &vehicles {
                if owner_vehicle == Some(*id) {
                    continue;
                }
                if let Some(t) = segment_cylinder_t(
                    x0, y0, z0, x1, y1, z1, *vx, *vy, *radius, *vz, *vz + *height,
                ) {
                    consider(
                        &mut best_t,
                        &mut best_rank,
                        &mut best,
                        t,
                        1,
                        HitKind::Vehicle,
                        Some(*id),
                    );
                }
            }
            for (id, player_id, squad_id, ax, ay, az, _) in &actors {
                if *id == owner {
                    continue;
                }
                if !FRIENDLY_FIRE
                    && (owner_player == Some(*player_id) || owner_squad == Some(*squad_id))
                {
                    continue;
                }
                if let Some(t) = segment_cylinder_t(
                    x0,
                    y0,
                    z0,
                    x1,
                    y1,
                    z1,
                    *ax,
                    *ay,
                    PLAYER_RADIUS,
                    *az,
                    *az + PLAYER_HEIGHT,
                ) {
                    consider(
                        &mut best_t,
                        &mut best_rank,
                        &mut best,
                        t,
                        0,
                        HitKind::Actor,
                        Some(*id),
                    );
                }
            }
            hits.push(best);
        }

        for (index, hit) in hits.into_iter().enumerate() {
            let Some(hit) = hit else {
                let shot = &mut self.shots[index];
                shot.life -= DT;
                shot.proj.x += shot.proj.vx * DT;
                shot.proj.y += shot.proj.vy * DT;
                shot.proj.z += shot.proj.vz * DT;
                continue;
            };
            let damage = self.shots[index].damage;
            let x0 = self.shots[index].proj.x;
            let y0 = self.shots[index].proj.y;
            let z0 = self.shots[index].proj.z;
            let x1 = x0 + self.shots[index].proj.vx * DT;
            let y1 = y0 + self.shots[index].proj.vy * DT;
            let z1 = z0 + self.shots[index].proj.vz * DT;
            let t = hit.t;
            let ix = x0 + (x1 - x0) * t;
            let iy = y0 + (y1 - y0) * t;
            let iz = z0 + (z1 - z0) * t;
            self.shots[index].life = 0.0;
            self.shots[index].proj.x = ix;
            self.shots[index].proj.y = iy;
            self.shots[index].proj.z = iz;
            self.impacts.push(Impact {
                x: ix,
                y: iy,
                z: iz,
                from_x: x0,
                from_y: y0,
                from_z: z0,
                hit_kind: hit.kind,
            });
            match hit.kind {
                HitKind::Actor => {
                    if let Some(id) = hit.target {
                        self.apply_actor_damage(id, damage);
                    }
                }
                HitKind::Vehicle => {
                    if let Some(id) = hit.target {
                        self.damage_vehicle(id, damage);
                    }
                }
                HitKind::Wall => {}
            }
        }
        self.shots.retain(|s| s.life > 0.0);
    }

    fn apply_actor_damage(&mut self, actor_id: Uuid, damage: f32) {
        let Some(p) = self.actors.get_mut(&actor_id) else {
            return;
        };
        let absorbed = damage.min(p.armor).max(0.0);
        p.armor -= absorbed;
        p.health = (p.health - (damage - absorbed)).max(0.0);
        let dead = p.health <= 0.0;
        if dead {
            self.kill_actor(actor_id);
        }
    }

    fn damage_vehicle(&mut self, id: Uuid, damage: f32) {
        let Some(v) = self.vehicles.get_mut(&id) else {
            return;
        };
        if v.hp <= 0.0 {
            return;
        }
        v.hp = (v.hp - damage).max(0.0);
        if v.hp > 0.0 {
            return;
        }
        v.speed = 0.0;
        let driver = v.driver_id.take();
        let kind = v.kind;
        let (vx, vy, vz, yaw) = (v.x, v.y, v.z, v.yaw);
        let (radius, _) = vehicle_size(kind);
        let Some(driver) = driver else {
            return;
        };
        let ex = vx + yaw.cos() * (radius + PLAYER_RADIUS + 0.35);
        let ey = vy + yaw.sin() * (radius + PLAYER_RADIUS + 0.35);
        let (ex, ey) = self.clamp_bounds(ex, ey, PLAYER_RADIUS);
        let gz = self.map.ground_z;
        let z = if blocked_by_map(
            ex,
            ey,
            PLAYER_RADIUS,
            gz,
            gz + PLAYER_HEIGHT,
            &self.map.obstacles,
        ) {
            vz
        } else {
            gz
        };
        if let Some(p) = self.actors.get_mut(&driver) {
            p.vehicle_id = None;
            p.x = ex;
            p.y = ey;
            p.z = z;
            p.vx = 0.0;
            p.vy = 0.0;
            p.vz = 0.0;
        }
        self.apply_actor_damage(driver, damage);
    }

    fn kill_actor(&mut self, actor_id: Uuid) {
        let Some(actor) = self.actors.get(&actor_id) else {
            return;
        };
        let squad_id = actor.squad_id;
        let controllers = self.controllers_of(actor_id);
        self.crew_remove(squad_id, actor_id);
        self.drop_actor(actor_id, true);
        for pid in controllers {
            if let Some(session) = self.squads.get_mut(&pid) {
                if session.controlled_actor_id == Some(actor_id) {
                    session.controlled_actor_id = None;
                }
            }
            if let Some(next) = self.claimable(squad_id, pid) {
                if let Some(session) = self.squads.get_mut(&pid) {
                    session.controlled_actor_id = Some(next);
                }
                if let Some(body) = self.actors.get_mut(&next) {
                    body.pending_inputs.clear();
                }
            }
        }
        self.finish_dead_players(squad_id);
    }

    fn finish_dead_players(&mut self, squad_id: Uuid) {
        let players: Vec<Uuid> = self
            .squads
            .iter()
            .filter(|(_, s)| s.squad_id == Some(squad_id))
            .map(|(id, _)| *id)
            .collect();
        let living = self.crews.get(&squad_id).map(|c| c.len()).unwrap_or(0);
        if living == 0 {
            for pid in players {
                self.fail_player(pid, "You died. Unextracted gear was dropped.");
            }
            self.crews.remove(&squad_id);
            return;
        }
        for pid in players {
            let controlled = self.squads.get(&pid).and_then(|s| s.controlled_actor_id);
            if controlled.is_none() && self.claimable(squad_id, pid).is_none() {
                self.fail_player(pid, "No crew left to control.");
            }
        }
    }

    fn fail_player(&mut self, player_id: Uuid, message: &str) {
        let Some(session) = self.squads.get(&player_id) else {
            return;
        };
        if session.squad_id.is_none() && session.controlled_actor_id.is_none() {
            return;
        }
        self.events.push(RaidEvent {
            player_id,
            outcome: "died".into(),
            items_banked: Vec::new(),
            cash_delta: 0,
            message: message.into(),
        });
        if let Some(session) = self.squads.get_mut(&player_id) {
            session.controlled_actor_id = None;
            session.squad_id = None;
        }
    }

    fn try_move(
        &self,
        x: f32,
        y: f32,
        nx: f32,
        ny: f32,
        radius: f32,
        z0: f32,
        z1: f32,
    ) -> (f32, f32) {
        let mut ox = nx;
        let mut oy = ny;
        if blocked_by_map(ox, oy, radius, z0, z1, &self.map.obstacles) {
            if !blocked_by_map(nx, y, radius, z0, z1, &self.map.obstacles) {
                oy = y;
                ox = nx;
            } else if !blocked_by_map(x, ny, radius, z0, z1, &self.map.obstacles) {
                ox = x;
                oy = ny;
            } else {
                ox = x;
                oy = y;
            }
        }
        (ox, oy)
    }

    fn clamp_bounds(&self, x: f32, y: f32, radius: f32) -> (f32, f32) {
        let b = &self.map.bounds;
        (
            x.clamp(b.x + radius, b.x + b.width - radius),
            y.clamp(b.y + radius, b.y + b.depth - radius),
        )
    }

    pub fn current_tick(&self) -> u64 {
        self.tick
    }

    pub fn snapshot_for(&self, observer_player_id: Uuid) -> ServerMessage {
        let session = self.squads.get(&observer_player_id);
        let squad_id = session.and_then(|s| s.squad_id);
        let controlled_actor_id = session.and_then(|s| s.controlled_actor_id);
        let observer_actor = controlled_actor_id.and_then(|id| self.actors.get(&id));
        let last_seq = observer_actor.map(|a| a.last_seq).unwrap_or(0);
        let squad_actor_ids: Vec<Uuid> = squad_id
            .and_then(|id| self.crews.get(&id))
            .cloned()
            .unwrap_or_default();

        let (ox, oy, oz) = observer_actor
            .map(|a| (a.x, a.y, a.z))
            .unwrap_or((0.0, 0.0, 0.0));
        let observing = observer_actor.is_some();

        let visible_actor = |a: &Actor| {
            if squad_actor_ids.contains(&a.id) {
                return true;
            }
            if !observing {
                return false;
            }
            in_vision_radius(ox, oy, a.x, a.y, VISION_RADIUS)
                && !sightline_blocked(ox, oy, oz, a.x, a.y, a.z, &self.map.obstacles)
        };

        let visible_point = |x: f32, y: f32, z: f32| {
            if !observing {
                return false;
            }
            in_vision_radius(ox, oy, x, y, VISION_RADIUS)
                && !sightline_blocked(ox, oy, oz, x, y, z, &self.map.obstacles)
        };

        ServerMessage::Snapshot {
            tick: self.tick,
            last_processed_input_seq: last_seq,
            squad_id,
            controlled_actor_id,
            crew: squad_id
                .map(|id| self.crew_members(id))
                .unwrap_or_default(),
            players: self
                .actors
                .values()
                .filter(|a| visible_actor(a))
                .map(|p| self.actor_state(p, p.id == controlled_actor_id.unwrap_or_default()))
                .collect(),
            vehicles: self
                .vehicles
                .values()
                .filter(|v| {
                    observer_actor
                        .and_then(|a| a.vehicle_id)
                        .map(|id| id == v.id)
                        .unwrap_or(false)
                        || visible_point(v.x, v.y, v.z)
                })
                .map(|v| VehicleState {
                    id: v.id,
                    kind: v.kind,
                    x: v.x,
                    y: v.y,
                    z: v.z,
                    yaw: v.yaw,
                    speed: v.speed,
                    hp: v.hp,
                    driver_id: v.driver_id,
                })
                .collect(),
            loot_nodes: self
                .loot
                .iter()
                .filter(|n| n.node.taken || visible_point(n.node.x, n.node.y, n.node.z))
                .map(|n| {
                    let mut node = n.node.clone();
                    node.respawn_secs = n
                        .respawn_at
                        .filter(|_| node.taken)
                        .map(|at| at.saturating_sub(self.tick) as f32 * DT)
                        .unwrap_or(0.0);
                    node
                })
                .collect(),
            death_crates: self
                .death_crates
                .iter()
                .filter(|c| visible_point(c.data.x, c.data.y, c.data.z))
                .map(|c| c.data.clone())
                .collect(),
            exfil_zones: self.map.exfil_zones.clone(),
            projectiles: self
                .shots
                .iter()
                .filter(|s| visible_point(s.proj.x, s.proj.y, s.proj.z))
                .map(|s| s.proj.clone())
                .collect(),
            impacts: self
                .impacts
                .iter()
                .filter(|i| visible_point(i.x, i.y, i.z))
                .cloned()
                .collect(),
        }
    }

    fn actor_state(&self, p: &Actor, include_backpack: bool) -> PlayerState {
        let extracting = p.extract_time > 0.0;
        let zone = self.map.exfil_zones.iter().find(|z| {
            point_in_rect(p.x, p.y, z.x, z.y, z.width, z.depth)
                && intervals_overlap(p.z, p.z + PLAYER_HEIGHT, z.z_min, z.z_max)
        });
        let progress = zone
            .map(|z| (p.extract_time / z.duration_secs.max(0.1)).clamp(0.0, 1.0))
            .unwrap_or(0.0);
        PlayerState {
            id: p.id,
            player_id: p.player_id,
            squad_id: p.squad_id,
            name: p.name.clone(),
            role: p.role.clone(),
            is_ai: !self.is_human_controlled(p.id),
            x: p.x,
            y: p.y,
            z: p.z,
            yaw: p.yaw,
            vx: p.vx,
            vy: p.vy,
            vz: p.vz,
            health: p.health,
            armor: p.armor,
            backpack: if include_backpack {
                p.backpack.clone()
            } else {
                Vec::new()
            },
            vehicle_id: p.vehicle_id,
            extracting,
            extract_progress: progress,
            last_processed_input_seq: p.last_seq,
        }
    }

    fn crew_members(&self, squad_id: Uuid) -> Vec<CrewMember> {
        let Some(ids) = self.crews.get(&squad_id) else {
            return Vec::new();
        };
        ids.iter()
            .filter_map(|id| self.actors.get(id))
            .map(|a| {
                let human = self.is_human_controlled(a.id);
                let (ox, oy, oz) = a.order.point().unwrap_or((0.0, 0.0, 0.0));
                CrewMember {
                    actor_id: a.id,
                    name: a.name.clone(),
                    role: a.role.clone(),
                    hp: a.health,
                    max_hp: a.max_hp,
                    is_ai: !human,
                    order_label: order_label(a, human),
                    order_x: ox,
                    order_y: oy,
                    order_z: oz,
                }
            })
            .collect()
    }

    fn controller_of(&self, actor_id: Uuid) -> Option<Uuid> {
        self.squads.iter().find_map(|(pid, s)| {
            if s.controlled_actor_id == Some(actor_id) {
                Some(*pid)
            } else {
                None
            }
        })
    }

    fn controllers_of(&self, actor_id: Uuid) -> Vec<Uuid> {
        self.controller_of(actor_id).into_iter().collect()
    }

    fn is_human_controlled(&self, actor_id: Uuid) -> bool {
        self.controller_of(actor_id).is_some()
    }

    fn claimable(&self, squad_id: Uuid, player_id: Uuid) -> Option<Uuid> {
        let ids = self.crews.get(&squad_id)?;
        let mut fallback = None;
        for id in ids {
            let Some(actor) = self.actors.get(id) else {
                continue;
            };
            if actor.health <= 0.0 {
                continue;
            }
            if let Some(other) = self.controller_of(*id) {
                if other != player_id {
                    continue;
                }
            }
            if actor.player_id == player_id {
                return Some(*id);
            }
            if fallback.is_none() {
                fallback = Some(*id);
            }
        }
        fallback
    }

    fn crew_remove(&mut self, squad_id: Uuid, actor_id: Uuid) {
        if let Some(ids) = self.crews.get_mut(&squad_id) {
            ids.retain(|id| *id != actor_id);
        }
    }

    fn prune_crew(&mut self, squad_id: Uuid) {
        let empty = self.crews.get(&squad_id).map(|c| c.is_empty()).unwrap_or(true);
        if empty {
            self.crews.remove(&squad_id);
            let stranded: Vec<Uuid> = self
                .squads
                .iter()
                .filter(|(_, s)| s.squad_id == Some(squad_id))
                .map(|(id, _)| *id)
                .collect();
            for pid in stranded {
                if let Some(session) = self.squads.get_mut(&pid) {
                    session.squad_id = None;
                    session.controlled_actor_id = None;
                }
            }
        }
    }

    fn spawn_anchor(&self, squad_id: Uuid) -> (f32, f32, f32) {
        if let Some(ids) = self.crews.get(&squad_id) {
            let mut n = 0.0_f32;
            let (mut x, mut y, mut z) = (0.0, 0.0, 0.0);
            for id in ids {
                if let Some(a) = self.actors.get(id) {
                    x += a.x;
                    y += a.y;
                    z += a.z;
                    n += 1.0;
                }
            }
            if n > 0.0 {
                return (x / n, y / n, z / n);
            }
        }
        (
            self.map.bounds.x + self.map.bounds.width * 0.5,
            self.map.bounds.y + self.map.bounds.depth * 0.5 + 6.0,
            self.map.ground_z,
        )
    }

    fn bot_think(&mut self, id: Uuid) -> InputState {
        let Some(actor) = self.actors.get(&id) else {
            return InputState::default();
        };
        let order = actor.order.clone();
        let squad_id = actor.squad_id;
        let owner = actor.player_id;
        let role = actor.role.clone();
        let (x, y, yaw) = (actor.x, actor.y, actor.yaw);
        let in_vehicle = actor.vehicle_id.is_some();
        let slot = self
            .crews
            .get(&squad_id)
            .and_then(|ids| ids.iter().position(|a| *a == id))
            .unwrap_or(1);

        let (mut tx, mut ty, mut tz) = match order {
            Order::Follow => {
                let lead = self.lead_pose(squad_id, id, owner);
                let (fx, fy) = follow_point(lead, slot);
                (fx, fy, lead.2)
            }
            Order::Hold { x: hx, y: hy, z: hz }
            | Order::MoveOrDriveTo { x: hx, y: hy, z: hz }
            | Order::Engage { x: hx, y: hy, z: hz } => (hx, hy, hz),
        };

        let mut aim = (ty - y).atan2(tx - x);
        let mut shoot = false;
        if let Order::Engage { .. } = order {
            if let Some(hostile) = self.best_hostile(id, &role) {
                aim = (hostile.1 - y).atan2(hostile.0 - x);
                shoot = true;
                let dist = ((hostile.0 - x).powi(2) + (hostile.1 - y).powi(2)).sqrt();
                if dist > 8.0 {
                    tx = hostile.0;
                    ty = hostile.1;
                    tz = hostile.2;
                }
            }
        } else if let Order::Hold { .. } = order {
            let dist = ((tx - x).powi(2) + (ty - y).powi(2)).sqrt();
            if dist < 0.8 {
                let lead = self.lead_pose(squad_id, id, owner);
                if lead_is_other(lead, x, y) {
                    aim = (lead.1 - y).atan2(lead.0 - x);
                } else {
                    aim = yaw;
                }
            }
        }

        let travel_far = matches!(order, Order::MoveOrDriveTo { .. });
        if in_vehicle || (travel_far && self.should_board(id, tx, ty, tz)) {
            if !in_vehicle {
                if let Some((vx, vy)) = self.board_walk_target(id, tx, ty, tz) {
                    return self.foot_toward(id, vx, vy, (vy - y).atan2(vx - x), false);
                }
            }
            let mut input = self.drive_toward(id, tx, ty, tz);
            input.aim_angle = aim;
            input.shoot = shoot && in_vehicle;
            if self.actors.get(&id).and_then(|a| a.vehicle_id).is_some() {
                input.shoot = shoot;
                return input;
            }
        }

        let hold_arrived = matches!(order, Order::Hold { .. } | Order::Engage { .. })
            && ((tx - x).powi(2) + (ty - y).powi(2)).sqrt() < 1.1
            && !shoot;
        if hold_arrived && !in_vehicle {
            return InputState {
                dt: DT,
                aim_angle: aim,
                shoot,
                ..InputState::default()
            };
        }
        if matches!(order, Order::Engage { .. }) && shoot {
            let dist_order = ((tx - x).powi(2) + (ty - y).powi(2)).sqrt();
            if dist_order < 1.1 {
                return InputState {
                    dt: DT,
                    aim_angle: aim,
                    shoot: true,
                    ..InputState::default()
                };
            }
        }
        self.foot_toward(id, tx, ty, aim, shoot)
    }

    fn should_board(&self, id: Uuid, tx: f32, ty: f32, tz: f32) -> bool {
        let Some(actor) = self.actors.get(&id) else {
            return false;
        };
        if actor.vehicle_id.is_some() {
            return false;
        }
        let dist = ((tx - actor.x).powi(2) + (ty - actor.y).powi(2)).sqrt();
        dist > 7.0 || (tz - self.map.ground_z).abs() > 3.0
    }

    /// Walk-to point for a nearby vehicle, or `None` after boarding (caller drives).
    fn board_walk_target(&mut self, id: Uuid, _tx: f32, _ty: f32, tz: f32) -> Option<(f32, f32)> {
        let Some(actor) = self.actors.get(&id) else {
            return None;
        };
        let prefer_hover = tz > self.map.ground_z + 3.0;
        let (ax, ay, az) = (actor.x, actor.y, actor.z);
        let vid = nearest_vehicle(&self.vehicles, ax, ay, prefer_hover, 8.0)?;
        let v = self.vehicles.get(&vid)?;
        let vd = ((v.x - ax).powi(2) + (v.y - ay).powi(2)).sqrt();
        let close = vd <= INTERACT_RANGE && (v.z - az).abs() <= INTERACT_Z + 1.5;
        let (vx, vy) = (v.x, v.y);
        if close {
            self.enter_vehicle(id, vid);
            None
        } else {
            Some((vx, vy))
        }
    }

    fn enter_vehicle(&mut self, actor_id: Uuid, vid: Uuid) {
        let Some(v) = self.vehicles.get(&vid) else {
            return;
        };
        if v.driver_id.is_some() || v.hp <= 0.0 {
            return;
        }
        if let Some(v) = self.vehicles.get_mut(&vid) {
            v.driver_id = Some(actor_id);
        }
        if let Some(p) = self.actors.get_mut(&actor_id) {
            p.vehicle_id = Some(vid);
        }
    }

    fn lead_pose(&self, squad_id: Uuid, self_id: Uuid, owner: Uuid) -> (f32, f32, f32, f32) {
        let pick = |cid: Uuid, actors: &HashMap<Uuid, Actor>| -> Option<(f32, f32, f32, f32)> {
            if cid == self_id {
                return None;
            }
            actors.get(&cid).map(|a| (a.x, a.y, a.z, a.yaw))
        };
        if let Some(session) = self.squads.get(&owner) {
            if let Some(cid) = session.controlled_actor_id {
                if let Some(pose) = pick(cid, &self.actors) {
                    return pose;
                }
            }
        }
        for session in self.squads.values() {
            if session.squad_id != Some(squad_id) {
                continue;
            }
            if let Some(cid) = session.controlled_actor_id {
                if let Some(pose) = pick(cid, &self.actors) {
                    return pose;
                }
            }
        }
        self.actors
            .get(&self_id)
            .map(|a| (a.x, a.y, a.z, a.yaw))
            .unwrap_or((0.0, 0.0, 0.0, 0.0))
    }

    fn best_hostile(&self, id: Uuid, role: &str) -> Option<(f32, f32, f32)> {
        let me = self.actors.get(&id)?;
        let range = if role == "Scout" { 28.0 } else { 18.0 };
        let mut best: Option<(f32, f32, f32, f32)> = None;
        for other in self.actors.values() {
            if other.id == id || other.squad_id == me.squad_id || other.health <= 0.0 {
                continue;
            }
            let dx = other.x - me.x;
            let dy = other.y - me.y;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > range {
                continue;
            }
            if sightline_blocked(me.x, me.y, me.z, other.x, other.y, other.z, &self.map.obstacles)
            {
                continue;
            }
            if best.map(|(bd, _, _, _)| dist < bd).unwrap_or(true) {
                best = Some((dist, other.x, other.y, other.z));
            }
        }
        best.map(|(_, x, y, z)| (x, y, z))
    }

    fn foot_toward(&self, id: Uuid, tx: f32, ty: f32, aim: f32, shoot: bool) -> InputState {
        let Some(actor) = self.actors.get(&id) else {
            return InputState::default();
        };
        let dist = ((tx - actor.x).powi(2) + (ty - actor.y).powi(2)).sqrt();
        if dist < 0.7 {
            return InputState {
                dt: DT,
                aim_angle: aim,
                shoot,
                ..InputState::default()
            };
        }
        let (mx, my) = self.avoid_dir(actor.x, actor.y, actor.z, tx, ty);
        InputState {
            dt: DT,
            move_x: mx,
            move_y: my,
            aim_angle: aim,
            shoot,
            ..InputState::default()
        }
    }

    fn avoid_dir(&self, x: f32, y: f32, z: f32, tx: f32, ty: f32) -> (f32, f32) {
        let base = (ty - y).atan2(tx - x);
        let z1 = z + PLAYER_HEIGHT;
        for ang in [0.0_f32, 0.55, -0.55, 1.15, -1.15, 2.2, -2.2] {
            let dir = base + ang;
            let mx = dir.cos();
            let my = dir.sin();
            let nx = x + mx * 1.3;
            let ny = y + my * 1.3;
            if !blocked_by_map(nx, ny, PLAYER_RADIUS, z, z1, &self.map.obstacles) {
                return (mx, my);
            }
        }
        (0.0, 0.0)
    }

    fn drive_toward(&self, id: Uuid, tx: f32, ty: f32, tz: f32) -> InputState {
        let Some(actor) = self.actors.get(&id) else {
            return InputState::default();
        };
        let Some(vid) = actor.vehicle_id else {
            return self.foot_toward(id, tx, ty, (ty - actor.y).atan2(tx - actor.x), false);
        };
        let Some(v) = self.vehicles.get(&vid) else {
            return InputState::default();
        };
        let dx = tx - v.x;
        let dy = ty - v.y;
        let dist = (dx * dx + dy * dy).sqrt();
        let desired = dy.atan2(dx);
        let diff = wrap_angle(desired - v.yaw);
        let steer = (diff / 0.65).clamp(-1.0, 1.0);
        let aligned = diff.abs() < 0.75;
        let mut throttle = if dist < 1.7 {
            0.0
        } else if aligned {
            1.0
        } else {
            0.3
        };
        let mut brake = dist < 2.4 && v.speed.abs() > 3.0;
        if dist < 1.5 {
            throttle = 0.0;
            brake = v.speed.abs() > 0.35;
        }
        let ascend = match v.kind {
            VehicleKind::HoverCraft => (tz - v.z).clamp(-1.0, 1.0),
            VehicleKind::GroundCar => 0.0,
        };
        InputState {
            dt: DT,
            move_x: if dist < 1.2 { 0.0 } else { steer },
            move_y: throttle,
            ascend_descend: ascend,
            aim_angle: desired,
            brake,
            ..InputState::default()
        }
    }
}

fn order_label(actor: &Actor, human: bool) -> String {
    if human {
        return String::new();
    }
    match actor.order {
        Order::Follow => "Follow".into(),
        Order::Hold { .. } => "Hold".into(),
        Order::MoveOrDriveTo { .. } => {
            if actor.vehicle_id.is_some() {
                "Drive".into()
            } else {
                "Move".into()
            }
        }
        Order::Engage { .. } => "Engage".into(),
    }
}

fn issued_sidearm() -> StashItem {
    StashItem {
        id: Uuid::new_v4(),
        item_type: "Weapon".into(),
        item_name: "Sidearm".into(),
        quantity: 1,
        stats_json: r#"{"damage":14,"cash_value":0,"issued":true}"#.into(),
    }
}

fn is_issued(item: &StashItem) -> bool {
    item.stats_json.contains("\"issued\":true")
}

fn follow_point(lead: (f32, f32, f32, f32), slot: usize) -> (f32, f32) {
    let (lx, ly, _, yaw) = lead;
    let back = 2.4 + slot as f32 * 0.35;
    let side = if slot % 2 == 0 { 1.2 } else { -1.2 };
    (
        lx - yaw.cos() * back - yaw.sin() * side,
        ly - yaw.sin() * back + yaw.cos() * side,
    )
}

fn lead_is_other(lead: (f32, f32, f32, f32), x: f32, y: f32) -> bool {
    let dx = lead.0 - x;
    let dy = lead.1 - y;
    dx * dx + dy * dy > 0.4
}

fn nearest_vehicle(
    vehicles: &HashMap<Uuid, Vehicle>,
    x: f32,
    y: f32,
    prefer_hover: bool,
    max_dist: f32,
) -> Option<Uuid> {
    let mut best: Option<(f32, Uuid)> = None;
    for v in vehicles.values() {
        if v.driver_id.is_some() || v.hp <= 0.0 {
            continue;
        }
        if prefer_hover && v.kind != VehicleKind::HoverCraft {
            continue;
        }
        let d = ((v.x - x).powi(2) + (v.y - y).powi(2)).sqrt();
        if d > max_dist {
            continue;
        }
        if best.map(|(bd, _)| d < bd).unwrap_or(true) {
            best = Some((d, v.id));
        }
    }
    if best.is_none() && prefer_hover {
        return nearest_vehicle(vehicles, x, y, false, max_dist);
    }
    best.map(|(_, id)| id)
}

fn wrap_angle(mut diff: f32) -> f32 {
    while diff > std::f32::consts::PI {
        diff -= std::f32::consts::TAU;
    }
    while diff < -std::f32::consts::PI {
        diff += std::f32::consts::TAU;
    }
    diff
}

fn vehicle_size(kind: VehicleKind) -> (f32, f32) {
    match kind {
        VehicleKind::GroundCar => (CAR_RADIUS, CAR_HEIGHT),
        VehicleKind::HoverCraft => (HOVER_RADIUS, HOVER_HEIGHT),
    }
}

fn vehicle_max_hp(kind: VehicleKind) -> f32 {
    match kind {
        VehicleKind::GroundCar => CAR_HP,
        VehicleKind::HoverCraft => HOVER_HP,
    }
}

fn interact_dist(px: f32, py: f32, pz: f32, x: f32, y: f32, z: f32) -> Option<f32> {
    let dx = x - px;
    let dy = y - py;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist <= INTERACT_RANGE && (z - pz).abs() <= INTERACT_Z {
        Some(dist)
    } else {
        None
    }
}

fn stat_f32(item: &StashItem, key: &str) -> Option<f32> {
    let v: serde_json::Value = serde_json::from_str(&item.stats_json).ok()?;
    let n = v.get(key)?;
    n.as_f64()
        .map(|x| x as f32)
        .or_else(|| n.as_i64().map(|x| x as f32))
}

fn item_weight(item: &StashItem) -> f32 {
    let w = stat_f32(item, "weight").unwrap_or(1.0).max(0.0);
    w * item.quantity.max(1) as f32
}

fn backpack_weight(items: &[StashItem]) -> f32 {
    // `Iterator::sum` for `f32` uses a -0.0 identity, which serializes as `-0.0`.
    items.iter().fold(0.0, |acc, item| acc + item_weight(item))
}

fn can_stow(pack: &[StashItem], item: &StashItem) -> bool {
    pack.len() < BACKPACK_SLOTS && backpack_weight(pack) + item_weight(item) <= BACKPACK_MAX_WEIGHT + 1.0e-3
}

fn weapon_damage(backpack: &[StashItem]) -> f32 {
    backpack
        .iter()
        .find(|i| i.item_type.eq_ignore_ascii_case("weapon"))
        .and_then(|i| stat_f32(i, "damage"))
        .filter(|d| *d > 0.0)
        .unwrap_or(18.0)
}

fn armor_from_items(items: &[StashItem]) -> f32 {
    items
        .iter()
        .filter(|i| i.item_type.eq_ignore_ascii_case("armor"))
        .filter_map(|i| stat_f32(i, "armor"))
        .fold(0.0, |acc, a| acc + a.max(0.0))
}

fn loot_respawn_ticks(tier: LootTier) -> u64 {
    let secs = match tier {
        LootTier::Common => LOOT_RESPAWN_COMMON_SECS,
        LootTier::Rare => LOOT_RESPAWN_RARE_SECS,
        LootTier::Epic => LOOT_RESPAWN_EPIC_SECS,
    };
    (secs * TICK_HZ as f32).round() as u64
}

fn loot_to_item(tier: LootTier) -> StashItem {
    let (name, value) = match tier {
        LootTier::Common => ("Street Cache", 50),
        LootTier::Rare => ("Warehouse Cache", 150),
        LootTier::Epic => ("Rooftop Cache", 400),
    };
    StashItem {
        id: Uuid::new_v4(),
        item_type: "Loot".into(),
        item_name: name.into(),
        quantity: 1,
        stats_json: format!(r#"{{"cash_value":{value},"tier":"{tier:?}"}}"#),
    }
}

fn lerp_angle(from: f32, to: f32, t: f32) -> f32 {
    let mut diff = to - from;
    while diff > std::f32::consts::PI {
        diff -= std::f32::consts::TAU;
    }
    while diff < -std::f32::consts::PI {
        diff += std::f32::consts::TAU;
    }
    from + diff * t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{LootZone, MapBounds, Obstacle};

    fn wall_map() -> MapData {
        MapData {
            name: "t".into(),
            bounds: MapBounds {
                x: 0.0,
                y: 0.0,
                width: 80.0,
                depth: 80.0,
            },
            ground_z: 0.0,
            max_altitude: 24.0,
            roads: vec![],
            obstacles: vec![Obstacle {
                id: "shed".into(),
                x: 12.0,
                y: 0.0,
                width: 2.0,
                depth: 20.0,
                z_min: 0.0,
                z_max: 3.0,
            }],
            vehicle_spawns: vec![],
            loot_zones: vec![],
            exfil_zones: vec![],
        }
    }

    fn weapon(damage: i32) -> StashItem {
        StashItem {
            id: Uuid::new_v4(),
            item_type: "Weapon".into(),
            item_name: "Street Pistol".into(),
            quantity: 1,
            stats_json: format!(r#"{{"damage":{damage}}}"#),
        }
    }

    fn filler(name: &str) -> StashItem {
        StashItem {
            id: Uuid::new_v4(),
            item_type: "Loot".into(),
            item_name: name.into(),
            quantity: 1,
            stats_json: r#"{"cash_value":10}"#.into(),
        }
    }

    fn flying(owner: Uuid, x: f32, y: f32, z: f32, damage: f32) -> FlyingShot {
        FlyingShot {
            proj: Projectile {
                id: Uuid::new_v4(),
                x,
                y,
                z,
                vx: PROJECTILE_SPEED,
                vy: 0.0,
                vz: 0.0,
                owner_id: owner,
            },
            life: PROJECTILE_LIFE,
            damage,
        }
    }

    #[test]
    fn street_shot_stops_on_shed() {
        let mut world = World::new(wall_map());
        world.shots.push(flying(Uuid::new_v4(), 10.0, 10.0, 1.0, 18.0));
        world.tick_projectiles();
        assert!(world.shots.is_empty());
        assert_eq!(world.impacts.len(), 1);
        assert_eq!(world.impacts[0].hit_kind, HitKind::Wall);
        assert!(world.impacts[0].x <= 12.05, "x={}", world.impacts[0].x);
    }

    #[test]
    fn shot_from_above_clears_low_shed() {
        let mut world = World::new(wall_map());
        world.shots.push(flying(Uuid::new_v4(), 10.0, 10.0, 6.0, 18.0));
        world.tick_projectiles();
        world.tick_projectiles();
        assert!(world.impacts.is_empty());
        assert!(world.shots[0].proj.x > 14.0);
    }

    #[test]
    fn closest_actor_wins_over_wall_behind() {
        let mut world = World::new(wall_map());
        let pid = world.register_squad("runner".into(), 1);
        let id = world.deploy(pid, vec![weapon(18)]).unwrap();
        {
            let a = world.actors.get_mut(&id).unwrap();
            a.x = 11.0;
            a.y = 10.0;
            a.z = 0.0;
        }
        world.shots.push(flying(Uuid::new_v4(), 10.0, 10.0, 1.0, 18.0));
        world.tick_projectiles();
        assert_eq!(world.impacts[0].hit_kind, HitKind::Actor);
        assert!((world.actors[&id].health - 82.0).abs() < 0.01);
    }

    #[test]
    fn same_squad_and_shooter_are_not_damaged() {
        let mut world = World::new(wall_map());
        let pid = world.register_squad("runner".into(), 1);
        let id = world.deploy(pid, vec![weapon(18)]).unwrap();
        let ally_player = world.register_squad("same".into(), 2);
        let ally = world.deploy(ally_player, vec![weapon(18)]).unwrap();
        {
            let a = world.actors.get_mut(&ally).unwrap();
            a.player_id = pid;
            a.x = 11.0;
            a.y = 10.0;
            a.z = 0.0;
            a.health = 100.0;
        }
        {
            let a = world.actors.get_mut(&id).unwrap();
            a.x = 10.0;
            a.y = 10.0;
            a.health = 40.0;
        }
        world.shots.push(flying(id, 10.0, 10.0, 1.0, 18.0));
        world.tick_projectiles();
        assert!((world.actors[&ally].health - 100.0).abs() < 0.01);
        assert!((world.actors[&id].health - 40.0).abs() < 0.01);
        assert_eq!(world.impacts[0].hit_kind, HitKind::Wall);
    }

    #[test]
    fn vehicle_destruction_ejects_driver() {
        let mut world = World::new(wall_map());
        let pid = world.register_squad("driver".into(), 1);
        let id = world.deploy(pid, vec![weapon(18)]).unwrap();
        let vid = Uuid::new_v4();
        world.vehicles.insert(
            vid,
            Vehicle {
                id: vid,
                kind: VehicleKind::GroundCar,
                x: 10.0,
                y: 10.0,
                z: 0.0,
                yaw: 0.0,
                vel_yaw: 0.0,
                speed: 0.0,
                hp: 10.0,
                driver_id: Some(id),
            },
        );
        {
            let a = world.actors.get_mut(&id).unwrap();
            a.vehicle_id = Some(vid);
            a.x = 10.0;
            a.y = 10.0;
            a.z = 0.0;
        }
        world.shots.push(flying(Uuid::new_v4(), 7.0, 10.0, 0.7, 18.0));
        world.tick_projectiles();
        assert!(world.vehicles[&vid].hp <= 0.0);
        assert!(world.vehicles[&vid].driver_id.is_none());
        let driver = &world.actors[&id];
        assert!(driver.vehicle_id.is_none());
        assert!((driver.health - 82.0).abs() < 0.01);
        assert!((driver.x - 10.0).abs() > 0.5);
        assert_eq!(world.impacts[0].hit_kind, HitKind::Vehicle);
    }

    #[test]
    fn death_drops_backpack_into_crate() {
        let mut world = World::new(wall_map());
        let pid = world.register_squad("runner".into(), 1);
        let id = world.deploy(pid, vec![weapon(18), filler("cache")]).unwrap();
        {
            let a = world.actors.get_mut(&id).unwrap();
            a.x = 11.0;
            a.y = 10.0;
            a.z = 0.0;
            a.health = 10.0;
        }
        world.shots.push(flying(Uuid::new_v4(), 10.0, 10.0, 1.0, 18.0));
        world.tick_projectiles();
        assert!(world.actors.get(&id).is_none());
        assert_eq!(world.death_crates.len(), 1);
        assert_eq!(world.death_crates[0].data.items.len(), 2);
        assert_eq!(world.events[0].outcome, "died");
        assert!(world.events[0].items_banked.is_empty());
    }

    #[test]
    fn empty_crate_despawns_and_ninth_item_stays() {
        let mut world = World::new(wall_map());
        let pid = world.register_squad("runner".into(), 1);
        let mut pack: Vec<StashItem> = (0..8).map(|i| filler(&format!("s{i}"))).collect();
        pack[0] = weapon(18);
        let id = world.deploy(pid, pack).unwrap();
        {
            let a = world.actors.get_mut(&id).unwrap();
            a.x = 20.0;
            a.y = 20.0;
            a.z = 0.0;
        }
        world.death_crates.push(TrackedCrate {
            data: DeathCrate {
                id: Uuid::new_v4(),
                x: 21.0,
                y: 20.0,
                z: 0.0,
                items: vec![filler("ninth")],
            },
            age: 0.0,
        });
        world.try_interact(id);
        assert_eq!(world.actors[&id].backpack.len(), 8);
        assert_eq!(world.death_crates.len(), 1);
        assert_eq!(world.death_crates[0].data.items.len(), 1);

        world.actors.get_mut(&id).unwrap().backpack.pop();
        world.try_interact(id);
        assert!(world.death_crates.is_empty());
        assert_eq!(world.actors[&id].backpack.len(), 8);
    }

    #[test]
    fn interact_loots_crate_before_vehicle() {
        let mut world = World::new(wall_map());
        let pid = world.register_squad("runner".into(), 1);
        let id = world.deploy(pid, vec![weapon(18)]).unwrap();
        {
            let a = world.actors.get_mut(&id).unwrap();
            a.x = 30.0;
            a.y = 30.0;
            a.z = 0.0;
        }
        let vid = Uuid::new_v4();
        world.vehicles.insert(
            vid,
            Vehicle {
                id: vid,
                kind: VehicleKind::GroundCar,
                x: 31.0,
                y: 30.0,
                z: 0.0,
                yaw: 0.0,
                vel_yaw: 0.0,
                speed: 0.0,
                hp: CAR_HP,
                driver_id: None,
            },
        );
        world.death_crates.push(TrackedCrate {
            data: DeathCrate {
                id: Uuid::new_v4(),
                x: 31.0,
                y: 30.0,
                z: 0.0,
                items: vec![filler("drop")],
            },
            age: 0.0,
        });
        world.try_interact(id);
        assert_eq!(world.actors[&id].backpack.len(), 2);
        assert!(world.actors[&id].vehicle_id.is_none());
        assert!(world.vehicles[&vid].driver_id.is_none());
        assert!(world.death_crates.is_empty());
    }

    #[test]
    fn crate_ttl_despawns() {
        let mut world = World::new(wall_map());
        world.death_crates.push(TrackedCrate {
            data: DeathCrate {
                id: Uuid::new_v4(),
                x: 5.0,
                y: 5.0,
                z: 0.0,
                items: vec![filler("old")],
            },
            age: DEATH_CRATE_TTL_SECS,
        });
        world.tick();
        assert!(world.death_crates.is_empty());
    }

    #[test]
    fn common_loot_returns_after_cooldown_with_nobody_deployed() {
        let mut map = wall_map();
        map.loot_zones.push(LootZone {
            id: "cache".into(),
            x: 20.0,
            y: 20.0,
            width: 2.0,
            depth: 2.0,
            z_min: 0.0,
            z_max: 1.5,
            tier: LootTier::Common,
        });
        let mut world = World::new(map);
        let node_id = world.loot[0].node.id;
        let pid = world.register_squad("runner".into(), 1);
        let id = world.deploy(pid, Vec::new()).unwrap();
        {
            let a = world.actors.get_mut(&id).unwrap();
            a.x = 21.0;
            a.y = 21.0;
            a.z = 0.0;
        }
        world.try_interact(id);
        assert!(world.loot[0].node.taken);
        world.remove_session(pid);
        assert!(world.actors.is_empty());

        let wait = loot_respawn_ticks(LootTier::Common);
        for _ in 0..wait - 1 {
            world.tick();
        }
        assert!(world.loot[0].node.taken);
        assert_eq!(world.loot[0].node.id, node_id);
        world.tick();
        assert!(!world.loot[0].node.taken);
        assert!(world.loot[0].respawn_at.is_none());
        assert_eq!(world.loot[0].node.id, node_id);
    }

    #[test]
    fn armor_absorbs_before_health() {
        let mut world = World::new(wall_map());
        let pid = world.register_squad("runner".into(), 1);
        let id = world.deploy(pid, vec![weapon(30)]).unwrap();
        assert!(!world.actors[&id].armor.is_sign_negative());
        {
            let a = world.actors.get_mut(&id).unwrap();
            a.armor = 10.0;
            a.health = 100.0;
        }
        world.apply_actor_damage(id, 18.0);
        let a = &world.actors[&id];
        assert!((a.armor - 0.0).abs() < 0.01);
        assert!((a.health - 92.0).abs() < 0.01);
        assert!((weapon_damage(&a.backpack) - 30.0).abs() < 0.01);
    }

    fn crew(name: &str, role: &str, hp: f32) -> CrewSpawn {
        CrewSpawn {
            specialist_id: Uuid::new_v4(),
            name: name.into(),
            role: role.into(),
            max_hp: hp,
        }
    }

    #[test]
    fn deploy_spawns_selected_crew_and_swap_rebinds() {
        let mut world = World::new(wall_map());
        let pid = world.register_squad("lead".into(), 1);
        let controlled = world
            .deploy_crew(
                pid,
                vec![weapon(18)],
                vec![
                    crew("Vesper", "Point", 100.0),
                    crew("Rook", "Driver", 100.0),
                    crew("Iris", "Scout", 90.0),
                ],
                None,
            )
            .unwrap();
        let squad_id = world.squads[&pid].squad_id.unwrap();
        assert_eq!(world.crews[&squad_id].len(), 3);
        assert_eq!(world.squads[&pid].controlled_actor_id, Some(controlled));
        let ai = world.crews[&squad_id][1];
        assert!(world.swap_actor(pid, ai).is_ok());
        assert_eq!(world.squads[&pid].controlled_actor_id, Some(ai));
        assert!(matches!(world.actors[&controlled].order, Order::Hold { .. }));
        let other = world.register_squad("rival".into(), 2);
        let foreign = world.deploy(other, vec![weapon(18)]).unwrap();
        assert!(world.swap_actor(pid, foreign).is_err());
        world.actors.get_mut(&controlled).unwrap().health = 0.0;
        assert!(world.swap_actor(pid, controlled).is_err());
    }

    #[test]
    fn coop_cannot_take_a_controlled_actor() {
        let mut world = World::new(wall_map());
        let a = world.register_squad("a".into(), 1);
        let lead = world
            .deploy_crew(a, vec![], vec![crew("Vesper", "Point", 100.0)], None)
            .unwrap();
        let squad_id = world.squads[&a].squad_id.unwrap();
        let b = world.register_squad("b".into(), 2);
        let second = world
            .deploy_crew(
                b,
                vec![],
                vec![crew("Rook", "Driver", 100.0)],
                Some(squad_id),
            )
            .unwrap();
        assert_ne!(lead, second);
        assert_eq!(world.crews[&squad_id].len(), 2);
        assert!(world.swap_actor(b, lead).is_err());
        assert!(world.swap_actor(a, second).is_err());
    }

    #[test]
    fn follow_order_closes_on_the_lead() {
        let mut world = World::new(wall_map());
        let pid = world.register_squad("lead".into(), 1);
        let lead = world
            .deploy_crew(
                pid,
                vec![],
                vec![crew("Vesper", "Point", 100.0), crew("Rook", "Driver", 100.0)],
                None,
            )
            .unwrap();
        let ai = world.crews[&world.squads[&pid].squad_id.unwrap()][1];
        world.actors.get_mut(&lead).unwrap().x = 36.0;
        world.actors.get_mut(&lead).unwrap().y = 40.0;
        world.actors.get_mut(&ai).unwrap().x = 20.0;
        world.actors.get_mut(&ai).unwrap().y = 40.0;
        let start = world.actors[&ai].x;
        for _ in 0..30 {
            world.tick();
        }
        assert!(
            world.actors[&ai].x > start + 3.0,
            "ai x {}",
            world.actors[&ai].x
        );
    }

    #[test]
    fn engage_shoots_hostile_not_squadmate() {
        let mut world = World::new(wall_map());
        let pid = world.register_squad("lead".into(), 1);
        let _lead = world
            .deploy_crew(
                pid,
                vec![weapon(18)],
                vec![crew("Vesper", "Point", 100.0), crew("Iris", "Scout", 90.0)],
                None,
            )
            .unwrap();
        let squad_id = world.squads[&pid].squad_id.unwrap();
        let ai = world.crews[&squad_id][1];
        world.actors.get_mut(&ai).unwrap().x = 20.0;
        world.actors.get_mut(&ai).unwrap().y = 40.0;
        world.actors.get_mut(&ai).unwrap().order = Order::Engage {
            x: 20.0,
            y: 40.0,
            z: 0.0,
        };
        let ally = world.crews[&squad_id][0];
        world.actors.get_mut(&ally).unwrap().x = 21.5;
        world.actors.get_mut(&ally).unwrap().y = 40.0;
        let foe_player = world.register_squad("foe".into(), 9);
        let foe = world.deploy(foe_player, vec![weapon(18)]).unwrap();
        world.actors.get_mut(&foe).unwrap().x = 26.0;
        world.actors.get_mut(&foe).unwrap().y = 40.0;
        let ally_hp = world.actors[&ally].health;
        let foe_hp = world.actors[&foe].health;
        for _ in 0..12 {
            world.tick();
        }
        assert!((world.actors[&ally].health - ally_hp).abs() < 0.01);
        assert!(
            world.actors[&foe].health < foe_hp - 1.0,
            "foe hp {}",
            world.actors[&foe].health
        );
    }
}
