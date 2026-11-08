mod audio;
mod flight;
mod hud;
mod objectives;
mod rendering;
mod results;
mod scene;
#[cfg(test)]
mod tests;
mod ui;

use crate::framebuffer::Framebuffer;
use crate::obj_loader::Vec3;
use crate::ship::Ship;

use audio::{Audio, Cue};
use scene::{Scene, CAPSULE_COUNT, CAPSULE_POSITIONS};

const MISSION_SECONDS: f32 = 240.0;
const INVERSION_AT: f32 = 120.0;
const INVERSION_WARNING: f32 = 6.0;
const INVERSION_DURATION: f32 = 2.8;
const RESCUE_SECONDS: f32 = 1.0;
const RESCUE_DISTANCE: f32 = 10.0;
const RESCUE_MAX_SPEED: f32 = 4.5;
const EXIT_POSITION: Vec3 = Vec3::new(0.0, 155.0, -505.0);
const SAND_RISE: f32 = 18.0;
const SAND_FALL_RATE: f32 = (SAND_RISE + 1.8) / (MISSION_SECONDS - INVERSION_AT);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Intro,
    Playing,
    Warning,
    Inverting,
    Paused,
    Won,
    Lost,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    pub forward: bool,
    pub brake: bool,
    pub left: bool,
    pub right: bool,
    pub rise: bool,
    pub descend: bool,
    pub boost: bool,
    pub rescue: bool,
    pub pause: bool,
    pub exit: bool,
    pub restart: bool,
    pub recover: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Continue,
    Exit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CapsuleState {
    Pending,
    Rescued,
}

pub struct HourglassRescue {
    phase: Phase,
    phase_before_pause: Phase,
    phase_timer: f32,
    elapsed: f32,
    visual_time: f32,
    shields: u8,
    hit_cooldown: f32,
    inversion_complete: bool,
    boost_reserve: f32,
    sand_offset: f32,
    rescue_progress: f32,
    rescue_target: Option<usize>,
    capsules: [CapsuleState; CAPSULE_COUNT],
    rescued: u8,
    speed: f32,
    vertical_speed: f32,
    safe_position: Vec3,
    feedback: &'static str,
    feedback_timer: f32,
    audio: Audio,
    scene: Scene,
}

impl HourglassRescue {
    pub fn new() -> Self {
        Self {
            phase: Phase::Intro,
            phase_before_pause: Phase::Intro,
            phase_timer: 3.0,
            elapsed: 0.0,
            visual_time: 0.0,
            shields: 3,
            hit_cooldown: 0.0,
            inversion_complete: false,
            boost_reserve: 2.2,
            sand_offset: 0.0,
            rescue_progress: 0.0,
            rescue_target: None,
            capsules: [CapsuleState::Pending; CAPSULE_COUNT],
            rescued: 0,
            speed: 0.0,
            vertical_speed: 0.0,
            safe_position: Vec3::new(0.0, 12.0, 42.0),
            feedback: "RESCATA LAS CINCO CAPSULAS Y ALCANZA LA SALIDA",
            feedback_timer: 4.0,
            audio: Audio::load(),
            scene: Scene::new(),
        }
    }

    pub fn reset(&mut self, ship: &mut Ship) {
        *self = Self::new();
        self.place_ship_at_start(ship);
    }

    pub fn place_ship_at_start(&mut self, ship: &mut Ship) {
        ship.position = Vec3::new(0.0, 12.0, 42.0);
        ship.yaw = 0.0;
        ship.pitch = 0.0;
        ship.roll = 0.0;
        ship.roll_target = 0.0;
        ship.yaw_rate = 0.0;
        ship.pitch_rate = 0.0;
        ship.steering_bank = 0.0;
        ship.steering_pitch = 0.0;
        ship.planet_locked = false;
        ship.spiky_planet_locked = false;
        ship.race_planet_locked = false;
        ship.orbit_center = None;
        ship.orbit_plane_normal = None;
        ship.orbit_camera_blend = 0.0;
        ship.orbit_release_active = false;
        ship.speed = 0.0;
        self.safe_position = ship.position;
    }

    pub fn update(&mut self, ship: &mut Ship, input: Input, delta_time: f32) -> Action {
        if input.exit {
            return Action::Exit;
        }
        if matches!(self.phase, Phase::Won | Phase::Lost) {
            if input.restart {
                self.reset(ship);
            }
            return Action::Continue;
        }
        if input.pause {
            if self.phase == Phase::Paused {
                self.phase = self.phase_before_pause;
            } else {
                self.phase_before_pause = self.phase;
                self.phase = Phase::Paused;
            }
            return Action::Continue;
        }
        if self.phase == Phase::Paused {
            if input.restart {
                self.reset(ship);
            }
            return Action::Continue;
        }

        let dt = delta_time.clamp(0.0, 0.05);
        self.visual_time += dt;
        self.feedback_timer = (self.feedback_timer - dt).max(0.0);
        self.hit_cooldown = (self.hit_cooldown - dt).max(0.0);

        match self.phase {
            Phase::Intro => {
                self.phase_timer = (self.phase_timer - dt).max(0.0);
                if self.phase_timer == 0.0 {
                    self.phase = Phase::Playing;
                }
                return Action::Continue;
            }
            Phase::Inverting => {
                self.phase_timer = (self.phase_timer - dt).max(0.0);
                if self.phase_timer == 0.0 {
                    self.inversion_complete = true;
                    self.phase = Phase::Playing;
                    self.feedback = "GRAVEDAD INVERTIDA - SANTUARIO SUPERIOR ABIERTO";
                    self.feedback_timer = 3.2;
                }
                self.scene.update_sand(self.sand_offset);
                self.update_capsule_visibility();
                return Action::Continue;
            }
            Phase::Playing | Phase::Warning => {}
            Phase::Paused | Phase::Won | Phase::Lost => return Action::Continue,
        }

        self.elapsed += dt;
        self.sand_offset = if self.elapsed < INVERSION_AT {
            (self.elapsed * (SAND_RISE / INVERSION_AT)).min(SAND_RISE)
        } else {
            (SAND_RISE - (self.elapsed - INVERSION_AT) * SAND_FALL_RATE).max(-1.8)
        };
        if self.elapsed >= MISSION_SECONDS {
            self.phase = Phase::Lost;
            self.feedback = "EL RELOJ SE HA VACIADO";
            self.feedback_timer = 0.0;
            return Action::Continue;
        }

        let until_inversion = INVERSION_AT - self.elapsed;
        if !self.inversion_complete && until_inversion <= INVERSION_WARNING {
            self.phase = Phase::Warning;
        }
        if !self.inversion_complete && until_inversion <= 0.0 {
            self.phase = Phase::Inverting;
            self.phase_timer = INVERSION_DURATION;
            self.rescue_progress = 0.0;
            self.rescue_target = None;
            self.feedback = "INVERSION DE GRAVEDAD";
            self.feedback_timer = INVERSION_DURATION;
            self.audio.play(Cue::Inversion);
            self.scene.update_sand(self.sand_offset);
            self.update_capsule_visibility();
            return Action::Continue;
        }

        if input.recover {
            ship.position = self.safe_position;
            self.speed = 0.0;
            self.vertical_speed = 0.0;
            self.rescue_progress = 0.0;
            self.rescue_target = None;
            self.feedback = "NAVE RECUPERADA EN POSICION SEGURA";
            self.feedback_timer = 2.0;
        }
        self.move_ship(ship, input, dt);
        self.resolve_collisions(ship, dt);
        self.update_rescue(ship, input, dt);
        if self.shields == 0 {
            self.phase = Phase::Lost;
            self.feedback = "ESCUDOS AGOTADOS";
            self.feedback_timer = 0.0;
        } else if self.rescued as usize == CAPSULE_COUNT
            && ship
                .position
                .sub(EXIT_POSITION)
                .dot(ship.position.sub(EXIT_POSITION))
                < 14.0 * 14.0
        {
            self.phase = Phase::Won;
            self.feedback = "RESCATE COMPLETADO";
            self.feedback_timer = 0.0;
        }
        self.scene.update_sand(self.sand_offset);
        self.update_capsule_visibility();
        Action::Continue
    }

    pub fn draw_hud(&self, frame: &mut Framebuffer, ship: &Ship) {
        hud::draw(self, frame, ship);
    }

    pub fn rescued_count(&self) -> u8 {
        self.rescued
    }

    fn remaining(&self) -> f32 {
        (MISSION_SECONDS - self.elapsed).max(0.0)
    }

    fn inversion_countdown(&self) -> f32 {
        (INVERSION_AT - self.elapsed).max(0.0)
    }

    fn target_distance_for_hud(&self, ship: &Ship) -> Option<f32> {
        self.target_distance(ship)
    }

    fn rescue_progress_ratio(&self) -> f32 {
        (self.rescue_progress / RESCUE_SECONDS).clamp(0.0, 1.0)
    }
}
