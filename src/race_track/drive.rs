use crate::{obj_loader::Vec3, ship::Ship};

use super::course::{HALF_ROAD, RaceCourse};

const MAX_SPEED: f32 = 42.0;
const CRUISE_SPEED: f32 = 37.0;
const MAX_REVERSE: f32 = 12.0;
const ACCELERATION: f32 = 30.0;
const BRAKING: f32 = 56.0;
const FRICTION: f32 = 8.0;
const TURBO_CAPACITY: f32 = 3.2;
const TURBO_RECHARGE: f32 = 0.38;
const SAFE_EDGE_MARGIN: f32 = 0.2;

#[derive(Clone, Copy, Default)]
pub(crate) struct RaceInput {
    pub accelerate: bool,
    pub reverse: bool,
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
    pub turbo: f32,
    pub boost_flash: f32,
    pub collision_flash: f32,
    pub boost_pad_cooldown: f32,
    was_drifting: bool,
}

impl DriveState {
    pub fn new() -> Self {
        Self {
            speed: 0.0,
            steering: 0.0,
            drift: 0.0,
            drift_charge: 0.0,
            turbo: TURBO_CAPACITY,
            boost_flash: 0.0,
            collision_flash: 0.0,
            boost_pad_cooldown: 0.0,
            was_drifting: false,
        }
    }

    pub fn update(&mut self, ship: &mut Ship, course: &RaceCourse, input: RaceInput, dt: f32) {
        let dt = dt.clamp(0.0, 0.05);
        self.boost_flash = (self.boost_flash - dt * 1.6).max(0.0);
        self.collision_flash = (self.collision_flash - dt * 2.0).max(0.0);
        self.boost_pad_cooldown = (self.boost_pad_cooldown - dt).max(0.0);

        let (_, offroad, _) = course.nearest(ship.position);
        let on_road = offroad <= HALF_ROAD + 0.8;
        let drift_input = input.drift && on_road && self.speed.abs() > 7.0;

        if input.boost && self.turbo > 0.0 {
            self.turbo = (self.turbo - dt).max(0.0);
            self.speed += ACCELERATION * 0.53 * dt;
            self.boost_flash = 1.0;
        } else {
            self.turbo = (self.turbo + TURBO_RECHARGE * dt).min(TURBO_CAPACITY);
        }

        if input.accelerate {
            self.speed += ACCELERATION * if on_road { 1.0 } else { 0.52 } * dt;
        } else if input.reverse {
            self.speed = if self.speed > 0.0 {
                (self.speed - BRAKING * dt).max(0.0)
            } else {
                (self.speed - ACCELERATION * 0.68 * dt).max(-MAX_REVERSE)
            };
        } else if input.brake {
            if self.speed > 0.0 {
                self.speed = (self.speed - BRAKING * dt).max(0.0);
            } else {
                self.speed -= ACCELERATION * 0.42 * dt;
            }
        } else if on_road && self.speed < CRUISE_SPEED {
            // Keep the player moving with the pack if throttle is released or
            // a key event is missed. W still brings the car to full speed; S
            // remains the deliberate brake/reverse input.
            self.speed = (self.speed + ACCELERATION * 0.58 * dt).min(CRUISE_SPEED);
        } else {
            let drag = FRICTION * if on_road { 1.0 } else { 2.9 } * dt;
            self.speed = if on_road {
                (self.speed - drag).max(CRUISE_SPEED)
            } else if self.speed > 0.0 {
                (self.speed - drag).max(0.0)
            } else {
                (self.speed + drag).min(0.0)
            };
        }

        let top_speed = MAX_SPEED
            * if input.boost && self.turbo > 0.0 {
                1.24
            } else {
                1.0
            };
        self.speed = self.speed.clamp(-MAX_REVERSE, top_speed);
        let steering_target = input.steering.clamp(-1, 1) as f32;
        let steering_blend = 1.0 - (-9.0 * dt).exp();
        self.steering += (steering_target - self.steering) * steering_blend;
        if steering_target == 0.0 {
            self.steering *= (1.0 - 5.0 * dt).max(0.0);
        }

        let speed_ratio = (self.speed.abs() / MAX_SPEED).clamp(0.0, 1.0);
        let grip_loss = if drift_input { 0.68 } else { 1.0 };
        let turn_rate = (0.85 + speed_ratio * 1.18) * grip_loss;
        let reverse = if self.speed < -0.2 { -1.0 } else { 1.0 };
        ship.yaw =
            (ship.yaw - self.steering * turn_rate * reverse * dt).rem_euclid(std::f32::consts::TAU);
        ship.pitch = 0.0;
        ship.steering_bank = -self.steering * 0.1 - self.steering * self.drift * 0.25;
        ship.steering_pitch = 0.0;

        let drift_target = if drift_input && self.steering.abs() > 0.12 {
            1.0
        } else {
            0.0
        };
        let drift_response = if drift_target > 0.0 { 4.0 } else { 2.2 };
        self.drift += (drift_target - self.drift) * (1.0 - (-drift_response * dt).exp());
        if drift_target > 0.0 {
            self.drift_charge =
                (self.drift_charge + self.steering.abs() * speed_ratio * 0.28 * dt).min(1.0);
            self.speed *= (1.0 - 0.09 * dt).max(0.0);
        } else if self.was_drifting && self.drift_charge >= 0.2 {
            self.turbo = (self.turbo + 0.42 * self.drift_charge).min(TURBO_CAPACITY);
            self.drift_charge = 0.0;
            self.boost_flash = 0.55;
        }
        self.was_drifting = drift_target > 0.0;

        let (forward, _, _) = ship.movement_basis();
        let movement = forward.mul(self.speed * dt);
        let steps = (movement.dot(movement).sqrt() / 1.0).ceil().max(1.0) as usize;
        let step = movement.mul(1.0 / steps as f32);
        let mut touched_boundary = false;
        for _ in 0..steps {
            let next = ship.position.add(step);
            let (_, distance_from_road, nearest) = course.nearest(next);
            if distance_from_road <= HALF_ROAD + 0.8 {
                ship.position = next;
            } else {
                // Project onto the safe edge instead of cancelling movement.
                // The longitudinal part of the step is preserved, so cars
                // slide along the grey barrier and cannot become wedged in it.
                let side = Vec3::new(-nearest.tangent.z, 0.0, nearest.tangent.x);
                let signed_lateral = next.sub(nearest.position).dot(side);
                let safe_edge = HALF_ROAD + 0.8 - SAFE_EDGE_MARGIN;
                ship.position = nearest
                    .position
                    .add(side.mul(signed_lateral.clamp(-safe_edge, safe_edge)))
                    .add(Vec3::new(0.0, 1.2, 0.0));
                touched_boundary = true;
            }
        }
        if touched_boundary {
            self.speed *= 0.82;
            self.collision_flash = 0.65;
        }

        if self.boost_pad_cooldown <= 0.0
            && course.boost_pads.iter().any(|pad| {
                let dx = ship.position.x - pad.x;
                let dz = ship.position.z - pad.z;
                dx * dx + dz * dz < 26.0
            })
        {
            self.speed = (self.speed + 7.0).min(MAX_SPEED * 1.18);
            self.turbo = (self.turbo + 0.28).min(TURBO_CAPACITY);
            self.boost_flash = 1.0;
            self.boost_pad_cooldown = 1.2;
        }

        ship.position.y = 1.2;
        ship.speed = self.speed.abs();
    }

    pub fn recover(&mut self) {
        self.speed = 0.0;
        self.steering = 0.0;
        self.drift = 0.0;
        self.drift_charge = 0.0;
        self.boost_flash = 0.0;
        self.collision_flash = 0.0;
        self.was_drifting = false;
    }

    pub fn max_speed(&self) -> f32 {
        MAX_SPEED
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ship::{ASTRA_CAR_SPEC, ShipController};

    #[test]
    fn player_coasts_at_rival_pace_and_throttle_reaches_the_shared_top_speed() {
        let course = RaceCourse::new();
        let start = course.sample_at(0.0);
        let mut controller = ShipController::new(&ASTRA_CAR_SPEC);
        controller.ship.position = start.position.add(crate::obj_loader::Vec3::new(0.0, 1.2, 0.0));
        controller.ship.yaw = (-start.tangent.z).atan2(-start.tangent.x);
        let mut drive = DriveState::new();

        for _ in 0..180 {
            drive.update(
                &mut controller.ship,
                &course,
                RaceInput::default(),
                1.0 / 60.0,
            );
        }
        assert!((drive.speed - CRUISE_SPEED).abs() < 0.1);

        for _ in 0..120 {
            drive.update(
                &mut controller.ship,
                &course,
                RaceInput {
                    accelerate: true,
                    ..RaceInput::default()
                },
                1.0 / 60.0,
            );
        }
        assert_eq!(drive.speed, MAX_SPEED);
    }

    #[test]
    fn reverse_input_brakes_forward_motion_then_drives_backwards() {
        let course = RaceCourse::new();
        let start = course.sample_at(0.0);
        let mut controller = ShipController::new(&ASTRA_CAR_SPEC);
        controller.ship.position = start.position.add(crate::obj_loader::Vec3::new(0.0, 1.2, 0.0));
        controller.ship.yaw = (-start.tangent.z).atan2(-start.tangent.x);
        let mut drive = DriveState::new();
        drive.speed = 20.0;

        for _ in 0..150 {
            drive.update(
                &mut controller.ship,
                &course,
                RaceInput {
                    reverse: true,
                    ..RaceInput::default()
                },
                1.0 / 60.0,
            );
        }
        assert!(drive.speed < 0.0);
    }

    #[test]
    fn road_edge_projects_cars_inside_instead_of_trapping_them() {
        let course = RaceCourse::new();
        let sample = course.sample_at(course.length * 0.93);
        let side = crate::obj_loader::Vec3::new(-sample.tangent.z, 0.0, sample.tangent.x);
        let outside = sample
            .position
            .add(side.mul(HALF_ROAD + 5.0))
            .add(crate::obj_loader::Vec3::new(0.0, 1.2, 0.0));
        let (_, _, nearest) = course.nearest(outside);
        let nearest_side = crate::obj_loader::Vec3::new(
            -nearest.tangent.z,
            0.0,
            nearest.tangent.x,
        );
        let safe_edge = HALF_ROAD + 0.8 - SAFE_EDGE_MARGIN;
        let corrected = nearest
            .position
            .add(nearest_side.mul(
                outside
                    .sub(nearest.position)
                    .dot(nearest_side)
                    .clamp(-safe_edge, safe_edge),
            ));
        let (_, corrected_distance, _) = course.nearest(corrected);
        assert!(corrected_distance <= safe_edge + 0.05);
    }
}
