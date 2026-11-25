use std::collections::{HashMap, VecDeque};

use uuid::Uuid;

use crate::collision::{
    blocked_by_map, in_vision_radius, intervals_overlap, point_in_rect, segment_cylinder_t,
    segment_obstacle_t, sightline_blocked,
};
use crate::protocol::{
    DeathCrate, ExfilZone, HitKind, Impact, LootNode, LootTier, MapData, PlayerState, Projectile,
    ServerMessage, StashItem, VehicleKind, VehicleState,
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

pub struct World {
    pub map: MapData,
    tick: u64,
    squads: HashMap<Uuid, Squad>,
    actors: HashMap<Uuid, Actor>,
    vehicles: HashMap<Uuid, Vehicle>,
    loot: Vec<LootNode>,
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
                armor: armor_from_items(&loadout),
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
        self.impacts.clear();
        self.tick_crates();
        let ids: Vec<Uuid> = self.actors.keys().copied().collect();
        for id in ids {
            self.tick_actor(id);
        }
        self.tick_projectiles();
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
            if node.taken {
                continue;
            }
            if let Some(dist) = interact_dist(px, py, pz, node.x, node.y, node.z) {
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
                if node.taken {
                    return;
                }
                loot_to_item(node.tier)
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
                    node.taken = true;
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

        let actors: Vec<(Uuid, Uuid, f32, f32, f32, Option<Uuid>)> = self
            .actors
            .values()
            .map(|a| (a.id, a.player_id, a.x, a.y, a.z, a.vehicle_id))
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
            let owner_vehicle = actors.iter().find(|a| a.0 == owner).and_then(|a| a.5);

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
            for (id, player_id, ax, ay, az, _) in &actors {
                if *id == owner {
                    continue;
                }
                if !FRIENDLY_FIRE && owner_player == Some(*player_id) {
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
                .map(|p| self.actor_state(p, squad_actor_ids.contains(&p.id)))
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
                .filter(|n| !n.taken && visible_point(n.x, n.y, n.z))
                .cloned()
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
            name: p.name.clone(),
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
    use crate::protocol::{MapBounds, Obstacle};

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
}
