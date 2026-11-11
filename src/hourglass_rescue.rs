mod audio;
mod flight;
mod hud;
mod objectives;
mod rendering;
mod results;
mod scene;
#[cfg(test)]
mod tests;
mod ui;

use crate::framebuffer::Framebuffer;
use crate::obj_loader::Vec3;
use crate::ship::{Projectile, Ship};

use audio::{Audio, Cue};
use scene::{CAPSULE_COUNT, CAPSULE_POSITIONS, Scene};

const ENEMY_SPAWNS: [Vec3; scene::ENEMY_COUNT] = [
    Vec3::new(-35.0, 30.0, -155.0),
    Vec3::new(105.0, 72.0, -325.0),
    Vec3::new(-95.0, 132.0, -445.0),
];

#[derive(Clone, Copy)]
struct TimeEnemy {
    position: Vec3,
    health: u8,
    alive: bool,
    attack_timer: f32,
}

// The level starts deliberately tight: exploring for time pickups is part of
// the route rather than an optional score bonus.
const MISSION_SECONDS: f32 = 20.0;
const CLOCK_BONUS_SECONDS: f32 = 20.0;
const CLOCK_COUNT: usize = 6;
const CLOCK_POSITIONS: [Vec3; CLOCK_COUNT] = [
    // The first pickup is visible shortly after launch. The remaining clocks
    // alternate across the valley and climb with the route, giving the player
    // a reachable 20-second extension in every major section of the map.
    Vec3::new(25.0, 18.0, -35.0),
    Vec3::new(-80.0, 20.0, -120.0),
    Vec3::new(95.0, 42.0, -215.0),
    Vec3::new(-145.0, 70.0, -320.0),
    Vec3::new(150.0, 118.0, -420.0),
    Vec3::new(-75.0, 158.0, -505.0),
];
const RESCUE_DISTANCE: f32 = 10.0;
const REWARD_DURATION: f32 = 5.2;
const REWARD_COLLECT_TIME: f32 = 3.35;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Intro,
    Playing,
    Paused,
    Reward,
    Won,
    Lost,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    pub forward: bool,
    pub brake: bool,
    pub left: bool,
    pub right: bool,
    pub rise: bool,
    pub descend: bool,
    pub roll_left: bool,
    pub roll_right: bool,
    pub boost: bool,
    pub firing: bool,
    pub pause: bool,
    pub exit: bool,
    pub restart: bool,
    pub recover: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Continue,
    Exit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CapsuleState {
    Pending,
    Rescued,
}

pub struct HourglassRescue {
    phase: Phase,
    phase_before_pause: Phase,
    phase_timer: f32,
    elapsed: f32,
    visual_time: f32,
    shields: u8,
    hit_cooldown: f32,
    boost_reserve: f32,
    sand_offset: f32,
    rescue_target: Option<usize>,
    capsules: [CapsuleState; CAPSULE_COUNT],
    rescued: u8,
    clocks: [bool; CLOCK_COUNT],
    clocks_collected: u8,
    speed: f32,
    vertical_speed: f32,
    safe_position: Vec3,
    feedback: &'static str,
    feedback_timer: f32,
    audio: Audio,
    scene: Scene,
    enemies: [TimeEnemy; scene::ENEMY_COUNT],
    enemies_destroyed: u8,
    enemy_projectiles: Vec<Projectile>,
    reward_elapsed: f32,
    reward_position: Vec3,
}

impl HourglassRescue {
    pub fn new() -> Self {
        Self {
            phase: Phase::Intro,
            phase_before_pause: Phase::Intro,
            phase_timer: 3.0,
            elapsed: 0.0,
            visual_time: 0.0,
            shields: 3,
            hit_cooldown: 0.0,
            boost_reserve: 2.2,
            sand_offset: 0.0,
            rescue_target: None,
            capsules: [CapsuleState::Pending; CAPSULE_COUNT],
            rescued: 0,
            clocks: [false; CLOCK_COUNT],
            clocks_collected: 0,
            speed: 0.0,
            vertical_speed: 0.0,
            safe_position: Vec3::new(0.0, 12.0, 42.0),
            feedback: "REUNE LOS FRAGMENTOS, ADMINISTRA EL TIEMPO Y BUSCA LA ESFERA",
            feedback_timer: 4.0,
            audio: Audio::load(),
            scene: Scene::new(),
            enemies: std::array::from_fn(|index| TimeEnemy {
                position: ENEMY_SPAWNS[index],
                health: 3 + index as u8,
                alive: true,
                attack_timer: 0.7 + index as f32 * 0.35,
            }),
            enemies_destroyed: 0,
            enemy_projectiles: Vec::new(),
            reward_elapsed: 0.0,
            reward_position: Vec3::default(),
        }
    }

    pub fn reset(&mut self, ship: &mut Ship) {
        *self = Self::new();
        self.place_ship_at_start(ship);
    }

    pub fn place_ship_at_start(&mut self, ship: &mut Ship) {
        ship.position = Vec3::new(0.0, 12.0, 42.0);
        // Free-flight yaw uses PI/2 as the heading toward the level's -Z route.
        ship.yaw = std::f32::consts::FRAC_PI_2;
        ship.pitch = 0.0;
        ship.roll = 0.0;
        ship.roll_target = 0.0;
        ship.yaw_rate = 0.0;
        ship.pitch_rate = 0.0;
        ship.steering_bank = 0.0;
        ship.steering_pitch = 0.0;
        ship.planet_locked = false;
        ship.spiky_planet_locked = false;
        ship.race_planet_locked = false;
        ship.orbit_center = None;
        ship.orbit_plane_normal = None;
        ship.orbit_camera_blend = 0.0;
        ship.orbit_release_active = false;
        ship.speed = ship.spec.cruise_speed;
        self.speed = ship.speed;
        self.safe_position = ship.position;
    }

    pub fn update(&mut self, ship: &mut Ship, input: Input, delta_time: f32) -> Action {
        if input.exit {
            return Action::Exit;
        }
        if matches!(self.phase, Phase::Won | Phase::Lost) {
            if input.restart {
                self.reset(ship);
            }
            return Action::Continue;
        }
        if input.pause {
            if self.phase == Phase::Paused {
                self.phase = self.phase_before_pause;
            } else {
                self.phase_before_pause = self.phase;
                self.phase = Phase::Paused;
            }
            return Action::Continue;
        }
        if self.phase == Phase::Paused {
            if input.restart {
                self.reset(ship);
            }
            return Action::Continue;
        }

        let dt = delta_time.clamp(0.0, 0.05);
        self.visual_time += dt;
        self.feedback_timer = (self.feedback_timer - dt).max(0.0);
        self.hit_cooldown = (self.hit_cooldown - dt).max(0.0);

        match self.phase {
            Phase::Intro => {
                self.phase_timer = (self.phase_timer - dt).max(0.0);
                if self.phase_timer == 0.0 {
                    self.phase = Phase::Playing;
                }
                return Action::Continue;
            }
            Phase::Playing => {}
            Phase::Reward => {
                self.reward_elapsed = (self.reward_elapsed + dt).min(REWARD_DURATION);
                ship.position = self.reward_position;
                ship.speed = 0.0;
                let descent = (self.reward_elapsed / 2.2).clamp(0.0, 1.0);
                let descent = descent * descent * (3.0 - 2.0 * descent);
                let hover = (self.reward_elapsed * 3.2).sin() * 0.22;
                let center =
                    self.reward_position
                        .add(Vec3::new(0.0, 13.0 - descent * 8.0 + hover, 0.0));
                let scale = (1.0 - (self.reward_elapsed - REWARD_COLLECT_TIME).max(0.0) / 0.55)
                    .clamp(0.0, 1.0);
                self.scene
                    .set_reward_transform(center, scale, self.reward_elapsed * 1.8);
                self.feedback = if self.reward_elapsed >= REWARD_COLLECT_TIME {
                    "ESFERA DE 5 ESTRELLAS OBTENIDA"
                } else {
                    "RECIBIENDO ESFERA DE 5 ESTRELLAS"
                };
                self.feedback_timer = 1.0;
                if self.reward_elapsed >= REWARD_DURATION {
                    self.phase = Phase::Won;
                    return Action::Exit;
                }
                return Action::Continue;
            }
            Phase::Paused | Phase::Won | Phase::Lost => return Action::Continue,
        }

        self.elapsed += dt;
        self.sand_offset = 0.0;
        if self.elapsed >= MISSION_SECONDS {
            self.phase = Phase::Lost;
            self.feedback = "EL RELOJ SE HA VACIADO";
            self.feedback_timer = 0.0;
            return Action::Continue;
        }

        if input.recover {
            ship.position = self.safe_position;
            self.speed = 0.0;
            self.vertical_speed = 0.0;
            self.rescue_target = None;
            self.feedback = "NAVE RECUPERADA EN POSICION SEGURA";
            self.feedback_timer = 2.0;
        }
        self.move_ship(ship, input, dt);
        self.resolve_collisions(ship, dt);
        self.update_clocks(ship);
        self.update_rescue(ship);
        if self.shields == 0 {
            self.phase = Phase::Lost;
            self.feedback = "ESCUDOS AGOTADOS";
            self.feedback_timer = 0.0;
        }
        self.scene.update_sand(self.sand_offset);
        self.update_capsule_visibility();
        Action::Continue
    }

    pub fn draw_hud(&self, frame: &mut Framebuffer, ship: &Ship) {
        hud::draw(self, frame, ship);
    }

    pub fn update_enemies(
        &mut self,
        ship: &mut Ship,
        projectiles: &mut [Projectile],
        delta_time: f32,
    ) {
        if self.phase != Phase::Playing {
            return;
        }
        let dt = delta_time.clamp(0.0, 0.05);
        for shot in self
            .enemy_projectiles
            .iter_mut()
            .filter(|shot| shot.lifetime > 0.0)
        {
            shot.position = shot.position.add(shot.direction.mul(48.0 * dt));
            shot.lifetime -= dt;
            let offset = shot.position.sub(ship.position);
            if offset.dot(offset) < 4.5 * 4.5 && self.hit_cooldown <= 0.0 {
                shot.lifetime = 0.0;
                self.shields = self.shields.saturating_sub(1);
                self.elapsed += 5.0;
                self.hit_cooldown = 1.2;
                self.feedback = "DISPARO TEMPORAL  -5 SEGUNDOS";
                self.feedback_timer = 1.5;
                self.audio.play(Cue::Damage);
            }
        }
        for index in 0..self.enemies.len() {
            if !self.enemies[index].alive {
                continue;
            }
            let to_ship = ship.position.sub(self.enemies[index].position);
            let distance = to_ship.dot(to_ship).sqrt();
            if distance > 0.001 {
                // Fast enough to remain a visible threat while the player
                // crosses the large valley with unlimited boost.
                let speed = 17.0 + index as f32 * 2.5;
                self.enemies[index].position = self.enemies[index]
                    .position
                    .add(to_ship.mul((speed * dt / distance).min(1.0)));
            }
            self.enemies[index].attack_timer -= dt;
            if distance < 150.0 && self.enemies[index].attack_timer <= 0.0 {
                self.enemy_projectiles.push(Projectile {
                    position: self.enemies[index].position,
                    direction: to_ship.normalize(),
                    lifetime: 4.0,
                    scale: 0.85,
                });
                self.enemies[index].attack_timer = 1.6 + index as f32 * 0.25;
            }

            for shot in projectiles.iter_mut().filter(|shot| shot.lifetime > 0.0) {
                let offset = shot.position.sub(self.enemies[index].position);
                let hit_radius = 6.0 + shot.scale * 0.65;
                if offset.dot(offset) <= hit_radius * hit_radius {
                    let damage = if shot.scale > 2.0 { 3 } else { 1 };
                    self.enemies[index].health = self.enemies[index].health.saturating_sub(damage);
                    shot.lifetime = 0.0;
                    self.feedback = if self.enemies[index].health == 0 {
                        "ENEMIGO TEMPORAL ELIMINADO"
                    } else {
                        "IMPACTO EN ENEMIGO TEMPORAL"
                    };
                    self.feedback_timer = 1.2;
                    if self.enemies[index].health == 0 {
                        self.enemies[index].alive = false;
                        self.enemies_destroyed += 1;
                    }
                    break;
                }
            }

            if self.enemies[index].alive {
                let offset = ship.position.sub(self.enemies[index].position);
                if offset.dot(offset) < 7.5 * 7.5 && self.hit_cooldown <= 0.0 {
                    self.shields = self.shields.saturating_sub(1);
                    self.elapsed += 5.0;
                    self.hit_cooldown = 1.2;
                    self.feedback = "ATAQUE TEMPORAL  -5 SEGUNDOS";
                    self.feedback_timer = 1.5;
                    self.audio.play(Cue::Damage);
                    let push = offset.normalize();
                    ship.position = ship.position.add(push.mul(9.0));
                }
            }

            let visible_position = if self.enemies[index].alive {
                self.enemies[index].position
            } else {
                Vec3::new(0.0, -10_000.0 - index as f32 * 20.0, 0.0)
            };
            self.scene.set_enemy_position(index, visible_position);
        }
    }

    pub fn rescued_count(&self) -> u8 {
        self.rescued
    }

    fn remaining(&self) -> f32 {
        (MISSION_SECONDS - self.elapsed).max(0.0)
    }

    fn target_distance_for_hud(&self, ship: &Ship) -> Option<f32> {
        self.target_distance(ship)
    }
}
