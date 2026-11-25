use std::collections::HashMap;

use uuid::Uuid;

use crate::collision::{blocked_by_map, intervals_overlap, los_blocked, point_in_rect};
use crate::protocol::{
    ExfilZone, LootNode, MapData, PlayerState, Projectile, ServerMessage, VehicleKind, VehicleState,
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

#[derive(Clone, Debug, Default)]
pub struct InputState {
    pub seq: u32,
    pub move_x: f32,
    pub move_y: f32,
    pub ascend_descend: f32,
    pub aim_angle: f32,
    pub shoot: bool,
    pub interact: bool,
    pub brake: bool,
}

struct Player {
    id: Uuid,
    name: String,
    x: f32,
    y: f32,
    z: f32,
    yaw: f32,
    health: f32,
    vehicle_id: Option<Uuid>,
    extract_time: f32,
    input: InputState,
    shoot_cd: f32,
    interact_edge: bool,
    last_seq: u32,
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

pub struct World {
    pub map: MapData,
    tick: u64,
    players: HashMap<Uuid, Player>,
    vehicles: HashMap<Uuid, Vehicle>,
    loot: Vec<LootNode>,
    shots: Vec<FlyingShot>,
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
            players: HashMap::new(),
            vehicles,
            loot,
            shots: Vec::new(),
        }
    }

    pub fn spawn_player(&mut self, name: String) -> Uuid {
        let id = Uuid::new_v4();
        let spawn_x = self.map.bounds.x + self.map.bounds.width * 0.5;
        let spawn_y = self.map.bounds.y + self.map.bounds.depth * 0.5 + 6.0;
        self.players.insert(
            id,
            Player {
                id,
                name,
                x: spawn_x,
                y: spawn_y,
                z: self.map.ground_z,
                yaw: 0.0,
                health: 100.0,
                vehicle_id: None,
                extract_time: 0.0,
                input: InputState::default(),
                shoot_cd: 0.0,
                interact_edge: false,
                last_seq: 0,
            },
        );
        id
    }

    pub fn remove_player(&mut self, id: Uuid) {
        if let Some(p) = self.players.remove(&id) {
            if let Some(vid) = p.vehicle_id {
                if let Some(v) = self.vehicles.get_mut(&vid) {
                    v.driver_id = None;
                }
            }
        }
    }

    pub fn set_input(&mut self, id: Uuid, input: InputState) {
        if let Some(p) = self.players.get_mut(&id) {
            p.last_seq = input.seq;
            p.input = input;
        }
    }

    pub fn tick(&mut self) {
        self.tick += 1;
        let ids: Vec<Uuid> = self.players.keys().copied().collect();
        for id in ids {
            self.tick_player(id);
        }
        self.tick_projectiles();
    }

    fn tick_player(&mut self, id: Uuid) {
        let Some(player) = self.players.get(&id) else {
            return;
        };
        let interact = player.input.interact && !player.interact_edge;
        let vehicle_id = player.vehicle_id;
        let input = player.input.clone();

        if let Some(vid) = vehicle_id {
            self.drive_vehicle(id, vid, &input);
            self.follow_vehicle(id, vid);
        } else {
            self.walk_player(id, &input);
        }

        if interact {
            self.try_interact(id);
        }
        if let Some(p) = self.players.get_mut(&id) {
            p.interact_edge = input.interact;
            p.yaw = input.aim_angle;
            p.shoot_cd = (p.shoot_cd - DT).max(0.0);
        }
        self.update_extraction(id);
        self.try_shoot(id);
    }

    fn walk_player(&mut self, id: Uuid, input: &InputState) {
        let Some(p) = self.players.get(&id) else {
            return;
        };
        let mut mx = input.move_x;
        let mut my = input.move_y;
        let mag = (mx * mx + my * my).sqrt();
        if mag > 1.0 {
            mx /= mag;
            my /= mag;
        }
        let nx = p.x + mx * PLAYER_SPEED * DT;
        let ny = p.y + my * PLAYER_SPEED * DT;
        let z = self.map.ground_z;
        let z1 = z + PLAYER_HEIGHT;
        let (x, y) = self.try_move(p.x, p.y, nx, ny, PLAYER_RADIUS, z, z1);
        let (x, y) = self.clamp_bounds(x, y, PLAYER_RADIUS);
        if let Some(p) = self.players.get_mut(&id) {
            p.x = x;
            p.y = y;
            p.z = z;
        }
    }

    fn drive_vehicle(&mut self, player_id: Uuid, vid: Uuid, input: &InputState) {
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
        speed += throttle * accel * DT;
        if input.brake {
            speed *= 0.82;
        }
        speed = speed.clamp(-max_speed * 0.4, max_speed);

        let mut yaw = v.yaw;
        yaw += steer * turn * DT * (speed.abs() / max_speed).max(0.25);
        let mut vel_yaw = v.vel_yaw;
        let grip = match kind {
            VehicleKind::GroundCar => CAR_GRIP,
            VehicleKind::HoverCraft => 10.0,
        };
        vel_yaw = lerp_angle(vel_yaw, yaw, (grip * DT).min(1.0));

        let nx = v.x + vel_yaw.cos() * speed * DT;
        let ny = v.y + vel_yaw.sin() * speed * DT;

        let mut z = v.z;
        match kind {
            VehicleKind::GroundCar => z = self.map.ground_z,
            VehicleKind::HoverCraft => {
                z += input.ascend_descend.clamp(-1.0, 1.0) * HOVER_CLIMB * DT;
                z = z.clamp(self.map.ground_z, self.map.max_altitude);
            }
        }

        let z1 = z + height;
        let (x, y) = self.try_move(v.x, v.y, nx, ny, radius, z, z1);
        let (x, y) = self.clamp_bounds(x, y, radius);

        // Vertical move: if the new altitude collides, keep previous z.
        let z = if blocked_by_map(x, y, radius, z, z1, &self.map.obstacles) {
            v.z
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
            v.driver_id = Some(player_id);
        }
    }

    fn follow_vehicle(&mut self, player_id: Uuid, vid: Uuid) {
        let Some(v) = self.vehicles.get(&vid) else {
            return;
        };
        let (x, y, z, yaw) = (v.x, v.y, v.z, v.yaw);
        if let Some(p) = self.players.get_mut(&player_id) {
            p.x = x;
            p.y = y;
            p.z = z;
            p.yaw = yaw;
        }
    }

    fn try_interact(&mut self, id: Uuid) {
        let Some(p) = self.players.get(&id) else {
            return;
        };
        if let Some(vid) = p.vehicle_id {
            if let Some(v) = self.vehicles.get_mut(&vid) {
                v.driver_id = None;
            }
            if let Some(p) = self.players.get_mut(&id) {
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
            if let Some(p) = self.players.get_mut(&id) {
                p.vehicle_id = Some(vid);
            }
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
                break;
            }
        }
    }

    fn update_extraction(&mut self, id: Uuid) {
        let Some(p) = self.players.get(&id) else {
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
            if let Some(p) = self.players.get_mut(&id) {
                p.extract_time += DT;
                if p.extract_time >= dur {
                    p.extract_time = dur;
                    p.health = 100.0;
                }
            }
        } else if let Some(p) = self.players.get_mut(&id) {
            p.extract_time = 0.0;
        }
    }

    fn try_shoot(&mut self, id: Uuid) {
        let Some(p) = self.players.get(&id) else {
            return;
        };
        if !p.input.shoot || p.shoot_cd > 0.0 {
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
            // still spawn; projectile dies on first obstacle in flight
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
        if let Some(p) = self.players.get_mut(&id) {
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

        let player_snapshot: Vec<(Uuid, f32, f32, f32)> = self
            .players
            .values()
            .map(|p| (p.id, p.x, p.y, p.z))
            .collect();
        for shot in &self.shots {
            for (pid, x, y, z) in &player_snapshot {
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
        for (shot_id, pid) in hits {
            self.shots.retain(|s| s.proj.id != shot_id);
            if let Some(p) = self.players.get_mut(&pid) {
                p.health = (p.health - 18.0).max(0.0);
                if p.health <= 0.0 {
                    p.health = 100.0;
                    p.x = self.map.bounds.x + self.map.bounds.width * 0.5;
                    p.y = self.map.bounds.y + 8.0;
                    p.z = self.map.ground_z;
                    if let Some(vid) = p.vehicle_id.take() {
                        if let Some(v) = self.vehicles.get_mut(&vid) {
                            v.driver_id = None;
                        }
                    }
                }
            }
        }
        self.shots.retain(|s| s.life > 0.0);
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

    pub fn snapshot(&self) -> ServerMessage {
        ServerMessage::Snapshot {
            tick: self.tick,
            players: self
                .players
                .values()
                .map(|p| {
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
                        name: p.name.clone(),
                        x: p.x,
                        y: p.y,
                        z: p.z,
                        yaw: p.yaw,
                        health: p.health,
                        vehicle_id: p.vehicle_id,
                        extracting,
                        extract_progress: progress,
                    }
                })
                .collect(),
            vehicles: self
                .vehicles
                .values()
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
            loot_nodes: self.loot.clone(),
            exfil_zones: self.map.exfil_zones.clone(),
            projectiles: self.shots.iter().map(|s| s.proj.clone()).collect(),
        }
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
