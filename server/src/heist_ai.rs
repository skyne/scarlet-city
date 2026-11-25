use uuid::Uuid;

use crate::collision::{blocked_by_map, sightline_blocked};
use crate::protocol::{AlarmPing, HeistMarker, HeistSite, HeistState, LootTier, StashItem, VehicleKind};

use super::{
    default_appearance, Actor, CityNotice, InputState, LiveAlarm, NpcKind, Vehicle, World,
    APPROACH_RADIUS, DT, HEAT_DECAY_PER_SEC, HEAT_MAX, INTERACT_Z, PLAYER_HEIGHT, PLAYER_RADIUS,
};

const LAW_SQUAD: Uuid = Uuid::nil();

impl World {
    pub(super) fn seed_guards(&mut self) {
        let sites: Vec<HeistSite> = self.heists.iter().map(|job| job.site.clone()).collect();
        let ground = self.map.ground_z;
        for site in sites {
            let spots = [
                (site.loud_x + 2.3, site.loud_y + 0.6),
                (site.loud_x - 2.5, site.loud_y - 1.4),
            ];
            for (i, (x, y)) in spots.into_iter().enumerate() {
                let (x, y) = self.clamp_bounds(x, y, PLAYER_RADIUS);
                if blocked_by_map(
                    x,
                    y,
                    PLAYER_RADIUS,
                    ground,
                    ground + PLAYER_HEIGHT,
                    &self.map.obstacles,
                ) {
                    continue;
                }
                let id = self.spawn_law(NpcKind::Guard, x, y, ground, None, false, x, y, ground);
                if let Some(actor) = self.actors.get_mut(&id) {
                    actor.site_id = site.id.clone();
                    actor.home_x = x;
                    actor.home_y = y;
                    actor.home_z = ground;
                    actor.patrol = i as f32 * 1.7;
                    actor.alert = false;
                    actor.name = format!("Guard {}", i + 1);
                }
            }
        }
    }

    pub(super) fn begin_heist_tick(&mut self) {
        for job in &mut self.heists {
            job.worked = false;
        }
    }

    /// Hold-to-hack / hold-to-breach. Returns true when this interact belongs to a heist approach.
    pub(super) fn advance_heist(&mut self, id: Uuid, dt: f32) -> bool {
        let Some(actor) = self.actors.get(&id) else {
            return false;
        };
        if actor.npc.is_law() {
            return false;
        }
        let squad = actor.squad_id;
        let (px, py, pz) = if let Some(vid) = actor.vehicle_id {
            let Some(vehicle) = self.vehicles.get(&vid) else {
                return false;
            };
            if vehicle.kind != VehicleKind::HoverCraft || vehicle.speed.abs() > 1.6 {
                return false;
            }
            (vehicle.x, vehicle.y, vehicle.z)
        } else {
            (actor.x, actor.y, actor.z)
        };

        let mut best: Option<(usize, f32, String, f32)> = None;
        for (index, job) in self.heists.iter().enumerate() {
            if !matches!(
                job.state,
                HeistState::Ready | HeistState::Locked | HeistState::InProgress
            ) {
                continue;
            }
            if let Some((name, secs, dist)) = approach_at(&job.site, &job.approach, job.state, px, py, pz)
            {
                if best.as_ref().map(|(_, d, _, _)| dist < *d).unwrap_or(true) {
                    best = Some((index, dist, name, secs));
                }
            }
        }
        let Some((index, _, approach, secs)) = best else {
            return false;
        };
        let finished = {
            let job = &mut self.heists[index];
            job.worked = true;
            job.owner = Some(squad);
            if job.state != HeistState::InProgress {
                job.state = HeistState::InProgress;
                job.approach = approach.clone();
                job.progress = 0.0;
            }
            job.progress = (job.progress + dt / secs.max(0.25)).min(1.0);
            job.progress >= 1.0
        };
        if finished {
            self.finish_breach(index, squad, approach == "Loud");
        }
        true
    }

    fn finish_breach(&mut self, index: usize, squad: Uuid, loud: bool) {
        let site = self.heists[index].site.clone();
        let (x, y, z) = if loud {
            (site.loud_x, site.loud_y, site.loud_z)
        } else {
            (site.quiet_x, site.quiet_y, site.quiet_z)
        };
        let crate_id = Uuid::new_v4();
        self.death_crates.push(super::TrackedCrate {
            data: crate::protocol::DeathCrate {
                id: crate_id,
                x,
                y,
                z,
                items: vec![heist_bag(&site)],
            },
            age: 0.0,
        });
        {
            let job = &mut self.heists[index];
            job.state = HeistState::Breached;
            job.progress = 1.0;
            job.crate_id = Some(crate_id);
            job.approach = if loud { "Loud".into() } else { "Quiet".into() };
            if loud {
                job.quiet_alarm_in = -1.0;
            } else {
                job.alarm = false;
                job.quiet_alarm_in = 12.0;
            }
        }
        if loud {
            self.raise_heist_alarm(index, Some(squad), true);
        } else {
            self.bump_heat(squad, 0.25, x, y, z);
        }
    }

    pub(super) fn resolve_heists(&mut self) {
        let mut trips = Vec::new();
        for (index, job) in self.heists.iter_mut().enumerate() {
            if job.state == HeistState::InProgress && !job.worked {
                job.progress = (job.progress - DT * 0.45).max(0.0);
                if job.progress <= 0.0 {
                    job.state = HeistState::Ready;
                    job.approach.clear();
                    job.owner = None;
                }
            }
            if job.quiet_alarm_in > 0.0 {
                job.quiet_alarm_in -= DT;
                if job.quiet_alarm_in <= 0.0 {
                    job.quiet_alarm_in = -1.0;
                    trips.push(index);
                }
            }
        }
        for index in trips {
            let squad = self.heists[index].owner;
            self.raise_heist_alarm(index, squad, false);
        }

        let looted: Vec<usize> = self
            .heists
            .iter()
            .enumerate()
            .filter(|(_, job)| {
                job.state == HeistState::Breached
                    && job
                        .crate_id
                        .map(|id| {
                            self.death_crates
                                .iter()
                                .find(|c| c.data.id == id)
                                .map(|c| c.data.items.is_empty())
                                .unwrap_or(true)
                        })
                        .unwrap_or(false)
            })
            .map(|(index, _)| index)
            .collect();
        for index in looted {
            let job = &mut self.heists[index];
            job.state = HeistState::Looted;
            job.cooldown = 45.0;
        }

        for job in &mut self.heists {
            if matches!(job.state, HeistState::Looted | HeistState::Cooldown) {
                job.state = HeistState::Cooldown;
                job.cooldown -= DT;
                if job.cooldown <= 0.0 {
                    job.state = HeistState::Ready;
                    job.progress = 0.0;
                    job.approach.clear();
                    job.alarm = false;
                    job.owner = None;
                    job.crate_id = None;
                    job.cooldown = 0.0;
                    job.quiet_alarm_in = -1.0;
                    job.worked = false;
                }
            }
        }
    }

    fn raise_heist_alarm(&mut self, index: usize, squad: Option<Uuid>, loud: bool) {
        let site = self.heists[index].site.clone();
        let name = site.name.clone();
        let id = site.id.clone();
        let (x, y, z) = site_center(&site);
        self.heists[index].alarm = true;
        self.alert_guards(&id);
        if let Some(squad) = squad {
            self.bump_heat(squad, if loud { 2.2 } else { 1.15 }, x, y, z);
        }
        self.alarms.retain(|alarm| alarm.ping.id != id);
        self.alarms.push(LiveAlarm {
            ping: AlarmPing {
                id,
                x,
                y,
                z,
                radius: 78.0,
                kind: "heist".into(),
                intensity: 1.0,
            },
            ttl: 28.0,
        });
        let how = if loud {
            "is hot — loud breach"
        } else {
            "is hot — silent alarm"
        };
        self.notices.push(CityNotice {
            kind: "heist_alarm".into(),
            message: format!("{name} {how}"),
            x,
            y,
            hear: 110.0,
        });
    }

    fn alert_guards(&mut self, site_id: &str) {
        for actor in self.actors.values_mut() {
            if actor.npc == NpcKind::Guard && actor.site_id == site_id {
                actor.alert = true;
            }
        }
    }

    pub(super) fn note_gunfire(&mut self, squad: Uuid, x: f32, y: f32, z: f32) {
        self.bump_heat(squad, 0.18, x, y, z);
        let near: Vec<String> = self
            .heists
            .iter()
            .filter(|job| {
                let (cx, cy, _) = site_center(&job.site);
                let dx = cx - x;
                let dy = cy - y;
                dx * dx + dy * dy < 22.0 * 22.0
            })
            .map(|job| job.site.id.clone())
            .collect();
        for site_id in &near {
            self.alert_guards(site_id);
        }
        let cooling = self
            .crew_heat
            .get(&squad)
            .map(|heat| heat.noise_cd > 0.0)
            .unwrap_or(false);
        if cooling {
            return;
        }
        if let Some(heat) = self.crew_heat.get_mut(&squad) {
            heat.noise_cd = 2.4;
        }
        self.alarms.push(LiveAlarm {
            ping: AlarmPing {
                id: format!("gun-{squad}"),
                x,
                y,
                z,
                radius: 36.0,
                kind: "gunfire".into(),
                intensity: 0.55,
            },
            ttl: 1.3,
        });
        self.notices.push(CityNotice {
            kind: "gunfire".into(),
            message: "Unsuppressed gunfire nearby".into(),
            x,
            y,
            hear: 42.0,
        });
    }

    pub(super) fn note_assault(&mut self, squad: Uuid, x: f32, y: f32, z: f32) {
        self.bump_heat(squad, 0.55, x, y, z);
        self.notices.push(CityNotice {
            kind: "assault".into(),
            message: "Crew engaged law enforcement".into(),
            x,
            y,
            hear: 36.0,
        });
    }

    pub(super) fn note_theft(&mut self, squad: Uuid, vehicle: Uuid, x: f32, y: f32, z: f32, law: bool) {
        let already = self
            .crew_heat
            .get(&squad)
            .map(|heat| heat.stolen.contains(&vehicle))
            .unwrap_or(false);
        if already {
            return;
        }
        self.bump_heat(squad, if law { 1.05 } else { 0.7 }, x, y, z);
        if let Some(heat) = self.crew_heat.get_mut(&squad) {
            heat.stolen.push(vehicle);
        }
        self.notices.push(CityNotice {
            kind: "theft".into(),
            message: if law {
                "Police vehicle stolen".into()
            } else {
                "Vehicle theft reported".into()
            },
            x,
            y,
            hear: 30.0,
        });
    }

    pub(super) fn mark_combat(&mut self, squad: Uuid) {
        if squad == LAW_SQUAD {
            return;
        }
        let heat = self.crew_heat.entry(squad).or_default();
        heat.combat = heat.combat.max(2.5);
    }

    fn bump_heat(&mut self, squad: Uuid, amount: f32, x: f32, y: f32, z: f32) {
        if squad == LAW_SQUAD {
            return;
        }
        let heat = self.crew_heat.entry(squad).or_default();
        heat.heat = (heat.heat + amount).clamp(0.0, HEAT_MAX);
        heat.combat = heat.combat.max(5.0);
        heat.contact_x = x;
        heat.contact_y = y;
        heat.contact_z = z;
        heat.has_contact = true;
    }

    pub(super) fn tick_heat_and_response(&mut self) {
        let squads: Vec<Uuid> = self.crew_heat.keys().copied().collect();
        for squad in squads {
            let seen = self.law_sees(squad);
            if let Some(heat) = self.crew_heat.get_mut(&squad) {
                heat.noise_cd = (heat.noise_cd - DT).max(0.0);
                if seen {
                    heat.combat = heat.combat.max(1.2);
                }
                if heat.combat > 0.0 {
                    heat.combat = (heat.combat - DT).max(0.0);
                } else if !seen {
                    heat.heat = (heat.heat - HEAT_DECAY_PER_SEC * DT).max(0.0);
                }
            }
            self.dispatch_response(squad);
        }
        self.crew_heat
            .retain(|id, heat| heat.heat > 0.02 || self.crews.contains_key(id));
    }

    fn law_sees(&self, squad: Uuid) -> bool {
        let members: Vec<(f32, f32, f32)> = self
            .actors
            .values()
            .filter(|actor| !actor.npc.is_law() && actor.squad_id == squad && actor.health > 0.0)
            .map(|actor| (actor.x, actor.y, actor.z))
            .collect();
        for law in self.actors.values() {
            if !law.npc.is_law() || law.health <= 0.0 {
                continue;
            }
            if law.npc != NpcKind::Guard && law.focus.is_some() && law.focus != Some(squad) {
                continue;
            }
            for (x, y, z) in &members {
                let dx = law.x - x;
                let dy = law.y - y;
                if dx * dx + dy * dy > 36.0 * 36.0 {
                    continue;
                }
                if !sightline_blocked(law.x, law.y, law.z, *x, *y, *z, &self.map.obstacles) {
                    return true;
                }
            }
        }
        false
    }

    fn dispatch_response(&mut self, squad: Uuid) {
        let Some(heat) = self.crew_heat.get(&squad).map(|heat| heat.heat) else {
            return;
        };
        let stars = heat.floor() as i32;
        if stars < 2 {
            return;
        }
        let want_police = if stars >= 4 { 3 } else if stars >= 3 { 2 } else { 1 };
        let want_swat = if stars >= 5 { 2 } else if stars >= 4 { 1 } else { 0 };
        if self.count_focus(squad, NpcKind::Police) < want_police {
            self.spawn_response(squad, NpcKind::Police, true);
            return;
        }
        if self.count_focus(squad, NpcKind::Swat) < want_swat {
            self.spawn_response(squad, NpcKind::Swat, true);
        }
    }

    fn count_focus(&self, squad: Uuid, kind: NpcKind) -> usize {
        self.actors
            .values()
            .filter(|actor| actor.health > 0.0 && actor.npc == kind && actor.focus == Some(squad))
            .count()
    }

    fn spawn_response(&mut self, squad: Uuid, kind: NpcKind, car: bool) {
        let (cx, cy, cz) = self
            .crew_heat
            .get(&squad)
            .filter(|heat| heat.has_contact)
            .map(|heat| (heat.contact_x, heat.contact_y, heat.contact_z))
            .unwrap_or((
                self.map.bounds.x + self.map.bounds.width * 0.5,
                self.map.bounds.y + self.map.bounds.depth * 0.5,
                self.map.ground_z,
            ));
        let ground = self.map.ground_z;
        for step in 0..10 {
            let ang = self.tick as f32 * 0.13 + step as f32 * 0.85;
            let dist = 28.0 + (step % 3) as f32 * 5.0;
            let x = cx + ang.cos() * dist;
            let y = cy + ang.sin() * dist;
            let (x, y) = self.clamp_bounds(x, y, 1.3);
            if blocked_by_map(x, y, 1.2, ground, ground + 2.0, &self.map.obstacles) {
                continue;
            }
            let id = self.spawn_law(kind, x, y, ground, Some(squad), car, cx, cy, cz);
            if let Some(actor) = self.actors.get_mut(&id) {
                actor.seen = true;
                actor.seen_x = cx;
                actor.seen_y = cy;
                actor.seen_z = cz;
                actor.alert = true;
            }
            return;
        }
    }

    fn spawn_law(
        &mut self,
        kind: NpcKind,
        x: f32,
        y: f32,
        z: f32,
        focus: Option<Uuid>,
        car: bool,
        seen_x: f32,
        seen_y: f32,
        seen_z: f32,
    ) -> Uuid {
        let id = Uuid::new_v4();
        let (name, hp) = match kind {
            NpcKind::Guard => ("Guard".to_string(), 70.0),
            NpcKind::Police => ("Police".to_string(), 90.0),
            NpcKind::Swat => ("SWAT".to_string(), 130.0),
            NpcKind::None => ("Unit".to_string(), 80.0),
        };
        self.actors.insert(
            id,
            Actor {
                id,
                player_id: LAW_SQUAD,
                squad_id: LAW_SQUAD,
                specialist_id: id,
                name,
                role: kind.label().into(),
                max_hp: hp,
                appearance: default_appearance(),
                order: super::Order::Hold { x, y, z },
                x,
                y,
                z,
                yaw: 0.0,
                vx: 0.0,
                vy: 0.0,
                vz: 0.0,
                health: hp,
                armor: if kind == NpcKind::Swat { 30.0 } else { 0.0 },
                vehicle_id: None,
                extract_time: 0.0,
                input: InputState::default(),
                pending_inputs: Default::default(),
                shoot_cd: 0.4,
                interact_edge: false,
                last_seq: 0,
                backpack: vec![super::issued_sidearm()],
                npc: kind,
                site_id: String::new(),
                focus,
                home_x: x,
                home_y: y,
                home_z: z,
                patrol: 0.0,
                seen_x,
                seen_y,
                seen_z,
                seen: false,
                search: 0.0,
                alert: kind != NpcKind::Guard,
            },
        );
        if car {
            let vid = Uuid::new_v4();
            self.vehicles.insert(
                vid,
                Vehicle {
                    id: vid,
                    kind: VehicleKind::GroundCar,
                    x,
                    y,
                    z,
                    yaw: (seen_y - y).atan2(seen_x - x),
                    vel_yaw: (seen_y - y).atan2(seen_x - x),
                    speed: 0.0,
                    hp: 110.0,
                    driver_id: None,
                    law: true,
                },
            );
            self.enter_vehicle(id, vid);
        }
        id
    }

    pub(super) fn hostile_think(&mut self, id: Uuid) -> InputState {
        let Some(me) = self.actors.get(&id) else {
            return InputState::default();
        };
        let kind = me.npc;
        let (x, y, z, yaw) = (me.x, me.y, me.z, me.yaw);
        let alert = me.alert || self.site_alarm(&me.site_id);
        let focus = me.focus;
        let in_vehicle = me.vehicle_id;
        let seen = me.seen;
        let (sx, sy, sz) = (me.seen_x, me.seen_y, me.seen_z);
        let search = me.search;
        let home = (me.home_x, me.home_y);
        let patrol = me.patrol;
        let range = match kind {
            NpcKind::Swat => 30.0,
            NpcKind::Police => 26.0,
            NpcKind::Guard => 16.0,
            NpcKind::None => 0.0,
        };
        let engage = if kind == NpcKind::Guard && !alert {
            8.0
        } else {
            range
        };

        let mut best: Option<(f32, f32, f32, f32, Uuid)> = None;
        for other in self.actors.values() {
            if other.id == id || other.npc.is_law() || other.health <= 0.0 {
                continue;
            }
            if kind != NpcKind::Guard {
                if let Some(focus) = focus {
                    if other.squad_id != focus {
                        continue;
                    }
                }
            }
            let dx = other.x - x;
            let dy = other.y - y;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > engage {
                continue;
            }
            if sightline_blocked(x, y, z, other.x, other.y, other.z, &self.map.obstacles) {
                continue;
            }
            if best.as_ref().map(|(bd, _, _, _, _)| dist < *bd).unwrap_or(true) {
                best = Some((dist, other.x, other.y, other.z, other.squad_id));
            }
        }

        if let Some((dist, tx, ty, tz, squad)) = best {
            if let Some(me) = self.actors.get_mut(&id) {
                me.alert = true;
                me.seen = true;
                me.seen_x = tx;
                me.seen_y = ty;
                me.seen_z = tz;
                me.search = 0.0;
            }
            self.mark_combat(squad);
            if let Some(heat) = self.crew_heat.get_mut(&squad) {
                heat.contact_x = tx;
                heat.contact_y = ty;
                heat.contact_z = tz;
                heat.has_contact = true;
            }
            let aim = (ty - y).atan2(tx - x);
            let shoot = dist <= range;
            if in_vehicle.is_some() && dist < 11.0 {
                self.leave_vehicle(id);
            } else if in_vehicle.is_some() {
                let mut input = self.drive_toward(id, tx, ty, tz);
                input.aim_angle = aim;
                input.shoot = shoot;
                return input;
            }
            if dist > 7.2 {
                return self.foot_toward(id, tx, ty, aim, shoot);
            }
            return InputState {
                dt: DT,
                aim_angle: aim,
                shoot,
                ..InputState::default()
            };
        }

        if in_vehicle.is_some() && seen {
            let dist = ((sx - x).powi(2) + (sy - y).powi(2)).sqrt();
            if dist < 8.0 {
                self.leave_vehicle(id);
            } else {
                return self.drive_toward(id, sx, sy, sz);
            }
        }
        if seen && search < 8.0 {
            if let Some(me) = self.actors.get_mut(&id) {
                me.search = (me.search + DT).min(8.0);
            }
            let dist = ((sx - x).powi(2) + (sy - y).powi(2)).sqrt();
            if dist > 1.4 {
                let aim = (sy - y).atan2(sx - x);
                return self.foot_toward(id, sx, sy, aim, false);
            }
        }
        let step = if ((home.0 - x).powi(2) + (home.1 - y).powi(2)).sqrt() < 1.0 {
            1.35
        } else {
            0.0
        };
        let ang = patrol + step;
        if let Some(me) = self.actors.get_mut(&id) {
            me.patrol = ang;
            if alert {
                me.alert = true;
            }
        }
        let tx = home.0 + ang.cos() * 3.4;
        let ty = home.1 + ang.sin() * 3.4;
        self.foot_toward(id, tx, ty, yaw, false)
    }

    fn site_alarm(&self, site_id: &str) -> bool {
        if site_id.is_empty() {
            return false;
        }
        self.heists
            .iter()
            .any(|job| job.site.id == site_id && job.alarm)
    }

    fn leave_vehicle(&mut self, actor_id: Uuid) {
        let Some(actor) = self.actors.get(&actor_id) else {
            return;
        };
        let Some(vid) = actor.vehicle_id else {
            return;
        };
        let (vx, vy, vz, yaw) = self
            .vehicles
            .get(&vid)
            .map(|v| (v.x, v.y, v.z, v.yaw))
            .unwrap_or((actor.x, actor.y, actor.z, actor.yaw));
        if let Some(vehicle) = self.vehicles.get_mut(&vid) {
            vehicle.driver_id = None;
            vehicle.speed = 0.0;
        }
        let ground = self.map.ground_z;
        let ex = vx + yaw.cos() * 1.8;
        let ey = vy + yaw.sin() * 1.8;
        let (ex, ey) = self.clamp_bounds(ex, ey, PLAYER_RADIUS);
        let z = if blocked_by_map(
            ex,
            ey,
            PLAYER_RADIUS,
            ground,
            ground + PLAYER_HEIGHT,
            &self.map.obstacles,
        ) {
            vz
        } else {
            ground
        };
        if let Some(actor) = self.actors.get_mut(&actor_id) {
            actor.vehicle_id = None;
            actor.x = ex;
            actor.y = ey;
            actor.z = z;
            actor.vx = 0.0;
            actor.vy = 0.0;
            actor.vz = 0.0;
        }
    }

    pub(super) fn tick_alarms(&mut self) {
        for alarm in &mut self.alarms {
            alarm.ttl -= DT;
        }
        self.alarms.retain(|alarm| alarm.ttl > 0.0);
        let hot: Vec<String> = self
            .heists
            .iter()
            .filter(|job| job.alarm)
            .map(|job| job.site.id.clone())
            .collect();
        self.alarms.retain(|alarm| {
            alarm.ping.kind != "heist" || hot.iter().any(|id| id == &alarm.ping.id)
        });
    }

    pub(super) fn heist_markers(&self) -> Vec<HeistMarker> {
        self.heists
            .iter()
            .map(|job| {
                let (x, y, z) = site_center(&job.site);
                HeistMarker {
                    id: job.site.id.clone(),
                    name: job.site.name.clone(),
                    kind: job.site.kind,
                    state: job.state,
                    x,
                    y,
                    z,
                    quiet_x: job.site.quiet_x,
                    quiet_y: job.site.quiet_y,
                    quiet_z: job.site.quiet_z,
                    loud_x: job.site.loud_x,
                    loud_y: job.site.loud_y,
                    loud_z: job.site.loud_z,
                    progress: job.progress,
                    approach: job.approach.clone(),
                    alarm: job.alarm,
                    owner_squad: job.owner,
                }
            })
            .collect()
    }

    pub(super) fn alarms_for(&self, ox: f32, oy: f32, oz: f32, observing: bool) -> Vec<AlarmPing> {
        if !observing {
            return Vec::new();
        }
        self.alarms
            .iter()
            .filter(|alarm| {
                let dx = alarm.ping.x - ox;
                let dy = alarm.ping.y - oy;
                let hear = alarm.ping.radius;
                if dx * dx + dy * dy > hear * hear {
                    return false;
                }
                if alarm.ping.kind == "heist" {
                    return true;
                }
                !sightline_blocked(ox, oy, oz, alarm.ping.x, alarm.ping.y, alarm.ping.z, &self.map.obstacles)
            })
            .map(|alarm| alarm.ping.clone())
            .collect()
    }
}

fn site_center(site: &HeistSite) -> (f32, f32, f32) {
    (
        site.x + site.width * 0.5,
        site.y + site.depth * 0.5,
        site.z_min,
    )
}

fn approach_at(
    site: &HeistSite,
    locked_approach: &str,
    state: HeistState,
    px: f32,
    py: f32,
    pz: f32,
) -> Option<(String, f32, f32)> {
    let quiet = point_near(px, py, pz, site.quiet_x, site.quiet_y, site.quiet_z)
        .map(|dist| ("Quiet".to_string(), site.quiet_secs, dist));
    let loud = point_near(px, py, pz, site.loud_x, site.loud_y, site.loud_z)
        .map(|dist| ("Loud".to_string(), site.loud_secs, dist));
    let pick = match (quiet, loud) {
        (Some(q), Some(l)) => Some(if q.2 <= l.2 { q } else { l }),
        (Some(q), None) => Some(q),
        (None, Some(l)) => Some(l),
        (None, None) => None,
    }?;
    if state == HeistState::InProgress && !locked_approach.is_empty() && locked_approach != pick.0 {
        return None;
    }
    Some(pick)
}

fn point_near(px: f32, py: f32, pz: f32, x: f32, y: f32, z: f32) -> Option<f32> {
    let dx = px - x;
    let dy = py - y;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist <= APPROACH_RADIUS && (pz - z).abs() <= INTERACT_Z + 0.5 {
        Some(dist)
    } else {
        None
    }
}

fn heist_bag(site: &HeistSite) -> StashItem {
    let value = match site.loot_tier {
        LootTier::Common => 400,
        LootTier::Rare => 900,
        LootTier::Epic => 1600,
    };
    StashItem {
        id: Uuid::new_v4(),
        item_type: "Loot".into(),
        item_name: format!("{} Take", site.name),
        quantity: 1,
        stats_json: format!(
            r#"{{"cash_value":{value},"heist":true,"weight":3.5,"tier":"{:?}"}}"#,
            site.loot_tier
        ),
    }
}
