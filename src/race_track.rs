//! Jardín Rust geometry with the project's existing vehicle controller/rendering.
use crate::{
    chase_camera, draw_selection_label,
    framebuffer::Framebuffer,
    obj_loader::{load_obj, Model, Vec3},
    queue_model, render_ship_components,
    ship::{ShipController, ShipInput},
    TriangleBatch,
};

pub(crate) const PLANET_POSITION: Vec3 = Vec3 {
    x: -180.0,
    y: 10.0,
    z: -100.0,
};
pub(crate) const PLANET_RADIUS: f32 = 28.0;
const TILE_SIZE: f32 = 6.0;
const COLLISION_RADIUS: f32 = 1.4;
const MAP: &str = include_str!("../assets/levels/kart_racer/jardin_rust.tiles");

pub(crate) struct RaceTrack {
    pub planet: Model,
    map: Model,
    tiles: Vec<Vec<u8>>,
}

impl RaceTrack {
    pub fn load() -> Result<Self, String> {
        Ok(Self {
            planet: load_obj("assets/planets/planeta_carreras.obj")?,
            map: load_obj("assets/levels/kart_racer/jardin_rust.obj")?,
            tiles: MAP.lines().map(|line| line.as_bytes().to_vec()).collect(),
        })
    }

    pub fn enter(&self, controller: &mut ShipController) {
        controller.reset();
        // Same spawn and heading as kart_racer's default track (+X).
        controller.ship.position = Vec3::new(15.0 * TILE_SIZE, 2.0, 34.0 * TILE_SIZE);
        controller.ship.yaw = std::f32::consts::PI;
    }

    pub fn leave(&self, controller: &mut ShipController) {
        controller.reset();
        controller.ship.position = PLANET_POSITION.add(Vec3::new(48.0, 8.0, 0.0));
        controller.ship.planet_locked = true;
        controller.ship.race_planet_locked = true;
        controller.ship.orbit_center = Some(PLANET_POSITION);
    }

    fn blocked(&self, x: f32, z: f32) -> bool {
        if !x.is_finite() || !z.is_finite() {
            return true;
        }
        let min_x = ((x - COLLISION_RADIUS) / TILE_SIZE).floor() as isize;
        let max_x = ((x + COLLISION_RADIUS) / TILE_SIZE).floor() as isize;
        let min_z = ((z - COLLISION_RADIUS) / TILE_SIZE).floor() as isize;
        let max_z = ((z + COLLISION_RADIUS) / TILE_SIZE).floor() as isize;
        for row in min_z..=max_z {
            for col in min_x..=max_x {
                let tile = if row < 0 || col < 0 {
                    None
                } else {
                    self.tiles
                        .get(row as usize)
                        .and_then(|r| r.get(col as usize))
                };
                if tile.is_none_or(|t| matches!(t, b'#' | b'H' | b'S' | b'W' | b'D')) {
                    let near_x = x.clamp(col as f32 * TILE_SIZE, (col + 1) as f32 * TILE_SIZE);
                    let near_z = z.clamp(row as f32 * TILE_SIZE, (row + 1) as f32 * TILE_SIZE);
                    if (x - near_x).powi(2) + (z - near_z).powi(2) < COLLISION_RADIUS.powi(2) {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn update(&self, controller: &mut ShipController, input: &ShipInput, dt: f32) {
        controller.update(input, dt);
        let ship = &mut controller.ship;
        // Use the existing acceleration/steering; keep the vehicle on the circuit.
        ship.pitch = 0.0;
        let (forward, _, _) = ship.movement_basis();
        let movement = forward.mul(ship.speed * dt);
        let steps = (movement.dot(movement).sqrt() / 0.5).ceil().max(1.0) as usize;
        let step = movement.mul(1.0 / steps as f32);
        for _ in 0..steps {
            if !self.blocked(ship.position.x + step.x, ship.position.z) {
                ship.position.x += step.x;
            }
            if !self.blocked(ship.position.x, ship.position.z + step.z) {
                ship.position.z += step.z;
            }
        }
        ship.position.y = 2.0;
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
        frame.clear(0x78beeb);
        let camera = chase_camera(&controller.ship);
        queue_model(&self.map, batch, &camera, |v| v, None, None);
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
            0.0,
            controller.charge_ratio(),
            false,
        );
        batch.flush(frame);
    }

    pub fn hud(&self, frame: &mut Framebuffer, position: Vec3) {
        draw_selection_label(
            frame,
            "JARDIN RUST - PLANETA DE CARRERAS",
            600,
            52,
            2,
            0xffe778,
        );
        draw_selection_label(
            frame,
            "A/D: GIRAR  ESPACIO: ACELERAR  CTRL: FRENAR  R: REINICIAR  BACKSPACE: VOLVER",
            600,
            730,
            1,
            0xffffff,
        );
        // Small map of the original tile layout, to make the full circuit navigable.
        for (row, tiles) in self.tiles.iter().enumerate() {
            for (col, tile) in tiles.iter().enumerate() {
                let color = match tile {
                    b'P' | b'K' | b'L' | b'R' => 0x737a86,
                    b'M' | b'N' => 0xf8e660,
                    b'#' | b'H' => 0x153823,
                    _ => 0x326e42,
                };
                for dy in 0..3 {
                    for dx in 0..3 {
                        frame.pixels[(70 + row * 3 + dy) * frame.width + 975 + col * 3 + dx] =
                            color;
                    }
                }
            }
        }
        let x = 975 + (position.x / TILE_SIZE * 3.0) as usize;
        let y = 70 + (position.z / TILE_SIZE * 3.0) as usize;
        for dy in 0..4 {
            for dx in 0..4 {
                if x + dx < frame.width && y + dy < frame.height {
                    frame.pixels[(y + dy) * frame.width + x + dx] = 0xff4545;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ship::MCQUEEN_CAR_SPEC;

    #[test]
    fn imported_map_and_planet_render_with_their_native_materials() {
        let track = RaceTrack::load().unwrap();
        for model in [&track.map, &track.planet] {
            assert!(model
                .vertices
                .iter()
                .all(|v| v.x.is_finite() && v.y.is_finite() && v.z.is_finite()));
            assert!(model
                .faces
                .iter()
                .all(|f| f.indices.iter().all(|&i| i < model.vertices.len())));
        }
        assert!(track
            .map
            .faces
            .iter()
            .any(|f| f.color == crate::MaterialColor::Fixed([70, 72, 76])));
        let camera = crate::Camera {
            position: Vec3::new(195.0, 340.0, 123.0),
            forward: Vec3::new(0.0, -1.0, 0.0),
            right: Vec3::new(1.0, 0.0, 0.0),
            up: Vec3::new(0.0, 0.0, -1.0),
        };
        let mut frame = Framebuffer::new(crate::WIDTH, crate::HEIGHT);
        frame.clear(0x101a2c);
        crate::render_model(&track.map, &mut frame, &camera, |v| v, None, None);
        assert!(frame.pixels.iter().filter(|&&p| p != 0x101a2c).count() > 100_000);
        if let Ok(path) = std::env::var("RACE_MAP_PREVIEW") {
            frame.save_png(&path).unwrap();
        }
    }

    #[test]
    fn track_uses_existing_vehicle_renderer_and_returns_to_home_orbit() {
        let track = RaceTrack::load().unwrap();
        let vehicle = load_obj("assets/vehicles/mcqueen/mcqueen_carro_detallado.obj").unwrap();
        let mut controller = ShipController::new(&MCQUEEN_CAR_SPEC);
        track.enter(&mut controller);
        let mut frame = Framebuffer::new(crate::WIDTH, crate::HEIGHT);
        let mut batch = TriangleBatch::new(crate::WIDTH, crate::HEIGHT);
        track.render(
            &mut frame,
            &mut batch,
            &vehicle,
            &controller,
            &crate::make_turbo_plume(12),
            &crate::make_laser_bolt(),
            0.0,
        );
        track.hud(&mut frame, controller.ship.position);
        assert!(frame.pixels.iter().filter(|&&p| p != 0x78beeb).count() > 100_000);
        assert!(batch.commands.is_empty());
        if let Ok(path) = std::env::var("RACE_SCENE_PREVIEW") {
            frame.save_png(&path).unwrap();
        }
        track.leave(&mut controller);
        assert!(controller.ship.planet_locked);
        assert!(!controller.ship.spiky_planet_locked);
        assert!(controller.ship.race_planet_locked);
        assert!(controller.projectiles.is_empty());
    }

    #[test]
    fn racing_planet_is_visible_in_the_home_galaxy() {
        let track = RaceTrack::load().unwrap();
        let mut controller = ShipController::new(&MCQUEEN_CAR_SPEC);
        controller.ship.position = PLANET_POSITION.add(Vec3::new(80.0, 30.0, 70.0));
        let direction = PLANET_POSITION.sub(controller.ship.position).normalize();
        (controller.ship.yaw, controller.ship.pitch) = crate::direction_to_angles(direction);
        let camera = chase_camera(&controller.ship);
        let mut frame = Framebuffer::new(crate::WIDTH, crate::HEIGHT);
        crate::draw_skybox(&mut frame, 0.0, crate::galaxy::Galaxy::Home);
        let before = frame.pixels.clone();
        crate::render_model(
            &track.planet,
            &mut frame,
            &camera,
            |v| v.add(PLANET_POSITION),
            None,
            None,
        );
        assert!(
            frame
                .pixels
                .iter()
                .zip(&before)
                .filter(|(a, b)| a != b)
                .count()
                > 1000
        );
        if let Ok(path) = std::env::var("RACE_PLANET_PREVIEW") {
            frame.save_png(&path).unwrap();
        }
    }

    #[test]
    fn map_matches_source_dimensions_spawn_and_boundary_collisions() {
        let track = RaceTrack::load().unwrap();
        assert_eq!(track.tiles.len(), 41);
        assert!(track.tiles.iter().all(|row| row.len() == 65));
        assert!(!track.blocked(15.0 * TILE_SIZE, 34.0 * TILE_SIZE));
        assert!(track.blocked(-1.0, 40.0));
        assert!(track.blocked(10.0, 1.0));
        assert!(track.blocked(30.0 * TILE_SIZE, 20.0 * TILE_SIZE));
        assert!(track.map.faces.iter().any(|f| f.group == "Tile_M"));
    }

    #[test]
    fn existing_controller_moves_on_track_and_cannot_tunnel_through_boundary() {
        let track = RaceTrack::load().unwrap();
        let mut controller = ShipController::new(&MCQUEEN_CAR_SPEC);
        track.enter(&mut controller);
        let start = controller.ship.position;
        let input = ShipInput {
            pitch: 0,
            yaw: 0,
            roll: 0,
            accelerating: true,
            braking: false,
            firing: false,
        };
        track.update(&mut controller, &input, 0.05);
        assert!(controller.ship.position.x > start.x);
        controller.ship.position = Vec3::new(12.0, 2.0, 12.0);
        controller.ship.yaw = 0.0;
        track.update(&mut controller, &input, 1.0);
        assert!(!track.blocked(controller.ship.position.x, controller.ship.position.z));
        assert!(controller.ship.position.x > 6.0);
        track.leave(&mut controller);
        assert!(controller.ship.planet_locked);
        assert!(
            controller
                .ship
                .position
                .sub(PLANET_POSITION)
                .dot(controller.ship.position.sub(PLANET_POSITION))
                > PLANET_RADIUS.powi(2)
        );
    }
}
