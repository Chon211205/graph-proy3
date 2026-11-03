use crate::ship::Ship;

use super::{COLLISION_RADIUS, TILE_SIZE};

const MAX_SPEED: f32 = 62.0;
const MAX_REVERSE: f32 = 20.0;
const ACCELERATION: f32 = 58.0;
const BRAKING: f32 = 94.0;
const FRICTION: f32 = 18.0;

#[derive(Clone, Copy, Default)]
pub(crate) struct RaceInput {
    pub accelerate: bool,
    pub brake: bool,
    pub steering: i8,
    pub drift: bool,
    pub boost: bool,
    pub pause: bool,
    pub confirm: bool,
}

pub(super) struct DriveState {
    pub speed: f32,
    pub steering: f32,
    pub drift: f32,
    pub drift_charge: f32,
    pub boost_timer: f32,
    pub boost_flash: f32,
    pub collision_flash: f32,
    pub boost_pad_cooldown: f32,
    was_drifting: bool,
    boost_was_down: bool,
}

impl DriveState {
    pub fn new() -> Self {
        Self {
            speed: 0.0,
            steering: 0.0,
            drift: 0.0,
            drift_charge: 0.0,
            boost_timer: 0.0,
            boost_flash: 0.0,
            collision_flash: 0.0,
            boost_pad_cooldown: 0.0,
            was_drifting: false,
            boost_was_down: false,
        }
    }

    pub fn update(&mut self, ship: &mut Ship, tiles: &[Vec<u8>], input: RaceInput, dt: f32) {
        self.boost_timer = (self.boost_timer - dt).max(0.0);
        self.boost_flash = (self.boost_flash - dt * 1.8).max(0.0);
        self.collision_flash = (self.collision_flash - dt * 2.4).max(0.0);
        self.boost_pad_cooldown = (self.boost_pad_cooldown - dt).max(0.0);

        let tile = tile_at(tiles, ship.position.x, ship.position.z);
        let surface = surface_factor(tile);
        let max_speed = MAX_SPEED * surface * if self.boost_timer > 0.0 { 1.32 } else { 1.0 };

        if input.accelerate {
            self.speed += ACCELERATION * surface * dt;
        } else if input.brake {
            if self.speed > 0.0 {
                self.speed = (self.speed - BRAKING * dt).max(0.0);
            } else {
                self.speed -= ACCELERATION * 0.48 * dt;
            }
        } else {
            let drag = FRICTION
                * (if tile == b'U' || tile == b'.' {
                    1.35
                } else {
                    1.0
                })
                * dt;
            self.speed = if self.speed > 0.0 {
                (self.speed - drag).max(0.0)
            } else {
                (self.speed + drag).min(0.0)
            };
        }
        self.speed = self.speed.clamp(-MAX_REVERSE, max_speed);

        let steering_target = input.steering.clamp(-1, 1) as f32;
        let steering_blend = 1.0 - (-12.0 * dt).exp();
        self.steering += (steering_target - self.steering) * steering_blend;
        if steering_target == 0.0 {
            self.steering *= (1.0 - 9.0 * dt).max(0.0);
        }

        let speed_ratio = (self.speed.abs() / MAX_SPEED).clamp(0.0, 1.0);
        let mut turn_rate = 1.35 + speed_ratio * 2.05;
        if input.drift && self.speed.abs() > 7.0 {
            turn_rate *= 1.25;
        }
        let reverse = if self.speed < -0.2 { -1.0 } else { 1.0 };
        ship.yaw =
            (ship.yaw - self.steering * turn_rate * reverse * dt).rem_euclid(std::f32::consts::TAU);
        ship.pitch = 0.0;
        ship.steering_bank = -self.steering * 0.10 - self.steering * self.drift * 0.28;
        ship.steering_pitch = 0.0;

        let can_drift = input.drift && self.speed.abs() > 7.0 && self.steering.abs() > 0.08;
        let drift_target = if can_drift { 1.0 } else { 0.0 };
        let drift_response = if can_drift { 5.0 } else { 3.0 };
        self.drift += (drift_target - self.drift) * (1.0 - (-drift_response * dt).exp());
        if can_drift {
            self.speed *= (1.0 - 0.16 * dt).max(0.0);
            self.drift_charge =
                (self.drift_charge + self.steering.abs() * speed_ratio * 0.42 * dt).min(1.0);
        } else if self.was_drifting && self.drift_charge >= 0.22 {
            self.trigger_boost(0.55 + self.drift_charge * 0.95);
            self.drift_charge = 0.0;
        }
        self.was_drifting = can_drift;

        if input.boost && !self.boost_was_down && self.drift_charge >= 0.25 {
            self.trigger_boost(0.7 + self.drift_charge * 0.9);
            self.drift_charge = 0.0;
        }
        self.boost_was_down = input.boost;

        self.move_ship(ship, tiles, dt);
        if tile_at(tiles, ship.position.x, ship.position.z) == b'R'
            && self.boost_pad_cooldown <= 0.0
        {
            self.trigger_boost(0.95);
            self.speed = (self.speed + 10.0).min(MAX_SPEED * 1.32);
            self.boost_pad_cooldown = 1.1;
        }
        ship.speed = self.speed.abs();
    }

    fn move_ship(&mut self, ship: &mut Ship, tiles: &[Vec<u8>], dt: f32) {
        let (forward, _, _) = ship.movement_basis();
        let displacement = forward.mul(self.speed * dt);
        let steps = (displacement.dot(displacement).sqrt() / 1.25)
            .ceil()
            .max(1.0) as usize;
        let step = displacement.mul(1.0 / steps as f32);
        for _ in 0..steps {
            let next_x = ship.position.x + step.x;
            if !blocked(tiles, next_x, ship.position.z) {
                ship.position.x = next_x;
            } else {
                self.hit_wall();
            }
            let next_z = ship.position.z + step.z;
            if !blocked(tiles, ship.position.x, next_z) {
                ship.position.z = next_z;
            } else {
                self.hit_wall();
            }
        }
        ship.position.y = 2.0;
    }

    fn hit_wall(&mut self) {
        self.speed *= 0.42;
        self.collision_flash = 1.0;
    }

    fn trigger_boost(&mut self, duration: f32) {
        self.boost_timer = self.boost_timer.max(duration);
        self.boost_flash = 1.0;
    }

    pub fn recover(&mut self) {
        self.speed = 0.0;
        self.steering = 0.0;
        self.drift = 0.0;
        self.drift_charge = 0.0;
        self.boost_timer = 0.0;
        self.collision_flash = 0.0;
        self.was_drifting = false;
    }

    pub fn max_speed(&self) -> f32 {
        MAX_SPEED
    }
}

pub(super) fn blocked(tiles: &[Vec<u8>], x: f32, z: f32) -> bool {
    if !x.is_finite() || !z.is_finite() || x < 0.0 || z < 0.0 {
        return true;
    }
    let min_x = ((x - COLLISION_RADIUS) / TILE_SIZE).floor() as isize;
    let max_x = ((x + COLLISION_RADIUS) / TILE_SIZE).floor() as isize;
    let min_z = ((z - COLLISION_RADIUS) / TILE_SIZE).floor() as isize;
    let max_z = ((z + COLLISION_RADIUS) / TILE_SIZE).floor() as isize;
    for row in min_z..=max_z {
        for col in min_x..=max_x {
            let tile = if row < 0 || col < 0 {
                None
            } else {
                tiles
                    .get(row as usize)
                    .and_then(|line| line.get(col as usize))
            };
            if tile.is_none_or(|tile| matches!(tile, b'#' | b'H' | b'S' | b'W' | b'D')) {
                let near_x = x.clamp(col as f32 * TILE_SIZE, (col + 1) as f32 * TILE_SIZE);
                let near_z = z.clamp(row as f32 * TILE_SIZE, (row + 1) as f32 * TILE_SIZE);
                if (x - near_x).powi(2) + (z - near_z).powi(2) < COLLISION_RADIUS.powi(2) {
                    return true;
                }
            }
        }
    }
    false
}

fn tile_at(tiles: &[Vec<u8>], x: f32, z: f32) -> u8 {
    if x < 0.0 || z < 0.0 {
        return b'#';
    }
    let col = (x / TILE_SIZE).floor() as usize;
    let row = (z / TILE_SIZE).floor() as usize;
    tiles
        .get(row)
        .and_then(|line| line.get(col))
        .copied()
        .unwrap_or(b'#')
}

fn surface_factor(tile: u8) -> f32 {
    match tile {
        b'P' | b'M' | b'R' | b'L' | b'K' => 1.0,
        b'F' => 0.70,
        b'U' => 0.76,
        b'.' => 0.58,
        _ => 0.82,
    }
}
