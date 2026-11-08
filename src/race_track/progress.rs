use crate::obj_loader::Vec3;

use super::course::{RaceCourse, HALF_ROAD};

pub(super) const TOTAL_LAPS: u8 = 4;
const CHECKPOINT_COUNT: usize = 12;

#[derive(Clone, Copy, Debug)]
pub(super) struct Checkpoint {
    pub center: Vec3,
    pub tangent: Vec3,
    pub half_width: f32,
    pub course_distance: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ProgressEvent {
    Checkpoint,
    Lap,
    FinalLap,
    Finished,
}

pub(super) struct RaceProgress {
    checkpoints: Vec<Checkpoint>,
    current_checkpoint: usize,
    current_lap: u8,
    race_time: f32,
    lap_time: f32,
    last_lap_time: Option<f32>,
    best_lap_time: Option<f32>,
    finished: bool,
    event: Option<ProgressEvent>,
    event_time: f32,
}

impl RaceProgress {
    pub fn new(course: &RaceCourse) -> Self {
        let mut checkpoints = Vec::with_capacity(CHECKPOINT_COUNT);
        for index in 1..CHECKPOINT_COUNT {
            let distance = course.length * index as f32 / CHECKPOINT_COUNT as f32;
            let sample = course.sample_at(distance);
            checkpoints.push(Checkpoint {
                center: sample.position,
                tangent: sample.tangent,
                half_width: HALF_ROAD + 1.5,
                course_distance: distance,
            });
        }
        let start = course.sample_at(0.0);
        checkpoints.push(Checkpoint {
            center: start.position,
            tangent: start.tangent,
            half_width: HALF_ROAD + 1.5,
            course_distance: course.length,
        });
        Self::from_checkpoints(checkpoints)
    }

    fn from_checkpoints(checkpoints: Vec<Checkpoint>) -> Self {
        Self {
            checkpoints,
            current_checkpoint: 0,
            current_lap: 1,
            race_time: 0.0,
            lap_time: 0.0,
            last_lap_time: None,
            best_lap_time: None,
            finished: false,
            event: None,
            event_time: 0.0,
        }
    }

    pub fn update(&mut self, previous: Vec3, current: Vec3, dt: f32) -> Option<ProgressEvent> {
        if self.finished {
            return None;
        }
        let dt = dt.max(0.0);
        self.race_time += dt;
        self.lap_time += dt;
        self.event_time = (self.event_time - dt).max(0.0);
        if self.event_time == 0.0 {
            self.event = None;
        }

        let gate = self.checkpoints[self.current_checkpoint];
        if !crosses_gate_forward(previous, current, gate) {
            return None;
        }
        self.current_checkpoint += 1;
        if self.current_checkpoint < self.checkpoints.len() {
            self.set_event(ProgressEvent::Checkpoint, 0.8);
            return Some(ProgressEvent::Checkpoint);
        }

        self.current_checkpoint = 0;
        let completed_time = self.lap_time;
        self.last_lap_time = Some(completed_time);
        self.best_lap_time = Some(
            self.best_lap_time
                .map_or(completed_time, |best| best.min(completed_time)),
        );
        self.lap_time = 0.0;
        if self.current_lap >= TOTAL_LAPS {
            self.finished = true;
            self.set_event(ProgressEvent::Finished, 4.0);
            return Some(ProgressEvent::Finished);
        }

        self.current_lap += 1;
        let event = if self.current_lap == TOTAL_LAPS {
            ProgressEvent::FinalLap
        } else {
            ProgressEvent::Lap
        };
        self.set_event(event, 1.5);
        Some(event)
    }

    fn set_event(&mut self, event: ProgressEvent, duration: f32) {
        self.event = Some(event);
        self.event_time = duration;
    }

    pub fn active_checkpoint(&self) -> Option<Checkpoint> {
        self.checkpoints.get(self.current_checkpoint).copied()
    }

    pub fn checkpoints(&self) -> &[Checkpoint] {
        &self.checkpoints
    }

    pub fn current_checkpoint(&self) -> usize {
        self.current_checkpoint
    }

    pub fn checkpoint_count(&self) -> usize {
        self.checkpoints.len()
    }

    pub fn current_lap(&self) -> u8 {
        self.current_lap
    }

    pub fn total_laps(&self) -> u8 {
        TOTAL_LAPS
    }

    pub fn race_time(&self) -> f32 {
        self.race_time
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn event(&self) -> Option<ProgressEvent> {
        if self.event_time > 0.0 {
            self.event
        } else {
            None
        }
    }

    pub fn progress_score(&self, course_distance: f32, course_length: f32) -> f32 {
        if self.finished {
            return TOTAL_LAPS as f32 * course_length;
        }
        let course_length = course_length.max(1.0);
        let normalized_distance = course_distance.rem_euclid(course_length);
        let previous_gate_distance = if self.current_checkpoint == 0 {
            0.0
        } else {
            self.checkpoints[self.current_checkpoint - 1].course_distance
        };
        let next_gate_distance = self.checkpoints[self.current_checkpoint].course_distance;
        let segment_length =
            (next_gate_distance - previous_gate_distance).rem_euclid(course_length);
        let from_previous =
            (normalized_distance - previous_gate_distance).rem_euclid(course_length);
        let segment_progress = if self.current_checkpoint == 0
            && self.current_lap == 1
            && normalized_distance > course_length * 0.82
        {
            0.0
        } else {
            from_previous.min(segment_length)
        };
        (self.current_lap.saturating_sub(1) as f32) * course_length
            + previous_gate_distance
            + segment_progress
    }
}

fn crosses_gate_forward(previous: Vec3, current: Vec3, gate: Checkpoint) -> bool {
    let before = Vec3::new(previous.x - gate.center.x, 0.0, previous.z - gate.center.z);
    let after = Vec3::new(current.x - gate.center.x, 0.0, current.z - gate.center.z);
    let before_along = before.dot(gate.tangent);
    let after_along = after.dot(gate.tangent);
    if before_along > 0.0 || after_along < 0.0 || after_along - before_along < 0.001 {
        return false;
    }
    let amount = (-before_along / (after_along - before_along)).clamp(0.0, 1.0);
    let crossing = before.add(after.sub(before).mul(amount));
    let lateral = crossing.sub(gate.tangent.mul(crossing.dot(gate.tangent)));
    lateral.dot(lateral) <= gate.half_width * gate.half_width
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one_gate() -> Checkpoint {
        Checkpoint {
            center: Vec3::new(0.0, 0.0, 0.0),
            tangent: Vec3::new(1.0, 0.0, 0.0),
            half_width: 5.0,
            course_distance: 10.0,
        }
    }

    #[test]
    fn finish_gate_counts_forward_crossings_but_not_reverse_crossings() {
        let mut progress = RaceProgress::from_checkpoints(vec![one_gate()]);
        let behind = Vec3::new(-1.0, 0.0, 0.0);
        let ahead = Vec3::new(1.0, 0.0, 0.0);
        assert_eq!(progress.update(ahead, behind, 1.0), None);
        assert_eq!(progress.current_lap(), 1);
        assert_eq!(
            progress.update(behind, ahead, 2.0),
            Some(ProgressEvent::Lap)
        );
        assert_eq!(progress.current_lap(), 2);
        assert_eq!(progress.update(ahead, behind, 1.0), None);
        assert_eq!(
            progress.update(behind, ahead, 3.0),
            Some(ProgressEvent::Lap)
        );
        assert_eq!(
            progress.update(behind, ahead, 4.0),
            Some(ProgressEvent::FinalLap)
        );
        assert_eq!(
            progress.update(behind, ahead, 5.0),
            Some(ProgressEvent::Finished)
        );
        assert!(progress.is_finished());
    }

    #[test]
    fn skips_and_off_lane_crossings_do_not_advance_checkpoint() {
        let first = one_gate();
        let second = Checkpoint {
            center: Vec3::new(10.0, 0.0, 0.0),
            ..first
        };
        let mut progress = RaceProgress::from_checkpoints(vec![first, second]);
        assert_eq!(
            progress.update(Vec3::new(-1.0, 0.0, 6.0), Vec3::new(1.0, 0.0, 6.0), 0.1),
            None
        );
        assert_eq!(progress.current_checkpoint(), 0);
        assert_eq!(
            progress.update(Vec3::new(-1.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), 0.1),
            Some(ProgressEvent::Checkpoint)
        );
        assert_eq!(progress.current_checkpoint(), 1);
    }

    #[test]
    fn finish_gate_covers_the_full_drivable_road_width() {
        let mut gate = one_gate();
        gate.half_width = HALF_ROAD + 1.5;
        let previous = Vec3::new(-1.0, 0.0, HALF_ROAD + 0.8);
        let current = Vec3::new(1.0, 0.0, HALF_ROAD + 0.8);
        assert!(crosses_gate_forward(previous, current, gate));
    }

    #[test]
    fn finished_race_time_is_frozen() {
        let mut progress = RaceProgress::from_checkpoints(vec![one_gate()]);
        let behind = Vec3::new(-1.0, 0.0, 0.0);
        let ahead = Vec3::new(1.0, 0.0, 0.0);
        for _ in 0..TOTAL_LAPS - 1 {
            progress.update(behind, ahead, 1.0);
            progress.update(ahead, behind, 0.1);
        }
        progress.update(behind, ahead, 1.0);
        let finished_at = progress.race_time();
        progress.update(behind, ahead, 20.0);
        assert_eq!(progress.race_time(), finished_at);
    }
}
