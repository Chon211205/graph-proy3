use super::config::EnemyKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Intro,
    Encounter,
    Resolving,
    Transition,
    Boss,
    Paused,
    Victory,
    Defeat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameAction {
    Continue,
    Restart,
    Exit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioCue {
    Shot,
    Impact,
    Damage,
    BossAlert,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct InputFrame {
    pub cursor: (f32, f32),
    pub cursor_moved: bool,
    pub fire: bool,
    pub mouse_clicked: bool,
    pub confirm: bool,
    pub exit: bool,
    pub reload: bool,
    pub arrow_x: i8,
    pub arrow_y: i8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifeState {
    Telegraph,
    Walking,
    Covered,
    Escaping,
    Falling,
    Gone,
}

pub struct Enemy {
    pub kind: EnemyKind,
    pub state: LifeState,
    pub health: u8,
    pub resolved: bool,
    pub start_x: f32,
    pub target_x: f32,
    pub x: f32,
    pub depth: f32,
    pub progress: f32,
    pub lifetime: f32,
    pub elapsed: f32,
    pub state_timer: f32,
    pub cover_used: bool,
    pub hit_flash: f32,
    pub killed_by_headshot: bool,
    pub seed: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BossKind {
    Hat,
    Rocket,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BossCycle {
    Reposition,
    Covered,
    Windup,
    Vulnerable,
    Attack,
    PrepareRocket,
    RocketFlying,
    ExposedCore,
    Recover,
    Dying,
    Dead,
}

pub struct BossState {
    pub kind: BossKind,
    pub cycle: BossCycle,
    pub health: i32,
    pub max_health: i32,
    pub cycle_time: f32,
    pub x: f32,
    pub target_x: f32,
    pub phase_two: bool,
    pub helper_spawned: u8,
    pub helper_timer: f32,
    pub attacks_resolved: u32,
    pub hits_this_window: u8,
    pub hit_flash: f32,
    pub interrupted: bool,
    pub rocket: Option<Rocket>,
}

pub struct Rocket {
    pub x: f32,
    pub y: f32,
    pub progress: f32,
    pub resolved: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Statistics {
    pub shots: u32,
    pub hits: u32,
    pub headshots: u32,
    pub kills: u32,
    pub escapes: u32,
    pub score: u32,
}

impl Statistics {
    pub fn accuracy(self) -> u32 {
        if self.shots == 0 {
            0
        } else {
            ((self.hits as f32 / self.shots as f32) * 100.0).round() as u32
        }
    }
}
