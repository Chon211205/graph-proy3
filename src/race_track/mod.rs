mod drive;
mod hud;
mod progress;

use std::f32::consts::TAU;

use crate::{
    Camera, TriangleBatch,
    framebuffer::Framebuffer,
    obj_loader::{Model, Vec3, load_obj},
    queue_model, render_ship_components,
    ship::ShipController,
};

use self::{
    drive::DriveState,
    progress::{ProgressEvent, RaceProgress},
};

pub(crate) use self::drive::RaceInput;

pub(crate) const PLANET_POSITION: Vec3 = Vec3 {
    x: -180.0,
    y: 10.0,
    z: -100.0,
};
pub(crate) const PLANET_RADIUS: f32 = 28.0;
const TILE_SIZE: f32 = 6.0;
const COLLISION_RADIUS: f32 = 1.4;
const MAP: &str = include_str!("../../assets/levels/kart_racer/jardin_rust.tiles");

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum RacePhase {
    Countdown,
    Driving,
    Paused,
    Finished,
}

pub(crate) struct RaceTrack {
    pub planet: Model,
    map: Model,
    tiles: Vec<Vec<u8>>,
    drive: DriveState,
    progress: RaceProgress,
    phase: RacePhase,
    countdown: f32,
    safe_position: Vec3,
    safe_yaw: f32,
}

impl RaceTrack {
    pub fn load() -> Result<Self, String> {
        Ok(Self {
            planet: load_obj("assets/planets/planeta_carreras_detallado.obj")?,
            map: load_obj("assets/levels/kart_racer/jardin_rust.obj")?,
            tiles: MAP.lines().map(|line| line.as_bytes().to_vec()).collect(),
            drive: DriveState::new(),
            progress: RaceProgress::new(),
            phase: RacePhase::Countdown,
            countdown: 0.0,
            safe_position: Vec3::new(15.0 * TILE_SIZE, 2.0, 34.0 * TILE_SIZE),
            safe_yaw: std::f32::consts::PI,
        })
    }

    pub fn enter(&mut self, controller: &mut ShipController) {
        controller.reset();
        controller.ship.position = Vec3::new(15.0 * TILE_SIZE, 2.0, 34.0 * TILE_SIZE);
        controller.ship.yaw = std::f32::consts::PI;
        controller.ship.pitch = 0.0;
        controller.ship.planet_locked = false;
        controller.ship.race_planet_locked = false;
        controller.ship.orbit_center = None;
        controller.ship.orbit_plane_normal = None;
        controller.ship.orbit_camera_blend = 0.0;
        self.drive = DriveState::new();
        self.progress = RaceProgress::new();
        self.phase = RacePhase::Countdown;
        self.countdown = 3.25;
        self.safe_position = controller.ship.position;
        self.safe_yaw = controller.ship.yaw;
    }

    pub fn leave(&self, controller: &mut ShipController) {
        controller.reset();
        let radial = Vec3::new(1.0, 0.24, -0.12).normalize();
        let ship = &mut controller.ship;
        ship.position = PLANET_POSITION.add(radial.mul(PLANET_RADIUS + 72.0));
        ship.yaw = (-radial.z).atan2(-radial.x).rem_euclid(TAU);
        ship.pitch = radial.y.asin();
        ship.planet_locked = false;
        ship.race_planet_locked = false;
        ship.spiky_planet_locked = false;
        ship.orbit_center = None;
        ship.orbit_plane_normal = None;
        ship.orbit_camera_blend = 0.0;
        ship.orbit_release_active = false;
    }

    pub fn recover(&mut self, controller: &mut ShipController) {
        controller.projectiles.clear();
        controller.ship.position = self.safe_position;
        controller.ship.yaw = self.safe_yaw;
        controller.ship.pitch = 0.0;
        controller.ship.steering_bank = 0.0;
        self.drive.recover();
        controller.ship.speed = 0.0;
    }

    pub fn pause(&mut self) {
        if matches!(self.phase, RacePhase::Driving | RacePhase::Countdown) {
            self.phase = RacePhase::Paused;
        }
    }

    pub fn update(&mut self, controller: &mut ShipController, input: RaceInput, dt: f32) {
        match self.phase {
            RacePhase::Countdown => {
                self.countdown = (self.countdown - dt).max(0.0);
                if self.countdown <= 0.0 {
                    self.phase = RacePhase::Driving;
                }
                return;
            }
            RacePhase::Paused => {
                if input.pause || input.confirm {
                    self.phase = RacePhase::Driving;
                }
                return;
            }
            RacePhase::Finished => {
                if input.confirm {
                    self.enter(controller);
                }
                return;
            }
            RacePhase::Driving => {
                if input.pause {
                    self.phase = RacePhase::Paused;
                    return;
                }
            }
        }

        let previous = controller.ship.position;
        self.drive
            .update(&mut controller.ship, &self.tiles, input, dt);
        if let Some(event) = self.progress.update(previous, controller.ship.position, dt) {
            if matches!(
                event,
                ProgressEvent::Checkpoint
                    | ProgressEvent::Lap
                    | ProgressEvent::FinalLap
                    | ProgressEvent::Finished
            ) {
                self.safe_position = controller.ship.position;
                self.safe_yaw = controller.ship.yaw;
            }
            if event == ProgressEvent::Finished {
                self.phase = RacePhase::Finished;
            }
        }
    }

    pub fn render(
        &self,
        frame: &mut Framebuffer,
        batch: &mut TriangleBatch,
        vehicle: &Model,
        controller: &ShipController,
        plume: &Model,
        laser: &Model,
        time: f32,
    ) {
        draw_race_sky(frame, time);
        let camera = chase_camera_for(&controller.ship);
        queue_model(&self.map, batch, &camera, |v| v, None, None);
        let speed_power = (self.drive.speed.abs() / self.drive.max_speed()).clamp(0.0, 1.0);
        let turbo = (speed_power * 0.38 + self.drive.boost_flash * 0.92).clamp(0.0, 1.0);
        render_ship_components(
            vehicle,
            crate::model_center(vehicle),
            plume,
            laser,
            &controller.projectiles,
            batch,
            &camera,
            &controller.ship,
            time,
            turbo,
            0.0,
            true,
        );
        batch.flush(frame);
    }

    pub fn hud(&self, frame: &mut Framebuffer, controller: &ShipController) {
        hud::draw(
            frame,
            &self.tiles,
            &self.progress,
            &self.drive,
            controller.ship.position,
            controller.ship.yaw,
            self.phase,
            self.countdown,
        );
    }
}

fn chase_camera_for(ship: &crate::ship::Ship) -> Camera {
    let (forward, _, _) = ship.camera_basis();
    let position = ship
        .position
        .sub(forward.mul(28.0))
        .add(Vec3::new(0.0, 18.0, 0.0));
    let target = ship.position.add(forward.mul(22.0));
    let camera_forward = target.sub(position).normalize();
    let mut right = camera_forward.cross(Vec3::new(0.0, 1.0, 0.0)).normalize();
    if right.dot(right) < 0.001 {
        right = Vec3::new(1.0, 0.0, 0.0);
    }
    Camera {
        position,
        forward: camera_forward,
        right,
        up: right.cross(camera_forward).normalize(),
    }
}

fn draw_race_sky(frame: &mut Framebuffer, time: f32) {
    // The shared scene framebuffer retains depth between frames and modes.
    // Clear both color and depth before drawing the procedural sky gradient.
    frame.clear(0x112349);
    for y in 0..frame.height {
        let t = y as f32 / frame.height as f32;
        let horizon = (1.0 - (t - 0.48).abs() * 1.8).clamp(0.0, 1.0);
        let red = (17.0 + horizon * 66.0) as u32;
        let green = (35.0 + horizon * 87.0) as u32;
        let blue = (73.0 + horizon * 88.0) as u32;
        let color = (red << 16) | (green << 8) | blue;
        let row = y * frame.width;
        frame.pixels[row..row + frame.width].fill(color);
    }
    for index in 0..90_u32 {
        let mut seed = index.wrapping_mul(0x9e37_79b9).wrapping_add(0x85eb_ca6b);
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        let drift = (time * (2.0 + (index % 5) as f32 * 0.8)) as usize;
        let x = (seed as usize % frame.width + drift) % frame.width;
        let y = (seed.rotate_left(11) as usize % (frame.height / 2).max(1)).min(frame.height - 1);
        frame.pixels[y * frame.width + x] = 0xd9edff;
    }
}
