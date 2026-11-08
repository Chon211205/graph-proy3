use crate::obj_loader::Vec3;
use crate::ship::Ship;

use super::scene::dune_height;
use super::{Cue, HourglassRescue, Input, Phase};

impl HourglassRescue {
    pub(super) fn move_ship(&mut self, ship: &mut Ship, input: Input, dt: f32) {
        let steering = input.right as i8 as f32 - input.left as i8 as f32;
        ship.yaw = (ship.yaw + steering * 1.35 * dt).rem_euclid(std::f32::consts::TAU);
        ship.steering_bank += (-steering * 0.34 - ship.steering_bank) * (1.0 - (-6.0 * dt).exp());
        let target_speed = if input.forward {
            24.0
        } else if input.brake {
            0.0
        } else {
            self.speed
        };
        let response = if input.brake { 24.0 } else { 10.5 };
        self.speed += (target_speed - self.speed) * (1.0 - (-response * dt).exp());
        if input.boost && self.boost_reserve > 0.0 {
            self.boost_reserve = (self.boost_reserve - dt).max(0.0);
        } else {
            self.boost_reserve = (self.boost_reserve + dt * 0.26).min(2.2);
        }
        let boost = if input.boost && self.boost_reserve > 0.0 {
            12.0
        } else {
            0.0
        };
        let forward = Vec3::new(-ship.yaw.sin(), 0.0, -ship.yaw.cos());
        ship.position = ship.position.add(forward.mul((self.speed + boost) * dt));

        let vertical_control = input.rise as i8 as f32 - input.descend as i8 as f32;
        let gravity = if self.inversion_complete { 2.0 } else { -2.0 };
        self.vertical_speed =
            (self.vertical_speed + (vertical_control * 14.0 + gravity) * dt) * (-0.72 * dt).exp();
        self.vertical_speed = self.vertical_speed.clamp(-10.0, 10.0);
        ship.position.y += self.vertical_speed * dt;
        ship.pitch = (self.vertical_speed * 0.025).clamp(-0.28, 0.28);
        ship.roll_target = ship.steering_bank;
        let difference = ship.roll_target - ship.roll;
        ship.roll += difference * (1.0 - (-5.0 * dt).exp());
        ship.speed = self.speed;
    }

    pub(super) fn resolve_collisions(&mut self, ship: &mut Ship, _dt: f32) {
        let radius = ship.position.x.hypot(ship.position.z);
        let room_radius = if ship.position.y > 77.0 { 58.0 } else { 69.0 };
        let mut impact = false;
        if radius > room_radius {
            let corrected = ship.position.xz_normalized().mul(room_radius);
            ship.position.x = corrected.x;
            ship.position.z = corrected.z;
            impact = self.speed > 13.0;
            self.speed *= 0.35;
        }
        if ship.position.y > 49.0 && ship.position.y < 80.0 && radius > 15.5 {
            let corrected = ship.position.xz_normalized().mul(15.2);
            ship.position.x = corrected.x;
            ship.position.z = corrected.z;
            impact |= self.speed > 13.0;
            self.speed *= 0.25;
        }
        let sand = -8.0 + dune_height(ship.position.x, ship.position.z) + self.sand_offset;
        if ship.position.y < sand + 4.0 {
            ship.position.y = sand + 4.0;
            impact |= self.vertical_speed < -7.0;
            self.vertical_speed = if self.inversion_complete { 0.4 } else { 3.2 };
            self.feedback = "CONTACTO CON ARENA - GANA ALTURA";
            self.feedback_timer = 1.2;
        }
        if ship.position.y > 133.0 {
            ship.position.y = 133.0;
            impact |= self.vertical_speed > 8.0;
            self.vertical_speed = 0.0;
        }
        if impact && self.hit_cooldown <= 0.0 && self.phase != Phase::Inverting {
            self.shields = self.shields.saturating_sub(1);
            self.hit_cooldown = 1.2;
            self.feedback = "IMPACTO - ESCUDO REDUCIDO";
            self.feedback_timer = 1.6;
            self.audio.play(Cue::Damage);
        }
        if self.hit_cooldown <= 0.0 && ship.position.y > sand + 7.0 && radius < room_radius - 5.0 {
            self.safe_position = ship.position;
        }
    }
}

trait HorizontalNormalize {
    fn xz_normalized(self) -> Vec3;
}

impl HorizontalNormalize for Vec3 {
    fn xz_normalized(self) -> Vec3 {
        let length = self.x.hypot(self.z).max(0.001);
        Vec3::new(self.x / length, 0.0, self.z / length)
    }
}
