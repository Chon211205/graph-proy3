use crate::ship::Ship;

use super::{
    CapsuleState, Cue, HourglassRescue, Input, CAPSULE_COUNT, CAPSULE_POSITIONS, RESCUE_DISTANCE,
    RESCUE_MAX_SPEED, RESCUE_SECONDS,
};

impl HourglassRescue {
    pub(super) fn update_clocks(&mut self, ship: &Ship) {
        for index in 0..super::CLOCK_COUNT {
            if self.clocks[index] {
                continue;
            }
            let offset = ship.position.sub(super::CLOCK_POSITIONS[index]);
            if offset.dot(offset) <= 8.0 * 8.0 {
                self.clocks[index] = true;
                self.clocks_collected += 1;
                self.elapsed -= super::CLOCK_BONUS_SECONDS;
                self.scene.hide_clock(index);
                self.audio.play(Cue::Capsule);
                self.feedback = "RELOJ RECOGIDO  +20 SEGUNDOS";
                self.feedback_timer = 2.0;
            }
        }
    }

    pub(super) fn update_rescue(&mut self, ship: &Ship, input: Input, dt: f32) {
        let target = (0..CAPSULE_COUNT)
            .filter(|&index| self.capsules[index] == CapsuleState::Pending)
            .min_by(|&a, &b| {
                let da = ship.position.sub(CAPSULE_POSITIONS[a]);
                let db = ship.position.sub(CAPSULE_POSITIONS[b]);
                da.dot(da).total_cmp(&db.dot(db))
            });
        self.rescue_target = target;
        let Some(index) = target else {
            self.rescue_progress = 0.0;
            return;
        };
        let offset = ship.position.sub(CAPSULE_POSITIONS[index]);
        let close_enough = offset.dot(offset) <= RESCUE_DISTANCE * RESCUE_DISTANCE;
        if close_enough && self.speed <= RESCUE_MAX_SPEED && input.rescue {
            self.rescue_progress += dt;
            if self.rescue_progress >= RESCUE_SECONDS {
                self.capsules[index] = CapsuleState::Rescued;
                self.scene.hide_capsule(index);
                self.rescued += 1;
                self.audio.play(Cue::Capsule);
                self.rescue_progress = 0.0;
                self.rescue_target = None;
                self.feedback = "FRAGMENTO DE ENERGIA RECOGIDO";
                self.feedback_timer = 1.5;
                if self.rescued as usize == CAPSULE_COUNT {
                    self.scene.set_portal_open(true);
                    self.feedback = "PUERTA DESBLOQUEADA - BUSCA LA ESFERA DEL DRAGON";
                    self.feedback_timer = 3.0;
                }
            }
        } else {
            self.rescue_progress = 0.0;
        }
    }

    pub(super) fn update_capsule_visibility(&mut self) {
        for index in 0..CAPSULE_COUNT {
            let visible = self.capsules[index] == CapsuleState::Pending;
            self.scene.set_capsule_visible(index, visible);
        }
    }

    pub(super) fn target_distance(&self, ship: &Ship) -> Option<f32> {
        if self.rescued as usize == CAPSULE_COUNT {
            let offset = ship.position.sub(super::EXIT_POSITION);
            return Some(offset.dot(offset).sqrt());
        }
        self.rescue_target.map(|index| {
            let offset = ship.position.sub(CAPSULE_POSITIONS[index]);
            offset.dot(offset).sqrt()
        })
    }
}
