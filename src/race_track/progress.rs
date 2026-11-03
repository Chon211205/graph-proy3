use crate::obj_loader::Vec3;

use super::TILE_SIZE;

const TOTAL_LAPS: u8 = 3;

#[derive(Clone, Copy, Debug)]
pub(super) struct Checkpoint {
    pub center: Vec3,
    pub radius: f32,
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
    pub fn new() -> Self {
        // Gates follow the existing Garden track centerline in tile space.
        let gates = [
            (29.0, 34.0, 2.8),
            (48.0, 34.0, 2.8),
            (57.0, 30.0, 2.8),
            (57.0, 20.0, 2.8),
            (51.0, 7.0, 2.8),
            (38.0, 7.0, 2.8),
            (25.0, 7.0, 2.8),
            (12.0, 7.0, 2.8),
            (8.0, 16.0, 2.8),
            (8.0, 27.0, 2.8),
            (17.0, 33.8, 2.8),
            (22.5, 33.9, 1.6),
        ];
        Self {
            checkpoints: gates
                .into_iter()
                .map(|(x, z, radius)| Checkpoint {
                    center: Vec3::new(x * TILE_SIZE, 2.0, z * TILE_SIZE),
                    radius: radius * TILE_SIZE,
                })
                .collect(),
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
        self.race_time += dt;
        self.lap_time += dt;
        self.event_time = (self.event_time - dt).max(0.0);
        if self.event_time == 0.0 {
            self.event = None;
        }

        let gate = self.checkpoints[self.current_checkpoint];
        if !segment_crosses_circle(previous, current, gate.center, gate.radius) {
            return None;
        }

        self.current_checkpoint += 1;
        if self.current_checkpoint < self.checkpoints.len() {
            self.set_event(ProgressEvent::Checkpoint, 0.9);
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
            Some(ProgressEvent::Finished)
        } else {
            self.current_lap += 1;
            let event = if self.current_lap == TOTAL_LAPS {
                ProgressEvent::FinalLap
            } else {
                ProgressEvent::Lap
            };
            self.set_event(event, 1.6);
            Some(event)
        }
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

    pub fn lap_time(&self) -> f32 {
        self.lap_time
    }

    pub fn best_lap_time(&self) -> Option<f32> {
        self.best_lap_time
    }

    pub fn last_lap_time(&self) -> Option<f32> {
        self.last_lap_time
    }

    pub fn event(&self) -> Option<ProgressEvent> {
        if self.event_time > 0.0 {
            self.event
        } else {
            None
        }
    }
}

fn segment_crosses_circle(start: Vec3, end: Vec3, center: Vec3, radius: f32) -> bool {
    let start = Vec3::new(start.x, 0.0, start.z);
    let end = Vec3::new(end.x, 0.0, end.z);
    let center = Vec3::new(center.x, 0.0, center.z);
    let movement = end.sub(start);
    let length_squared = movement.dot(movement);
    let t = if length_squared > 0.0001 {
        center.sub(start).dot(movement) / length_squared
    } else {
        0.0
    }
    .clamp(0.0, 1.0);
    let closest = start.add(movement.mul(t));
    let offset = closest.sub(center);
    offset.dot(offset) <= radius * radius
}
