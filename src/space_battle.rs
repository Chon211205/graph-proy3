//! Lightweight ambient dogfights for the two explorable star systems.
//!
//! Bolts are decorative only: they are steered toward rival fighters or toward
//! a point well beside the player, and never participate in player collision.

use crate::obj_loader::Vec3;
use crate::ship::{Projectile, ShipSpec};

pub const FIGHTER_COUNT: usize = 4;

pub struct Fighter {
    pub ship_index: usize,
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub shots: Vec<Projectile>,
    next_shot: f32,
    shot_serial: u32,
}

pub struct SpaceBattle {
    pub fighters: [Fighter; FIGHTER_COUNT],
}

impl SpaceBattle {
    pub fn new() -> Self {
        Self {
            fighters: std::array::from_fn(|index| Fighter {
                ship_index: index,
                position: Vec3::new(0.0, 0.0, 0.0),
                yaw: 0.0,
                pitch: 0.0,
                shots: Vec::with_capacity(2),
                next_shot: 1.2 + index as f32 * 0.73,
                shot_serial: index as u32 * 17,
            }),
        }
    }

    /// Assign four distinct non-player ships and advance a restrained dogfight.
    /// Positions are deterministic orbital passes around each system's center.
    pub fn update(
        &mut self,
        delta_time: f32,
        elapsed: f32,
        center: Vec3,
        player_position: Vec3,
        selected_ship: usize,
        choices: &[(&crate::obj_loader::Model, &'static ShipSpec)],
    ) {
        let mut available = (0..choices.len())
            .filter(|&index| index != selected_ship)
            .cycle();
        let positions: [Vec3; FIGHTER_COUNT] = std::array::from_fn(|slot| {
            let model_index = (0..choices.len())
                .filter(|&index| index != selected_ship)
                .cycle()
                .nth(slot)
                .unwrap_or(0);
            let phase = elapsed * 0.075 + model_index as f32 * std::f32::consts::FRAC_PI_2;
            let radius = 112.0 + (slot % 2) as f32 * 40.0;
            let angle = phase + if slot < 2 { -1.0 } else { 1.0 } * (elapsed * 0.022).sin() * 0.55;
            center.add(Vec3::new(
                angle.cos() * radius,
                24.0 + (elapsed * 0.21 + slot as f32).sin() * 15.0,
                angle.sin() * radius * 0.68,
            ))
        });
        for (slot, fighter) in self.fighters.iter_mut().enumerate() {
            fighter.ship_index = available.next().unwrap_or(0);
            let target = positions[slot];
            fighter.position = target;
            // The two teams cross through one another rather than forming a ring.
            let opposing_index = if slot < 2 { slot + 2 } else { slot - 2 };
            let facing = positions[opposing_index].sub(target);
            let (yaw, pitch) = direction_angles(facing);
            fighter.yaw = yaw;
            fighter.pitch = pitch;

            for shot in &mut fighter.shots {
                shot.position = shot.position.add(shot.direction.mul(155.0 * delta_time));
                shot.lifetime -= delta_time;
            }
            fighter.shots.retain(|shot| shot.lifetime > 0.0);
            fighter.next_shot -= delta_time;
            if fighter.next_shot <= 0.0 && fighter.shots.len() < 2 {
                fighter.shot_serial = fighter.shot_serial.wrapping_add(1);
                let aim = if fighter.shot_serial % 3 == 0 {
                    // These shots pass conspicuously near the player but always miss.
                    let side = (fighter.shot_serial % 2) as f32 * 2.0 - 1.0;
                    let miss = Vec3::new(side * 58.0, 18.0, -side * 42.0);
                    player_position.add(miss)
                } else {
                    positions[opposing_index]
                };
                let direction = aim.sub(target).normalize();
                fighter.shots.push(Projectile {
                    position: target.add(direction.mul(5.0)),
                    direction,
                    lifetime: (aim.sub(target).dot(aim.sub(target)).sqrt() / 155.0)
                        .clamp(0.45, 2.6),
                    scale: 1.8,
                });
                fighter.next_shot = 2.6 + ((fighter.shot_serial * 13 % 19) as f32 * 0.11);
            }
        }
    }
}

fn direction_angles(direction: Vec3) -> (f32, f32) {
    let direction = direction.normalize();
    (
        (-direction.z).atan2(-direction.x).rem_euclid(std::f32::consts::TAU),
        direction.y.asin(),
    )
}
