use std::f32::consts::{FRAC_PI_2, TAU};

use crate::framebuffer::Framebuffer;
use crate::obj_loader::{Face, Model, Vec3, load_obj};
use crate::ship::{Projectile, ShipController, ShipInput};
use crate::{Camera, draw_projectile_glow, draw_selection_label, render_model};

const START_Z: f32 = 24.0;
const FURNACE_Z: f32 = -105.0;
const SANCTUM_Z: f32 = -215.0;
const CORRIDOR_Z: f32 = -325.0;
const SEAL_Z: f32 = -382.0;
const CHAMBER_Z: f32 = -470.0;
// Keep the adjacent room fully resolved before it enters the camera view.  More
// distant rooms remain present through their inexpensive Blender LOD instead of
// being created abruptly at a hard draw-distance boundary.
const DETAILED_ZONE_DISTANCE: f32 = 190.0;
const PLAYER_HEALTH: i32 = 6;
const REWARD_DURATION: f32 = 5.5;
const REWARD_COLLECT_TIME: f32 = 2.8;
const CONTROL_SPEED_FACTOR: f32 = 0.55;
const CONTROL_HALF_WIDTH: f32 = 14.5;
const CONTROL_SOFT_BORDER: f32 = 12.2;
const CONTROL_BORDER_STRENGTH: f32 = 26.0;
const CONTROL_MIN_HEIGHT: f32 = 2.4;
const CONTROL_MAX_HEIGHT: f32 = 11.5;
const CONTROL_HEIGHT_OFFSET: f32 = 26.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Continue,
    Exit,
    Victory,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Exploring,
    Reward,
    Complete,
    Defeat,
}

#[derive(Clone, Copy)]
struct Fragment {
    position: Vec3,
    collected: bool,
}

#[derive(Clone, Copy)]
struct Hazard {
    position: Vec3,
    kind: HazardKind,
    phase: f32,
}

struct InteriorEnemy {
    position: Vec3,
    anchor: Vec3,
    health: i32,
    phase: f32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HazardKind {
    FireJet,
    LaserHorizontal,
    LaserVertical,
    Rotor,
    Spikes,
    MovingPlatform,
}

pub struct LavaCastleInterior {
    phase: Phase,
    elapsed: f32,
    health: i32,
    invulnerability: f32,
    progress: f32,
    lateral: f32,
    height: f32,
    fragments: [Fragment; 3],
    seal_open: f32,
    checkpoint: Vec3,
    reward_elapsed: f32,
    reward_position: Vec3,
    message: &'static str,
    message_timer: f32,
    zones: [Model; 5],
    zone_lods: [Model; 5],
    hazards: Vec<Hazard>,
    enemies: Vec<InteriorEnemy>,
    enemy_shots: Vec<Projectile>,
    enemy_fire_timer: f32,
    enemy_fire_serial: usize,
    crystals: [Model; 3],
    hazard_bar: Model,
    moving_platform: Model,
    enemy_model: Model,
    enemy_bolt: Model,
    seal: Model,
}

impl LavaCastleInterior {
    pub fn new() -> Self {
        Self {
            phase: Phase::Exploring,
            elapsed: 0.0,
            health: PLAYER_HEALTH,
            invulnerability: 0.0,
            fragments: [
                Fragment {
                    position: Vec3::new(-11.5, 30.0, SANCTUM_Z),
                    collected: false,
                },
                Fragment {
                    position: Vec3::new(0.0, 37.0, SANCTUM_Z),
                    collected: false,
                },
                Fragment {
                    position: Vec3::new(11.5, 33.5, SANCTUM_Z),
                    collected: false,
                },
            ],
            seal_open: 0.0,
            progress: 0.0,
            lateral: 0.0,
            height: 5.0,
            checkpoint: Vec3::new(0.0, 31.0, START_Z),
            reward_elapsed: 0.0,
            reward_position: Vec3::default(),
            message: "EXPLORA EL CASTILLO  //  REUNE 3 FRAGMENTOS",
            message_timer: 5.0,
            zones: build_zones(),
            zone_lods: build_zone_lods(),
            hazards: build_hazards(),
            enemies: build_enemies(),
            enemy_shots: Vec::with_capacity(32),
            enemy_fire_timer: 0.8,
            enemy_fire_serial: 0,
            crystals: [
                make_crystal([35, 155, 255], "race_rgb_35_155_255"),
                make_crystal([255, 116, 18], "race_rgb_255_116_18"),
                make_crystal([195, 55, 255], "race_rgb_195_55_255"),
            ],
            hazard_bar: make_hazard_bar(),
            moving_platform: make_moving_platform(),
            enemy_model: load_obj("assets/levels/lava_castle_interior/lava_sentinel.obj")
                .unwrap_or_else(|_| make_interior_enemy()),
            enemy_bolt: make_enemy_bolt(),
            seal: make_seal(),
        }
    }

    pub fn enter(&mut self, controller: &mut ShipController) {
        *self = Self::new();
        controller.projectiles.clear();
        controller.ship.position = self.checkpoint;
        controller.ship.yaw = FRAC_PI_2;
        controller.ship.pitch = 0.0;
        controller.ship.roll = 0.0;
        controller.ship.yaw_rate = 0.0;
        controller.ship.pitch_rate = 0.0;
        controller.ship.speed = controller.ship.spec.cruise_speed;
    }

    pub fn update(
        &mut self,
        controller: &mut ShipController,
        input: &ShipInput,
        restart: bool,
        exit: bool,
        delta_time: f32,
    ) -> Action {
        if exit {
            return Action::Exit;
        }
        if restart && self.phase == Phase::Defeat {
            self.enter(controller);
            return Action::Continue;
        }
        let dt = delta_time.clamp(0.0, 0.05);
        self.elapsed += dt;
        self.invulnerability = (self.invulnerability - dt).max(0.0);
        self.message_timer = (self.message_timer - dt).max(0.0);

        match self.phase {
            Phase::Defeat => return Action::Continue,
            Phase::Complete => return Action::Victory,
            Phase::Reward => {
                self.reward_elapsed = (self.reward_elapsed + dt).min(REWARD_DURATION);
                controller.ship.position = self.reward_position;
                controller.ship.speed = 0.0;
                if self.reward_elapsed >= REWARD_DURATION {
                    self.phase = Phase::Complete;
                }
                return Action::Continue;
            }
            Phase::Exploring => {}
        }

        controller.update(input, dt);
        self.progress += controller.ship.speed * CONTROL_SPEED_FACTOR * dt;
        let sanctum_progress = START_Z - SANCTUM_Z;
        if self.fragment_count() < 3 {
            self.progress = self.progress.min(sanctum_progress);
        }
        self.lateral += input.yaw as f32 * 17.0 * dt;
        let border_distance = self.lateral.abs();
        if border_distance > CONTROL_SOFT_BORDER {
            let border_zone = CONTROL_HALF_WIDTH - CONTROL_SOFT_BORDER;
            let border_force =
                ((border_distance - CONTROL_SOFT_BORDER) / border_zone).clamp(0.0, 1.0);
            self.lateral -= self.lateral.signum() * CONTROL_BORDER_STRENGTH * border_force * dt;
        }
        self.lateral = self.lateral.clamp(-CONTROL_HALF_WIDTH, CONTROL_HALF_WIDTH);
        self.height = (self.height + input.pitch as f32 * 15.0 * dt)
            .clamp(CONTROL_MIN_HEIGHT, CONTROL_MAX_HEIGHT);

        let seal_progress = START_Z - (SEAL_Z + 8.0);
        if self.seal_open < 0.92 {
            self.progress = self.progress.min(seal_progress);
        }
        let previous = controller.ship.position;
        controller.ship.position = Vec3::new(
            self.lateral,
            CONTROL_HEIGHT_OFFSET + self.height,
            START_Z - self.progress,
        );
        controller.ship.yaw = FRAC_PI_2;
        controller.ship.pitch = 0.0;
        controller.ship.orbit_plane_normal = None;
        controller.ship.orbit_center = None;
        controller.ship.orbit_camera_blend = 0.0;
        controller.ship.planet_locked = false;
        controller.ship.orbit_release_active = false;
        self.resolve_bounds(&mut controller.ship.position, previous);
        self.lateral = controller.ship.position.x;
        self.height = controller.ship.position.y - CONTROL_HEIGHT_OFFSET;
        self.progress = START_Z - controller.ship.position.z;
        self.update_checkpoint(controller.ship.position);
        self.collect_fragments(controller.ship.position);
        self.update_enemies(controller, dt);
        self.resolve_player_fire(&mut controller.projectiles);

        if self.fragment_count() == 3 {
            self.seal_open = (self.seal_open + dt / 2.4).min(1.0);
        }
        if self.seal_open < 0.92 && controller.ship.position.z < SEAL_Z + 8.0 {
            controller.ship.position.z = SEAL_Z + 8.0;
            self.progress = START_Z - controller.ship.position.z;
            controller.ship.speed = controller.ship.speed.min(controller.ship.spec.cruise_speed);
            self.message = if self.fragment_count() == 3 {
                "SELLO ABRIENDOSE"
            } else {
                "PUERTA SELLADA  //  FALTAN FRAGMENTOS"
            };
            self.message_timer = 1.0;
        }

        self.resolve_hazards(controller);
        if controller
            .ship
            .position
            .sub(Vec3::new(0.0, 36.0, CHAMBER_Z))
            .dot(
                controller
                    .ship
                    .position
                    .sub(Vec3::new(0.0, 36.0, CHAMBER_Z)),
            )
            < 9.0 * 9.0
            && self.seal_open >= 1.0
        {
            self.phase = Phase::Reward;
            self.reward_elapsed = 0.0;
            self.reward_position = controller.ship.position;
            controller.ship.speed = 0.0;
            controller.projectiles.clear();
            self.message = "RECIBIENDO ESFERA FINAL";
            self.message_timer = 3.35;
        }
        if self.health <= 0 {
            self.phase = Phase::Defeat;
            controller.ship.speed = 0.0;
            self.message = "NAVE DESTRUIDA  //  R PARA REINTENTAR";
            self.message_timer = 999.0;
        }
        Action::Continue
    }

    fn resolve_bounds(&mut self, position: &mut Vec3, _previous: Vec3) {
        position.x = position.x.clamp(-51.0, 51.0);
        position.y = position.y.clamp(9.0, 73.0);
        position.z = position.z.clamp(CHAMBER_Z - 28.0, START_Z + 18.0);
        // The sanctum side chambers open only around their shared hub.
        if position.z > SANCTUM_Z + 42.0 || position.z < SANCTUM_Z - 55.0 {
            position.x = position.x.clamp(-34.0, 34.0);
        }
        if position.y <= 9.1 {
            self.damage_player(position);
        }
    }

    fn update_checkpoint(&mut self, position: Vec3) {
        let next = if position.z < SEAL_Z + 34.0 {
            Vec3::new(0.0, 31.0, CORRIDOR_Z + 42.0)
        } else if position.z < SANCTUM_Z + 38.0 {
            Vec3::new(0.0, 34.0, SANCTUM_Z + 38.0)
        } else if position.z < FURNACE_Z + 34.0 {
            Vec3::new(0.0, 32.0, FURNACE_Z + 44.0)
        } else {
            return;
        };
        if next.z < self.checkpoint.z {
            self.checkpoint = next;
        }
    }

    fn collect_fragments(&mut self, ship: Vec3) {
        let mut collected = false;
        for fragment in &mut self.fragments {
            let offset = ship.sub(fragment.position);
            if !fragment.collected && offset.dot(offset) < 8.0 * 8.0 {
                fragment.collected = true;
                collected = true;
            }
        }
        if collected {
            self.message = "FRAGMENTO DE ENERGIA OBTENIDO";
            self.message_timer = 2.2;
        }
    }

    fn fragment_count(&self) -> usize {
        self.fragments
            .iter()
            .filter(|fragment| fragment.collected)
            .count()
    }

    fn update_enemies(&mut self, controller: &mut ShipController, dt: f32) {
        let player = controller.ship.position;
        for enemy in self.enemies.iter_mut().filter(|enemy| enemy.health > 0) {
            if (player.z - enemy.anchor.z).abs() < 105.0 {
                let follow = Vec3::new(player.x, player.y, enemy.anchor.z).sub(enemy.position);
                enemy.position = enemy.position.add(Vec3::new(
                    follow.x.clamp(-6.5 * dt, 6.5 * dt),
                    follow.y.clamp(-5.0 * dt, 5.0 * dt),
                    (self.elapsed * 1.4 + enemy.phase).sin() * 1.6 * dt,
                ));
            } else {
                enemy.position = enemy
                    .position
                    .add(enemy.anchor.sub(enemy.position).mul((dt * 1.8).min(1.0)));
            }
        }
        self.enemy_fire_timer -= dt;
        if self.enemy_fire_timer <= 0.0 {
            let active = self
                .enemies
                .iter()
                .enumerate()
                .filter(|(_, enemy)| enemy.health > 0 && (player.z - enemy.position.z).abs() < 95.0)
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            if let Some(&index) = active.get(self.enemy_fire_serial % active.len().max(1)) {
                let enemy = &self.enemies[index];
                self.enemy_shots.push(Projectile {
                    position: enemy.position,
                    direction: player.sub(enemy.position).normalize(),
                    lifetime: 4.0,
                    scale: 0.8,
                });
                self.enemy_fire_serial += 1;
                self.enemy_fire_timer = 0.72;
            } else {
                self.enemy_fire_timer = 0.25;
            }
        }
        let mut hit = false;
        for shot in &mut self.enemy_shots {
            shot.position = shot.position.add(shot.direction.mul(34.0 * dt));
            shot.lifetime -= dt;
            let offset = shot.position.sub(player);
            if offset.dot(offset) < 4.2 * 4.2 {
                shot.lifetime = 0.0;
                hit = true;
            }
        }
        self.enemy_shots.retain(|shot| shot.lifetime > 0.0);
        hit |= self
            .enemies
            .iter()
            .filter(|enemy| enemy.health > 0)
            .any(|enemy| {
                let offset = enemy.position.sub(player);
                offset.dot(offset) < 6.0 * 6.0
            });
        if hit {
            self.damage_player(&mut controller.ship.position);
        }
    }

    fn resolve_player_fire(&mut self, projectiles: &mut [Projectile]) {
        for projectile in projectiles.iter_mut().filter(|shot| shot.lifetime > 0.0) {
            for enemy in self.enemies.iter_mut().filter(|enemy| enemy.health > 0) {
                let offset = projectile.position.sub(enemy.position);
                if offset.dot(offset) < (5.2 + projectile.scale).powi(2) {
                    enemy.health -= if projectile.scale > 1.35 { 2 } else { 1 };
                    projectile.lifetime = 0.0;
                    if enemy.health <= 0 {
                        self.message = "ENEMIGO DE LAVA DESTRUIDO";
                        self.message_timer = 1.2;
                    }
                    break;
                }
            }
        }
    }

    fn hazard_active(&self, hazard: Hazard) -> bool {
        match hazard.kind {
            HazardKind::FireJet => (self.elapsed * 2.15 + hazard.phase).sin() > 0.12,
            HazardKind::Spikes => (self.elapsed * 1.7 + hazard.phase).sin() > 0.35,
            _ => true,
        }
    }

    fn hazard_position(&self, hazard: Hazard) -> Vec3 {
        if hazard.kind == HazardKind::MovingPlatform {
            hazard.position.add(Vec3::new(
                (self.elapsed * 0.85 + hazard.phase).sin() * 17.0,
                (self.elapsed * 1.15 + hazard.phase).cos() * 7.0,
                0.0,
            ))
        } else {
            hazard.position
        }
    }

    fn resolve_hazards(&mut self, controller: &mut ShipController) {
        let ship = controller.ship.position;
        let mut hit = false;
        for &hazard in &self.hazards {
            if !self.hazard_active(hazard) {
                continue;
            }
            let position = self.hazard_position(hazard);
            let d = ship.sub(position);
            let intersects = match hazard.kind {
                HazardKind::FireJet => d.z.abs() < 5.0 && d.x.abs() < 9.0 && d.y.abs() < 22.0,
                HazardKind::LaserHorizontal => {
                    d.z.abs() < 2.8 && d.y.abs() < 2.2 && d.x.abs() < 46.0
                }
                HazardKind::LaserVertical => d.z.abs() < 2.8 && d.x.abs() < 2.2 && d.y.abs() < 34.0,
                HazardKind::Rotor => {
                    let angle = self.elapsed * 1.55 + hazard.phase;
                    let perpendicular = (-angle.sin() * d.x + angle.cos() * d.y).abs();
                    d.z.abs() < 3.0 && perpendicular < 2.6 && d.x.hypot(d.y) < 40.0
                }
                HazardKind::Spikes => d.z.abs() < 5.0 && d.x.abs() < 13.0 && d.y.abs() < 8.0,
                HazardKind::MovingPlatform => {
                    if d.z.abs() < 7.0 && d.x.abs() < 10.0 && d.y > -3.0 && d.y < 4.0 {
                        controller.ship.position.y = position.y + 5.0;
                    }
                    false
                }
            };
            hit |= intersects;
        }
        if hit {
            self.damage_player(&mut controller.ship.position);
        }
    }

    fn damage_player(&mut self, position: &mut Vec3) {
        if self.invulnerability <= 0.0 {
            self.health -= 1;
            self.invulnerability = 1.15;
            *position = self.checkpoint;
            self.progress = START_Z - self.checkpoint.z;
            self.lateral = self.checkpoint.x;
            self.height = self.checkpoint.y - CONTROL_HEIGHT_OFFSET;
            self.message = "IMPACTO  //  REGRESO AL CHECKPOINT";
            self.message_timer = 1.5;
        }
    }

    pub fn render(
        &self,
        frame: &mut Framebuffer,
        ship_model: &Model,
        laser_model: &Model,
        dragon_ball: &Model,
        controller: &ShipController,
    ) {
        draw_interior_sky(frame);
        frame.buffers_mut().1.fill(f32::NEG_INFINITY);
        let camera = interior_camera(controller);
        let centers = [START_Z, FURNACE_Z, SANCTUM_Z, CORRIDOR_Z, CHAMBER_Z];
        for (index, center) in centers.into_iter().enumerate() {
            let distance = (controller.ship.position.z - center).abs();
            let zone = if distance < DETAILED_ZONE_DISTANCE {
                &self.zones[index]
            } else {
                &self.zone_lods[index]
            };
            render_model(zone, frame, &camera, |v| v, Some(self.elapsed), None);
        }
        self.render_fragments(frame, &camera);
        self.render_hazards(frame, &camera);
        self.render_enemies(frame, &camera);
        self.render_seal(frame, &camera);
        self.render_reward(frame, &camera, dragon_ball);
        render_player_and_weapon(
            frame,
            &camera,
            ship_model,
            laser_model,
            controller,
            self.elapsed,
        );
        self.draw_hud(frame, controller.ship.position.z);
    }

    fn render_fragments(&self, frame: &mut Framebuffer, camera: &Camera) {
        for (index, fragment) in self.fragments.iter().enumerate() {
            if fragment.collected {
                continue;
            }
            let crystal = &self.crystals[index];
            let center = crystal.bounds().0;
            let angle = self.elapsed * 1.7 + index as f32;
            let (sin, cos) = angle.sin_cos();
            let hover = (self.elapsed * 3.0 + index as f32).sin() * 0.8;
            render_model(
                crystal,
                frame,
                camera,
                |v| {
                    let local = v.sub(center);
                    Vec3::new(
                        local.x * cos - local.z * sin,
                        local.y,
                        local.x * sin + local.z * cos,
                    )
                    .add(fragment.position.add(Vec3::new(0.0, hover, 0.0)))
                },
                Some(self.elapsed),
                None,
            );
            for spark in 0..3 {
                let orbit = angle * 1.7 + spark as f32 * TAU / 3.0;
                let spark_position = fragment.position.add(Vec3::new(
                    orbit.cos() * 6.0,
                    hover + (orbit * 1.3).sin() * 3.0,
                    orbit.sin() * 6.0,
                ));
                render_model(
                    crystal,
                    frame,
                    camera,
                    |v| v.sub(center).mul(0.18).add(spark_position),
                    Some(self.elapsed),
                    None,
                );
            }
        }
    }

    fn render_hazards(&self, frame: &mut Framebuffer, camera: &Camera) {
        let center = self.hazard_bar.bounds().0;
        for &hazard in &self.hazards {
            if !self.hazard_active(hazard) {
                continue;
            }
            if hazard.kind == HazardKind::MovingPlatform {
                let platform_center = self.moving_platform.bounds().0;
                let position = self.hazard_position(hazard);
                render_model(
                    &self.moving_platform,
                    frame,
                    camera,
                    |v| v.sub(platform_center).add(position),
                    Some(self.elapsed),
                    None,
                );
                continue;
            }
            let rotation = if hazard.kind == HazardKind::Rotor {
                self.elapsed * 1.55 + hazard.phase
            } else {
                0.0
            };
            let (sin, cos) = rotation.sin_cos();
            render_model(
                &self.hazard_bar,
                frame,
                camera,
                |v| {
                    let mut local = v.sub(center);
                    local = match hazard.kind {
                        HazardKind::FireJet => Vec3::new(local.x * 0.22, local.y * 3.0, local.z),
                        HazardKind::LaserVertical => Vec3::new(local.y, local.x, local.z),
                        HazardKind::Spikes => {
                            Vec3::new(local.x * 0.35, local.y * 0.7, local.z * 2.0)
                        }
                        HazardKind::Rotor => Vec3::new(
                            local.x * cos - local.y * sin,
                            local.x * sin + local.y * cos,
                            local.z,
                        ),
                        HazardKind::LaserHorizontal => local,
                        HazardKind::MovingPlatform => local,
                    };
                    local.add(self.hazard_position(hazard))
                },
                Some(self.elapsed),
                None,
            );
        }
    }

    fn render_enemies(&self, frame: &mut Framebuffer, camera: &Camera) {
        let center = self.enemy_model.bounds().0;
        for enemy in self.enemies.iter().filter(|enemy| enemy.health > 0) {
            let (sin, cos) = (self.elapsed * 1.8 + enemy.phase).sin_cos();
            render_model(
                &self.enemy_model,
                frame,
                camera,
                |v| {
                    let local = v.sub(center);
                    Vec3::new(
                        local.x * cos - local.z * sin,
                        local.y,
                        local.x * sin + local.z * cos,
                    )
                    .add(enemy.position)
                },
                Some(self.elapsed),
                None,
            );
        }
        let center = self.enemy_bolt.bounds().0;
        for shot in &self.enemy_shots {
            render_model(
                &self.enemy_bolt,
                frame,
                camera,
                |v| v.sub(center).mul(shot.scale).add(shot.position),
                Some(self.elapsed),
                None,
            );
        }
    }

    fn render_seal(&self, frame: &mut Framebuffer, camera: &Camera) {
        let center = self.seal.bounds().0;
        render_model(
            &self.seal,
            frame,
            camera,
            |v| {
                v.sub(center)
                    .add(Vec3::new(0.0, -self.seal_open * 72.0, SEAL_Z))
            },
            Some(self.elapsed),
            None,
        );
        for (index, fragment) in self.fragments.iter().enumerate() {
            if !fragment.collected {
                continue;
            }
            let crystal = &self.crystals[index];
            let crystal_center = crystal.bounds().0;
            let x = [-13.0, 0.0, 13.0][index];
            render_model(
                crystal,
                frame,
                camera,
                |v| {
                    v.sub(crystal_center).mul(0.42).add(Vec3::new(
                        x,
                        36.0 - self.seal_open * 72.0,
                        SEAL_Z - 4.5,
                    ))
                },
                Some(self.elapsed),
                None,
            );
        }
    }

    fn render_reward(&self, frame: &mut Framebuffer, camera: &Camera, ball: &Model) {
        let (position, elapsed) = if self.phase == Phase::Reward {
            let descent = (self.reward_elapsed / 2.2).clamp(0.0, 1.0);
            let smooth = descent * descent * (3.0 - 2.0 * descent);
            (
                self.reward_position
                    .add(Vec3::new(0.0, 13.0 - smooth * 8.0, -5.0)),
                self.reward_elapsed,
            )
        } else if self.phase == Phase::Exploring {
            (
                Vec3::new(0.0, 36.0 + (self.elapsed * 2.0).sin(), CHAMBER_Z),
                0.0,
            )
        } else {
            return;
        };
        let scale = if self.phase == Phase::Reward {
            (1.0 - (elapsed - REWARD_COLLECT_TIME).max(0.0) / 0.55).clamp(0.0, 1.0)
        } else {
            1.0
        };
        if scale <= 0.0 {
            return;
        }
        let center = ball.bounds().0;
        let (sin, cos) = (self.elapsed * 1.8).sin_cos();
        render_model(
            ball,
            frame,
            camera,
            |v| {
                let local = v.sub(center).mul(scale);
                Vec3::new(
                    local.x * cos - local.z * sin,
                    local.y,
                    local.x * sin + local.z * cos,
                )
                .add(position)
            },
            Some(self.elapsed),
            None,
        );
    }

    fn draw_hud(&self, frame: &mut Framebuffer, player_z: f32) {
        let w = frame.width as i32;
        let h = frame.height as i32;
        let scale = (frame.width as f32 / 600.0).round().clamp(1.0, 3.0) as i32;
        draw_selection_label(
            frame,
            "CASTILLO DE LAVA  //  INTERIOR",
            w / 2,
            12,
            scale,
            0xffc16a,
        );
        draw_selection_label(
            frame,
            &format!("FRAGMENTOS {}/3", self.fragment_count()),
            w - 155,
            45,
            scale,
            if self.fragment_count() == 3 {
                0x8fffe1
            } else {
                0xffd15c
            },
        );
        draw_selection_label(
            frame,
            &format!("ESCUDO {}/{}", self.health.max(0), PLAYER_HEALTH),
            110,
            45,
            scale,
            0x8feaff,
        );
        let zone = zone_name(player_z);
        draw_selection_label(frame, zone, w / 2, 45, scale, 0xff8450);
        if self.message_timer > 0.0 {
            draw_selection_label(frame, self.message, w / 2, h - 62, scale, 0xffffff);
        }
        if self.phase == Phase::Reward {
            let text = if self.reward_elapsed < 3.35 {
                "RECIBIENDO ESFERA DE 7 ESTRELLAS"
            } else {
                "NIVEL FINAL COMPLETADO"
            };
            draw_selection_label(frame, text, w / 2, h / 3, scale * 2, 0xffc438);
        } else if self.phase == Phase::Defeat {
            draw_selection_label(
                frame,
                "DERROTA  //  R: REINTENTAR",
                w / 2,
                h / 2,
                scale * 2,
                0xff4545,
            );
        }
    }

    #[cfg(test)]
    fn force_position(&mut self, controller: &mut ShipController, position: Vec3) {
        controller.ship.position = position;
        self.progress = START_Z - position.z;
        self.lateral = position.x;
        self.height = position.y - CONTROL_HEIGHT_OFFSET;
    }
}

fn zone_name(z: f32) -> &'static str {
    if z > -55.0 {
        "1  START HALL"
    } else if z > -165.0 {
        "2  MOLTEN FURNACE"
    } else if z > -275.0 {
        "3  FRAGMENT SANCTUMS"
    } else if z > -410.0 {
        "4  SEAL GATE CORRIDOR"
    } else {
        "5  DRAGON BALL CHAMBER"
    }
}

fn interior_camera(controller: &ShipController) -> Camera {
    let (forward, right, up) = controller.ship.camera_basis();
    Camera {
        position: controller
            .ship
            .position
            .sub(forward.mul(18.0))
            .add(up.mul(6.0)),
        forward,
        right,
        up,
    }
}

fn render_player_and_weapon(
    frame: &mut Framebuffer,
    camera: &Camera,
    ship_model: &Model,
    laser_model: &Model,
    controller: &ShipController,
    elapsed: f32,
) {
    let ship_center = ship_model.bounds().0;
    let (forward, right, up) = controller.ship.basis();
    render_model(
        ship_model,
        frame,
        camera,
        |v| {
            let local = v
                .sub(ship_center)
                .mul(controller.ship.spec.model_scale * 2.2);
            controller.ship.position.add(
                controller
                    .ship
                    .spec
                    .model_to_world(local, forward, right, up),
            )
        },
        None,
        Some(controller.ship.spec),
    );
    let center = laser_model.bounds().0;
    for projectile in controller
        .projectiles
        .iter()
        .filter(|shot| shot.lifetime > 0.0)
    {
        render_model(
            laser_model,
            frame,
            camera,
            |v| v.sub(center).mul(projectile.scale).add(projectile.position),
            None,
            Some(controller.ship.spec),
        );
    }
    draw_projectile_glow(
        frame,
        &controller.projectiles,
        camera,
        elapsed,
        controller.ship.spec.laser_color,
        1.65,
    );
}

fn draw_interior_sky(frame: &mut Framebuffer) {
    for y in 0..frame.height {
        let t = y as f32 / frame.height as f32;
        let color = (((22.0 + t * 42.0) as u32) << 16)
            | (((5.0 + t * 10.0) as u32) << 8)
            | (8.0 + t * 6.0) as u32;
        frame.pixels[y * frame.width..(y + 1) * frame.width].fill(color);
    }
}

fn build_hazards() -> Vec<Hazard> {
    vec![
        Hazard {
            position: Vec3::new(-20.0, 28.0, -82.0),
            kind: HazardKind::FireJet,
            phase: 0.0,
        },
        Hazard {
            position: Vec3::new(20.0, 30.0, -116.0),
            kind: HazardKind::FireJet,
            phase: 2.1,
        },
        Hazard {
            position: Vec3::new(-18.0, 31.0, -148.0),
            kind: HazardKind::FireJet,
            phase: 4.2,
        },
        Hazard {
            position: Vec3::new(0.0, 36.0, -294.0),
            kind: HazardKind::Rotor,
            phase: 0.0,
        },
        Hazard {
            position: Vec3::new(0.0, 36.0, -322.0),
            kind: HazardKind::LaserHorizontal,
            phase: 0.0,
        },
        Hazard {
            position: Vec3::new(0.0, 36.0, -342.0),
            kind: HazardKind::LaserVertical,
            phase: 0.0,
        },
        Hazard {
            position: Vec3::new(0.0, 20.0, -360.0),
            kind: HazardKind::Spikes,
            phase: 1.4,
        },
        Hazard {
            position: Vec3::new(0.0, 24.0, -72.0),
            kind: HazardKind::MovingPlatform,
            phase: 0.0,
        },
        Hazard {
            position: Vec3::new(0.0, 42.0, -108.0),
            kind: HazardKind::MovingPlatform,
            phase: 2.1,
        },
        Hazard {
            position: Vec3::new(0.0, 29.0, -144.0),
            kind: HazardKind::MovingPlatform,
            phase: 4.2,
        },
    ]
}

fn build_enemies() -> Vec<InteriorEnemy> {
    [
        Vec3::new(-9.0, 31.0, -68.0),
        Vec3::new(9.0, 35.0, -91.0),
        Vec3::new(-7.0, 37.0, -116.0),
        Vec3::new(8.0, 30.0, -141.0),
        Vec3::new(0.0, 34.0, -166.0),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, position)| InteriorEnemy {
        position,
        anchor: position,
        health: 3,
        phase: index as f32 * 1.37,
    })
    .collect()
}

fn build_zones() -> [Model; 5] {
    [
        load_obj("assets/levels/lava_castle_interior/start_hall.obj")
            .unwrap_or_else(|_| build_start_hall()),
        load_obj("assets/levels/lava_castle_interior/molten_furnace.obj")
            .unwrap_or_else(|_| build_furnace()),
        load_obj("assets/levels/lava_castle_interior/fragment_sanctums.obj")
            .unwrap_or_else(|_| build_sanctums()),
        load_obj("assets/levels/lava_castle_interior/seal_gate_corridor.obj")
            .unwrap_or_else(|_| build_corridor()),
        load_obj("assets/levels/lava_castle_interior/dragon_ball_chamber.obj")
            .unwrap_or_else(|_| build_chamber()),
    ]
}

fn build_zone_lods() -> [Model; 5] {
    [
        load_obj("assets/levels/lava_castle_interior/start_hall_lod.obj")
            .unwrap_or_else(|_| build_start_hall()),
        load_obj("assets/levels/lava_castle_interior/molten_furnace_lod.obj")
            .unwrap_or_else(|_| build_furnace()),
        load_obj("assets/levels/lava_castle_interior/fragment_sanctums_lod.obj")
            .unwrap_or_else(|_| build_sanctums()),
        load_obj("assets/levels/lava_castle_interior/seal_gate_corridor_lod.obj")
            .unwrap_or_else(|_| build_corridor()),
        load_obj("assets/levels/lava_castle_interior/dragon_ball_chamber_lod.obj")
            .unwrap_or_else(|_| build_chamber()),
    ]
}

fn shell(builder: &mut Builder, z: f32, length: f32, width: f32, height: f32, name: &str) {
    builder.cube(
        Vec3::new(0.0, 3.0, z),
        Vec3::new(width, 3.0, length),
        [65, 43, 43],
        "Castle_Rock",
        name,
    );
    builder.cube(
        Vec3::new(-width, height * 0.5, z),
        Vec3::new(3.0, height * 0.5, length),
        [48, 30, 34],
        "Castle_Rock_Dark",
        name,
    );
    builder.cube(
        Vec3::new(width, height * 0.5, z),
        Vec3::new(3.0, height * 0.5, length),
        [48, 30, 34],
        "Castle_Rock_Dark",
        name,
    );
    builder.cube(
        Vec3::new(0.0, height, z),
        Vec3::new(width, 3.0, length),
        [42, 25, 30],
        "Castle_Bricks_Dark",
        name,
    );
}

fn build_start_hall() -> Model {
    let mut b = Builder::new();
    shell(&mut b, START_Z - 24.0, 55.0, 48.0, 78.0, "start_hall");
    b.cylinder(
        Vec3::new(0.0, 7.0, START_Z),
        18.0,
        3.0,
        24,
        [85, 61, 62],
        "Castle_Bricks",
        "entry_platform",
    );
    for x in [-30.0, 30.0] {
        for z in [18.0, -12.0, -42.0] {
            b.cube(
                Vec3::new(x, 38.0, z),
                Vec3::new(1.0, 12.0, 0.7),
                [220, 36, 14],
                "Castle_Lava_Hot",
                "magma_torch",
            );
        }
    }
    b.finish()
}

fn build_furnace() -> Model {
    let mut b = Builder::new();
    shell(&mut b, FURNACE_Z, 58.0, 48.0, 78.0, "molten_furnace");
    for z in [-158.0, -132.0, -106.0, -80.0, -54.0] {
        b.cube(
            Vec3::new(0.0, 8.0, z),
            Vec3::new(43.0, 1.2, 5.0),
            [255, 55, 4],
            "Castle_Lava",
            "lava_channel",
        );
    }
    b.finish()
}

fn build_sanctums() -> Model {
    let mut b = Builder::new();
    shell(&mut b, SANCTUM_Z, 56.0, 50.0, 82.0, "fragment_sanctums");
    for (x, y, z, color, name) in [
        (-36.0, 25.0, SANCTUM_Z - 3.0, [28, 115, 220], "blue_sanctum"),
        (0.0, 39.0, SANCTUM_Z - 28.0, [255, 85, 10], "orange_sanctum"),
        (
            36.0,
            18.0,
            SANCTUM_Z - 3.0,
            [145, 35, 205],
            "purple_sanctum",
        ),
    ] {
        b.cylinder(
            Vec3::new(x, y, z),
            11.0,
            4.0,
            20,
            color,
            "Castle_Portal_Core",
            name,
        );
        for side in [-1.0, 1.0] {
            b.cube(
                Vec3::new(x + side * 9.0, y + 12.0, z),
                Vec3::new(1.6, 13.0, 1.6),
                [77, 54, 62],
                "Castle_Bricks",
                "sanctum_arch",
            );
        }
    }
    b.finish()
}

fn build_corridor() -> Model {
    let mut b = Builder::new();
    shell(
        &mut b,
        CORRIDOR_Z - 8.0,
        66.0,
        39.0,
        72.0,
        "seal_gate_corridor",
    );
    for z in [-288.0, -316.0, -344.0, -372.0] {
        b.torus(
            Vec3::new(0.0, 36.0, z),
            29.0,
            2.2,
            24,
            [96, 63, 61],
            "Castle_Bricks",
            "mechanical_ring",
        );
    }
    b.cube(
        Vec3::new(0.0, 8.0, CORRIDOR_Z),
        Vec3::new(34.0, 1.0, 64.0),
        [101, 75, 70],
        "Castle_Bricks",
        "lava_bridge",
    );
    b.finish()
}

fn build_chamber() -> Model {
    let mut b = Builder::new();
    shell(
        &mut b,
        CHAMBER_Z + 18.0,
        58.0,
        52.0,
        88.0,
        "dragon_ball_chamber",
    );
    b.cylinder(
        Vec3::new(0.0, 9.0, CHAMBER_Z),
        24.0,
        5.0,
        28,
        [105, 70, 62],
        "Castle_Bricks",
        "final_altar",
    );
    b.torus(
        Vec3::new(0.0, 12.0, CHAMBER_Z),
        18.0,
        1.0,
        28,
        [255, 116, 8],
        "Castle_Lava_Hot",
        "altar_rune",
    );
    for side in [-1.0, 1.0] {
        b.cone(
            Vec3::new(side * 45.0, 50.0, CHAMBER_Z - 4.0),
            8.0,
            15.0,
            10,
            [88, 54, 55],
            "Castle_Rock",
            "dragon_statue",
        );
    }
    b.finish()
}

fn make_crystal(color: [u8; 3], material: &str) -> Model {
    let mut b = Builder::new();
    b.octa(Vec3::default(), 4.0, color, material, "energy_fragment");
    b.finish()
}

fn make_moving_platform() -> Model {
    let mut b = Builder::new();
    b.cube(
        Vec3::default(),
        Vec3::new(10.0, 2.0, 7.0),
        [102, 72, 69],
        "Castle_Bricks",
        "moving_platform",
    );
    b.cube(
        Vec3::new(0.0, 2.2, 0.0),
        Vec3::new(5.0, 0.35, 3.0),
        [255, 116, 12],
        "Castle_Lava_Hot",
        "platform_rune",
    );
    b.finish()
}

fn make_interior_enemy() -> Model {
    let mut b = Builder::new();
    b.octa(
        Vec3::default(),
        3.8,
        [72, 49, 55],
        "Castle_Rock_Dark",
        "lava_sentinel_body",
    );
    for side in [-1.0, 1.0] {
        b.cube(
            Vec3::new(side * 4.2, 0.0, 0.0),
            Vec3::new(3.8, 0.6, 1.5),
            [92, 60, 62],
            "Castle_Bricks",
            "lava_sentinel_wing",
        );
    }
    b.octa(
        Vec3::new(0.0, 0.0, -3.2),
        1.35,
        [255, 96, 8],
        "Castle_Lava_Hot",
        "lava_sentinel_core",
    );
    b.finish()
}

fn make_enemy_bolt() -> Model {
    let mut b = Builder::new();
    b.octa(
        Vec3::default(),
        1.5,
        [255, 58, 8],
        "Castle_Lava_Hot",
        "enemy_lava_bolt",
    );
    b.finish()
}

fn make_hazard_bar() -> Model {
    let mut b = Builder::new();
    b.cube(
        Vec3::default(),
        Vec3::new(42.0, 1.2, 1.2),
        [255, 35, 12],
        "Castle_Lava_Hot",
        "active_hazard",
    );
    b.finish()
}

fn make_seal() -> Model {
    let mut b = Builder::new();
    b.cube(
        Vec3::new(0.0, 36.0, 0.0),
        Vec3::new(36.0, 35.0, 3.5),
        [54, 35, 44],
        "Castle_Bricks_Dark",
        "final_seal",
    );
    for x in [-13.0, 0.0, 13.0] {
        b.cube(
            Vec3::new(x, 36.0, -4.0),
            Vec3::new(2.3, 18.0, 0.7),
            [255, 112, 12],
            "Castle_Lava_Hot",
            "fragment_slot",
        );
    }
    b.finish()
}

struct Builder {
    vertices: Vec<Vec3>,
    faces: Vec<Face>,
}

impl Builder {
    fn new() -> Self {
        Self {
            vertices: Vec::new(),
            faces: Vec::new(),
        }
    }
    fn tri(&mut self, a: Vec3, b: Vec3, c: Vec3, color: [u8; 3], material: &str, group: &str) {
        let base = self.vertices.len();
        self.vertices.extend([a, b, c]);
        let mut face = Face::new([base, base + 1, base + 2], material, group);
        face.color = crate::obj_loader::MaterialColor::Fixed(color);
        self.faces.push(face);
    }
    fn cube(&mut self, c: Vec3, s: Vec3, color: [u8; 3], material: &str, group: &str) {
        let p = [
            Vec3::new(c.x - s.x, c.y - s.y, c.z - s.z),
            Vec3::new(c.x + s.x, c.y - s.y, c.z - s.z),
            Vec3::new(c.x + s.x, c.y + s.y, c.z - s.z),
            Vec3::new(c.x - s.x, c.y + s.y, c.z - s.z),
            Vec3::new(c.x - s.x, c.y - s.y, c.z + s.z),
            Vec3::new(c.x + s.x, c.y - s.y, c.z + s.z),
            Vec3::new(c.x + s.x, c.y + s.y, c.z + s.z),
            Vec3::new(c.x - s.x, c.y + s.y, c.z + s.z),
        ];
        for q in [
            [0, 2, 1],
            [0, 3, 2],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [3, 7, 6],
            [3, 6, 2],
            [0, 4, 7],
            [0, 7, 3],
            [1, 2, 6],
            [1, 6, 5],
        ] {
            self.tri(p[q[0]], p[q[1]], p[q[2]], color, material, group);
        }
    }
    fn cylinder(
        &mut self,
        c: Vec3,
        r: f32,
        depth: f32,
        segments: usize,
        color: [u8; 3],
        material: &str,
        group: &str,
    ) {
        let y0 = c.y - depth * 0.5;
        let y1 = c.y + depth * 0.5;
        for i in 0..segments {
            let a = i as f32 * TAU / segments as f32;
            let b = (i + 1) as f32 * TAU / segments as f32;
            let p0 = Vec3::new(c.x + a.cos() * r, y0, c.z + a.sin() * r);
            let p1 = Vec3::new(c.x + b.cos() * r, y0, c.z + b.sin() * r);
            let p2 = Vec3::new(c.x + b.cos() * r, y1, c.z + b.sin() * r);
            let p3 = Vec3::new(c.x + a.cos() * r, y1, c.z + a.sin() * r);
            self.tri(p0, p1, p2, color, material, group);
            self.tri(p0, p2, p3, color, material, group);
            self.tri(Vec3::new(c.x, y0, c.z), p1, p0, color, material, group);
            self.tri(Vec3::new(c.x, y1, c.z), p3, p2, color, material, group);
        }
    }
    fn torus(
        &mut self,
        c: Vec3,
        major: f32,
        minor: f32,
        segments: usize,
        color: [u8; 3],
        material: &str,
        group: &str,
    ) {
        let rings = 6;
        for i in 0..segments {
            for j in 0..rings {
                let a = i as f32 * TAU / segments as f32;
                let b = (i + 1) as f32 * TAU / segments as f32;
                let u = j as f32 * TAU / rings as f32;
                let v = (j + 1) as f32 * TAU / rings as f32;
                let point = |q: f32, t: f32| {
                    Vec3::new(
                        c.x + (major + minor * t.cos()) * q.cos(),
                        c.y + minor * t.sin(),
                        c.z + (major + minor * t.cos()) * q.sin(),
                    )
                };
                let p0 = point(a, u);
                let p1 = point(b, u);
                let p2 = point(b, v);
                let p3 = point(a, v);
                self.tri(p0, p1, p2, color, material, group);
                self.tri(p0, p2, p3, color, material, group);
            }
        }
    }
    fn cone(
        &mut self,
        c: Vec3,
        r: f32,
        height: f32,
        segments: usize,
        color: [u8; 3],
        material: &str,
        group: &str,
    ) {
        let top = Vec3::new(c.x, c.y + height * 0.5, c.z);
        let y = c.y - height * 0.5;
        for i in 0..segments {
            let a = i as f32 * TAU / segments as f32;
            let b = (i + 1) as f32 * TAU / segments as f32;
            let p0 = Vec3::new(c.x + a.cos() * r, y, c.z + a.sin() * r);
            let p1 = Vec3::new(c.x + b.cos() * r, y, c.z + b.sin() * r);
            self.tri(p0, p1, top, color, material, group);
            self.tri(Vec3::new(c.x, y, c.z), p1, p0, color, material, group);
        }
    }
    fn octa(&mut self, c: Vec3, r: f32, color: [u8; 3], material: &str, group: &str) {
        let top = c.add(Vec3::new(0.0, r * 1.6, 0.0));
        let bottom = c.add(Vec3::new(0.0, -r * 1.6, 0.0));
        let ring = [
            c.add(Vec3::new(r, 0.0, 0.0)),
            c.add(Vec3::new(0.0, 0.0, r)),
            c.add(Vec3::new(-r, 0.0, 0.0)),
            c.add(Vec3::new(0.0, 0.0, -r)),
        ];
        for i in 0..4 {
            let n = (i + 1) % 4;
            self.tri(top, ring[i], ring[n], color, material, group);
            self.tri(bottom, ring[n], ring[i], color, material, group);
        }
    }
    fn finish(self) -> Model {
        Model::new(self.vertices, self.faces)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ship::{ARWING_SPEC, ShipController};

    #[test]
    fn interior_contains_five_distinct_volumetric_zones() {
        let level = LavaCastleInterior::new();
        assert_eq!(level.zones.len(), 5);
        for ((zone, group), expected_z) in level
            .zones
            .iter()
            .zip([
                "StartHall_Floor",
                "Furnace_LavaChannel_0",
                "Sanctum_Blue_Pedestal",
                "SealCorridor_MechanicalRing_0",
                "DragonBallChamber_Altar",
            ])
            .zip([0.0, FURNACE_Z, SANCTUM_Z, -333.0, -452.0])
        {
            assert!(zone.faces.iter().any(|face| face.group == group));
            assert!(zone.faces.len() > 1_000);
            assert!(zone.faces.len() < 8_000);
            let center = zone.bounds().0;
            assert!((center.z - expected_z).abs() < 70.0);
            assert!(center.y > 10.0 && center.y < 70.0);
        }
    }

    #[test]
    fn blender_interior_preserves_lava_stone_and_gothic_details() {
        let level = LavaCastleInterior::new();
        let faces = level.zones.iter().flat_map(|zone| &zone.faces);
        let materials = faces.map(|face| face.material.as_str()).collect::<Vec<_>>();
        for material in [
            "Castle_Rock",
            "Castle_Bricks",
            "Castle_Lava",
            "Castle_Lava_Hot",
        ] {
            assert!(materials.contains(&material));
        }
        assert!(
            level.zones[0]
                .faces
                .iter()
                .any(|face| face.group.starts_with("StartHall_Chain"))
        );
        assert!(
            level.zones[1]
                .faces
                .iter()
                .any(|face| face.group.starts_with("Furnace_Gear"))
        );
        assert!(
            level.zones[4]
                .faces
                .iter()
                .any(|face| face.group.starts_with("DragonBallChamber_Dragon"))
        );
    }

    #[test]
    fn distant_zone_lods_reduce_geometry_substantially() {
        let level = LavaCastleInterior::new();
        for (detailed, lod) in level.zones.iter().zip(&level.zone_lods) {
            assert!(lod.faces.len() < detailed.faces.len());
            assert!(lod.faces.len() * 100 < detailed.faces.len() * 55);
        }
    }

    #[test]
    fn exactly_three_fragments_are_collectible_once() {
        let mut level = LavaCastleInterior::new();
        let mut controller = ShipController::new(&ARWING_SPEC);
        level.enter(&mut controller);
        for fragment in level.fragments {
            level.force_position(&mut controller, fragment.position);
            level.collect_fragments(controller.ship.position);
        }
        assert_eq!(level.fragment_count(), 3);
        level.collect_fragments(controller.ship.position);
        assert_eq!(level.fragment_count(), 3);
    }

    #[test]
    fn final_seal_never_opens_without_all_fragments() {
        let mut level = LavaCastleInterior::new();
        let mut controller = ShipController::new(&ARWING_SPEC);
        level.enter(&mut controller);
        level.force_position(&mut controller, Vec3::new(0.0, 36.0, SEAL_Z - 5.0));
        level.update(&mut controller, &ShipInput::default(), false, false, 0.05);
        assert_eq!(level.seal_open, 0.0);
        assert!(controller.ship.position.z >= SEAL_Z + 8.0);
    }

    #[test]
    fn three_fragments_open_seal_and_final_ball_triggers_victory() {
        let mut level = LavaCastleInterior::new();
        let mut controller = ShipController::new(&ARWING_SPEC);
        level.enter(&mut controller);
        for fragment in &mut level.fragments {
            fragment.collected = true;
        }
        for _ in 0..50 {
            level.update(&mut controller, &ShipInput::default(), false, false, 0.05);
        }
        assert_eq!(level.seal_open, 1.0);
        level.force_position(&mut controller, Vec3::new(0.0, 36.0, CHAMBER_Z));
        level.update(&mut controller, &ShipInput::default(), false, false, 0.01);
        assert_eq!(level.phase, Phase::Reward);
        for _ in 0..112 {
            level.update(&mut controller, &ShipInput::default(), false, false, 0.05);
        }
        assert_eq!(
            level.update(&mut controller, &ShipInput::default(), false, false, 0.05),
            Action::Victory
        );
    }

    #[test]
    fn hazards_use_timed_activation_and_invulnerability() {
        let mut level = LavaCastleInterior::new();
        let mut position = level.checkpoint;
        level.damage_player(&mut position);
        level.damage_player(&mut position);
        assert_eq!(level.health, PLAYER_HEALTH - 1);
    }

    #[test]
    fn interior_uses_water_level_lateral_and_vertical_response() {
        let mut level = LavaCastleInterior::new();
        let mut controller = ShipController::new(&ARWING_SPEC);
        level.enter(&mut controller);
        let input = ShipInput {
            yaw: 1,
            pitch: 1,
            ..ShipInput::default()
        };
        for _ in 0..4 {
            level.update(&mut controller, &input, false, false, 0.05);
        }
        assert!((level.lateral - 3.4).abs() < 0.001);
        assert!((level.height - 8.0).abs() < 0.001);
        assert_eq!(controller.ship.yaw, FRAC_PI_2);
        assert_eq!(controller.ship.pitch, 0.0);
    }

    #[test]
    fn enemies_return_and_blender_pillars_are_completely_removed() {
        let mut level = LavaCastleInterior::new();
        assert_eq!(level.enemies.len(), 5);
        for group in [
            "LavaSentinel_CoreCage",
            "LavaSentinel_EnergyCore",
            "LavaSentinel_WingBlade_1",
            "LavaSentinel_Horn_-1",
            "LavaSentinel_ThrusterFlame_1",
        ] {
            assert!(
                level
                    .enemy_model
                    .faces
                    .iter()
                    .any(|face| face.group == group)
            );
        }
        assert!(
            level
                .zones
                .iter()
                .flat_map(|zone| &zone.faces)
                .all(|face| { !face.group.to_ascii_lowercase().contains("pillar") })
        );
        let target = level.enemies[0].position;
        let mut shots = vec![Projectile {
            position: target,
            direction: Vec3::new(0.0, 0.0, -1.0),
            lifetime: 1.0,
            scale: 2.0,
        }];
        level.resolve_player_fire(&mut shots);
        assert_eq!(level.enemies[0].health, 1);
    }
}
