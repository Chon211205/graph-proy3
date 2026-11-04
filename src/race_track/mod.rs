mod course;
mod drive;
mod hud;
mod progress;
mod racer;
mod scenery;

use std::f32::consts::TAU;

use crate::{
    Camera, TriangleBatch, draw_projectile_glow,
    framebuffer::Framebuffer,
    obj_loader::{Model, Vec3, load_obj},
    queue_model, render_ship_components,
    ship::{ShipController, ShipSpec},
};

use self::{
    course::RaceCourse,
    drive::DriveState,
    progress::{ProgressEvent, RaceProgress},
    racer::{Rival, yaw_from_direction},
    scenery::RaceScenery,
};

pub(crate) use self::drive::RaceInput;

pub(crate) const PLANET_POSITION: Vec3 = Vec3 {
    x: -180.0,
    y: 10.0,
    z: -100.0,
};
pub(crate) const PLANET_RADIUS: f32 = 28.0;
const RIVAL_COUNT: usize = 12;
pub(crate) type RaceVehicles<'a> = [Option<(&'a Model, &'static ShipSpec)>; 6];

pub(crate) struct RaceSky<'a> {
    pub water_planet: &'a Model,
    pub water_center: Vec3,
    pub water_scale: f32,
    pub pink_planet: &'a Model,
    pub pink_center: Vec3,
    pub pink_scale: f32,
    pub sun: &'a Model,
    pub sun_center: Vec3,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum RacePhase {
    Countdown,
    Driving,
    Paused,
    Finished,
}

pub(crate) struct RaceTrack {
    pub planet: Model,
    course: RaceCourse,
    scenery: RaceScenery,
    drive: DriveState,
    progress: RaceProgress,
    rivals: Option<Vec<Rival>>,
    selected_vehicle: usize,
    finish_position: usize,
    camera: Option<Camera>,
    last_render_time: f32,
    phase: RacePhase,
    countdown: f32,
    safe_position: Vec3,
    safe_yaw: f32,
}

impl RaceTrack {
    pub fn load() -> Result<Self, String> {
        let course = RaceCourse::new();
        let scenery = RaceScenery::new(&course);
        let progress = RaceProgress::new(&course);
        Ok(Self {
            planet: load_obj("assets/planets/planeta_carreras_detallado.obj")?,
            course,
            scenery,
            drive: DriveState::new(),
            progress,
            rivals: None,
            selected_vehicle: 2,
            finish_position: 1,
            camera: None,
            last_render_time: 0.0,
            phase: RacePhase::Countdown,
            countdown: 0.0,
            safe_position: Vec3::new(72.0, 1.2, 207.0),
            safe_yaw: std::f32::consts::PI,
        })
    }

    pub fn enter(
        &mut self,
        controller: &mut ShipController,
        selected_vehicle: usize,
        vehicles: &RaceVehicles<'_>,
    ) -> bool {
        let Some((_, _)) = vehicles.get(selected_vehicle).copied().flatten() else {
            return false;
        };
        let rival_vehicle_indices = vehicles
            .iter()
            .enumerate()
            .filter_map(|(index, vehicle)| vehicle.map(|_| index))
            .collect::<Vec<_>>();
        if rival_vehicle_indices.is_empty() {
            return false;
        }
        controller.reset();
        let start = self.course.sample_at(0.0);
        controller.ship.position = start.position.add(Vec3::new(0.0, 1.2, 0.0));
        controller.ship.yaw = yaw_from_direction(start.tangent);
        controller.ship.pitch = 0.0;
        controller.ship.planet_locked = false;
        controller.ship.race_planet_locked = false;
        controller.ship.orbit_center = None;
        controller.ship.orbit_plane_normal = None;
        controller.ship.orbit_camera_blend = 0.0;
        self.drive = DriveState::new();
        self.progress = RaceProgress::new(&self.course);
        self.selected_vehicle = selected_vehicle;
        self.finish_position = 1;
        self.camera = None;
        self.last_render_time = 0.0;
        self.rivals = Some((0..RIVAL_COUNT).map(|rival_index| {
            // Twelve rivals means every one of the six available cars appears
            // exactly twice, including the model selected by the player.
            let vehicle_index = rival_vehicle_indices[rival_index % rival_vehicle_indices.len()];
            let (_, spec) = vehicles[vehicle_index].expect("validated rival vehicle");
            let row = rival_index / 3;
            let grid = 16.0 + row as f32 * 13.0;
            let lane = [-4.6, 0.0, 4.6][rival_index % 3];
            let skill = [
                0.99, 0.94, 0.97, 0.91, 1.00, 0.95,
                0.98, 0.92, 0.96, 0.90, 0.97, 0.93,
            ][rival_index];
            Rival::new(&self.course, vehicle_index, spec, grid, lane, skill)
        }).collect());
        self.phase = RacePhase::Countdown;
        self.countdown = 3.25;
        self.safe_position = controller.ship.position;
        self.safe_yaw = controller.ship.yaw;
        true
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

    pub fn resume(&mut self) {
        if self.phase == RacePhase::Paused {
            self.phase = RacePhase::Driving;
        }
    }

    pub fn is_finished(&self) -> bool {
        self.phase == RacePhase::Finished
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
                    // The caller re-enters with the selected car table so all
                    // race participants are reset together.
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
            .update(&mut controller.ship, &self.course, input, dt);
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
                self.finish_position = self.player_position(controller);
                self.phase = RacePhase::Finished;
            }
        }
        if let Some(rivals) = &mut self.rivals {
            for rival in rivals.iter_mut() {
                rival.update(&self.course, dt);
            }
            resolve_racer_contacts(controller, &mut self.drive, rivals);
        }
        if self.progress.is_finished() {
            self.finish_position = self.player_position(controller);
            self.phase = RacePhase::Finished;
        }
    }

    pub fn render(
        &mut self,
        frame: &mut Framebuffer,
        batch: &mut TriangleBatch,
        vehicles: &RaceVehicles<'_>,
        sky: &RaceSky<'_>,
        controller: &ShipController,
        plume: &Model,
        laser: &Model,
        time: f32,
    ) {
        draw_race_sky(frame, time);
        let target_camera = chase_camera_for(&controller.ship);
        let camera_dt = (time - self.last_render_time).clamp(0.001, 0.05);
        self.last_render_time = time;
        let blend = 1.0 - (-5.5 * camera_dt).exp();
        let camera = if let Some(previous) = self.camera {
            let forward = previous
                .forward
                .mul(1.0 - blend)
                .add(target_camera.forward.mul(blend))
                .normalize();
            let position = previous
                .position
                .mul(1.0 - blend)
                .add(target_camera.position.mul(blend));
            let right = forward.cross(Vec3::new(0.0, 1.0, 0.0)).normalize();
            Camera {
                position,
                forward,
                right,
                up: right.cross(forward).normalize(),
            }
        } else {
            target_camera
        };
        self.camera = Some(camera);
        // These celestial bodies are placed in race-world coordinates, so
        // they remain fixed while the chase camera and car move around the loop.
        queue_model(
            sky.sun,
            batch,
            &camera,
            |v| v.sub(sky.sun_center).mul(0.45).add(Vec3::new(-280.0, 170.0, -130.0)),
            Some(time),
            None,
        );
        queue_sky_planet(
            sky.water_planet,
            sky.water_center,
            sky.water_scale * 1.2,
            Vec3::new(600.0, 150.0, 140.0),
            time * 0.08,
            batch,
            &camera,
            Some(time),
        );
        queue_sky_planet(
            sky.pink_planet,
            sky.pink_center,
            sky.pink_scale * 1.3,
            Vec3::new(100.0, 180.0, 520.0),
            time * 0.05,
            batch,
            &camera,
            None,
        );
        queue_model(&self.scenery.terrain, batch, &camera, |v| v, None, None);
        queue_model(&self.scenery.hills, batch, &camera, |v| v, None, None);
        for forest in &self.scenery.forests {
            queue_model(forest, batch, &camera, |v| v, None, None);
        }
        for arch in &self.scenery.arches {
            queue_model(arch, batch, &camera, |v| v, None, None);
        }
        queue_model(&self.course.barriers, batch, &camera, |v| v, None, None);
        queue_model(&self.course.curbs, batch, &camera, |v| v, None, None);
        queue_model(&self.course.road, batch, &camera, |v| v, None, None);
        queue_model(&self.course.markings, batch, &camera, |v| v, None, None);
        let speed_power = (self.drive.speed.abs() / self.drive.max_speed()).clamp(0.0, 1.0);
        let turbo = (speed_power * 0.18 + self.drive.boost_flash * 0.68).clamp(0.0, 1.0);
        let (vehicle, _) =
            vehicles[self.selected_vehicle].expect("player car was checked on entry");
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
            1.55,
        );
        if let Some(rivals) = &self.rivals {
            for rival in rivals {
                let Some((model, _)) = vehicles[rival.vehicle_index] else {
                    continue;
                };
                let speed_power =
                    (rival.drive.speed.abs() / rival.drive.max_speed()).clamp(0.0, 1.0);
                render_ship_components(
                    model,
                    crate::model_center(model),
                    plume,
                    laser,
                    &[],
                    batch,
                    &camera,
                    &rival.controller.ship,
                    time,
                    (speed_power * 0.12 + rival.drive.boost_flash * 0.5).clamp(0.0, 1.0),
                    0.0,
                    true,
                    1.55,
                );
            }
        }
        batch.flush(frame);
        apply_distant_mist(frame);
        draw_projectile_glow(
            frame,
            &controller.projectiles,
            &camera,
            time,
            controller.ship.spec.laser_color,
            1.65,
        );
    }

    pub fn hud(&self, frame: &mut Framebuffer, controller: &ShipController) {
        let place = if self.progress.is_finished() {
            self.finish_position
        } else {
            self.player_position(controller)
        };
        hud::draw(
            frame,
            &self.course,
            &self.progress,
            &self.drive,
            controller.ship.position,
            controller.ship.yaw,
            self.rivals.as_deref(),
            place,
            self.phase,
            self.countdown,
        );
    }

    pub fn player_position(&self, controller: &ShipController) -> usize {
        let (distance, _, _) = self.course.nearest(controller.ship.position);
        let player = Standing {
            progress: self.progress.progress_score(distance, self.course.length),
            time: self.progress.race_time(),
            finished: self.progress.is_finished(),
        };
        let rival_standings = self
            .rivals
            .iter()
            .flatten()
            .map(|rival| {
                let (distance, _, _) = self.course.nearest(rival.controller.ship.position);
                Standing {
                    progress: rival.progress.progress_score(distance, self.course.length),
                    time: rival.progress.race_time(),
                    finished: rival.progress.is_finished(),
                }
            })
            .collect::<Vec<_>>();
        place_for_player(player, &rival_standings)
    }
}

#[derive(Clone, Copy)]
struct Standing {
    progress: f32,
    time: f32,
    finished: bool,
}

fn is_ahead(left: Standing, right: Standing) -> bool {
    match (left.finished, right.finished) {
        (true, false) => true,
        (false, true) => false,
        (true, true) => left.time < right.time,
        (false, false) => left.progress > right.progress,
    }
}

fn place_for_player(player: Standing, rivals: &[Standing]) -> usize {
    1 + rivals
        .iter()
        .filter(|&&rival| is_ahead(rival, player))
        .count()
}

fn queue_sky_planet(
    model: &Model,
    center: Vec3,
    scale: f32,
    position: Vec3,
    rotation: f32,
    batch: &mut TriangleBatch,
    camera: &Camera,
    water_time: Option<f32>,
) {
    let (sin, cos) = rotation.sin_cos();
    queue_model(
        model,
        batch,
        camera,
        |vertex| {
            let local = vertex.sub(center).mul(scale);
            Vec3::new(
                local.x * cos - local.z * sin,
                local.y,
                local.x * sin + local.z * cos,
            )
            .add(position)
        },
        water_time,
        None,
    );
}

fn resolve_racer_contacts(
    controller: &mut ShipController,
    player_drive: &mut DriveState,
    rivals: &mut [Rival],
) {
    for rival in rivals.iter_mut() {
        let offset = Vec3::new(
            rival.controller.ship.position.x - controller.ship.position.x,
            0.0,
            rival.controller.ship.position.z - controller.ship.position.z,
        );
        let distance_sq = offset.dot(offset);
        if distance_sq < 16.0 && distance_sq > 0.001 {
            let separation = offset.normalize().mul(1.0);
            rival.controller.ship.position = rival.controller.ship.position.add(separation);
            controller.ship.position = controller.ship.position.sub(separation);
            rival.drive.speed *= 0.72;
            rival.drive.collision_flash = 0.8;
            player_drive.speed *= 0.72;
            player_drive.collision_flash = 0.8;
        }
    }
    for first in 0..rivals.len() {
        for second in first + 1..rivals.len() {
            let offset = Vec3::new(
                rivals[second].controller.ship.position.x
                    - rivals[first].controller.ship.position.x,
                0.0,
                rivals[second].controller.ship.position.z
                    - rivals[first].controller.ship.position.z,
            );
            let distance_sq = offset.dot(offset);
            if distance_sq < 16.0 && distance_sq > 0.001 {
                let separation = offset.normalize().mul(0.8);
                rivals[first].controller.ship.position =
                    rivals[first].controller.ship.position.sub(separation);
                rivals[second].controller.ship.position =
                    rivals[second].controller.ship.position.add(separation);
                rivals[first].drive.speed *= 0.88;
                rivals[second].drive.speed *= 0.88;
                rivals[first].drive.collision_flash = 0.6;
                rivals[second].drive.collision_flash = 0.6;
            }
        }
    }
    controller.ship.speed = player_drive.speed.abs();
}

fn chase_camera_for(ship: &crate::ship::Ship) -> Camera {
    let (forward, _, _) = ship.camera_basis();
    let position = ship
        .position
        .sub(forward.mul(12.0))
        .add(Vec3::new(0.0, 7.0, 0.0));
    let target = ship.position.add(forward.mul(30.0));
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

fn apply_distant_mist(frame: &mut Framebuffer) {
    let width = frame.width;
    let height = frame.height;
    let start_y = (height as f32 * 0.36) as usize;
    let end_y = (height as f32 * 0.78) as usize;
    let (pixels, depth) = frame.buffers_mut();
    for y in start_y..end_y.min(height) {
        let normalized_y = y as f32 / height as f32;
        let vertical_falloff = (1.0 - ((normalized_y - 0.56) / 0.22).abs()).clamp(0.0, 1.0);
        let row = y * width;
        for x in 0..width {
            let index = row + x;
            let inverse_distance = depth[index];
            if !inverse_distance.is_finite() || inverse_distance <= 0.0 {
                continue;
            }
            let distance = 1.0 / inverse_distance;
            let distance_fog = ((distance - 72.0) / 205.0).clamp(0.0, 1.0);
            let amount = distance_fog * vertical_falloff * 0.28;
            if amount < 0.01 {
                continue;
            }
            let pixel = pixels[index];
            let mix = |channel: u32, haze: u32| {
                (channel as f32 * (1.0 - amount) + haze as f32 * amount) as u32
            };
            let red = mix((pixel >> 16) & 0xff, 159);
            let green = mix((pixel >> 8) & 0xff, 184);
            let blue = mix(pixel & 0xff, 151);
            pixels[index] = (red << 16) | (green << 8) | blue;
        }
    }
}

#[cfg(test)]
mod standings_tests {
    use super::*;
    use crate::ship::{
        ARWING_CAR_SPEC, ASTRA_CAR_SPEC, BUZZ_CAR_SPEC, COWBOYB_CAR_SPEC,
        KIRBY_CAR_SPEC, MCQUEEN_CAR_SPEC,
    };

    #[test]
    fn race_grid_has_twelve_rivals_and_uses_every_available_car_twice() {
        let dummy = Model::new(Vec::new(), Vec::new());
        let vehicles: RaceVehicles<'_> = [
            Some((&dummy, &ARWING_CAR_SPEC)),
            Some((&dummy, &KIRBY_CAR_SPEC)),
            Some((&dummy, &ASTRA_CAR_SPEC)),
            Some((&dummy, &COWBOYB_CAR_SPEC)),
            Some((&dummy, &BUZZ_CAR_SPEC)),
            Some((&dummy, &MCQUEEN_CAR_SPEC)),
        ];
        let mut track = RaceTrack::load().unwrap();
        let mut controller = ShipController::new(&ARWING_CAR_SPEC);
        assert!(track.enter(&mut controller, 0, &vehicles));

        let rivals = track.rivals.as_ref().unwrap();
        assert_eq!(rivals.len(), RIVAL_COUNT);
        for vehicle_index in 0..vehicles.len() {
            assert_eq!(
                rivals
                    .iter()
                    .filter(|rival| rival.vehicle_index == vehicle_index)
                    .count(),
                2
            );
        }
    }

    #[test]
    fn standings_use_laps_and_checkpoint_progress_then_finish_time() {
        let player = Standing {
            progress: 125.0,
            time: 48.0,
            finished: false,
        };
        let rivals = [
            Standing {
                progress: 126.0,
                time: 48.0,
                finished: false,
            },
            Standing {
                progress: 100.0,
                time: 49.0,
                finished: false,
            },
            Standing {
                progress: 0.0,
                time: 47.0,
                finished: true,
            },
        ];
        assert_eq!(place_for_player(player, &rivals), 3);
    }

    #[test]
    fn finished_racers_rank_ahead_and_are_ordered_by_frozen_finish_time() {
        let player = Standing {
            progress: 900.0,
            time: 122.0,
            finished: true,
        };
        let rivals = [
            Standing {
                progress: 0.0,
                time: 121.0,
                finished: true,
            },
            Standing {
                progress: 990.0,
                time: 130.0,
                finished: false,
            },
            Standing {
                progress: 0.0,
                time: 0.0,
                finished: false,
            },
        ];
        assert_eq!(place_for_player(player, &rivals), 2);
    }
}
