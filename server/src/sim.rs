use std::collections::{HashMap, VecDeque};

use uuid::Uuid;

use crate::collision::{
    blocked_by_map, in_vision_radius, intervals_overlap, los_blocked, point_in_rect, sightline_blocked,
};
use crate::protocol::{
    DeathCrate, ExfilZone, LootNode, LootTier, MapData, PlayerState, Projectile, ServerMessage,
    StashItem, VehicleKind, VehicleState,
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
const PROJECTILE_RADIUS: f32 = 0.12;
const SHOOT_COOLDOWN: f32 = 0.22;
const INTERACT_RANGE: f32 = 2.4;
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
/// Milestone 3 can swap the controlled actor without changing the player session.
pub struct Squad {
    #[allow(dead_code)]
    pub player_id: Uuid,
    pub controlled_actor_id: Option<Uuid>,
    pub name: String,
    pub profile_id: i64,
}

struct Actor {
    id: Uuid,
    player_id: Uuid,
    name: String,
    x: f32,
    y: f32,
    z: f32,
    yaw: f32,
    vx: f32,
    vy: f32,
    vz: f32,
    health: f32,
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
    driver_id: Option<Uuid>,
}

struct FlyingShot {
    proj: Projectile,
    life: f32,
}

pub struct RaidEvent {
    pub player_id: Uuid,
    pub outcome: String,
    pub items_banked: Vec<StashItem>,
    pub cash_delta: i32,
    pub message: String,
}

pub struct World {
    pub map: MapData,
    tick: u64,
    squads: HashMap<Uuid, Squad>,
    actors: HashMap<Uuid, Actor>,
    vehicles: HashMap<Uuid, Vehicle>,
    loot: Vec<LootNode>,
    death_crates: Vec<DeathCrate>,
    shots: Vec<FlyingShot>,
    events: Vec<RaidEvent>,
}

impl World {
    pub fn new(map: MapData) -> Self {
        let loot = map
            .loot_zones
            .iter()
            .map(|z| LootNode {
                id: Uuid::new_v4(),
                x: z.x + z.width * 0.5,
                y: z.y + z.depth * 0.5,
                z: z.z_min,
                tier: z.tier,
                taken: false,
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
                        driver_id: None,
                    },
                )
            })
            .collect();
        Self {
            map,
            tick: 0,
            squads: HashMap::new(),
            actors: HashMap::new(),
            vehicles,
            loot,
            death_crates: Vec::new(),
            shots: Vec::new(),
            events: Vec::new(),
        }
    }

    pub fn register_squad(&mut self, name: String, profile_id: i64) -> Uuid {
        let player_id = Uuid::new_v4();
        self.squads.insert(
            player_id,
            Squad {
                player_id,
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
        if let Some(squad) = self.squads.remove(&player_id) {
            if let Some(aid) = squad.controlled_actor_id {
                self.drop_actor(aid, true);
            }
        }
    }

    pub fn deploy(&mut self, player_id: Uuid, loadout: Vec<StashItem>) -> Result<Uuid, String> {
        let squad = self
            .squads
            .get(&player_id)
            .ok_or_else(|| "unknown player".to_string())?;
        if squad.controlled_actor_id.is_some() {
            return Err("already in raid".into());
        }
        let name = squad.name.clone();
        let actor_id = Uuid::new_v4();
        let spawn_x = self.map.bounds.x + self.map.bounds.width * 0.5;
        let spawn_y = self.map.bounds.y + self.map.bounds.depth * 0.5 + 6.0;
        self.actors.insert(
            actor_id,
            Actor {
                id: actor_id,
                player_id,
                name,
                x: spawn_x,
                y: spawn_y,
                z: self.map.ground_z,
                yaw: 0.0,
                vx: 0.0,
                vy: 0.0,
                vz: 0.0,
                health: 100.0,
                vehicle_id: None,
                extract_time: 0.0,
                input: InputState::default(),
                pending_inputs: VecDeque::new(),
                shoot_cd: 0.0,
                interact_edge: false,
                last_seq: 0,
                backpack: loadout,
            },
        );
        if let Some(squad) = self.squads.get_mut(&player_id) {
            squad.controlled_actor_id = Some(actor_id);
        }
        Ok(actor_id)
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
        let ids: Vec<Uuid> = self.actors.keys().copied().collect();
        for id in ids {
            self.tick_actor(id);
        }
        self.tick_projectiles();
    }

    pub fn drain_events(&mut self) -> Vec<RaidEvent> {
        std::mem::take(&mut self.events)
    }

    fn tick_actor(&mut self, id: Uuid) {
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
        if let Some(vid) = p.vehicle_id {
            if let Some(v) = self.vehicles.get_mut(&vid) {
                v.driver_id = None;
            }
            if let Some(p) = self.actors.get_mut(&id) {
                p.vehicle_id = None;
                p.z = self.map.ground_z;
            }
            return;
        }

        let (px, py, pz) = (p.x, p.y, p.z);
        let mut nearest: Option<Uuid> = None;
        let mut best = INTERACT_RANGE;
        for v in self.vehicles.values() {
            if v.driver_id.is_some() {
                continue;
            }
            let dx = v.x - px;
            let dy = v.y - py;
            let dist = (dx * dx + dy * dy).sqrt();
            let z_ok = (v.z - pz).abs() < 2.5;
            if dist < best && z_ok {
                best = dist;
                nearest = Some(v.id);
            }
        }
        if let Some(vid) = nearest {
            if let Some(v) = self.vehicles.get_mut(&vid) {
                v.driver_id = Some(id);
            }
            if let Some(p) = self.actors.get_mut(&id) {
                p.vehicle_id = Some(vid);
            }
            return;
        }

        if let Some(crate_idx) = self.death_crates.iter().position(|c| {
            let dx = c.x - px;
            let dy = c.y - py;
            (dx * dx + dy * dy).sqrt() < INTERACT_RANGE && (c.z - pz).abs() < 2.0
        }) {
            let mut crate_items = std::mem::take(&mut self.death_crates[crate_idx].items);
            if let Some(p) = self.actors.get_mut(&id) {
                p.backpack.append(&mut crate_items);
            }
            self.death_crates.remove(crate_idx);
            return;
        }

        for node in &mut self.loot {
            if node.taken {
                continue;
            }
            let dx = node.x - px;
            let dy = node.y - py;
            if (dx * dx + dy * dy).sqrt() < INTERACT_RANGE && (node.z - pz).abs() < 2.0 {
                node.taken = true;
                let item = loot_to_item(node.tier);
                if let Some(p) = self.actors.get_mut(&id) {
                    p.backpack.push(item);
                }
                break;
            }
        }
    }

    fn update_extraction(&mut self, id: Uuid, dt: f32) {
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
        let player_id = actor.player_id;
        let items = actor.backpack.clone();
        let cash_delta: i32 = items.iter().map(crate::db::loot_cash_value).sum();
        self.events.push(RaidEvent {
            player_id,
            outcome: "extracted".into(),
            items_banked: items,
            cash_delta,
            message: "You extracted. Gear is in the stash.".into(),
        });
        self.drop_actor(actor_id, false);
        if let Some(squad) = self.squads.get_mut(&player_id) {
            squad.controlled_actor_id = None;
        }
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
            self.death_crates.push(DeathCrate {
                id: Uuid::new_v4(),
                x: actor.x,
                y: actor.y,
                z: actor.z,
                items: actor.backpack,
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
        let tx = p.x + aim.cos() * 40.0;
        let ty = p.y + aim.sin() * 40.0;
        if los_blocked(
            ox,
            oy,
            tx,
            ty,
            oz - 0.2,
            oz + 0.2,
            &self.map.obstacles,
        ) {
            // projectile still spawned; dies on first obstacle in flight
        }
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
        });
        if let Some(p) = self.actors.get_mut(&id) {
            p.shoot_cd = SHOOT_COOLDOWN;
        }
    }

    fn tick_projectiles(&mut self) {
        let obstacles = &self.map.obstacles;
        let mut hits: Vec<(Uuid, Uuid)> = Vec::new();
        for shot in &mut self.shots {
            shot.life -= DT;
            let nx = shot.proj.x + shot.proj.vx * DT;
            let ny = shot.proj.y + shot.proj.vy * DT;
            let nz = shot.proj.z + shot.proj.vz * DT;
            if blocked_by_map(
                nx,
                ny,
                PROJECTILE_RADIUS,
                nz - 0.1,
                nz + 0.1,
                obstacles,
            ) {
                shot.life = 0.0;
                continue;
            }
            shot.proj.x = nx;
            shot.proj.y = ny;
            shot.proj.z = nz;
        }

        let actor_snapshot: Vec<(Uuid, f32, f32, f32)> = self
            .actors
            .values()
            .map(|p| (p.id, p.x, p.y, p.z))
            .collect();
        for shot in &self.shots {
            for (pid, x, y, z) in &actor_snapshot {
                if *pid == shot.proj.owner_id {
                    continue;
                }
                let dx = shot.proj.x - x;
                let dy = shot.proj.y - y;
                let dz = shot.proj.z - (*z + 0.9);
                if dx * dx + dy * dy < 0.55 * 0.55 && dz.abs() < 1.0 {
                    hits.push((shot.proj.id, *pid));
                }
            }
        }
        let mut deaths = Vec::new();
        for (shot_id, pid) in hits {
            self.shots.retain(|s| s.proj.id != shot_id);
            if let Some(p) = self.actors.get_mut(&pid) {
                p.health = (p.health - 18.0).max(0.0);
                if p.health <= 0.0 {
                    deaths.push(pid);
                }
            }
        }
        for pid in deaths {
            self.kill_actor(pid);
        }
        self.shots.retain(|s| s.life > 0.0);
    }

    fn kill_actor(&mut self, actor_id: Uuid) {
        let Some(actor) = self.actors.get(&actor_id) else {
            return;
        };
        let player_id = actor.player_id;
        self.events.push(RaidEvent {
            player_id,
            outcome: "died".into(),
            items_banked: Vec::new(),
            cash_delta: 0,
            message: "You died. Unextracted gear was dropped.".into(),
        });
        self.drop_actor(actor_id, true);
        if let Some(squad) = self.squads.get_mut(&player_id) {
            squad.controlled_actor_id = None;
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
        let observer_actor = self
            .squads
            .get(&observer_player_id)
            .and_then(|s| s.controlled_actor_id)
            .and_then(|id| self.actors.get(&id));
        let last_seq = observer_actor.map(|a| a.last_seq).unwrap_or(0);
        let squad_actor_ids: Vec<Uuid> = self
            .squads
            .get(&observer_player_id)
            .and_then(|s| s.controlled_actor_id)
            .into_iter()
            .collect();

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
            players: self
                .actors
                .values()
                .filter(|a| visible_actor(a))
                .map(|p| self.actor_state(p))
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
                    driver_id: v.driver_id,
                })
                .collect(),
            loot_nodes: self
                .loot
                .iter()
                .filter(|n| !n.taken && visible_point(n.x, n.y, n.z))
                .cloned()
                .collect(),
            death_crates: self
                .death_crates
                .iter()
                .filter(|c| visible_point(c.x, c.y, c.z))
                .cloned()
                .collect(),
            exfil_zones: self.map.exfil_zones.clone(),
            projectiles: self
                .shots
                .iter()
                .filter(|s| visible_point(s.proj.x, s.proj.y, s.proj.z))
                .map(|s| s.proj.clone())
                .collect(),
        }
    }

    fn actor_state(&self, p: &Actor) -> PlayerState {
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
            name: p.name.clone(),
            x: p.x,
            y: p.y,
            z: p.z,
            yaw: p.yaw,
            vx: p.vx,
            vy: p.vy,
            vz: p.vz,
            health: p.health,
            vehicle_id: p.vehicle_id,
            extracting,
            extract_progress: progress,
            last_processed_input_seq: p.last_seq,
        }
    }
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
