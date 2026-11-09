use crate::obj_loader::Vec3;
use crate::ship::{Ship, ShipInput};

use super::scene::dune_height;
use super::{Cue, HourglassRescue, Input};

impl HourglassRescue {
    pub(super) fn move_ship(&mut self, ship: &mut Ship, input: Input, dt: f32) {
        // Same steering and speed response as the free-world ship. With no
        // input it returns to cruise speed, so the level always moves forward.
        ship.update_flight(
            &ShipInput {
                pitch: input.rise as i8 - input.descend as i8,
                yaw: input.right as i8 - input.left as i8,
                roll: input.roll_right as i8 - input.roll_left as i8,
                accelerating: input.forward,
                braking: input.brake,
                firing: false,
            },
            dt,
        );
        if input.boost && self.boost_reserve > 0.0 {
            self.boost_reserve = (self.boost_reserve - dt).max(0.0);
        } else {
            self.boost_reserve = (self.boost_reserve + dt * 0.26).min(2.2);
        }
        let boost = if input.boost && self.boost_reserve > 0.0 {
            45.0
        } else {
            0.0
        };
        // The time planet uses the free-flight controls with a slightly faster
        // traversal pace so its distant objectives do not feel sluggish.
        let travel_speed = ship.speed * 1.35 + boost;
        self.speed = travel_speed;
        let forward = ship.movement_basis().0;
        ship.position = ship.position.add(forward.mul(travel_speed * dt));
        self.vertical_speed = forward.y * travel_speed;
    }

    pub(super) fn resolve_collisions(&mut self, ship: &mut Ship, _dt: f32) {
        let radius = ship.position.x.hypot(ship.position.z);
        // The rescue mission takes place in a broad valley, not inside the
        // narrow neck of an hourglass. Keep a generous soft boundary so the
        // pilot can search the whole map at either gravity orientation.
        let room_radius = 520.0;
        let mut impact = false;
        if radius > room_radius {
            let corrected = ship.position.xz_normalized().mul(room_radius);
            ship.position.x = corrected.x;
            ship.position.z = corrected.z;
            impact = self.speed > 13.0;
            self.speed *= 0.35;
        }
        if let Some(corrected) = self.scene.resolve_obstacle(ship.position, 3.2) {
            ship.position = corrected;
            impact |= self.speed > 9.0;
            self.speed *= 0.35;
            self.vertical_speed *= 0.5;
        }
        let sand = -8.0 + dune_height(ship.position.x, ship.position.z) + self.sand_offset;
        if ship.position.y < sand + 4.0 {
            ship.position.y = sand + 4.0;
            impact |= self.vertical_speed < -7.0;
            self.vertical_speed = 3.2;
            self.feedback = "CONTACTO CON ARENA - GANA ALTURA";
            self.feedback_timer = 1.2;
        }
        if ship.position.y > 210.0 {
            ship.position.y = 210.0;
            impact |= self.vertical_speed > 8.0;
            self.vertical_speed = 0.0;
        }
        if impact && self.hit_cooldown <= 0.0 {
            self.shields = self.shields.saturating_sub(1);
            self.hit_cooldown = 1.2;
            self.elapsed += 8.0;
            self.feedback = "IMPACTO ENEMIGO  -8 SEGUNDOS";
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
