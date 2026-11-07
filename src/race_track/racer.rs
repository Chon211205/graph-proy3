use crate::{
    obj_loader::Vec3,
    ship::{ShipController, ShipSpec},
};

use super::{
    course::{HALF_ROAD, RaceCourse},
    drive::{DriveState, RaceInput},
    progress::{ProgressEvent, RaceProgress},
};

pub(super) struct Rival {
    pub controller: ShipController,
    pub drive: DriveState,
    pub progress: RaceProgress,
    pub vehicle_index: usize,
    pub skill: f32,
    pub use_lod: bool,
    lane_offset: f32,
    safe_position: Vec3,
    safe_yaw: f32,
    stuck_time: f32,
}

impl Rival {
    pub fn new(
        course: &RaceCourse,
        vehicle_index: usize,
        spec: &'static ShipSpec,
        grid_back: f32,
        lane_offset: f32,
        skill: f32,
    ) -> Self {
        let start = course.sample_at(course.length - grid_back);
        let side = Vec3::new(-start.tangent.z, 0.0, start.tangent.x);
        let position = start
            .position
            .add(side.mul(lane_offset))
            .add(Vec3::new(0.0, 1.2, 0.0));
        let mut controller = ShipController::new(spec);
        controller.ship.position = position;
        controller.ship.yaw = yaw_from_direction(start.tangent);
        controller.ship.pitch = 0.0;
        controller.ship.speed = 0.0;
        controller.ship.planet_locked = false;
        controller.ship.race_planet_locked = false;
        controller.ship.orbit_center = None;
        controller.ship.orbit_plane_normal = None;
        controller.ship.orbit_camera_blend = 0.0;
        Self {
            controller,
            drive: DriveState::new(),
            progress: RaceProgress::new(course),
            vehicle_index,
            skill,
            use_lod: false,
            lane_offset,
            safe_position: position,
            safe_yaw: yaw_from_direction(start.tangent),
            stuck_time: 0.0,
        }
    }

    pub fn update(&mut self, course: &RaceCourse, dt: f32) {
        if self.progress.is_finished() {
            return;
        }
        let (distance, offroad, current) = course.nearest(self.controller.ship.position);
        if offroad > HALF_ROAD * 1.5 {
            self.stuck_time += dt;
        } else {
            self.stuck_time = 0.0;
        }
        if self.stuck_time > 2.0 {
            self.controller.ship.position = self.safe_position;
            self.controller.ship.yaw = self.safe_yaw;
            self.drive.recover();
            self.stuck_time = 0.0;
        }

        let look_ahead = 12.0 + self.drive.speed.abs() * 0.52;
        let target = course.sample_at(distance + look_ahead);
        let side = Vec3::new(-target.tangent.z, 0.0, target.tangent.x);
        let target_position = target.position.add(side.mul(self.lane_offset));
        let to_target = Vec3::new(
            target_position.x - self.controller.ship.position.x,
            0.0,
            target_position.z - self.controller.ship.position.z,
        )
        .normalize();
        let desired_yaw = yaw_from_direction(to_target);
        let yaw_error = angle_delta(self.controller.ship.yaw, desired_yaw);
        let ahead = course.sample_at(distance + look_ahead + 16.0);
        let curvature = 1.0 - current.tangent.dot(ahead.tangent).clamp(-1.0, 1.0);
        let desired_speed = (41.0 - curvature * 14.0) * self.skill;
        let speed_error = desired_speed - self.drive.speed;
        let input = RaceInput {
            accelerate: speed_error > -1.0,
            brake: speed_error < -4.0,
            steering: if yaw_error.abs() < 0.035 {
                0
            } else if yaw_error > 0.0 {
                -1
            } else {
                1
            },
            drift: curvature > 0.16 && yaw_error.abs() > 0.32,
            boost: curvature < 0.08 && self.drive.speed > desired_speed * 0.65,
            ..RaceInput::default()
        };
        let previous = self.controller.ship.position;
        self.drive
            .update(&mut self.controller.ship, course, input, dt);
        if matches!(
            self.progress
                .update(previous, self.controller.ship.position, dt),
            Some(ProgressEvent::Checkpoint | ProgressEvent::Lap | ProgressEvent::FinalLap)
        ) {
            self.safe_position = self.controller.ship.position;
            self.safe_yaw = self.controller.ship.yaw;
        }
    }
}

pub(super) fn yaw_from_direction(direction: Vec3) -> f32 {
    (-direction.z)
        .atan2(-direction.x)
        .rem_euclid(std::f32::consts::TAU)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ship::COWBOYB_CAR_SPEC;

    #[test]
    fn rival_completes_four_laps_on_the_course_without_teleporting() {
        let course = RaceCourse::new();
        let mut rival = Rival::new(&course, 3, &COWBOYB_CAR_SPEC, 22.0, 0.0, 0.92);
        for _ in 0..(240 * 60) {
            rival.update(&course, 1.0 / 60.0);
            if rival.progress.is_finished() {
                break;
            }
        }
        assert!(rival.progress.is_finished(), "rival did not finish in 240s");
        assert!(rival.progress.race_time() >= 120.0);
        assert!(rival.progress.race_time() <= 240.0);
        assert!(rival.controller.ship.position.x.is_finite());
        assert!(rival.controller.ship.position.z.is_finite());
    }
}

fn angle_delta(from: f32, to: f32) -> f32 {
    (to - from + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}
