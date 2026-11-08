#[allow(dead_code)]
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

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioCue {
    Shot,
    Impact,
    Damage,
    BossAlert,
    Reload,
    Door,
    ZombieDie,
    NoAmmo,
    Axe,
    Flame,
    Key,
    Heal,
    CollectAmmo,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default)]
pub struct InputFrame {
    pub cursor: (f32, f32),
    pub cursor_moved: bool,
    pub fire: bool,
    pub fire_held: bool,
    pub mouse_clicked: bool,
    pub confirm: bool,
    pub exit: bool,
    pub reload: bool,
    pub arrow_x: i8,
    pub arrow_y: i8,
    pub move_x: i8,
    pub move_y: i8,
    pub interact: bool,
    pub look_x: f32,
    pub look_y: f32,
    pub aiming: bool,
    pub weapon: u8,
}
