use crate::framebuffer::Framebuffer;
use crate::obj_loader::{Face, MaterialColor, Model, SurfaceShader, Vec3, load_obj};
use crate::ship::{Projectile, ShipController, ShipInput};
use crate::{Camera, draw_selection_label, render_model};

const BOSS_MAX_HEALTH: i32 = 160;
const BOSS_BODY_RADIUS: f32 = 27.0;
const BOSS_CORE_RADIUS: f32 = 7.0;
const PLAYER_MAX_HEALTH: i32 = 6;
const ARENA_HALF_WIDTH: f32 = 115.0;
const ARENA_MIN_Y: f32 = 8.0;
const ARENA_MAX_Y: f32 = 105.0;
const ARENA_MIN_Z: f32 = -210.0;
const ARENA_MAX_Z: f32 = 52.0;
const PLAYER_PLANE_Z: f32 = 32.0;
const PLANE_SPEED_MULTIPLIER: f32 = 2.25;
const VERTICAL_SPEED_MULTIPLIER: f32 = 2.0;
const REWARD_DURATION: f32 = 5.5;
const REWARD_COLLECT_TIME: f32 = 2.8;
const REWARD_RECEIVE_TIME: f32 = 3.35;
const DOOR_LOWER_DISTANCE: f32 = 70.0;
const BOSS_POSITION: Vec3 = Vec3::new(0.0, 58.0, -128.0);
const DOOR_POSITION: Vec3 = Vec3::new(0.0, 34.0, -191.0);

#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    pub pitch: i8,
    pub yaw: i8,
    pub roll: i8,
    pub accelerate: bool,
    pub brake: bool,
    pub boost: bool,
    pub fire: bool,
    pub restart: bool,
    pub exit: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Continue,
    Exit,
    InteriorTransition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Waiting,
    Combat,
    Enraged,
    Dying,
    DoorOpening,
    Reward,
    Complete,
    Defeat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Attack {
    None,
    LavaVolley,
    BeamWarning,
    Beam,
    EruptionWarning,
    Eruption,
}

struct EnemyShot {
    position: Vec3,
    velocity: Vec3,
    lifetime: f32,
}

pub struct LavaCastle {
    phase: Phase,
    attack: Attack,
    attack_timer: f32,
    attack_cooldown: f32,
    attack_target: Vec3,
    boss_health: i32,
    boss_position: Vec3,
    boss_core_offset: Vec3,
    boss_hit_flash: f32,
    player_health: i32,
    player_invulnerability: f32,
    player_velocity: Vec3,
    enemy_shots: Vec<EnemyShot>,
    elapsed: f32,
    destruction_timer: f32,
    door_open: f32,
    reward_elapsed: f32,
    reward_position: Vec3,
    message: &'static str,
    message_timer: f32,
    environment: Model,
    boss: Model,
    door: Model,
    projectile: Model,
}

impl LavaCastle {
    pub fn new() -> Self {
        let boss = load_obj("assets/bosses/lava_guardian/guardian_lava_gigante_optimized.obj")
            .unwrap_or_else(|error| {
                eprintln!("No se pudo cargar el Guardian de Lava Blender: {error}");
                make_boss()
            });
        let boss_core_offset = model_group_center(&boss, "Guardian_Core_Inner")
            .unwrap_or_else(|| boss.bounds().0)
            .sub(boss.bounds().0);
        Self {
            phase: Phase::Waiting,
            attack: Attack::None,
            attack_timer: 0.0,
            attack_cooldown: 1.4,
            attack_target: Vec3::default(),
            boss_health: BOSS_MAX_HEALTH,
            boss_position: BOSS_POSITION,
            boss_core_offset,
            boss_hit_flash: 0.0,
            player_health: PLAYER_MAX_HEALTH,
            player_invulnerability: 0.0,
            player_velocity: Vec3::default(),
            enemy_shots: Vec::with_capacity(48),
            elapsed: 0.0,
            destruction_timer: 0.0,
            door_open: 0.0,
            reward_elapsed: 0.0,
            reward_position: Vec3::default(),
            message: "ACERCATE AL GUARDIAN DE LAVA",
            message_timer: 3.0,
            environment: load_obj(
                "assets/levels/lava_castle/lava_castle_environment_optimized.obj",
            )
            .unwrap_or_else(|error| {
                eprintln!("No se pudo cargar el escenario Blender del castillo: {error}");
                make_environment()
            }),
            boss,
            door: load_obj("assets/levels/lava_castle/lava_castle_door.obj").unwrap_or_else(
                |error| {
                    eprintln!("No se pudo cargar la puerta Blender del castillo: {error}");
                    make_door()
                },
            ),
            projectile: make_fireball(),
        }
    }

    pub fn enter(&mut self, controller: &mut ShipController) {
        *self = Self::new();
        controller.projectiles.clear();
        controller.ship.position = Vec3::new(0.0, 38.0, PLAYER_PLANE_Z);
        controller.ship.yaw = std::f32::consts::FRAC_PI_2;
        controller.ship.pitch = 0.0;
        controller.ship.roll = 0.0;
        controller.ship.yaw_rate = 0.0;
        controller.ship.pitch_rate = 0.0;
        controller.ship.speed = controller.ship.spec.cruise_speed;
    }

    pub fn update(
        &mut self,
        controller: &mut ShipController,
        input: Input,
        delta_time: f32,
    ) -> Action {
        if input.exit {
            return Action::Exit;
        }
        if input.restart && matches!(self.phase, Phase::Defeat | Phase::Complete) {
            self.enter(controller);
            return Action::Continue;
        }
        let dt = delta_time.clamp(0.0, 0.05);
        self.elapsed += dt;
        self.message_timer = (self.message_timer - dt).max(0.0);
        self.boss_hit_flash = (self.boss_hit_flash - dt).max(0.0);
        self.player_invulnerability = (self.player_invulnerability - dt).max(0.0);

        if self.phase == Phase::Defeat {
            return Action::Continue;
        }

        if !matches!(
            self.phase,
            Phase::Dying | Phase::DoorOpening | Phase::Reward
        ) {
            self.update_player_plane(controller, input, dt);
            controller.update_weapon_and_projectiles(input.fire, dt);
        }

        let distance = controller.ship.position.sub(self.boss_position);
        if self.phase == Phase::Waiting && distance.dot(distance) < 220.0 * 220.0 {
            self.phase = Phase::Combat;
            self.message = "GUARDIAN DE LAVA DESPERTADO";
            self.message_timer = 2.4;
        }

        if matches!(self.phase, Phase::Combat | Phase::Enraged) {
            self.update_boss_motion(dt);
            self.resolve_player_shots(&mut controller.projectiles);
            self.update_attack(controller.ship.position, dt);
            self.update_enemy_shots(controller.ship.position, dt);
            self.resolve_enemy_damage(&mut controller.ship.position);
            if self.boss_health <= BOSS_MAX_HEALTH / 2 && self.phase == Phase::Combat {
                self.phase = Phase::Enraged;
                self.message = "SEGUNDA FASE  //  NUCLEO SOBRECARGADO";
                self.message_timer = 3.0;
                self.attack_cooldown = 0.25;
            }
            if self.boss_health <= 0 {
                self.phase = Phase::Dying;
                self.attack = Attack::None;
                self.enemy_shots.clear();
                self.destruction_timer = 2.6;
                self.message = "GUARDIAN DE LAVA DESTRUIDO";
                self.message_timer = 3.0;
            }
        }

        if self.phase == Phase::Dying {
            self.destruction_timer -= dt;
            if self.destruction_timer <= 0.0 {
                self.phase = Phase::DoorOpening;
                self.message = "ENTRADA AL CASTILLO DESBLOQUEADA";
                self.message_timer = 4.0;
            }
        } else if self.phase == Phase::DoorOpening {
            self.door_open = (self.door_open + dt / 2.2).min(1.0);
            if self.door_open >= 1.0 {
                self.phase = Phase::Reward;
                self.reward_elapsed = 0.0;
                self.reward_position = controller.ship.position;
                controller.ship.speed = 0.0;
                self.message = "RECIBIENDO ESFERA DEL DRAGON";
                self.message_timer = REWARD_RECEIVE_TIME;
            }
        } else if self.phase == Phase::Reward {
            self.reward_elapsed = (self.reward_elapsed + dt).min(REWARD_DURATION);
            controller.ship.position = self.reward_position;
            controller.ship.speed = 0.0;
            self.player_velocity = Vec3::default();
            if self.reward_elapsed >= REWARD_DURATION {
                self.phase = Phase::Complete;
            }
        } else if self.phase == Phase::Complete {
            return Action::InteriorTransition;
        }

        if self.player_health <= 0 {
            self.phase = Phase::Defeat;
            self.message = "NAVE DESTRUIDA  //  R PARA REINTENTAR";
            self.message_timer = 999.0;
        }
        Action::Continue
    }

    fn update_player_plane(&mut self, controller: &mut ShipController, input: Input, dt: f32) {
        let horizontal = input.yaw as f32;
        let vertical = input.pitch as f32;
        // Use the selected ship's exact free-world steering response and speed
        // curve, then project those turn rates onto this fixed combat plane.
        controller.ship.update_flight(
            &ShipInput {
                pitch: input.pitch,
                yaw: input.yaw,
                roll: input.roll,
                accelerating: input.accelerate || input.boost,
                braking: input.brake,
                firing: input.fire,
            },
            dt,
        );
        let yaw_sensitivity = if controller.ship.spec.max_yaw_rate > 0.0 {
            controller.ship.yaw_rate / controller.ship.spec.max_yaw_rate
        } else {
            horizontal
        };
        let pitch_sensitivity = if controller.ship.spec.max_pitch_rate > 0.0 {
            controller.ship.pitch_rate / controller.ship.spec.max_pitch_rate
        } else {
            vertical
        };
        self.player_velocity.x = yaw_sensitivity * controller.ship.speed * PLANE_SPEED_MULTIPLIER;
        self.player_velocity.y = pitch_sensitivity
            * controller.ship.speed
            * PLANE_SPEED_MULTIPLIER
            * VERTICAL_SPEED_MULTIPLIER;
        controller.ship.position.x += self.player_velocity.x * dt;
        controller.ship.position.y += self.player_velocity.y * dt;
        controller.ship.position.z = PLAYER_PLANE_Z;
        controller.ship.yaw = std::f32::consts::FRAC_PI_2;
        let visual_speed = controller.ship.spec.max_speed.max(0.001);
        controller.ship.pitch = (-self.player_velocity.y / visual_speed * 0.18).clamp(-0.18, 0.18);
        let manual_roll = input.roll as f32 * 0.72;
        let movement_bank = (-self.player_velocity.x / visual_speed * 0.48).clamp(-0.48, 0.48);
        controller.ship.roll_target = manual_roll + movement_bank;
        let roll_response = 1.0 - (-9.0 * dt).exp();
        controller.ship.roll +=
            (controller.ship.roll_target - controller.ship.roll) * roll_response;
        clamp_ship_to_plane(&mut controller.ship.position);
    }

    fn update_boss_motion(&mut self, dt: f32) {
        let speed = if self.phase == Phase::Enraged {
            1.55
        } else {
            1.0
        };
        self.boss_position.x = (self.elapsed * 0.72 * speed).sin() * 48.0;
        self.boss_position.y = 56.0 + (self.elapsed * 1.35).sin() * 9.0;
        let _ = dt;
    }

    fn resolve_player_shots(&mut self, shots: &mut [Projectile]) {
        for shot in shots.iter_mut().filter(|shot| shot.lifetime > 0.0) {
            let offset = shot.position.sub(self.boss_position);
            let body_hit = offset.dot(offset)
                < (BOSS_BODY_RADIUS + shot.scale) * (BOSS_BODY_RADIUS + shot.scale);
            if body_hit {
                let core_offset = shot
                    .position
                    .sub(self.boss_position.add(self.boss_core_offset));
                let core_hit = core_offset.dot(core_offset) < BOSS_CORE_RADIUS * BOSS_CORE_RADIUS;
                let base = if shot.scale > 2.0 { 8 } else { 3 };
                self.boss_health -= if core_hit { base * 2 } else { base };
                self.boss_hit_flash = 0.13;
                shot.lifetime = 0.0;
            }
        }
    }

    fn update_attack(&mut self, target: Vec3, dt: f32) {
        self.attack_timer -= dt;
        self.attack_cooldown -= dt;
        match self.attack {
            Attack::None if self.attack_cooldown <= 0.0 => {
                let choice = ((self.elapsed * 0.83) as i32).rem_euclid(3);
                self.attack = match choice {
                    0 => Attack::LavaVolley,
                    1 => Attack::BeamWarning,
                    _ => Attack::EruptionWarning,
                };
                self.attack_timer = match self.attack {
                    Attack::LavaVolley => 0.7,
                    Attack::BeamWarning => 1.25,
                    Attack::EruptionWarning => 1.55,
                    _ => 0.0,
                };
                self.attack_target = target;
                if self.attack == Attack::LavaVolley {
                    self.spawn_volley(target);
                }
            }
            Attack::LavaVolley if self.attack_timer <= 0.0 => self.finish_attack(),
            Attack::BeamWarning if self.attack_timer <= 0.0 => {
                self.attack = Attack::Beam;
                self.attack_timer = 0.65;
            }
            Attack::Beam if self.attack_timer <= 0.0 => self.finish_attack(),
            Attack::EruptionWarning if self.attack_timer <= 0.0 => {
                self.attack = Attack::Eruption;
                self.attack_timer = 0.55;
            }
            Attack::Eruption if self.attack_timer <= 0.0 => self.finish_attack(),
            _ => {}
        }
    }

    fn finish_attack(&mut self) {
        self.attack = Attack::None;
        self.attack_cooldown = if self.phase == Phase::Enraged {
            1.0
        } else {
            1.65
        };
    }

    fn spawn_volley(&mut self, target: Vec3) {
        let direction = target.sub(self.boss_position).normalize();
        for spread in [-0.16_f32, -0.08, 0.0, 0.08, 0.16] {
            if self.enemy_shots.len() >= 48 {
                break;
            }
            let velocity = Vec3::new(direction.x + spread, direction.y, direction.z).normalize();
            self.enemy_shots.push(EnemyShot {
                position: self.boss_position,
                velocity: velocity.mul(if self.phase == Phase::Enraged {
                    58.0
                } else {
                    46.0
                }),
                lifetime: 5.0,
            });
        }
    }

    fn update_enemy_shots(&mut self, target: Vec3, dt: f32) {
        for shot in &mut self.enemy_shots {
            shot.position = shot.position.add(shot.velocity.mul(dt));
            shot.lifetime -= dt;
            if shot.position.x.abs() > ARENA_HALF_WIDTH + 20.0
                || shot.position.y < -10.0
                || shot.position.z > ARENA_MAX_Z + 20.0
                || shot.position.z < ARENA_MIN_Z - 20.0
            {
                shot.lifetime = 0.0;
            }
        }
        self.enemy_shots.retain(|shot| shot.lifetime > 0.0);
        if self.attack == Attack::Beam && self.attack_timer < 0.36 {
            let beam = self.attack_target.sub(self.boss_position).normalize();
            if distance_to_segment(
                target,
                self.boss_position,
                self.boss_position.add(beam.mul(220.0)),
            ) < 3.8
            {
                self.damage_player();
            }
        }
        if self.attack == Attack::Eruption && self.attack_timer < 0.32 {
            let d = target.sub(self.boss_position);
            if d.dot(d) < 52.0 * 52.0 {
                self.damage_player();
            }
        }
    }

    fn resolve_enemy_damage(&mut self, player: &mut Vec3) {
        let mut hit = false;
        for shot in &mut self.enemy_shots {
            let offset = shot.position.sub(*player);
            if offset.dot(offset) < 4.5 * 4.5 {
                shot.lifetime = 0.0;
                hit = true;
            }
        }
        if hit {
            self.damage_player();
        }
        self.enemy_shots.retain(|shot| shot.lifetime > 0.0);
        let boss_offset = player.sub(self.boss_position);
        if boss_offset.dot(boss_offset) < 25.0 * 25.0 {
            self.damage_player();
            *player = player.add(boss_offset.normalize().mul(12.0));
        }
    }

    fn damage_player(&mut self) {
        if self.player_invulnerability <= 0.0 {
            self.player_health -= 1;
            self.player_invulnerability = 1.0;
            self.message = "IMPACTO RECIBIDO";
            self.message_timer = 1.1;
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
        draw_sky(frame);
        draw_embers(frame, self.elapsed);
        frame.buffers_mut().1.fill(f32::NEG_INFINITY);
        let camera = camera_for(
            &controller.ship.position,
            &controller.ship.movement_basis().0,
        );
        render_model(
            &self.environment,
            frame,
            &camera,
            |v| v,
            Some(self.elapsed),
            None,
        );
        let door_y = -self.door_open * DOOR_LOWER_DISTANCE;
        render_model(
            &self.door,
            frame,
            &camera,
            |v| v.add(Vec3::new(0.0, door_y, 0.0)),
            None,
            None,
        );
        if matches!(self.phase, Phase::Waiting | Phase::Combat | Phase::Enraged)
            || (self.phase == Phase::Dying && (self.destruction_timer * 12.0) as i32 % 2 == 0)
        {
            let center = self.boss.bounds().0;
            let scale = if self.phase == Phase::Dying {
                (self.destruction_timer / 2.6).clamp(0.0, 1.0)
            } else if self.boss_hit_flash > 0.0 {
                1.08
            } else {
                1.0
            };
            render_model(
                &self.boss,
                frame,
                &camera,
                |v| v.sub(center).mul(scale).add(self.boss_position),
                Some(self.elapsed),
                None,
            );
        }
        for shot in &self.enemy_shots {
            let center = self.projectile.bounds().0;
            render_model(
                &self.projectile,
                frame,
                &camera,
                |v| v.sub(center).add(shot.position),
                Some(self.elapsed),
                None,
            );
        }
        self.render_player_weapon(frame, &camera, laser_model, controller);
        self.render_special_attack(frame, &camera);
        self.render_reward(frame, &camera, dragon_ball);
        let ship_center = ship_model.bounds().0;
        let (forward, right, up) = controller.ship.basis();
        render_model(
            ship_model,
            frame,
            &camera,
            |v| {
                let local = v
                    .sub(ship_center)
                    .mul(controller.ship.spec.model_scale * 2.5);
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
        self.draw_hud(frame);
    }

    fn render_reward(&self, frame: &mut Framebuffer, camera: &Camera, dragon_ball: &Model) {
        if self.phase != Phase::Reward {
            return;
        }
        let descent = (self.reward_elapsed / 2.2).clamp(0.0, 1.0);
        let descent = descent * descent * (3.0 - 2.0 * descent);
        let hover = (self.reward_elapsed * 3.2).sin() * 0.22;
        let position = self
            .reward_position
            .add(Vec3::new(0.0, 13.0 - descent * 8.0 + hover, -5.0));
        let center = dragon_ball.bounds().0;
        let rotation = self.reward_elapsed * 1.8;
        let (sin, cos) = rotation.sin_cos();
        let collect_scale =
            (1.0 - (self.reward_elapsed - REWARD_COLLECT_TIME).max(0.0) / 0.55).clamp(0.0, 1.0);
        if collect_scale <= 0.0 {
            return;
        }
        render_model(
            dragon_ball,
            frame,
            camera,
            |vertex| {
                let local = vertex.sub(center).mul(collect_scale);
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

    fn render_player_weapon(
        &self,
        frame: &mut Framebuffer,
        camera: &Camera,
        laser_model: &Model,
        controller: &ShipController,
    ) {
        for projectile in controller
            .projectiles
            .iter()
            .filter(|projectile| projectile.lifetime > 0.0)
        {
            let mut right = projectile
                .direction
                .cross(Vec3::new(0.0, 1.0, 0.0))
                .normalize();
            if right.dot(right) < 0.001 {
                right = Vec3::new(1.0, 0.0, 0.0);
            }
            let up = right.cross(projectile.direction).normalize();
            render_model(
                laser_model,
                frame,
                camera,
                |vertex| {
                    projectile
                        .position
                        .add(projectile.direction.mul(vertex.x * projectile.scale))
                        .add(up.mul(vertex.y * projectile.scale))
                        .add(right.mul(vertex.z * projectile.scale))
                },
                None,
                Some(controller.ship.spec),
            );
        }
        crate::draw_projectile_glow(
            frame,
            &controller.projectiles,
            camera,
            self.elapsed,
            controller.ship.spec.laser_color,
            1.65,
        );

        let charge = controller.charge_ratio();
        if charge > 0.0 {
            let (forward, right, up) = controller.ship.basis();
            let center = controller
                .ship
                .position
                .add(forward.mul(controller.ship.spec.muzzle_forward + 0.4));
            let scale = 0.35 + charge * 1.15;
            render_model(
                laser_model,
                frame,
                camera,
                |vertex| {
                    center
                        .add(forward.mul(vertex.x * scale))
                        .add(up.mul(vertex.y * scale))
                        .add(right.mul(vertex.z * scale))
                },
                None,
                Some(controller.ship.spec),
            );
        }
    }

    fn render_special_attack(&self, frame: &mut Framebuffer, camera: &Camera) {
        let center = self.projectile.bounds().0;
        if matches!(self.attack, Attack::BeamWarning) {
            let pulse = 1.4 + (self.elapsed * 18.0).sin().abs() * 1.1;
            render_model(
                &self.projectile,
                frame,
                camera,
                |v| v.sub(center).mul(pulse).add(self.boss_position),
                Some(self.elapsed),
                None,
            );
        } else if self.attack == Attack::Beam {
            let direction = self.attack_target.sub(self.boss_position).normalize();
            for step in 0..18 {
                let position = self.boss_position.add(direction.mul(step as f32 * 11.0));
                render_model(
                    &self.projectile,
                    frame,
                    camera,
                    |v| v.sub(center).mul(1.35).add(position),
                    Some(self.elapsed),
                    None,
                );
            }
        } else if matches!(self.attack, Attack::EruptionWarning | Attack::Eruption) {
            let progress = if self.attack == Attack::Eruption {
                (1.0 - self.attack_timer / 0.55).clamp(0.0, 1.0)
            } else {
                0.15 + (self.elapsed * 10.0).sin().abs() * 0.1
            };
            for ray in 0..12 {
                let angle = ray as f32 * std::f32::consts::TAU / 12.0;
                let position = self.boss_position.add(Vec3::new(
                    angle.cos() * progress * 34.0,
                    angle.sin() * progress * 20.0,
                    angle.sin() * progress * 34.0,
                ));
                render_model(
                    &self.projectile,
                    frame,
                    camera,
                    |v| v.sub(center).mul(1.2 + progress).add(position),
                    Some(self.elapsed),
                    None,
                );
            }
        }
    }

    fn draw_hud(&self, frame: &mut Framebuffer) {
        let w = frame.width as i32;
        let h = frame.height as i32;
        let scale = (frame.width as f32 / 600.0).round().clamp(1.0, 3.0) as i32;
        draw_bar(
            frame,
            30,
            34,
            260,
            17,
            self.player_health as f32 / PLAYER_MAX_HEALTH as f32,
            0x4de8ef,
        );
        draw_selection_label(frame, "NAVE", 145, 13, scale, 0xd9f7ff);
        draw_bar(
            frame,
            w / 2 - 240,
            55,
            480,
            19,
            self.boss_health.max(0) as f32 / BOSS_MAX_HEALTH as f32,
            0xff4a20,
        );
        draw_selection_label(frame, "GUARDIAN DE LAVA", w / 2, 22, scale, 0xffc16a);
        if matches!(self.attack, Attack::BeamWarning) {
            draw_selection_label(
                frame,
                "ALERTA: RAYO DE ENERGIA",
                w / 2,
                h / 4,
                scale * 2,
                0xff4545,
            );
        } else if matches!(self.attack, Attack::EruptionWarning) {
            draw_selection_label(
                frame,
                "ALERTA: EXPLOSION VOLCANICA",
                w / 2,
                h / 4,
                scale * 2,
                0xffb13b,
            );
        }
        if self.message_timer > 0.0 {
            draw_selection_label(frame, self.message, w / 2, h - 75, scale, 0xffd27a);
        }
        if self.phase == Phase::Reward && self.reward_elapsed < REWARD_RECEIVE_TIME {
            draw_selection_label(
                frame,
                "RECIBIENDO ESFERA DEL DRAGON",
                w / 2,
                h / 3,
                scale * 2,
                0xffd15c,
            );
        } else if self.phase == Phase::Reward {
            draw_selection_label(
                frame,
                "ESFERA DE 6 ESTRELLAS OBTENIDA",
                w / 2,
                h / 3,
                scale * 2,
                0xffa62b,
            );
            draw_selection_label(
                frame,
                &format!(
                    "AVANZANDO EN {}",
                    (REWARD_DURATION - self.reward_elapsed).ceil() as u32
                ),
                w / 2,
                h / 3 + 42,
                scale,
                0x8feaff,
            );
        } else if self.phase == Phase::Complete {
            draw_selection_label(
                frame,
                "ENTRANDO AL CASTILLO",
                w / 2,
                h / 3,
                scale * 2,
                0x8fffe1,
            );
        }
    }
}

fn clamp_ship_to_plane(p: &mut Vec3) {
    p.x = p.x.clamp(-ARENA_HALF_WIDTH, ARENA_HALF_WIDTH);
    p.y = p.y.clamp(ARENA_MIN_Y, ARENA_MAX_Y);
    p.z = PLAYER_PLANE_Z;
}

fn camera_for(position: &Vec3, forward: &Vec3) -> Camera {
    let world_up = Vec3::new(0.0, 1.0, 0.0);
    let camera_position = position.sub(forward.mul(18.0)).add(world_up.mul(7.0));
    let camera_forward = position
        .add(forward.mul(13.0))
        .sub(camera_position)
        .normalize();
    let right = camera_forward.cross(world_up).normalize();
    Camera {
        position: camera_position,
        forward: camera_forward,
        right,
        up: right.cross(camera_forward).normalize(),
    }
}

fn draw_sky(frame: &mut Framebuffer) {
    for y in 0..frame.height {
        let t = y as f32 / frame.height as f32;
        let r = (38.0 + t * 91.0) as u32;
        let g = (7.0 + t * 22.0) as u32;
        let b = (12.0 + t * 4.0) as u32;
        frame.pixels[y * frame.width..(y + 1) * frame.width].fill((r << 16) | (g << 8) | b);
    }
}

fn draw_embers(frame: &mut Framebuffer, time: f32) {
    for index in 0..72_i32 {
        let x = (index * 347 + (time * (13.0 + index as f32 % 7.0)) as i32)
            .rem_euclid(frame.width as i32);
        let y =
            (frame.height as i32 - 1 - (index * 91 + (time * (24.0 + index as f32 % 11.0)) as i32))
                .rem_euclid(frame.height as i32);
        let color = if index % 3 == 0 { 0xffd05a } else { 0xff5a16 };
        for dy in 0..=1 {
            for dx in 0..=1 {
                let px = x + dx;
                let py = y + dy;
                if px >= 0 && py >= 0 && px < frame.width as i32 && py < frame.height as i32 {
                    frame.pixels[py as usize * frame.width + px as usize] = color;
                }
            }
        }
    }
}

fn draw_bar(
    frame: &mut Framebuffer,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    ratio: f32,
    color: u32,
) {
    for py in y..y + height {
        for px in x..x + width {
            if px >= 0 && py >= 0 && px < frame.width as i32 && py < frame.height as i32 {
                let border = px == x || px == x + width - 1 || py == y || py == y + height - 1;
                frame.pixels[py as usize * frame.width + px as usize] = if border {
                    0xe8d7c6
                } else if px - x < ((width - 2) as f32 * ratio.clamp(0.0, 1.0)) as i32 {
                    color
                } else {
                    0x261923
                };
            }
        }
    }
}

fn distance_to_segment(point: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b.sub(a);
    let t = point.sub(a).dot(ab) / ab.dot(ab).max(0.001);
    point
        .sub(a.add(ab.mul(t.clamp(0.0, 1.0))))
        .dot(point.sub(a.add(ab.mul(t.clamp(0.0, 1.0)))))
        .sqrt()
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
    fn cube(&mut self, center: Vec3, half: Vec3, color: [u8; 3], group: &str) {
        let start = self.vertices.len();
        for z in [-half.z, half.z] {
            for y in [-half.y, half.y] {
                for x in [-half.x, half.x] {
                    self.vertices.push(center.add(Vec3::new(x, y, z)));
                }
            }
        }
        for [a, b, c, d] in [
            [0, 1, 3, 2],
            [4, 6, 7, 5],
            [0, 4, 5, 1],
            [2, 3, 7, 6],
            [0, 2, 6, 4],
            [1, 5, 7, 3],
        ] {
            self.face([start + a, start + b, start + c], color, group);
            self.face([start + a, start + c, start + d], color, group);
        }
    }
    fn octa(&mut self, center: Vec3, radius: f32, color: [u8; 3], group: &str) {
        let s = self.vertices.len();
        self.vertices.extend([
            center.add(Vec3::new(radius, 0., 0.)),
            center.add(Vec3::new(-radius, 0., 0.)),
            center.add(Vec3::new(0., radius, 0.)),
            center.add(Vec3::new(0., -radius, 0.)),
            center.add(Vec3::new(0., 0., radius)),
            center.add(Vec3::new(0., 0., -radius)),
        ]);
        for tri in [
            [0, 2, 4],
            [4, 2, 1],
            [1, 2, 5],
            [5, 2, 0],
            [4, 3, 0],
            [1, 3, 4],
            [5, 3, 1],
            [0, 3, 5],
        ] {
            self.face(tri.map(|i| s + i), color, group);
        }
    }
    fn face(&mut self, indices: [usize; 3], color: [u8; 3], group: &str) {
        let mut f = Face::new(indices, "procedural", group);
        f.color = MaterialColor::Fixed(color);
        f.shader = SurfaceShader::Flat;
        self.faces.push(f);
    }
    fn finish(self) -> Model {
        Model::new(self.vertices, self.faces)
    }
}

fn make_environment() -> Model {
    let mut b = Builder::new();
    b.cube(
        Vec3::new(0., -7., -75.),
        Vec3::new(125., 6., 140.),
        [39, 25, 28],
        "volcanic_rock",
    );
    b.cube(
        Vec3::new(0., -0.2, -75.),
        Vec3::new(112., 0.7, 132.),
        [255, 48, 5],
        "lava_sea",
    );
    for x in [-82., -42., 0., 42., 82.] {
        b.cube(
            Vec3::new(x, 4., -52.),
            Vec3::new(13., 4., 42.),
            [58, 45, 47],
            "rock_platform",
        );
    }
    b.cube(
        Vec3::new(0., 40., -202.),
        Vec3::new(82., 40., 10.),
        [34, 27, 40],
        "castle",
    );
    for x in [-70., -38., 38., 70.] {
        b.cube(
            Vec3::new(x, 72., -198.),
            Vec3::new(10., 34., 11.),
            [29, 22, 35],
            "castle_tower",
        );
        b.cube(
            Vec3::new(x, 69., -186.5),
            Vec3::new(3., 9., 0.8),
            [255, 77, 12],
            "lava_window",
        );
    }
    for x in [-100., 100.] {
        b.octa(Vec3::new(x, 35., -235.), 38., [54, 31, 34], "volcano");
    }
    b.finish()
}
fn make_door() -> Model {
    let mut b = Builder::new();
    b.cube(
        DOOR_POSITION,
        Vec3::new(15., 25., 2.5),
        [20, 13, 24],
        "castle_door",
    );
    b.finish()
}
fn make_boss() -> Model {
    let mut b = Builder::new();
    b.cube(
        Vec3::default(),
        Vec3::new(5., 7., 3.5),
        [42, 38, 48],
        "boss_armor",
    );
    b.octa(Vec3::new(0., 0., -3.8), 3.2, [255, 73, 16], "boss_core");
    for side in [-1., 1.] {
        b.cube(
            Vec3::new(side * 9., 1., 0.),
            Vec3::new(4.5, 1.2, 2.),
            [55, 47, 59],
            "boss_wing",
        );
        b.octa(
            Vec3::new(side * 4., 8., 0.),
            3.5,
            [120, 31, 22],
            "boss_horn",
        );
    }
    b.finish()
}

fn model_group_center(model: &Model, group: &str) -> Option<Vec3> {
    let mut sum = Vec3::default();
    let mut count = 0usize;
    for face in model.faces.iter().filter(|face| face.group == group) {
        for &index in &face.indices {
            sum = sum.add(model.vertices[index]);
            count += 1;
        }
    }
    (count > 0).then(|| sum.mul(1.0 / count as f32))
}

fn make_fireball() -> Model {
    let mut b = Builder::new();
    b.octa(Vec3::default(), 1.8, [255, 77, 12], "lava_projectile");
    b.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ship::{ARWING_SPEC, ShipController};

    #[test]
    fn blender_boss_is_gigantic_and_has_a_real_weak_core() {
        let boss = load_obj("assets/bosses/lava_guardian/guardian_lava_gigante_optimized.obj")
            .expect("giant Blender boss should load");
        for group in [
            "Guardian_Torso",
            "Guardian_Core_Inner",
            "Guardian_Head",
            "Guardian_Main_Horn_-1",
            "Guardian_Wing_Blade_1_0",
        ] {
            assert!(boss.faces.iter().any(|face| face.group == group));
        }
        assert!(boss.bounds().1 > 28.0, "boss must dominate the arena");
        assert!(boss.faces.iter().any(|face| {
            face.material == "Castle_Rock" && face.shader == SurfaceShader::CastleRockTexture
        }));
        assert!(boss.faces.iter().any(|face| {
            face.material == "Castle_Rock_Dark" && face.shader == SurfaceShader::CastleRockTexture
        }));
        assert!(boss.faces.iter().any(|face| {
            face.material == "Castle_Lava" && face.shader == SurfaceShader::CastleLavaTexture
        }));
        assert!(boss.faces.iter().any(|face| {
            face.material == "Castle_Lava_Hot" && face.shader == SurfaceShader::CastleLavaTexture
        }));
        let core = model_group_center(&boss, "Guardian_Core_Inner")
            .expect("boss should expose its modeled weak point");
        let (center, radius) = boss.bounds();
        assert!(core.sub(center).dot(core.sub(center)) < radius * radius);
    }

    #[test]
    fn blender_arena_contains_castle_lava_obstacles_and_animated_door() {
        let environment =
            load_obj("assets/levels/lava_castle/lava_castle_environment_optimized.obj")
                .expect("Blender castle environment should load");
        let door = load_obj("assets/levels/lava_castle/lava_castle_door.obj")
            .expect("Blender castle door should load");
        for group in [
            "Lava_Basin",
            "Arena_Island_0",
            "Castle_Main_Keep",
            "Castle_Tower_0",
            "Horizon_Volcano_0",
            "Volcanic_Spire_0",
        ] {
            assert!(environment.faces.iter().any(|face| face.group == group));
        }
        assert!(
            environment
                .faces
                .iter()
                .any(|face| face.material == "Castle_Lava")
        );
        assert!(
            door.faces
                .iter()
                .any(|face| face.group == "Animated_Castle_Door")
        );
    }

    #[test]
    fn runtime_meshes_reduce_part_one_geometry_without_losing_gameplay_groups() {
        let source_environment =
            load_obj("assets/levels/lava_castle/lava_castle_environment.obj").unwrap();
        let runtime_environment =
            load_obj("assets/levels/lava_castle/lava_castle_environment_optimized.obj").unwrap();
        let source_boss =
            load_obj("assets/bosses/lava_guardian/guardian_lava_gigante.obj").unwrap();
        let runtime_boss =
            load_obj("assets/bosses/lava_guardian/guardian_lava_gigante_optimized.obj").unwrap();

        assert!(runtime_environment.faces.len() * 100 < source_environment.faces.len() * 65);
        assert!(runtime_boss.faces.len() * 100 < source_boss.faces.len() * 75);
        for group in ["Lava_Basin", "Castle_Main_Keep", "Arena_Island_0"] {
            assert!(
                runtime_environment
                    .faces
                    .iter()
                    .any(|face| face.group == group)
            );
        }
        for group in ["Guardian_Torso", "Guardian_Core_Inner", "Guardian_Head"] {
            assert!(runtime_boss.faces.iter().any(|face| face.group == group));
        }
    }
    #[test]
    fn boss_enters_second_phase_and_opens_door_only_after_defeat() {
        let mut level = LavaCastle::new();
        let mut c = ShipController::new(&ARWING_SPEC);
        level.enter(&mut c);
        level.phase = Phase::Combat;
        level.boss_health = BOSS_MAX_HEALTH / 2;
        level.update(&mut c, Input::default(), 0.05);
        assert_eq!(level.phase, Phase::Enraged);
        assert_eq!(level.door_open, 0.0);
        level.boss_health = 0;
        level.update(&mut c, Input::default(), 0.05);
        assert_eq!(level.phase, Phase::Dying);
    }
    #[test]
    fn restart_restores_boss_and_player_health() {
        let mut level = LavaCastle::new();
        let mut c = ShipController::new(&ARWING_SPEC);
        level.phase = Phase::Defeat;
        level.player_health = 0;
        level.boss_health = 3;
        level.update(
            &mut c,
            Input {
                restart: true,
                ..Input::default()
            },
            0.05,
        );
        assert_eq!(level.player_health, PLAYER_MAX_HEALTH);
        assert_eq!(level.boss_health, BOSS_MAX_HEALTH);
    }

    #[test]
    fn player_moves_on_star_fox_plane_without_advancing() {
        let mut level = LavaCastle::new();
        let mut controller = ShipController::new(&ARWING_SPEC);
        level.enter(&mut controller);
        let start = controller.ship.position;
        for _ in 0..20 {
            level.update(
                &mut controller,
                Input {
                    yaw: 1,
                    pitch: 1,
                    accelerate: true,
                    ..Input::default()
                },
                0.05,
            );
        }
        assert!(controller.ship.position.x > start.x);
        assert!(controller.ship.position.y > start.y);
        assert!((controller.ship.position.z - PLAYER_PLANE_Z).abs() < f32::EPSILON);
    }

    #[test]
    fn arena_uses_free_world_speed_and_weapon_specs() {
        let mut level = LavaCastle::new();
        let mut arena = ShipController::new(&ARWING_SPEC);
        let mut free = ShipController::new(&ARWING_SPEC);
        level.enter(&mut arena);
        for _ in 0..12 {
            level.update(
                &mut arena,
                Input {
                    yaw: 1,
                    accelerate: true,
                    ..Input::default()
                },
                0.05,
            );
            free.ship.update_flight(
                &ShipInput {
                    accelerating: true,
                    ..ShipInput::default()
                },
                0.05,
            );
        }
        assert!((arena.ship.speed - free.ship.speed).abs() < 0.001);

        level.update(
            &mut arena,
            Input {
                fire: true,
                ..Input::default()
            },
            0.08,
        );
        level.update(&mut arena, Input::default(), 0.02);
        assert!(!arena.projectiles.is_empty());
        assert_eq!(
            arena.projectiles[0].lifetime,
            ARWING_SPEC.laser_lifetime - 0.02
        );
    }

    #[test]
    fn vertical_dodging_is_faster_than_horizontal_movement() {
        let mut vertical_level = LavaCastle::new();
        let mut vertical = ShipController::new(&ARWING_SPEC);
        vertical_level.enter(&mut vertical);
        let vertical_start = vertical.ship.position;

        let mut horizontal_level = LavaCastle::new();
        let mut horizontal = ShipController::new(&ARWING_SPEC);
        horizontal_level.enter(&mut horizontal);
        let horizontal_start = horizontal.ship.position;
        for _ in 0..10 {
            vertical_level.update(
                &mut vertical,
                Input {
                    pitch: 1,
                    ..Input::default()
                },
                0.05,
            );
            horizontal_level.update(
                &mut horizontal,
                Input {
                    yaw: 1,
                    ..Input::default()
                },
                0.05,
            );
        }
        assert!(
            vertical.ship.position.y - vertical_start.y
                > horizontal.ship.position.x - horizontal_start.x
        );
    }

    #[test]
    fn combat_plane_uses_the_free_world_steering_sensitivity() {
        let mut level = LavaCastle::new();
        let mut arena = ShipController::new(&ARWING_SPEC);
        let mut free = ShipController::new(&ARWING_SPEC);
        level.enter(&mut arena);
        for _ in 0..8 {
            level.update(
                &mut arena,
                Input {
                    yaw: 1,
                    pitch: 1,
                    ..Input::default()
                },
                0.05,
            );
            free.ship.update_flight(
                &ShipInput {
                    yaw: 1,
                    pitch: 1,
                    ..ShipInput::default()
                },
                0.05,
            );
        }
        assert!((arena.ship.yaw_rate - free.ship.yaw_rate).abs() < 0.001);
        assert!((arena.ship.pitch_rate - free.ship.pitch_rate).abs() < 0.001);
    }

    #[test]
    fn lowered_door_awards_reward_before_entering_part_two() {
        let mut level = LavaCastle::new();
        let mut controller = ShipController::new(&ARWING_SPEC);
        level.enter(&mut controller);
        level.phase = Phase::DoorOpening;
        for _ in 0..45 {
            assert_eq!(
                level.update(&mut controller, Input::default(), 0.05),
                Action::Continue
            );
        }
        assert_eq!(level.phase, Phase::Reward);
        assert_eq!(level.door_open, 1.0);
        let lowered_top = level
            .door
            .vertices
            .iter()
            .map(|vertex| vertex.y - DOOR_LOWER_DISTANCE)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(lowered_top < ARENA_MIN_Y);
        let frozen = controller.ship.position;
        let mut reward_frames = 0;
        while level.phase == Phase::Reward {
            assert_eq!(
                level.update(
                    &mut controller,
                    Input {
                        yaw: 1,
                        pitch: 1,
                        accelerate: true,
                        ..Input::default()
                    },
                    0.05,
                ),
                Action::Continue
            );
            let offset = controller.ship.position.sub(frozen);
            assert!(offset.dot(offset) < 0.0001);
            reward_frames += 1;
            assert!(reward_frames < 120);
        }
        assert_eq!(level.phase, Phase::Complete);
        assert_eq!(
            level.update(&mut controller, Input::default(), 0.05),
            Action::InteriorTransition
        );
    }
}
