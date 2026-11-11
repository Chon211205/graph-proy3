use crate::ship::Ship;

use super::{
    CAPSULE_COUNT, CAPSULE_POSITIONS, CapsuleState, Cue, HourglassRescue, RESCUE_DISTANCE,
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

    pub(super) fn update_rescue(&mut self, ship: &Ship) {
        let target = (0..CAPSULE_COUNT)
            .filter(|&index| self.capsules[index] == CapsuleState::Pending)
            .min_by(|&a, &b| {
                let da = ship.position.sub(CAPSULE_POSITIONS[a]);
                let db = ship.position.sub(CAPSULE_POSITIONS[b]);
                da.dot(da).total_cmp(&db.dot(db))
            });
        self.rescue_target = target;
        let Some(index) = target else {
            return;
        };
        let offset = ship.position.sub(CAPSULE_POSITIONS[index]);
        let close_enough = offset.dot(offset) <= RESCUE_DISTANCE * RESCUE_DISTANCE;
        if close_enough {
            self.capsules[index] = CapsuleState::Rescued;
            self.scene.hide_capsule(index);
            self.rescued += 1;
            self.audio.play(Cue::Capsule);
            self.rescue_target = None;
            self.feedback = "FRAGMENTO DE ENERGIA RECOGIDO";
            self.feedback_timer = 1.5;
            if self.rescued as usize == CAPSULE_COUNT {
                self.phase = super::Phase::Reward;
                self.reward_elapsed = 0.0;
                self.reward_position = ship.position;
                self.scene.set_reward_transform(
                    self.reward_position
                        .add(crate::obj_loader::Vec3::new(0.0, 13.0, 0.0)),
                    1.0,
                    0.0,
                );
                self.feedback = "RECIBIENDO ESFERA DE 5 ESTRELLAS";
                self.feedback_timer = 1.0;
            }
        }
    }

    pub(super) fn update_capsule_visibility(&mut self) {
        for index in 0..CAPSULE_COUNT {
            let visible = self.capsules[index] == CapsuleState::Pending;
            self.scene.set_capsule_visible(index, visible);
        }
    }

    pub(super) fn target_distance(&self, ship: &Ship) -> Option<f32> {
        self.rescue_target.map(|index| {
            let offset = ship.position.sub(CAPSULE_POSITIONS[index]);
            offset.dot(offset).sqrt()
        })
    }
}
