mod core_guardian;
mod framebuffer;
mod input;
mod menu;
mod obj_loader;
mod ship;
mod triangle;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::f32::consts::{PI, TAU};
use std::sync::OnceLock;
use std::thread;
use std::time::Instant;

use core_guardian::{CoreGuardian, GuardianAction, GuardianInput};
use framebuffer::Framebuffer;
use minifb::{Key, ScaleMode, Window, WindowOptions};
use obj_loader::{Face, MaterialColor, Model, SurfaceShader, Vec3, load_obj};
use ship::{
    ARWING_SPEC, ARWING_SUBMARINE_SPEC, ASTRA_CAR_SPEC, ASTRA_SPEC, ASTRA_SUBMARINE_SPEC,
    BUZZ_CAR_SPEC, BUZZ_SPEC, BUZZ_SUBMARINE_SPEC, COWBOYB_CAR_SPEC, COWBOYB_SPEC,
    COWBOYB_SUBMARINE_SPEC, KIRBY_SPEC, KIRBY_SUBMARINE_SPEC, MCQUEEN_CAR_SPEC, MCQUEEN_SPEC,
    MCQUEEN_SUBMARINE_SPEC, Projectile, Ship, ShipController, ShipInput, ShipSpec,
};
use triangle::{ScreenVertex, draw_triangle_shaded_region};

const WIDTH: usize = 1200;
const HEIGHT: usize = 800;
const NEAR_PLANE: f32 = 0.2;
const FAR_PLANE: f32 = 750.0;
const VERTICAL_FOV: f32 = 60.0_f32.to_radians();
const LIGHT_SPEED: f32 = 600.0;
const PLANET_ORBIT_SPEED: f32 = 0.02;
const SUN_POSITION_X: f32 = -520.0;
const PLANET_ORBIT_RADIUS: f32 = 280.0;
const SPIKY_PLANET_ORBIT_SPEED: f32 = 0.012;
const SPIKY_PLANET_ORBIT_RADIUS: f32 = 430.0;
const SPIKY_PLANET_ORBIT_PHASE: f32 = PI * 0.5;
const SPIKY_PLANET_ORBIT_INCLINATION: f32 = 0.30;
const SPIKY_PLANET_RADIUS: f32 = 26.0;
const SPIKY_PLANET_ROTATION_SPEED: f32 = 0.18;
const WATER_PLANET_RADIUS: f32 = 24.0;
const COLLISION_MARGIN: f32 = 4.0;
const AVOIDANCE_DISTANCE: f32 = 52.0;
const ORBIT_LIMIT_RADIUS: f32 = 60.0;
const ORBIT_FLIGHT_ALTITUDE: f32 = 8.0;
const WATER_LEVEL_LENGTH: f32 = 525.0;
const WATER_LEVEL_HALF_WIDTH: f32 = 10.0;
const WATER_LEVEL_SOFT_BORDER: f32 = 8.4;
const WATER_LEVEL_BORDER_STRENGTH: f32 = 26.0;
const WATER_LEVEL_MIN_HEIGHT: f32 = 2.4;
const WATER_LEVEL_MAX_HEIGHT: f32 = 11.5;
const WATER_LEVEL_VERTICAL_OFFSET: f32 = 5.2;
const WATER_LEVEL_SOURCE_START: f32 = 90.0;
const WATER_LEVEL_SPEED_FACTOR: f32 = 0.55;
const WATER_ENEMY_SCALE: f32 = 1.05;
const LEVEL_CHUNK_LENGTH: f32 = 32.0;
const LEVEL_RENDER_BEHIND: f32 = 48.0;
const LEVEL_RENDER_AHEAD: f32 = 112.0;
const COLLISION_GRID_CELL_SIZE: f32 = 12.0;
const MAX_BUBBLE_PARTICLES: usize = 256;
const RESOLUTION_SCALES: [f32; 4] = [1.0, 0.9, 0.75, 0.6];

fn render_worker_count() -> usize {
    static WORKER_COUNT: OnceLock<usize> = OnceLock::new();
    *WORKER_COUNT.get_or_init(|| thread::available_parallelism().map_or(1, usize::from))
}

struct DynamicResolution {
    enabled: bool,
    scale_index: usize,
    fps_average: f32,
    low_time: f32,
    high_time: f32,
    cooldown: f32,
}

impl DynamicResolution {
    fn new() -> Self {
        Self {
            enabled: true,
            scale_index: 0,
            fps_average: 60.0,
            low_time: 0.0,
            high_time: 0.0,
            cooldown: 0.0,
        }
    }

    fn dimensions(&self) -> (usize, usize) {
        let scale = RESOLUTION_SCALES[self.scale_index];
        (
            ((WIDTH as f32 * scale).round() as usize).max(1),
            ((HEIGHT as f32 * scale).round() as usize).max(1),
        )
    }

    fn toggle(&mut self) -> bool {
        self.enabled = !self.enabled;
        self.low_time = 0.0;
        self.high_time = 0.0;
        if !self.enabled && self.scale_index != 0 {
            self.scale_index = 0;
            return true;
        }
        false
    }

    fn update(&mut self, frame_time: f32) -> bool {
        let frame_time = frame_time.max(0.000_1);
        let fps = 1.0 / frame_time;
        let smoothing = 1.0 - (-frame_time * 3.0).exp();
        self.fps_average += (fps - self.fps_average) * smoothing;
        self.cooldown = (self.cooldown - frame_time).max(0.0);
        if !self.enabled || self.cooldown > 0.0 {
            return false;
        }

        if self.fps_average < 50.0 {
            self.low_time += frame_time;
            self.high_time = 0.0;
        } else if self.fps_average > 58.0 {
            self.high_time += frame_time;
            self.low_time = 0.0;
        } else {
            self.low_time = 0.0;
            self.high_time = 0.0;
        }

        if self.low_time >= 1.0 && self.scale_index + 1 < RESOLUTION_SCALES.len() {
            self.scale_index += 1;
            self.low_time = 0.0;
            self.cooldown = 2.0;
            true
        } else if self.high_time >= 3.0 && self.scale_index > 0 {
            self.scale_index -= 1;
            self.high_time = 0.0;
            self.cooldown = 3.0;
            true
        } else {
            false
        }
    }

    fn percentage(&self) -> u32 {
        (RESOLUTION_SCALES[self.scale_index] * 100.0).round() as u32
    }
}

fn upscale_scene(source: &Framebuffer, target: &mut Framebuffer) {
    debug_assert_eq!((target.width, target.height), (WIDTH, HEIGHT));
    if source.width == target.width && source.height == target.height {
        target.pixels.copy_from_slice(&source.pixels);
        return;
    }
    for y in 0..target.height {
        let source_y = y * source.height / target.height;
        let target_row = y * target.width;
        let source_row = source_y * source.width;
        for x in 0..target.width {
            let source_x = x * source.width / target.width;
            target.pixels[target_row + x] = source.pixels[source_row + source_x];
        }
    }
}

struct Camera {
    position: Vec3,
    forward: Vec3,
    right: Vec3,
    up: Vec3,
}

struct ObjectCollider {
    center: Vec3,
    radius: f32,
}

struct SpatialCollisionGrid {
    cell_size: f32,
    cells: HashMap<(i32, i32, i32), Vec<usize>>,
}

impl SpatialCollisionGrid {
    fn new(cell_size: f32) -> Self {
        debug_assert!(cell_size > 0.0);
        Self {
            cell_size,
            cells: HashMap::new(),
        }
    }

    fn coordinate(&self, value: f32) -> i32 {
        (value / self.cell_size).floor() as i32
    }

    fn insert_sphere(&mut self, index: usize, center: Vec3, radius: f32) {
        let min_x = self.coordinate(center.x - radius);
        let max_x = self.coordinate(center.x + radius);
        let min_y = self.coordinate(center.y - radius);
        let max_y = self.coordinate(center.y + radius);
        let min_z = self.coordinate(center.z - radius);
        let max_z = self.coordinate(center.z + radius);
        for x in min_x..=max_x {
            for y in min_y..=max_y {
                for z in min_z..=max_z {
                    self.cells.entry((x, y, z)).or_default().push(index);
                }
            }
        }
    }

    fn candidates(&self, point: Vec3) -> &[usize] {
        let key = (
            self.coordinate(point.x),
            self.coordinate(point.y),
            self.coordinate(point.z),
        );
        self.cells.get(&key).map(Vec::as_slice).unwrap_or(&[])
    }
}

#[derive(Clone, Copy)]
struct LevelCollider {
    min: Vec3,
    max: Vec3,
}

struct LevelChunk {
    min_progress: f32,
    max_progress: f32,
    model: Model,
}

struct LevelSpatialIndex {
    render_chunks: Vec<LevelChunk>,
    collision_chunks: BTreeMap<i32, Vec<LevelCollider>>,
}

struct LevelChunkBuilder {
    vertex_map: HashMap<usize, usize>,
    vertices: Vec<Vec3>,
    faces: Vec<Face>,
}

impl LevelChunkBuilder {
    fn new() -> Self {
        Self {
            vertex_map: HashMap::new(),
            vertices: Vec::new(),
            faces: Vec::new(),
        }
    }

    fn add_face(&mut self, source: &Model, face: &Face) {
        let mut indices = [0; 3];
        for (target, source_index) in indices.iter_mut().zip(face.indices) {
            *target = *self.vertex_map.entry(source_index).or_insert_with(|| {
                let index = self.vertices.len();
                self.vertices.push(source.vertices[source_index]);
                index
            });
        }
        self.faces.push(Face::new(
            indices,
            face.material.clone(),
            face.group.clone(),
        ));
    }
}

impl LevelSpatialIndex {
    fn new(level: &Model, colliders: Vec<LevelCollider>) -> Self {
        let mut builders: BTreeMap<i32, LevelChunkBuilder> = BTreeMap::new();
        for face in &level.faces {
            let average_source_x = face
                .indices
                .iter()
                .map(|&index| level.vertices[index].x)
                .sum::<f32>()
                / 3.0;
            let progress = average_source_x + WATER_LEVEL_SOURCE_START;
            let chunk_index = (progress / LEVEL_CHUNK_LENGTH).floor() as i32;
            builders
                .entry(chunk_index)
                .or_insert_with(LevelChunkBuilder::new)
                .add_face(level, face);
        }

        let render_chunks = builders
            .into_iter()
            .map(|(index, builder)| LevelChunk {
                min_progress: index as f32 * LEVEL_CHUNK_LENGTH,
                max_progress: (index + 1) as f32 * LEVEL_CHUNK_LENGTH,
                model: Model::new(builder.vertices, builder.faces),
            })
            .collect();

        let mut collision_chunks: BTreeMap<i32, Vec<LevelCollider>> = BTreeMap::new();
        for collider in colliders {
            // Expand the longitudinal range by the ship radius so collisions at
            // sector boundaries remain active without querying every sector.
            let min_progress = -collider.max.x - 4.0;
            let max_progress = -collider.min.x + 4.0;
            let first = (min_progress / LEVEL_CHUNK_LENGTH).floor() as i32;
            let last = (max_progress / LEVEL_CHUNK_LENGTH).floor() as i32;
            for index in first..=last {
                collision_chunks.entry(index).or_default().push(collider);
            }
        }

        Self {
            render_chunks,
            collision_chunks,
        }
    }

    fn visible_chunks(&self, progress: f32) -> impl Iterator<Item = &Model> {
        let min_progress = progress - LEVEL_RENDER_BEHIND;
        let max_progress = progress + LEVEL_RENDER_AHEAD;
        self.render_chunks
            .iter()
            .filter(move |chunk| {
                chunk.max_progress >= min_progress && chunk.min_progress <= max_progress
            })
            .map(|chunk| &chunk.model)
    }

    fn nearby_colliders(&self, progress: f32) -> &[LevelCollider] {
        let index = (progress / LEVEL_CHUNK_LENGTH).floor() as i32;
        self.collision_chunks
            .get(&index)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}

#[derive(Clone, Copy, PartialEq)]
enum GameMode {
    FreeWorld,
    WaterLevel,
    CoreGuardian,
}

struct WaterLevelState {
    progress: f32,
    lateral: f32,
    height: f32,
    elapsed: f32,
    enemies: Vec<WaterEnemy>,
    defeated_enemies: usize,
}

struct WaterEnemy {
    kind: usize,
    progress: f32,
    lateral: f32,
    height: f32,
    phase: f32,
    alive: bool,
}

struct BubbleParticle {
    position: Vec3,
    velocity: Vec3,
    lifetime: f32,
    scale: f32,
}

struct BubbleEmitter {
    particles: Vec<BubbleParticle>,
    timer: f32,
    serial: u32,
}

impl BubbleEmitter {
    fn new() -> Self {
        Self {
            particles: Vec::new(),
            timer: 0.0,
            serial: 0,
        }
    }

    fn clear(&mut self) {
        for particle in &mut self.particles {
            particle.lifetime = 0.0;
        }
        self.timer = 0.0;
    }

    fn spawn(&mut self, particle: BubbleParticle) {
        if let Some(slot) = self.particles.iter_mut().find(|slot| slot.lifetime <= 0.0) {
            *slot = particle;
        } else if self.particles.len() < MAX_BUBBLE_PARTICLES {
            self.particles.push(particle);
        } else if let Some(slot) = self
            .particles
            .iter_mut()
            .min_by(|a, b| a.lifetime.total_cmp(&b.lifetime))
        {
            *slot = particle;
        }
    }

    fn update(&mut self, ship: &Ship, delta_time: f32) {
        self.timer -= delta_time;
        if self.timer <= 0.0 {
            let (forward, right, up) = ship.basis();
            for side in ship.spec.turbo_sides {
                let random = hash(self.serial.wrapping_mul(977).wrapping_add(17));
                let jitter = random as f32 / u32::MAX as f32 - 0.5;
                self.spawn(BubbleParticle {
                    position: ship
                        .position
                        .sub(forward.mul(ship.spec.turbo_back + 0.25))
                        .add(right.mul(*side * ship.spec.turbo_side + jitter * 0.35))
                        .sub(up.mul(ship.spec.turbo_down)),
                    velocity: forward
                        .mul(-1.7 - jitter.abs())
                        .add(up.mul(2.2 + jitter.abs() * 1.8))
                        .add(right.mul(jitter * 0.7)),
                    lifetime: 2.4 + jitter.abs(),
                    scale: 0.12 + jitter.abs() * 0.22,
                });
                self.serial = self.serial.wrapping_add(1);
            }
            self.timer += 0.075;
        }

        for bubble in self
            .particles
            .iter_mut()
            .filter(|bubble| bubble.lifetime > 0.0)
        {
            bubble.position = bubble.position.add(bubble.velocity.mul(delta_time));
            bubble.velocity = bubble.velocity.add(Vec3::new(0.0, 0.45 * delta_time, 0.0));
            bubble.lifetime -= delta_time;
            bubble.scale += delta_time * 0.035;
        }
    }
}

impl WaterLevelState {
    fn new() -> Self {
        let enemies = (0..30)
            .map(|index| WaterEnemy {
                kind: index % 3,
                progress: 18.0 + index as f32 * 16.0,
                lateral: [-3.5, 2.6, -1.0, 3.4, 0.7, -2.9][index % 6],
                height: [3.0, 5.5, 8.5, 4.0, 10.0, 7.0][index % 6],
                phase: index as f32 * 1.37,
                alive: true,
            })
            .collect();
        Self {
            progress: 3.0,
            lateral: 0.0,
            height: 4.5,
            elapsed: 0.0,
            enemies,
            defeated_enemies: 0,
        }
    }

    fn update(&mut self, ship: &mut Ship, input: &ShipInput, delta_time: f32) {
        self.elapsed += delta_time;
        self.progress += ship.speed * WATER_LEVEL_SPEED_FACTOR * delta_time;
        self.lateral += input.yaw as f32 * 17.0 * delta_time;

        let border_distance = self.lateral.abs();
        if border_distance > WATER_LEVEL_SOFT_BORDER {
            let border_zone = WATER_LEVEL_HALF_WIDTH - WATER_LEVEL_SOFT_BORDER;
            let border_force =
                ((border_distance - WATER_LEVEL_SOFT_BORDER) / border_zone).clamp(0.0, 1.0);
            self.lateral -=
                self.lateral.signum() * WATER_LEVEL_BORDER_STRENGTH * border_force * delta_time;
        }
        self.lateral = self
            .lateral
            .clamp(-WATER_LEVEL_HALF_WIDTH, WATER_LEVEL_HALF_WIDTH);
        self.height = (self.height + input.pitch as f32 * 15.0 * delta_time)
            .clamp(WATER_LEVEL_MIN_HEIGHT, WATER_LEVEL_MAX_HEIGHT);

        let route_curve = (self.progress * 0.12).sin() * 3.0;
        let route_height = (self.progress * 0.018).sin() * 1.5;
        ship.position = Vec3::new(
            -self.progress,
            self.height + route_height,
            self.lateral + route_curve,
        );
        ship.yaw = 0.0;
        ship.pitch = 0.0;
        ship.orbit_plane_normal = None;
        ship.orbit_center = None;
        ship.orbit_camera_blend = 0.0;
        ship.planet_locked = false;
        ship.orbit_release_active = false;
    }
}

impl WaterEnemy {
    fn position(&self, time: f32) -> Vec3 {
        let route_curve = (self.progress * 0.12).sin() * 3.0;
        let route_height = (self.progress * 0.018).sin() * 1.5;
        let (lateral_motion, vertical_motion) = match self.kind {
            0 => (
                (time * 0.65 + self.phase).sin() * 0.7,
                (time * 1.3 + self.phase).sin() * 0.5,
            ),
            1 => (
                (time * 1.15 + self.phase).sin() * 1.4,
                (time * 1.8 + self.phase).sin(),
            ),
            _ => (
                (time * 0.55 + self.phase).sin(),
                (time * 1.1 + self.phase).sin() * 1.5,
            ),
        };
        Vec3::new(
            -self.progress,
            self.height + route_height + vertical_motion,
            self.lateral + route_curve + lateral_motion,
        )
    }

    fn collision_radius(&self) -> f32 {
        [2.0, 2.5, 2.1][self.kind] * (WATER_ENEMY_SCALE / 0.85)
    }
}

fn level_to_world(vertex: Vec3) -> Vec3 {
    Vec3::new(
        -(vertex.x + WATER_LEVEL_SOURCE_START),
        vertex.z + WATER_LEVEL_VERTICAL_OFFSET,
        vertex.y,
    )
}

fn extract_model_group(model: &Model, group_name: &str) -> Result<Model, String> {
    let mut index_map = HashMap::new();
    let mut vertices = Vec::new();
    let mut faces = Vec::new();

    for face in model
        .faces
        .iter()
        .filter(|face| face.group.starts_with(group_name))
    {
        let mut indices = [0_usize; 3];
        for (target, source) in indices.iter_mut().zip(face.indices) {
            *target = *index_map.entry(source).or_insert_with(|| {
                let index = vertices.len();
                vertices.push(model.vertices[source]);
                index
            });
        }
        let mut extracted_face = Face::new(indices, face.material.clone(), face.group.clone());
        extracted_face.smooth = face.smooth;
        faces.push(extracted_face);
    }

    if faces.is_empty() {
        return Err(format!("No se encontro el grupo {group_name}"));
    }
    Ok(Model::new(vertices, faces))
}

fn stretch_model_group_height(model: &mut Model, group_name: &str, base: f32, factor: f32) {
    let mut affected = vec![false; model.vertices.len()];
    for face in model
        .faces
        .iter()
        .filter(|face| face.group.starts_with(group_name))
    {
        for index in face.indices {
            affected[index] = true;
        }
    }
    for (vertex, should_stretch) in model.vertices.iter_mut().zip(affected) {
        if should_stretch {
            vertex.z = base + (vertex.z - base) * factor;
        }
    }
    model.recalculate_bounds();
    model.recalculate_normals();
}

fn build_level_colliders(model: &Model) -> Vec<LevelCollider> {
    let mut bounds: HashMap<&str, (Vec3, Vec3)> = HashMap::new();
    for face in &model.faces {
        if !face.group.to_ascii_lowercase().contains("obstacle") {
            continue;
        }
        let entry = bounds.entry(face.group.as_str()).or_insert((
            Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
            Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
        ));
        for index in face.indices {
            let vertex = model.vertices[index];
            entry.0.x = entry.0.x.min(vertex.x);
            entry.0.y = entry.0.y.min(vertex.y);
            entry.0.z = entry.0.z.min(vertex.z);
            entry.1.x = entry.1.x.max(vertex.x);
            entry.1.y = entry.1.y.max(vertex.y);
            entry.1.z = entry.1.z.max(vertex.z);
        }
    }
    bounds
        .into_values()
        .map(|(min, max)| LevelCollider {
            min: Vec3::new(
                -(max.x + WATER_LEVEL_SOURCE_START),
                min.z + WATER_LEVEL_VERTICAL_OFFSET,
                min.y,
            ),
            max: Vec3::new(
                -(min.x + WATER_LEVEL_SOURCE_START),
                max.z + WATER_LEVEL_VERTICAL_OFFSET,
                max.y,
            ),
        })
        .collect()
}

fn resolve_level_collisions(
    state: &mut WaterLevelState,
    ship: &mut Ship,
    level: &LevelSpatialIndex,
) {
    for collider in level.nearby_colliders(state.progress) {
        let closest = Vec3::new(
            ship.position.x.clamp(collider.min.x, collider.max.x),
            ship.position.y.clamp(collider.min.y, collider.max.y),
            ship.position.z.clamp(collider.min.z, collider.max.z),
        );
        let offset = ship.position.sub(closest);
        let distance = offset.dot(offset).sqrt();
        let separation = ship.spec.collision_radius;
        if distance < separation {
            let lateral_offset = Vec3::new(0.0, offset.y, offset.z);
            let lateral_distance = lateral_offset.dot(lateral_offset).sqrt();
            let normal = if lateral_distance > 0.001 {
                lateral_offset.mul(1.0 / lateral_distance)
            } else {
                Vec3::new(0.0, 1.0, 0.0)
            };
            let correction = normal.mul(separation - distance + 0.2);
            state.height =
                (state.height + correction.y).clamp(WATER_LEVEL_MIN_HEIGHT, WATER_LEVEL_MAX_HEIGHT);
            state.lateral = (state.lateral + correction.z)
                .clamp(-WATER_LEVEL_HALF_WIDTH, WATER_LEVEL_HALF_WIDTH);
            ship.position = ship.position.add(correction);
        }
    }
}

fn resolve_water_enemy_collisions(
    state: &mut WaterLevelState,
    ship: &mut Ship,
    projectiles: &mut [Projectile],
) {
    let enemy_positions: Vec<Vec3> = state
        .enemies
        .iter()
        .map(|enemy| enemy.position(state.elapsed))
        .collect();
    let mut collision_grid = SpatialCollisionGrid::new(COLLISION_GRID_CELL_SIZE);
    for (index, enemy) in state.enemies.iter().enumerate() {
        if enemy.alive {
            collision_grid.insert_sphere(
                index,
                enemy_positions[index],
                enemy.collision_radius() + ship.spec.collision_radius.max(2.0),
            );
        }
    }

    for projectile in projectiles.iter_mut().filter(|shot| shot.lifetime > 0.0) {
        for &enemy_index in collision_grid.candidates(projectile.position) {
            let enemy = &mut state.enemies[enemy_index];
            if !enemy.alive {
                continue;
            }
            let offset = projectile.position.sub(enemy_positions[enemy_index]);
            let hit_radius = enemy.collision_radius() + projectile.scale * 0.35;
            if offset.dot(offset) <= hit_radius * hit_radius {
                enemy.alive = false;
                projectile.lifetime = 0.0;
                state.defeated_enemies += 1;
                break;
            }
        }
    }

    for &enemy_index in collision_grid.candidates(ship.position) {
        let enemy = &state.enemies[enemy_index];
        if !enemy.alive {
            continue;
        }
        let enemy_position = enemy_positions[enemy_index];
        let offset = ship.position.sub(enemy_position);
        let separation = ship.spec.collision_radius + enemy.collision_radius();
        let distance_squared = offset.dot(offset);
        if distance_squared >= separation * separation {
            continue;
        }

        let lateral_offset = Vec3::new(0.0, offset.y, offset.z);
        let lateral_distance = lateral_offset.dot(lateral_offset).sqrt();
        let normal = if lateral_distance > 0.001 {
            lateral_offset.mul(1.0 / lateral_distance)
        } else {
            Vec3::new(0.0, 1.0, 0.0)
        };
        let distance = distance_squared.sqrt();
        let correction = normal.mul(separation - distance + 0.25);
        state.height =
            (state.height + correction.y).clamp(WATER_LEVEL_MIN_HEIGHT, WATER_LEVEL_MAX_HEIGHT);
        state.lateral =
            (state.lateral + correction.z).clamp(-WATER_LEVEL_HALF_WIDTH, WATER_LEVEL_HALF_WIDTH);
        ship.position = ship.position.add(correction);
    }
}

fn build_object_colliders(model: &Model, model_center: Vec3, scale: f32) -> Vec<ObjectCollider> {
    let mut bounds: HashMap<&str, (Vec3, Vec3)> = HashMap::new();

    for face in &model.faces {
        let name = face.group.as_str();
        if !(name.contains("Palmera")
            || name.contains("Roca")
            || name.contains("Bloque")
            || name.contains("Interrogacion"))
        {
            continue;
        }

        let entry = bounds.entry(name).or_insert((
            Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
            Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
        ));
        for index in face.indices {
            let vertex = model.vertices[index];
            entry.0.x = entry.0.x.min(vertex.x);
            entry.0.y = entry.0.y.min(vertex.y);
            entry.0.z = entry.0.z.min(vertex.z);
            entry.1.x = entry.1.x.max(vertex.x);
            entry.1.y = entry.1.y.max(vertex.y);
            entry.1.z = entry.1.z.max(vertex.z);
        }
    }

    bounds
        .into_values()
        .map(|(min, max)| {
            let center = min.add(max).mul(0.5);
            let half_size = max.sub(min).mul(0.5);
            ObjectCollider {
                center: center.sub(model_center).mul(scale),
                radius: half_size.dot(half_size).sqrt() * scale + 0.6,
            }
        })
        .filter(|collider| collider.radius > 0.7)
        .collect()
}

fn material_radius(model: &Model, center: Vec3, material_name: &str) -> f32 {
    let mut radius: f32 = 0.0;
    for face in &model.faces {
        if face.material.contains(material_name) {
            for index in face.indices {
                let offset = model.vertices[index].sub(center);
                radius = radius.max(offset.dot(offset).sqrt());
            }
        }
    }
    radius
}

fn screen_size() -> (usize, usize) {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN,
        };
        let width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        let height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
        if width > 0 && height > 0 {
            return (width as usize, height as usize);
        }
    }
    (WIDTH, HEIGHT)
}

fn create_window(fullscreen: bool) -> Result<Window, minifb::Error> {
    let (width, height) = if fullscreen {
        screen_size()
    } else {
        (WIDTH, HEIGHT)
    };
    let mut window = Window::new(
        "Sistema solar | Vuelo libre 3D | F11: fullscreen",
        width,
        height,
        WindowOptions {
            borderless: fullscreen,
            resize: !fullscreen,
            scale_mode: ScaleMode::AspectRatioStretch,
            ..WindowOptions::default()
        },
    )?;
    if fullscreen {
        window.set_position(0, 0);
    }
    input::attach(&mut window);
    window.set_target_fps(60);
    Ok(window)
}

fn color_for_material(color: MaterialColor, effect_spec: Option<&ShipSpec>) -> [u8; 3] {
    match color {
        MaterialColor::Fixed(color) => color,
        MaterialColor::LaserCore => {
            effect_spec.map_or([255, 245, 190], |spec| spec.laser_core_color)
        }
        MaterialColor::Laser => effect_spec.map_or([255, 48, 38], |spec| spec.laser_color),
        MaterialColor::TurboCore => {
            effect_spec.map_or([220, 252, 255], |spec| spec.turbo_core_color)
        }
        MaterialColor::Turbo => effect_spec.map_or([36, 142, 255], |spec| spec.turbo_color),
    }
}

fn pack(rgb: [u8; 3], brightness: f32) -> u32 {
    let channel = |value: u8| (value as f32 * brightness).clamp(0.0, 255.0) as u32;
    (channel(rgb[0]) << 16) | (channel(rgb[1]) << 8) | channel(rgb[2])
}

fn procedural_surface_color(
    shader: SurfaceShader,
    point: Vec3,
    x: i32,
    y: i32,
    time: f32,
    brightness: f32,
    fresnel: f32,
) -> u32 {
    let noise = ((point.x * 31.0).sin() * (point.y * 27.0).cos() + (point.z * 43.0).sin()) * 0.5;

    let (red, green, blue, light) = match shader {
        SurfaceShader::Ocean => {
            let direction = point.normalize();
            let wave_a = (direction.x * 19.0 + direction.z * 7.0 + time * 1.8).sin();
            let wave_b = (direction.z * 25.0 - direction.y * 9.0 - time * 1.35).sin();
            let current =
                (direction.x.atan2(direction.z) * 7.0 + direction.y * 16.0 + time * 0.55).sin();
            let fine_wave = (direction.x * 52.0 - direction.z * 47.0 + time * 2.7).sin();
            let wave = wave_a * 0.34 + wave_b * 0.28 + current * 0.24 + fine_wave * 0.14;
            let foam = (wave * 0.5 + current * 0.5 - 0.58).max(0.0).powf(2.0) * 2.8;
            let sparkle = (x as f32 * 0.07 + y as f32 * 0.04 + time * 3.2)
                .sin()
                .max(0.0)
                .powf(18.0);
            (
                3.0 + wave * 4.0 + fresnel * 38.0 + sparkle * 120.0 + foam * 74.0,
                58.0 + wave * 22.0 + fresnel * 92.0 + sparkle * 102.0 + foam * 92.0,
                132.0 + wave * 32.0 + fresnel * 118.0 + sparkle * 76.0 + foam * 80.0,
                0.82 + wave * 0.12 + fresnel * 0.16,
            )
        }
        SurfaceShader::Waterfall => {
            let flow = (point.y * 38.0 - time * 6.0 + (point.x * 17.0).sin()).sin();
            let foam = flow.max(0.65).powf(4.0) * 3.0;
            (
                35.0 + foam * 70.0,
                170.0 + foam * 65.0,
                205.0 + foam * 45.0,
                1.0,
            )
        }
        SurfaceShader::Earth => (105.0 + noise * 24.0, 67.0 + noise * 15.0, 38.0, 1.0),
        SurfaceShader::Grass => {
            let blades = (point.x * 85.0 + point.z * 63.0).sin().abs();
            (
                35.0 + noise * 8.0,
                120.0 + blades * 45.0 + noise * 18.0,
                48.0,
                1.0,
            )
        }
        SurfaceShader::Rock => {
            let grain = noise.abs();
            (
                105.0 + grain * 38.0,
                108.0 + grain * 35.0,
                112.0 + grain * 32.0,
                1.0,
            )
        }
        SurfaceShader::DarkRock => {
            let grain = noise.abs();
            (
                42.0 + grain * 30.0,
                46.0 + grain * 28.0,
                54.0 + grain * 32.0,
                1.0,
            )
        }
        SurfaceShader::Sand => {
            let ripples = (point.x * 52.0 + (point.z * 19.0).sin()).sin();
            (
                205.0 + ripples * 15.0,
                174.0 + ripples * 13.0,
                96.0 + ripples * 8.0,
                1.0,
            )
        }
        SurfaceShader::Wood => {
            let rings = (point.y * 35.0 + point.x.atan2(point.z) * 4.0).sin().abs();
            (
                75.0 + rings * 45.0,
                43.0 + rings * 24.0,
                20.0 + rings * 10.0,
                1.0,
            )
        }
        SurfaceShader::Leaves => {
            let veins = (point.x * 70.0 + point.z * 45.0).sin().abs();
            (
                24.0 + veins * 20.0,
                100.0 + veins * 75.0,
                42.0 + veins * 20.0,
                1.0,
            )
        }
        SurfaceShader::YellowBlock => {
            let pulse = (time * 2.2).sin() * 0.08 + 0.92;
            (245.0, 190.0, 28.0, pulse)
        }
        SurfaceShader::WhiteMark => (255.0, 250.0, 220.0, 1.15),
        SurfaceShader::ArwingHull => (
            238.0 + fresnel * 17.0,
            243.0 + fresnel * 12.0,
            248.0 + fresnel * 7.0,
            0.88 + fresnel * 0.28,
        ),
        SurfaceShader::ArwingMetal => (
            91.0 + fresnel * 70.0,
            105.0 + fresnel * 74.0,
            119.0 + fresnel * 80.0,
            0.78 + fresnel * 0.42,
        ),
        SurfaceShader::ArwingBlue => (
            30.0 + fresnel * 32.0,
            78.0 + fresnel * 58.0,
            190.0 + fresnel * 65.0,
            0.84 + fresnel * 0.35,
        ),
        SurfaceShader::ArwingGlass => (
            15.0 + fresnel * 52.0,
            47.0 + fresnel * 86.0,
            74.0 + fresnel * 115.0,
            0.72 + fresnel * 0.58,
        ),
        SurfaceShader::ArwingDarkMetal => (
            25.0 + fresnel * 48.0,
            31.0 + fresnel * 53.0,
            42.0 + fresnel * 60.0,
            0.72 + fresnel * 0.44,
        ),
        SurfaceShader::ArwingEmission => {
            let pulse = 0.96 + (time * 8.0 + point.x * 0.7).sin() * 0.04;
            (
                40.0 * pulse,
                225.0 * pulse,
                242.0 * pulse,
                1.0 / brightness.max(0.01),
            )
        }
        SurfaceShader::ArwingYellow => (
            250.0,
            194.0 + fresnel * 38.0,
            34.0 + fresnel * 28.0,
            0.88 + fresnel * 0.28,
        ),
        SurfaceShader::AstraCobalt => (
            28.0 + fresnel * 34.0,
            92.0 + fresnel * 58.0,
            224.0 + fresnel * 31.0,
            0.82 + fresnel * 0.36,
        ),
        SurfaceShader::AstraDeepBlue => (
            14.0 + fresnel * 28.0,
            38.0 + fresnel * 45.0,
            105.0 + fresnel * 72.0,
            0.76 + fresnel * 0.42,
        ),
        SurfaceShader::AstraRedEmission => {
            let pulse = 0.96 + (time * 7.0 + point.z).sin() * 0.04;
            (
                255.0 * pulse,
                35.0 * pulse,
                42.0 * pulse,
                1.0 / brightness.max(0.01),
            )
        }
        SurfaceShader::CowboyYellowPaint => (
            170.0 + fresnel * 82.0,
            91.0 + fresnel * 76.0,
            8.0 + fresnel * 56.0,
            0.78 + fresnel * 0.62,
        ),
        SurfaceShader::CowboyRedPaint => (
            145.0 + fresnel * 104.0,
            12.0 + fresnel * 42.0,
            9.0 + fresnel * 34.0,
            0.76 + fresnel * 0.66,
        ),
        SurfaceShader::CowboyOrangeEmission => {
            let pulse = 0.94 + (time * 8.0 + point.z * 1.7).sin() * 0.06;
            (
                255.0 * pulse,
                72.0 * pulse,
                12.0 * pulse,
                1.0 / brightness.max(0.01),
            )
        }
        SurfaceShader::BuzzWhiteMetal => (
            188.0 + fresnel * 66.0,
            198.0 + fresnel * 56.0,
            208.0 + fresnel * 46.0,
            0.82 + fresnel * 0.52,
        ),
        SurfaceShader::BuzzGreenMetal => (
            72.0 + fresnel * 72.0,
            164.0 + fresnel * 82.0,
            18.0 + fresnel * 52.0,
            0.80 + fresnel * 0.56,
        ),
        SurfaceShader::BuzzPurpleMetal => (
            76.0 + fresnel * 72.0,
            28.0 + fresnel * 48.0,
            142.0 + fresnel * 104.0,
            0.78 + fresnel * 0.58,
        ),
        SurfaceShader::BuzzRedMetal => (
            184.0 + fresnel * 66.0,
            18.0 + fresnel * 38.0,
            30.0 + fresnel * 42.0,
            0.80 + fresnel * 0.52,
        ),
        SurfaceShader::BuzzEmission => {
            let pulse = 0.94 + (time * 8.5 + point.x * 0.8).sin() * 0.06;
            (
                128.0 * pulse,
                255.0 * pulse,
                30.0 * pulse,
                1.0 / brightness.max(0.01),
            )
        }
        SurfaceShader::McQueenRedMetal => (
            174.0 + fresnel * 78.0,
            12.0 + fresnel * 38.0,
            18.0 + fresnel * 42.0,
            0.76 + fresnel * 0.68,
        ),
        SurfaceShader::McQueenYellowMetal => (
            210.0 + fresnel * 44.0,
            132.0 + fresnel * 74.0,
            12.0 + fresnel * 46.0,
            0.80 + fresnel * 0.58,
        ),
        SurfaceShader::McQueenWhiteMetal => (
            202.0 + fresnel * 52.0,
            208.0 + fresnel * 46.0,
            216.0 + fresnel * 38.0,
            0.84 + fresnel * 0.48,
        ),
        SurfaceShader::McQueenEmission => {
            let pulse = 0.93 + (time * 9.0 + point.z * 1.4).sin() * 0.07;
            (
                255.0 * pulse,
                82.0 * pulse,
                14.0 * pulse,
                1.0 / brightness.max(0.01),
            )
        }
        SurfaceShader::Flat => (220.0, 230.0, 238.0, 1.0),
    };

    let scale = brightness * light;
    ((red * scale).clamp(0.0, 255.0) as u32) << 16
        | ((green * scale).clamp(0.0, 255.0) as u32) << 8
        | (blue * scale).clamp(0.0, 255.0) as u32
}

fn refract_water_color(
    surface: u32,
    background: &[u32],
    width: usize,
    height: usize,
    x: i32,
    y: i32,
    point: Vec3,
    time: f32,
    fresnel: f32,
) -> u32 {
    let distortion_strength = (1.0 - fresnel).clamp(0.12, 1.0) * 5.5;
    let offset_x = ((point.y * 23.0 + point.z * 17.0 + time * 1.9).sin()
        + (point.x * 41.0 - time * 2.4).sin() * 0.45)
        * distortion_strength;
    let offset_y = ((point.x * 19.0 - point.z * 29.0 - time * 1.5).cos()
        + (point.y * 37.0 + time * 2.1).sin() * 0.38)
        * distortion_strength
        * 0.65;
    let sample_x = (x as f32 + offset_x)
        .round()
        .clamp(0.0, width.saturating_sub(1) as f32) as usize;
    let sample_y = (y as f32 + offset_y)
        .round()
        .clamp(0.0, height.saturating_sub(1) as f32) as usize;
    let refracted = background[sample_y * width + sample_x];

    let surface_r = ((surface >> 16) & 0xff) as f32;
    let surface_g = ((surface >> 8) & 0xff) as f32;
    let surface_b = (surface & 0xff) as f32;
    let refracted_r = ((refracted >> 16) & 0xff) as f32;
    let refracted_g = ((refracted >> 8) & 0xff) as f32;
    let refracted_b = (refracted & 0xff) as f32;
    let transmission = (0.66 * (1.0 - fresnel) + 0.10).clamp(0.18, 0.72);
    let reflection = 1.0 - transmission;
    let depth_tint = 0.82 + fresnel * 0.18;
    let red = (refracted_r * transmission * 0.38 + surface_r * reflection).min(255.0);
    let green = (refracted_g * transmission * 0.72 + surface_g * reflection + 8.0).min(255.0);
    let blue = (refracted_b * transmission * depth_tint + surface_b * reflection + 18.0).min(255.0);
    ((red as u32) << 16) | ((green as u32) << 8) | blue as u32
}

fn hash(mut value: u32) -> u32 {
    value ^= value >> 16;
    value = value.wrapping_mul(0x7feb_352d);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846c_a68b);
    value ^ (value >> 16)
}

fn draw_skybox(framebuffer: &mut Framebuffer, travel: f32) {
    for y in 0..framebuffer.height {
        let t = y as f32 / framebuffer.height as f32;
        let horizon = (1.0 - (t - 0.48).abs() * 2.0).max(0.0);
        let color = (((4.0 + horizon * 8.0) as u32) << 16)
            | (((7.0 + horizon * 11.0) as u32) << 8)
            | (20.0 + horizon * 24.0) as u32;
        let start = y * framebuffer.width;
        framebuffer.pixels[start..start + framebuffer.width].fill(color);
    }

    let center_x = framebuffer.width as f32 * 0.5;
    let center_y = framebuffer.height as f32 * 0.48;
    for index in 0..700_u32 {
        let world_x = hash(index * 3 + 1) as f32 / u32::MAX as f32 * 2.0 - 1.0;
        let world_y = hash(index * 3 + 2) as f32 / u32::MAX as f32 * 2.0 - 1.0;
        let initial_z = hash(index * 3 + 3) as f32 / u32::MAX as f32;
        let z = (initial_z - travel).rem_euclid(1.0).max(0.025);
        let perspective = 0.58 / z;
        let x = (center_x + world_x * framebuffer.width as f32 * perspective) as i32;
        let y = (center_y + world_y * framebuffer.height as f32 * perspective) as i32;
        if x < 0 || y < 0 || x >= framebuffer.width as i32 || y >= framebuffer.height as i32 {
            continue;
        }
        let brightness = (125.0 + (1.0 - z) * 130.0) as u32;
        let color = (brightness << 16) | (brightness << 8) | (brightness + 18).min(255);
        let size = if z < 0.16 {
            3
        } else if z < 0.34 {
            2
        } else {
            1
        };
        for dy in 0..size {
            for dx in 0..size {
                let px = x + dx;
                let py = y + dy;
                if px < framebuffer.width as i32 && py < framebuffer.height as i32 {
                    framebuffer.pixels[py as usize * framebuffer.width + px as usize] = color;
                }
            }
        }
    }
}

fn draw_water_planet_atmosphere(
    framebuffer: &mut Framebuffer,
    camera: &Camera,
    planet_position: Vec3,
    planet_radius: f32,
    time: f32,
) {
    let relative = planet_position.sub(camera.position);
    let camera_x = relative.dot(camera.right);
    let camera_y = relative.dot(camera.up);
    let camera_z = relative.dot(camera.forward);
    if camera_z <= NEAR_PLANE {
        return;
    }

    let focal = framebuffer.height as f32 / (VERTICAL_FOV * 0.5).tan() * 0.5;
    let center_x = framebuffer.width as f32 * 0.5 + camera_x * focal / camera_z;
    let center_y = framebuffer.height as f32 * 0.5 - camera_y * focal / camera_z;
    let radius = (planet_radius * focal / camera_z).max(1.0);
    let inner = radius * 0.94;
    let outer = radius * 1.32;
    let min_x = (center_x - outer).floor().max(0.0) as usize;
    let max_x = (center_x + outer)
        .ceil()
        .min((framebuffer.width.saturating_sub(1)) as f32) as usize;
    let min_y = (center_y - outer).floor().max(0.0) as usize;
    let max_y = (center_y + outer)
        .ceil()
        .min((framebuffer.height.saturating_sub(1)) as f32) as usize;
    if min_x > max_x || min_y > max_y {
        return;
    }

    let pulse = 0.92 + (time * 0.8).sin() * 0.08;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let dx = x as f32 + 0.5 - center_x;
            let dy = y as f32 + 0.5 - center_y;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance < inner || distance > outer {
                continue;
            }
            let edge = ((outer - distance) / (outer - inner)).clamp(0.0, 1.0);
            let rim = (1.0 - ((distance - radius) / (outer - radius)).abs()).clamp(0.0, 1.0);
            let strength = (edge * edge * 0.34 + rim.powf(3.0) * 0.42) * pulse;
            let index = y * framebuffer.width + x;
            let old = framebuffer.pixels[index];
            let old_r = ((old >> 16) & 0xff) as f32;
            let old_g = ((old >> 8) & 0xff) as f32;
            let old_b = (old & 0xff) as f32;
            let red = (old_r + 18.0 * strength).min(255.0) as u32;
            let green = (old_g + 126.0 * strength).min(255.0) as u32;
            let blue = (old_b + 220.0 * strength).min(255.0) as u32;
            framebuffer.pixels[index] = (red << 16) | (green << 8) | blue;
        }
    }
}

fn make_sphere(radius: f32, stacks: usize, slices: usize, sun: bool) -> Model {
    let mut vertices = Vec::new();
    for stack in 0..=stacks {
        let latitude = PI * stack as f32 / stacks as f32 - PI * 0.5;
        let ring = latitude.cos() * radius;
        let y = latitude.sin() * radius;
        for slice in 0..=slices {
            let longitude = TAU * slice as f32 / slices as f32;
            vertices.push(Vec3::new(ring * longitude.cos(), y, ring * longitude.sin()));
        }
    }

    let mut faces = Vec::new();
    for stack in 0..stacks {
        for slice in 0..slices {
            let first = stack * (slices + 1) + slice;
            let second = first + slices + 1;
            let material = if sun {
                "sun"
            } else if stack < 2 || stack + 2 >= stacks {
                "planet_ice"
            } else if hash((stack * 97 + slice * 31) as u32) % 5 < 2 {
                "planet_land"
            } else {
                "planet_ocean"
            };
            let group = if sun { "sun" } else { "planet" };
            faces.push(Face::new([first, second, first + 1], material, group));
            faces.push(Face::new([first + 1, second, second + 1], material, group));
        }
    }
    Model::new(vertices, faces)
}

fn make_turbo_plume(segments: usize) -> Model {
    let mut vertices = vec![Vec3::new(-1.0, 0.0, 0.0)];
    for segment in 0..segments {
        let angle = TAU * segment as f32 / segments as f32;
        vertices.push(Vec3::new(0.0, angle.cos(), angle.sin()));
    }

    let mut faces = Vec::new();
    for segment in 0..segments {
        let next = (segment + 1) % segments;
        faces.push(Face::new(
            [0, segment + 1, next + 1],
            if segment % 2 == 0 {
                "turbo_core"
            } else {
                "turbo_glow"
            },
            "turbo",
        ));
    }
    Model::new(vertices, faces)
}

fn make_laser_bolt() -> Model {
    let vertices = vec![
        Vec3::new(1.4, 0.0, 0.0),
        Vec3::new(-1.4, 0.0, 0.0),
        Vec3::new(0.0, 0.16, 0.0),
        Vec3::new(0.0, 0.0, 0.16),
        Vec3::new(0.0, -0.16, 0.0),
        Vec3::new(0.0, 0.0, -0.16),
    ];
    let mut faces = Vec::new();
    for side in 0..4 {
        let current = 2 + side;
        let next = 2 + (side + 1) % 4;
        let material = if side % 2 == 0 {
            "laser_core"
        } else {
            "laser_glow"
        };
        faces.push(Face::new([0, current, next], material, "laser"));
        faces.push(Face::new([1, next, current], material, "laser"));
    }
    Model::new(vertices, faces)
}

fn make_bubble_model() -> Model {
    let vertices = vec![
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, -1.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(-1.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 0.0, -1.0),
    ];
    let triangles = [
        [0, 2, 4],
        [0, 4, 3],
        [0, 3, 5],
        [0, 5, 2],
        [1, 4, 2],
        [1, 3, 4],
        [1, 5, 3],
        [1, 2, 5],
    ];
    let faces = triangles
        .into_iter()
        .map(|indices| Face::new(indices, "bubble_submarine", "bubble_submarine"))
        .collect();
    Model::new(vertices, faces)
}

fn model_center(model: &Model) -> Vec3 {
    let mut min = model.vertices[0];
    let mut max = min;
    for vertex in &model.vertices {
        min.x = min.x.min(vertex.x);
        min.y = min.y.min(vertex.y);
        min.z = min.z.min(vertex.z);
        max.x = max.x.max(vertex.x);
        max.y = max.y.max(vertex.y);
        max.z = max.z.max(vertex.z);
    }
    min.add(max).mul(0.5)
}

fn model_radius(model: &Model, center: Vec3) -> f32 {
    model
        .vertices
        .iter()
        .map(|vertex| vertex.sub(center).dot(vertex.sub(center)).sqrt())
        .fold(0.0, f32::max)
}

fn move_swordfish_wheels_outward(model: &mut Model) {
    // The OBJ has no wheel groups, so identify side-mounted rubber and hubs by position.
    let wheel_vertices: HashSet<usize> = model
        .faces
        .iter()
        .filter(|face| face.material == "Titanio" || face.material == "Grafito")
        .filter(|face| {
            let center = face
                .indices
                .iter()
                .map(|&index| model.vertices[index])
                .fold(Vec3::default(), |sum, vertex| sum.add(vertex))
                .mul(1.0 / face.indices.len() as f32);
            center.x.abs() > 0.9 && center.z.abs() > 0.9 && center.y < 1.4
        })
        .flat_map(|face| face.indices)
        .collect();
    if wheel_vertices.is_empty() {
        return;
    }
    for index in wheel_vertices {
        let vertex = &mut model.vertices[index];
        vertex.x += vertex.x.signum() * 0.45;
    }
}

fn chase_camera(ship: &Ship) -> Camera {
    let (forward, ship_right, ship_up) = ship.camera_basis();
    let chase_position = ship
        .position
        .sub(forward.mul(ship.spec.chase_distance))
        .add(ship_up.mul(ship.spec.chase_height));
    let chase_target = ship.position.add(forward.mul(ship.spec.chase_look_ahead));
    let world_up = Vec3::new(0.0, 1.0, 0.0);

    let (orbit_position, orbit_target) = if let Some(planet_center) = ship.orbit_center {
        let radial = ship.position.sub(planet_center).normalize();
        (
            planet_center.add(radial.mul(55.0)).add(world_up.mul(16.0)),
            ship.position.add(forward.mul(3.0)),
        )
    } else {
        (chase_position, chase_target)
    };

    let blend = ship.orbit_camera_blend;
    let position = chase_position
        .mul(1.0 - blend)
        .add(orbit_position.mul(blend));
    let target = chase_target.mul(1.0 - blend).add(orbit_target.mul(blend));
    let camera_forward = target.sub(position).normalize();
    let up_reference = ship_up
        .mul(1.0 - blend)
        .add(world_up.mul(blend))
        .normalize();
    let mut right = camera_forward.cross(up_reference).normalize();
    if right.dot(right) < 0.001 {
        right = ship_right;
    }
    let up = right.cross(camera_forward).normalize();
    Camera {
        position,
        forward: camera_forward,
        right,
        up,
    }
}

fn water_planet_position(world_time: f32) -> Vec3 {
    let sun_position = Vec3::new(SUN_POSITION_X, 20.0, 0.0);
    let orbit = world_time * PLANET_ORBIT_SPEED;
    sun_position.add(Vec3::new(
        orbit.cos() * PLANET_ORBIT_RADIUS,
        -20.0,
        orbit.sin() * PLANET_ORBIT_RADIUS,
    ))
}

fn spiky_planet_position(world_time: f32) -> Vec3 {
    let sun_position = Vec3::new(SUN_POSITION_X, 20.0, 0.0);
    let orbit = world_time * SPIKY_PLANET_ORBIT_SPEED + SPIKY_PLANET_ORBIT_PHASE;
    let (sin, cos) = orbit.sin_cos();
    let (sin_inclination, cos_inclination) = SPIKY_PLANET_ORBIT_INCLINATION.sin_cos();
    sun_position.add(Vec3::new(
        cos * SPIKY_PLANET_ORBIT_RADIUS,
        sin * SPIKY_PLANET_ORBIT_RADIUS * sin_inclination - 20.0,
        sin * SPIKY_PLANET_ORBIT_RADIUS * cos_inclination,
    ))
}

fn spiky_planet_is_closer(ship_position: Vec3, water_center: Vec3, spiky_center: Vec3) -> bool {
    let water_offset = ship_position.sub(water_center);
    let spiky_offset = ship_position.sub(spiky_center);
    spiky_offset.dot(spiky_offset) < water_offset.dot(water_offset)
}

fn place_ship_near_water_planet(controller: &mut ShipController, world_time: f32) {
    controller.reset();
    let ship = &mut controller.ship;
    ship.position = water_planet_position(world_time).add(Vec3::new(45.0, 8.0, 0.0));
    ship.spiky_planet_locked = false;
    ship.pitch = -0.08;
    ship.speed = ship.spec.cruise_speed;
}

fn place_ship_near_spiky_planet(controller: &mut ShipController, world_time: f32) {
    controller.reset();
    let ship = &mut controller.ship;
    let center = spiky_planet_position(world_time);
    let offset = Vec3::new(45.0, 8.0, 0.0);
    ship.position = center.add(offset);
    ship.pitch = direction_to_angles(offset.mul(-1.0)).1;
    ship.speed = ship.spec.cruise_speed;
    ship.planet_locked = true;
    ship.spiky_planet_locked = true;
    ship.orbit_center = Some(center);
    let radial = offset.normalize();
    let (_, right, _) = ship.movement_basis();
    ship.orbit_plane_normal = Some(radial.cross(right).normalize());
    ship.orbit_camera_blend = 1.0;
}

fn direction_to_angles(direction: Vec3) -> (f32, f32) {
    let direction = direction.normalize();
    let pitch = direction.y.clamp(-1.0, 1.0).asin();
    let yaw = (-direction.z).atan2(-direction.x).rem_euclid(TAU);
    (yaw, pitch)
}

fn move_around_planet(
    ship: &mut Ship,
    movement: f32,
    planet_center: Vec3,
    planet_surface_radius: f32,
    planet_rotation: f32,
    object_colliders: &[ObjectCollider],
    yaw_input: i8,
    pitch_input: i8,
) {
    let safe_radius = planet_surface_radius + ship.spec.collision_radius + COLLISION_MARGIN;
    let steps = movement.ceil().max(1.0) as usize;
    let step_distance = movement / steps as f32;

    if ship.planet_locked {
        if let Some(previous_center) = ship.orbit_center {
            ship.position = ship.position.add(planet_center.sub(previous_center));
        }
        ship.orbit_center = Some(planet_center);
    }

    let (sin_rotation, cos_rotation) = planet_rotation.sin_cos();
    let rotated_collider_centers: Vec<Vec3> = object_colliders
        .iter()
        .map(|collider| {
            let local = collider.center;
            Vec3::new(
                local.x * cos_rotation - local.z * sin_rotation,
                local.y,
                local.x * sin_rotation + local.z * cos_rotation,
            )
            .add(planet_center)
        })
        .collect();
    let mut object_grid = SpatialCollisionGrid::new(COLLISION_GRID_CELL_SIZE);
    for (index, collider) in object_colliders.iter().enumerate() {
        object_grid.insert_sphere(
            index,
            rotated_collider_centers[index],
            collider.radius + ship.spec.collision_radius + 0.8,
        );
    }

    for _ in 0..steps {
        let offset = ship.position.sub(planet_center);
        let distance = offset.dot(offset).sqrt();
        let normal = if distance > 0.001 {
            offset.mul(1.0 / distance)
        } else {
            Vec3::new(1.0, 0.0, 0.0)
        };
        let (mut forward, right, up) = ship.movement_basis();
        let approaching = forward.dot(normal) < 0.0;

        if ship.orbit_release_active && distance > ORBIT_LIMIT_RADIUS + 8.0 {
            ship.orbit_release_active = false;
        }
        if !ship.orbit_release_active && distance < AVOIDANCE_DISTANCE {
            if !ship.planet_locked {
                let level_roll = (ship.roll + PI).rem_euclid(TAU) - PI;
                ship.roll = level_roll.clamp(-ship.spec.max_manual_roll, ship.spec.max_manual_roll);
                ship.roll_target = ship.roll;
                ship.speed = ship.speed.max(ship.spec.min_speed);
            }
            ship.planet_locked = true;
        }

        if ship.planet_locked {
            ship.orbit_center = Some(planet_center);

            // Keep a fixed orbital plane and radius so steering updates cannot
            // accumulate vertical drift and carry the ship away from the planet.
            let plane_normal = ship.orbit_plane_normal.unwrap_or_else(|| {
                let initial_tangent = right.sub(normal.mul(right.dot(normal))).normalize();
                normal.cross(initial_tangent).normalize()
            });
            ship.orbit_plane_normal = Some(plane_normal);
            let tangent = plane_normal.cross(normal).normalize();
            let target_radius = safe_radius + ORBIT_FLIGHT_ALTITUDE;
            let angle = step_distance / target_radius;
            let (sin_angle, cos_angle) = angle.sin_cos();
            let next_normal = normal
                .mul(cos_angle)
                .add(tangent.mul(sin_angle))
                .normalize();
            let orbit_forward = tangent
                .mul(cos_angle)
                .sub(normal.mul(sin_angle))
                .normalize();
            ship.position = planet_center.add(next_normal.mul(target_radius));
            let (yaw, pitch) = direction_to_angles(orbit_forward);
            ship.yaw = yaw;
            ship.pitch = pitch.clamp(-1.25, 1.25);
            continue;
        } else if distance > AVOIDANCE_DISTANCE + 8.0 || ship.orbit_release_active {
            ship.orbit_plane_normal = None;
        }

        if distance < AVOIDANCE_DISTANCE && approaching {
            let strength = ((AVOIDANCE_DISTANCE - distance) / (AVOIDANCE_DISTANCE - safe_radius))
                .clamp(0.0, 1.0);
            let mut tangent = forward.sub(normal.mul(forward.dot(normal)));
            if tangent.dot(tangent) < 0.0001 {
                let side = if yaw_input == 0 {
                    1.0
                } else {
                    yaw_input as f32
                };
                tangent = right.mul(side).add(up.mul(pitch_input as f32 * 0.35));
            }
            let tangent = tangent.normalize();
            let steering = 0.12 + strength * 0.55;
            forward = forward
                .mul(1.0 - steering)
                .add(tangent.mul(steering))
                .normalize();
            let (yaw, pitch) = direction_to_angles(forward);
            ship.yaw = yaw;
            ship.pitch = pitch.clamp(-1.25, 1.25);
        }

        if ship.planet_locked {
            let mut tangent = forward.sub(normal.mul(forward.dot(normal)));
            if tangent.dot(tangent) < 0.0001 {
                tangent = right;
            }
            tangent = tangent.normalize();

            let target_radius = safe_radius + ORBIT_FLIGHT_ALTITUDE;
            let radial_correction = ((target_radius - distance) * 0.06).clamp(-0.35, 0.35);
            forward = tangent
                .mul((1.0 - radial_correction * radial_correction).sqrt())
                .add(normal.mul(radial_correction))
                .normalize();

            let (yaw, pitch) = direction_to_angles(forward);
            ship.yaw = yaw;
            ship.pitch = pitch.clamp(-1.25, 1.25);
        }

        let proposed = ship.position.add(forward.mul(step_distance));
        let proposed_offset = proposed.sub(planet_center);
        let proposed_distance = proposed_offset.dot(proposed_offset).sqrt();

        ship.position = if proposed_distance < safe_radius {
            let surface_normal = if proposed_distance > 0.001 {
                proposed_offset.mul(1.0 / proposed_distance)
            } else {
                normal
            };
            let sliding = forward
                .sub(surface_normal.mul(forward.dot(surface_normal)))
                .normalize();
            planet_center
                .add(surface_normal.mul(safe_radius))
                .add(sliding.mul(step_distance))
        } else if ship.planet_locked && proposed_distance > ORBIT_LIMIT_RADIUS {
            let boundary_normal = proposed_offset.mul(1.0 / proposed_distance);
            let mut tangent = forward.sub(boundary_normal.mul(forward.dot(boundary_normal)));
            if tangent.dot(tangent) < 0.0001 {
                tangent = right;
            }
            tangent = tangent.normalize();
            let curved_normal = boundary_normal
                .add(tangent.mul(step_distance / ORBIT_LIMIT_RADIUS))
                .normalize();
            let (yaw, pitch) = direction_to_angles(tangent);
            ship.yaw = yaw;
            ship.pitch = pitch.clamp(-1.25, 1.25);
            ship.yaw_rate *= 0.7;
            ship.pitch_rate *= 0.7;
            planet_center.add(curved_normal.mul(ORBIT_LIMIT_RADIUS))
        } else {
            proposed
        };

        for &collider_index in object_grid.candidates(ship.position) {
            let collider = &object_colliders[collider_index];
            let collider_center = rotated_collider_centers[collider_index];
            let separation = collider.radius + ship.spec.collision_radius + 0.8;
            let collider_offset = ship.position.sub(collider_center);
            let collider_distance = collider_offset.dot(collider_offset).sqrt();

            if collider_distance < separation {
                let object_normal = if collider_distance > 0.001 {
                    collider_offset.mul(1.0 / collider_distance)
                } else {
                    normal
                };
                let mut tangent = forward.sub(object_normal.mul(forward.dot(object_normal)));
                if tangent.dot(tangent) < 0.0001 {
                    tangent = right;
                }
                tangent = tangent.normalize();
                ship.position = collider_center
                    .add(object_normal.mul(separation))
                    .add(tangent.mul(step_distance));
                let (yaw, pitch) = direction_to_angles(tangent);
                ship.yaw = yaw;
                ship.pitch = pitch.clamp(-1.25, 1.25);
                forward = tangent;
            }
        }
    }
}

enum TriangleStyle {
    Flat(u32),
    Smooth {
        color: [u8; 3],
        brightness: [f32; 3],
    },
    Procedural {
        shader: SurfaceShader,
        points: [Vec3; 3],
        time: f32,
        brightness: f32,
        fresnel: f32,
    },
}

struct TriangleCommand {
    vertices: [ScreenVertex; 3],
    style: TriangleStyle,
}

struct TriangleBatch {
    commands: Vec<TriangleCommand>,
    viewport_width: usize,
    viewport_height: usize,
}

impl TriangleBatch {
    fn new(viewport_width: usize, viewport_height: usize) -> Self {
        Self {
            commands: Vec::new(),
            viewport_width,
            viewport_height,
        }
    }

    fn resize(&mut self, width: usize, height: usize) {
        debug_assert!(self.commands.is_empty());
        self.viewport_width = width;
        self.viewport_height = height;
    }

    fn flush(&mut self, framebuffer: &mut Framebuffer) {
        self.flush_with_workers(framebuffer, render_worker_count());
    }

    fn flush_with_workers(&mut self, framebuffer: &mut Framebuffer, worker_count: usize) {
        if self.commands.is_empty() {
            return;
        }
        let width = framebuffer.width;
        let height = framebuffer.height;
        debug_assert_eq!((width, height), (self.viewport_width, self.viewport_height));
        let worker_count = worker_count.clamp(1, height.max(1));
        let tile_height = height.div_ceil(worker_count).div_ceil(32) * 32;
        let commands = &self.commands;
        let refractive_background = commands
            .iter()
            .any(|command| {
                matches!(
                    &command.style,
                    TriangleStyle::Procedural {
                        shader: SurfaceShader::Ocean,
                        ..
                    }
                )
            })
            .then(|| framebuffer.pixels.clone());
        let refractive_background = refractive_background.as_deref();
        let (pixels, depth) = framebuffer.buffers_mut();
        thread::scope(|scope| {
            for (tile_index, (tile_pixels, tile_depth)) in pixels
                .chunks_mut(tile_height * width)
                .zip(depth.chunks_mut(tile_height * width))
                .enumerate()
            {
                let region_y = tile_index * tile_height;
                scope.spawn(move || {
                    for command in commands {
                        let [a, b, c] = command.vertices;
                        match &command.style {
                            TriangleStyle::Flat(color) => draw_triangle_shaded_region(
                                tile_pixels,
                                tile_depth,
                                width,
                                height,
                                region_y,
                                a,
                                b,
                                c,
                                |_, _, _, _, _| *color,
                            ),
                            TriangleStyle::Smooth { color, brightness } => {
                                draw_triangle_shaded_region(
                                    tile_pixels,
                                    tile_depth,
                                    width,
                                    height,
                                    region_y,
                                    a,
                                    b,
                                    c,
                                    |w0, w1, w2, _, _| {
                                        pack(
                                            *color,
                                            brightness[0] * w0
                                                + brightness[1] * w1
                                                + brightness[2] * w2,
                                        )
                                    },
                                )
                            }
                            TriangleStyle::Procedural {
                                shader,
                                points,
                                time,
                                brightness,
                                fresnel,
                            } => draw_triangle_shaded_region(
                                tile_pixels,
                                tile_depth,
                                width,
                                height,
                                region_y,
                                a,
                                b,
                                c,
                                |w0, w1, w2, x, y| {
                                    let point = points[0]
                                        .mul(w0)
                                        .add(points[1].mul(w1))
                                        .add(points[2].mul(w2));
                                    let surface = procedural_surface_color(
                                        *shader,
                                        point,
                                        x,
                                        y,
                                        *time,
                                        *brightness,
                                        *fresnel,
                                    );
                                    if *shader == SurfaceShader::Ocean {
                                        if let Some(background) = refractive_background {
                                            refract_water_color(
                                                surface, background, width, height, x, y, point,
                                                *time, *fresnel,
                                            )
                                        } else {
                                            surface
                                        }
                                    } else {
                                        surface
                                    }
                                },
                            ),
                        }
                    }
                });
            }
        });
        self.commands.clear();
    }
}

fn render_model<F>(
    model: &Model,
    framebuffer: &mut Framebuffer,
    camera: &Camera,
    transform: F,
    water_time: Option<f32>,
    effect_spec: Option<&ShipSpec>,
) where
    F: Fn(Vec3) -> Vec3 + Sync,
{
    let mut batch = TriangleBatch::new(framebuffer.width, framebuffer.height);
    queue_model(
        model,
        &mut batch,
        camera,
        transform,
        water_time,
        effect_spec,
    );
    batch.flush(framebuffer);
}

fn queue_model<F>(
    model: &Model,
    batch: &mut TriangleBatch,
    camera: &Camera,
    transform: F,
    water_time: Option<f32>,
    effect_spec: Option<&ShipSpec>,
) where
    F: Fn(Vec3) -> Vec3 + Sync,
{
    let mut camera_vertices = Vec::new();
    queue_model_with_scratch(
        model,
        batch,
        camera,
        transform,
        water_time,
        effect_spec,
        &mut camera_vertices,
    );
}

fn queue_model_with_scratch<F>(
    model: &Model,
    batch: &mut TriangleBatch,
    camera: &Camera,
    transform: F,
    water_time: Option<f32>,
    effect_spec: Option<&ShipSpec>,
    camera_vertices: &mut Vec<Vec3>,
) where
    F: Fn(Vec3) -> Vec3 + Sync,
{
    let tan_half_vertical = (VERTICAL_FOV * 0.5).tan();
    let tan_half_horizontal =
        tan_half_vertical * batch.viewport_width as f32 / batch.viewport_height as f32;
    let (local_center, local_radius) = model.bounds();
    let world_center = transform(local_center);
    let transformed_radius = if local_radius > 0.0 {
        [
            Vec3::new(local_radius, 0.0, 0.0),
            Vec3::new(0.0, local_radius, 0.0),
            Vec3::new(0.0, 0.0, local_radius),
        ]
        .into_iter()
        .map(|axis| transform(local_center.add(axis)).sub(world_center))
        .map(|axis| axis.dot(axis).sqrt())
        .fold(0.0, f32::max)
    } else {
        0.0
    };
    let relative_center = world_center.sub(camera.position);
    let camera_center = Vec3::new(
        relative_center.dot(camera.right),
        relative_center.dot(camera.up),
        relative_center.dot(camera.forward),
    );
    let horizontal_plane_length = (1.0 + tan_half_horizontal * tan_half_horizontal).sqrt();
    let vertical_plane_length = (1.0 + tan_half_vertical * tan_half_vertical).sqrt();
    if camera_center.z + transformed_radius < NEAR_PLANE
        || camera_center.z - transformed_radius > FAR_PLANE
        || camera_center.z * tan_half_horizontal + camera_center.x
            < -transformed_radius * horizontal_plane_length
        || camera_center.z * tan_half_horizontal - camera_center.x
            < -transformed_radius * horizontal_plane_length
        || camera_center.z * tan_half_vertical + camera_center.y
            < -transformed_radius * vertical_plane_length
        || camera_center.z * tan_half_vertical - camera_center.y
            < -transformed_radius * vertical_plane_length
    {
        return;
    }

    let transform_vertex = |vertex: &Vec3| {
        let relative = transform(*vertex).sub(camera.position);
        Vec3::new(
            relative.dot(camera.right),
            relative.dot(camera.up),
            relative.dot(camera.forward),
        )
    };
    let worker_count = render_worker_count();
    if model.vertices.len() >= 4_096 && worker_count > 1 {
        let chunk_size = model.vertices.len().div_ceil(worker_count);
        *camera_vertices = thread::scope(|scope| {
            let handles: Vec<_> = model
                .vertices
                .chunks(chunk_size)
                .map(|vertices| {
                    let transform_vertex = &transform_vertex;
                    scope.spawn(move || vertices.iter().map(transform_vertex).collect::<Vec<_>>())
                })
                .collect();
            handles
                .into_iter()
                .flat_map(|handle| handle.join().expect("vertex worker panicked"))
                .collect()
        });
    } else {
        camera_vertices.clear();
        camera_vertices.reserve(model.vertices.len());
        camera_vertices.extend(model.vertices.iter().map(transform_vertex));
    }
    let camera_normals: Vec<Vec3> = if model.faces.iter().any(|face| face.smooth) {
        model
            .vertex_normals
            .iter()
            .map(|normal| {
                let world_direction = transform(local_center.add(*normal)).sub(world_center);
                Vec3::new(
                    world_direction.dot(camera.right),
                    world_direction.dot(camera.up),
                    world_direction.dot(camera.forward),
                )
                .normalize()
            })
            .collect()
    } else {
        Vec::new()
    };
    let focal = batch.viewport_height as f32 / tan_half_vertical * 0.5;
    let light = Vec3::new(-0.3, 0.8, -0.5).normalize();
    for face in &model.faces {
        let [i0, i1, i2] = face.indices;
        let v0 = camera_vertices[i0];
        let v1 = camera_vertices[i1];
        let v2 = camera_vertices[i2];
        if v0.z <= NEAR_PLANE || v1.z <= NEAR_PLANE || v2.z <= NEAR_PLANE {
            continue;
        }
        if (v0.z > FAR_PLANE && v1.z > FAR_PLANE && v2.z > FAR_PLANE)
            || (v0.x < -v0.z * tan_half_horizontal
                && v1.x < -v1.z * tan_half_horizontal
                && v2.x < -v2.z * tan_half_horizontal)
            || (v0.x > v0.z * tan_half_horizontal
                && v1.x > v1.z * tan_half_horizontal
                && v2.x > v2.z * tan_half_horizontal)
            || (v0.y < -v0.z * tan_half_vertical
                && v1.y < -v1.z * tan_half_vertical
                && v2.y < -v2.z * tan_half_vertical)
            || (v0.y > v0.z * tan_half_vertical
                && v1.y > v1.z * tan_half_vertical
                && v2.y > v2.z * tan_half_vertical)
        {
            continue;
        }
        let project = |vertex: Vec3| ScreenVertex {
            x: batch.viewport_width as f32 * 0.5 + vertex.x * focal / vertex.z,
            y: batch.viewport_height as f32 * 0.5 - vertex.y * focal / vertex.z,
            z: 1.0 / vertex.z,
        };
        let normal = v1.sub(v0).cross(v2.sub(v0)).normalize();
        let brightness = 0.34 + normal.dot(light).abs() * 0.66;
        let a = project(v0);
        let b = project(v1);
        let c = project(v2);
        let shader = face.shader;
        let flat_color = color_for_material(face.color, effect_spec);

        if shader != SurfaceShader::Flat {
            let p0 = model.vertices[i0];
            let p1 = model.vertices[i1];
            let p2 = model.vertices[i2];
            let center = v0.add(v1).add(v2).mul(1.0 / 3.0);
            let view_direction = center.mul(-1.0).normalize();
            let fresnel = (1.0 - normal.dot(view_direction).abs()).powf(3.0);
            let time = water_time.unwrap_or_default();

            batch.commands.push(TriangleCommand {
                vertices: [a, b, c],
                style: TriangleStyle::Procedural {
                    shader,
                    points: [p0, p1, p2],
                    time,
                    brightness,
                    fresnel,
                },
            });
        } else if face.smooth {
            let brightness =
                [i0, i1, i2].map(|index| 0.34 + camera_normals[index].dot(light).abs() * 0.66);
            batch.commands.push(TriangleCommand {
                vertices: [a, b, c],
                style: TriangleStyle::Smooth {
                    color: flat_color,
                    brightness,
                },
            });
        } else {
            batch.commands.push(TriangleCommand {
                vertices: [a, b, c],
                style: TriangleStyle::Flat(pack(flat_color, brightness)),
            });
        }
    }
}

#[cfg(test)]
fn render_model_with_scratch<F>(
    model: &Model,
    framebuffer: &mut Framebuffer,
    camera: &Camera,
    transform: F,
    water_time: Option<f32>,
    effect_spec: Option<&ShipSpec>,
    camera_vertices: &mut Vec<Vec3>,
) where
    F: Fn(Vec3) -> Vec3 + Sync,
{
    let mut batch = TriangleBatch::new(framebuffer.width, framebuffer.height);
    queue_model_with_scratch(
        model,
        &mut batch,
        camera,
        transform,
        water_time,
        effect_spec,
        camera_vertices,
    );
    batch.flush(framebuffer);
}

fn render_scene(
    ship_model: &Model,
    ship_center: Vec3,
    turbo_plume: &Model,
    laser_bolt: &Model,
    projectiles: &[Projectile],
    planet: &Model,
    planet_center: Vec3,
    planet_scale: f32,
    spiky_planet: &Model,
    spiky_planet_center: Vec3,
    spiky_planet_scale: f32,
    sun: &Model,
    framebuffer: &mut Framebuffer,
    batch: &mut TriangleBatch,
    ship: &Ship,
    world_time: f32,
    travel: f32,
    turbo_power: f32,
    fire_charge: f32,
) {
    framebuffer.clear(0);
    draw_skybox(framebuffer, travel);
    let camera = chase_camera(ship);
    debug_assert!(batch.commands.is_empty());
    let sun_position = Vec3::new(SUN_POSITION_X, 20.0, 0.0);
    let planet_position = water_planet_position(world_time);
    let planet_rotation = world_time * 0.25;

    draw_water_planet_atmosphere(
        framebuffer,
        &camera,
        planet_position,
        WATER_PLANET_RADIUS,
        world_time,
    );

    queue_model(
        sun,
        batch,
        &camera,
        |vertex| vertex.add(sun_position),
        None,
        None,
    );
    queue_model(
        planet,
        batch,
        &camera,
        |vertex| {
            let vertex = vertex.sub(planet_center).mul(planet_scale);
            let (sin, cos) = planet_rotation.sin_cos();
            Vec3::new(
                vertex.x * cos - vertex.z * sin,
                vertex.y,
                vertex.x * sin + vertex.z * cos,
            )
            .add(planet_position)
        },
        Some(world_time),
        None,
    );
    let spiky_rotation = world_time * SPIKY_PLANET_ROTATION_SPEED;
    let spiky_position = spiky_planet_position(world_time);
    render_model(
        spiky_planet,
        framebuffer,
        &camera,
        |vertex| {
            let vertex = vertex.sub(spiky_planet_center).mul(spiky_planet_scale);
            let (sin, cos) = spiky_rotation.sin_cos();
            Vec3::new(
                vertex.x * cos - vertex.z * sin,
                vertex.y,
                vertex.x * sin + vertex.z * cos,
            )
            .add(spiky_position)
        },
        None,
        None,
    );

    render_ship_components(
        ship_model,
        ship_center,
        turbo_plume,
        laser_bolt,
        projectiles,
        batch,
        &camera,
        ship,
        world_time,
        turbo_power,
        fire_charge,
        true,
    );
    batch.flush(framebuffer);
}

fn render_ship_components(
    ship_model: &Model,
    ship_center: Vec3,
    turbo_plume: &Model,
    laser_bolt: &Model,
    projectiles: &[Projectile],
    batch: &mut TriangleBatch,
    camera: &Camera,
    ship: &Ship,
    world_time: f32,
    turbo_power: f32,
    fire_charge: f32,
    show_turbo: bool,
) {
    let (forward, right, up) = ship.basis();
    let pulse = 0.9 + (world_time * 28.0).sin() * 0.1;
    let plume_length = (1.3 + turbo_power * 5.5) * pulse;
    let plume_radius = 0.16 + turbo_power * 0.16;
    if show_turbo {
        for side in ship.spec.turbo_sides {
            let emitter = ship
                .position
                .sub(forward.mul(ship.spec.turbo_back))
                .add(right.mul(*side * ship.spec.turbo_side))
                .sub(up.mul(ship.spec.turbo_down));
            queue_model(
                turbo_plume,
                batch,
                &camera,
                |vertex| {
                    emitter
                        .add(forward.mul(vertex.x * plume_length))
                        .add(right.mul(vertex.y * plume_radius))
                        .add(up.mul(vertex.z * plume_radius))
                },
                None,
                Some(ship.spec),
            );
        }
    }
    for projectile in projectiles
        .iter()
        .filter(|projectile| projectile.lifetime > 0.0)
    {
        let mut laser_right = projectile
            .direction
            .cross(Vec3::new(0.0, 1.0, 0.0))
            .normalize();
        if laser_right.dot(laser_right) < 0.001 {
            laser_right = Vec3::new(1.0, 0.0, 0.0);
        }
        let laser_up = laser_right.cross(projectile.direction).normalize();
        queue_model(
            laser_bolt,
            batch,
            &camera,
            |vertex| {
                projectile
                    .position
                    .add(projectile.direction.mul(vertex.x * projectile.scale))
                    .add(laser_up.mul(vertex.y * projectile.scale))
                    .add(laser_right.mul(vertex.z * projectile.scale))
            },
            None,
            Some(ship.spec),
        );
    }
    if fire_charge > 0.0 {
        let charge_scale = 0.35 + fire_charge * 1.15;
        let charge_position = ship.position.add(forward.mul(1.9));
        queue_model(
            laser_bolt,
            batch,
            &camera,
            |vertex| {
                charge_position
                    .add(forward.mul(vertex.x * charge_scale * 0.35))
                    .add(up.mul(vertex.y * charge_scale))
                    .add(right.mul(vertex.z * charge_scale))
            },
            None,
            Some(ship.spec),
        );
    }
    queue_model(
        ship_model,
        batch,
        camera,
        |vertex| {
            let local = vertex.sub(ship_center).mul(ship.spec.model_scale);
            ship.position
                .add(ship.spec.model_to_world(local, forward, right, up))
        },
        None,
        None,
    );
}

fn draw_underwater_background(framebuffer: &mut Framebuffer, travel: f32) {
    let blend = |base: u32, overlay: [u8; 3], amount: f32| {
        let amount = amount.clamp(0.0, 1.0);
        let channel = |shift: u32, value: u8| {
            let current = ((base >> shift) & 255) as f32;
            (current + (value as f32 - current) * amount) as u32
        };
        (channel(16, overlay[0]) << 16) | (channel(8, overlay[1]) << 8) | channel(0, overlay[2])
    };

    // Gradiente de profundidad: superficie luminosa y abismo azul verdoso.
    for y in 0..framebuffer.height {
        let depth = y as f32 / framebuffer.height as f32;
        let surface_glow = (1.0 - depth).powf(3.2);
        let red = (2.0 + surface_glow * 16.0 + depth * 2.0) as u32;
        let green = (20.0 + surface_glow * 72.0 + depth * 10.0) as u32;
        let blue = (43.0 + surface_glow * 92.0 + depth * 18.0) as u32;
        let color = (red << 16) | (green << 8) | blue;
        let row = y * framebuffer.width;
        framebuffer.pixels[row..row + framebuffer.width].fill(color);
    }

    // Rayos volumetricos que bajan desde la superficie.
    for ray in 0..6 {
        let origin = framebuffer.width as f32
            * (0.08 + ray as f32 * 0.18 + (travel * 0.19 + ray as f32).sin() * 0.025);
        let direction = -0.20 + ray as f32 * 0.075;
        for y in 0..(framebuffer.height * 3 / 4) {
            let depth = y as f32 / framebuffer.height as f32;
            let center = origin + direction * y as f32;
            let half_width = 5.0 + depth * (32.0 + ray as f32 * 5.0);
            let left = (center - half_width).max(0.0) as usize;
            let right = (center + half_width).min(framebuffer.width as f32 - 1.0) as usize;
            let strength = (0.12 * (1.0 - depth).powf(1.4)).max(0.0);
            for x in left..=right {
                let edge = 1.0 - ((x as f32 - center).abs() / half_width);
                let pixel = &mut framebuffer.pixels[y * framebuffer.width + x];
                *pixel = blend(*pixel, [70, 180, 205], strength * edge);
            }
        }
    }

    // Siluetas del relieve lejano para cerrar visualmente el horizonte.
    for x in 0..framebuffer.width {
        let phase = x as f32 * 0.012 + travel * 14.0;
        let ridge = framebuffer.height as f32 * 0.70
            + phase.sin() * 24.0
            + (phase * 0.37 + 1.4).sin() * 38.0;
        for y in ridge.max(0.0) as usize..framebuffer.height {
            let depth = (y as f32 - ridge) / (framebuffer.height as f32 - ridge).max(1.0);
            framebuffer.pixels[y * framebuffer.width + x] = blend(
                framebuffer.pixels[y * framebuffer.width + x],
                [4, 31, 43],
                0.48 + depth * 0.30,
            );
        }
    }

    // Ondulaciones brillantes de la superficie vistas desde abajo.
    for band in 0..3 {
        for x in 0..framebuffer.width {
            let wave = (x as f32 * (0.018 + band as f32 * 0.004)
                + travel * (8.0 + band as f32 * 2.0))
                .sin();
            let y = 18 + band * 18 + (wave * 7.0) as i32;
            if y >= 0 && y < framebuffer.height as i32 {
                let index = y as usize * framebuffer.width + x;
                framebuffer.pixels[index] = blend(framebuffer.pixels[index], [105, 222, 232], 0.28);
            }
        }
    }

    // Plancton y particulas suspendidas con paralaje lento.
    for index in 0..150_u32 {
        let seed = hash(index.wrapping_mul(7919));
        let x = (seed as usize % framebuffer.width
            + (travel * (70.0 + (index % 9) as f32 * 11.0)) as usize)
            % framebuffer.width;
        let y = (hash(seed ^ 0x9e3779b9) as usize % framebuffer.height + framebuffer.height
            - ((travel * (14.0 + (index % 7) as f32)) as usize % framebuffer.height))
            % framebuffer.height;
        let color = if index % 5 == 0 { 0x8ce4e8 } else { 0x4ca7b8 };
        framebuffer.pixels[y * framebuffer.width + x] = color;
        if index % 6 == 0 && x + 1 < framebuffer.width {
            framebuffer.pixels[y * framebuffer.width + x + 1] = color;
        }
    }
}

fn apply_underwater_lighting(framebuffer: &mut Framebuffer, time: f32) {
    let width = framebuffer.width as f32;
    let height = framebuffer.height as f32;

    let swaying_center = (time * 0.22).sin() * 0.16;
    let angle_step = 36.0 / width;
    let (step_sin, step_cos) = angle_step.sin_cos();
    let mut edge_darkness = Vec::with_capacity(framebuffer.width);
    let mut normalized_x_values = Vec::with_capacity(framebuffer.width);
    for x in 0..framebuffer.width {
        let normalized_x = x as f32 / width * 2.0 - 1.0;
        normalized_x_values.push(normalized_x);
        edge_darkness.push((1.0 - normalized_x * normalized_x * 0.30).clamp(0.62, 1.0));
    }

    let worker_count = render_worker_count();
    let rows_per_worker = framebuffer.height.div_ceil(worker_count);
    thread::scope(|scope| {
        for (chunk_index, pixels) in framebuffer
            .pixels
            .chunks_mut(rows_per_worker * framebuffer.width)
            .enumerate()
        {
            let normalized_x_values = &normalized_x_values;
            let edge_darkness = &edge_darkness;
            let start_y = chunk_index * rows_per_worker;
            let framebuffer_width = framebuffer.width;
            scope.spawn(move || {
                for (local_y, row) in pixels.chunks_mut(framebuffer_width).enumerate() {
                    let y = start_y + local_y;
                    let depth = y as f32 / height;
                    let surface_light = (1.0 - depth).powf(1.65);
                    let vertical_darkness = 0.50 + surface_light * 0.34;
                    let cone_width = 0.12 + depth * 0.62;
                    let caustic_strength = surface_light * 0.13;
                    let (mut caustic_sin, mut caustic_cos) =
                        (-18.0 + depth * 11.0 + time * 1.7).sin_cos();

                    for (x, pixel) in row.iter_mut().enumerate() {
                        let normalized_x = normalized_x_values[x];
                        let cone = (1.0 - (normalized_x - swaying_center).abs() / cone_width)
                            .clamp(0.0, 1.0)
                            * (1.0 - depth * 0.72);
                        let positive_sine = caustic_sin.max(0.0);
                        let square = positive_sine * positive_sine;
                        let caustic = square * square * positive_sine * caustic_strength;
                        let light = (vertical_darkness * edge_darkness[x] + cone * 0.22 + caustic)
                            .clamp(0.30, 1.0);

                        let red = ((*pixel >> 16) & 255) as f32;
                        let green = ((*pixel >> 8) & 255) as f32;
                        let blue = (*pixel & 255) as f32;
                        let red = (red * light * 0.67).clamp(0.0, 255.0) as u32;
                        let green = (green * light * 0.91 + cone * 5.0).clamp(0.0, 255.0) as u32;
                        let blue = (blue * light + surface_light * 4.0 + cone * 8.0)
                            .clamp(0.0, 255.0) as u32;
                        *pixel = (red << 16) | (green << 8) | blue;

                        let next_sin = caustic_sin * step_cos + caustic_cos * step_sin;
                        caustic_cos = caustic_cos * step_cos - caustic_sin * step_sin;
                        caustic_sin = next_sin;
                    }
                }
            });
        }
    });
}

fn render_water_enemies(
    enemy_models: &[Model; 3],
    enemy_centers: &[Vec3; 3],
    enemies: &[WaterEnemy],
    elapsed: f32,
    current_progress: f32,
    batch: &mut TriangleBatch,
    camera: &Camera,
    instance_buffers: &mut [Vec<Vec3>; 3],
) {
    let up = Vec3::new(0.0, 1.0, 0.0);
    for kind in 0..enemy_models.len() {
        let model = &enemy_models[kind];
        let center = enemy_centers[kind];
        let camera_vertices = &mut instance_buffers[kind];
        for enemy in enemies.iter().filter(|enemy| {
            enemy.alive && enemy.kind == kind && (enemy.progress - current_progress).abs() < 150.0
        }) {
            let position = enemy.position(elapsed);
            let angle = match kind {
                0 => elapsed * 0.55 + enemy.phase,
                1 => (elapsed * 1.15 + enemy.phase).cos() * 0.18,
                _ => (elapsed * 0.45 + enemy.phase).sin() * 0.12,
            };
            let forward = Vec3::new(angle.cos(), 0.0, angle.sin());
            let right = Vec3::new(-angle.sin(), 0.0, angle.cos());
            queue_model_with_scratch(
                model,
                batch,
                camera,
                |vertex| {
                    let local = vertex.sub(center).mul(WATER_ENEMY_SCALE);
                    position
                        .add(forward.mul(local.x))
                        .add(right.mul(local.y))
                        .add(up.mul(local.z))
                },
                None,
                None,
                camera_vertices,
            );
        }
    }
}

fn render_water_level(
    level: &LevelSpatialIndex,
    enemy_models: &[Model; 3],
    enemy_centers: &[Vec3; 3],
    enemies: &[WaterEnemy],
    enemy_elapsed: f32,
    enemy_instance_buffers: &mut [Vec<Vec3>; 3],
    bubble_model: &Model,
    bubbles: &[BubbleParticle],
    ship_model: &Model,
    ship_center: Vec3,
    turbo_plume: &Model,
    laser_bolt: &Model,
    projectiles: &[Projectile],
    framebuffer: &mut Framebuffer,
    batch: &mut TriangleBatch,
    ship: &Ship,
    world_time: f32,
    progress: f32,
    turbo_power: f32,
    fire_charge: f32,
) {
    framebuffer.clear(0);
    draw_underwater_background(framebuffer, progress * 0.002);
    let camera = chase_camera(ship);
    debug_assert!(batch.commands.is_empty());
    for chunk in level.visible_chunks(progress) {
        queue_model(
            chunk,
            batch,
            &camera,
            level_to_world,
            Some(world_time),
            None,
        );
    }
    render_water_enemies(
        enemy_models,
        enemy_centers,
        enemies,
        enemy_elapsed,
        progress,
        batch,
        &camera,
        enemy_instance_buffers,
    );
    for bubble in bubbles.iter().filter(|bubble| bubble.lifetime > 0.0) {
        queue_model(
            bubble_model,
            batch,
            &camera,
            |vertex| bubble.position.add(vertex.mul(bubble.scale)),
            None,
            None,
        );
    }
    render_ship_components(
        ship_model,
        ship_center,
        turbo_plume,
        laser_bolt,
        projectiles,
        batch,
        &camera,
        ship,
        world_time,
        turbo_power,
        fire_charge,
        false,
    );
    batch.flush(framebuffer);
    apply_underwater_lighting(framebuffer, world_time);
}

fn draw_selection_panel(
    framebuffer: &mut Framebuffer,
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
    selected: bool,
) {
    let fill = if selected { 0x182b46 } else { 0x101722 };
    let border = if selected { 0xffd447 } else { 0x43536a };
    for y in top..bottom {
        for x in left..right {
            let edge = x < left + 5 || x + 5 >= right || y < top + 5 || y + 5 >= bottom;
            framebuffer.pixels[y * WIDTH + x] = if edge { border } else { fill };
        }
    }
}

fn draw_selection_label(
    framebuffer: &mut Framebuffer,
    text: &str,
    center_x: i32,
    top_y: i32,
    scale: i32,
    color: u32,
) {
    let width = text.len() as i32 * 6 * scale - scale;
    let start_x = center_x - width / 2;
    for (character_index, character) in text.chars().enumerate() {
        let rows = match character {
            'A' => [14, 17, 17, 31, 17, 17, 17],
            'B' => [30, 17, 17, 30, 17, 17, 30],
            'C' => [14, 17, 16, 16, 16, 17, 14],
            'D' => [30, 17, 17, 17, 17, 17, 30],
            'E' => [31, 16, 16, 30, 16, 16, 31],
            'F' => [31, 16, 16, 30, 16, 16, 16],
            'G' => [14, 17, 16, 23, 17, 17, 15],
            'H' => [17, 17, 17, 31, 17, 17, 17],
            'I' => [14, 4, 4, 4, 4, 4, 14],
            'J' => [7, 2, 2, 2, 18, 18, 12],
            'K' => [17, 18, 20, 24, 20, 18, 17],
            'L' => [16, 16, 16, 16, 16, 16, 31],
            'M' => [17, 27, 21, 21, 17, 17, 17],
            'N' => [17, 25, 25, 21, 19, 19, 17],
            'O' => [14, 17, 17, 17, 17, 17, 14],
            'P' => [30, 17, 17, 30, 16, 16, 16],
            'Q' => [14, 17, 17, 17, 21, 18, 13],
            'R' => [30, 17, 17, 30, 20, 18, 17],
            'S' => [15, 16, 16, 14, 1, 1, 30],
            'T' => [31, 4, 4, 4, 4, 4, 4],
            'U' => [17, 17, 17, 17, 17, 17, 14],
            'V' => [17, 17, 17, 17, 17, 10, 4],
            'W' => [17, 17, 17, 21, 21, 21, 10],
            'X' => [17, 17, 10, 4, 10, 17, 17],
            'Y' => [17, 17, 10, 4, 4, 4, 4],
            'Z' => [31, 1, 2, 4, 8, 16, 31],
            '0' => [14, 17, 19, 21, 25, 17, 14],
            '1' => [4, 12, 4, 4, 4, 4, 14],
            '2' => [14, 17, 1, 2, 4, 8, 31],
            '3' => [30, 1, 1, 14, 1, 1, 30],
            '4' => [2, 6, 10, 18, 31, 2, 2],
            '5' => [31, 16, 16, 30, 1, 1, 30],
            '6' => [14, 16, 16, 30, 17, 17, 14],
            '7' => [31, 1, 2, 4, 8, 8, 8],
            '8' => [14, 17, 17, 14, 17, 17, 14],
            '9' => [14, 17, 17, 15, 1, 1, 14],
            ':' => [0, 4, 4, 0, 4, 4, 0],
            '/' => [1, 1, 2, 4, 8, 16, 16],
            '-' => [0, 0, 0, 31, 0, 0, 0],
            _ => [0; 7],
        };
        let character_x = start_x + character_index as i32 * 6 * scale;
        for (row, bits) in rows.iter().enumerate() {
            for column in 0..5 {
                if bits & (1 << (4 - column)) == 0 {
                    continue;
                }
                for dy in 0..scale {
                    for dx in 0..scale {
                        let x = character_x + column * scale + dx;
                        let y = top_y + row as i32 * scale + dy;
                        if x >= 0 && y >= 0 && x < WIDTH as i32 && y < HEIGHT as i32 {
                            framebuffer.pixels[y as usize * WIDTH + x as usize] = color;
                        }
                    }
                }
            }
        }
    }
}

fn hangar_thumbnail(
    model: &Model,
    spec: &ShipSpec,
    framebuffer: &mut Framebuffer,
    camera: &Camera,
    screen_x: usize,
    screen_y: usize,
    angle: f32,
) -> Vec<u32> {
    const CARD_WIDTH: usize = 180;
    const CARD_HEIGHT: usize = 220;
    const TRANSPARENT: u32 = 0x0100_0000;
    framebuffer.clear(TRANSPARENT);
    let center = model_center(model);
    let depth = 25.0;
    let focal = HEIGHT as f32 / (60.0_f32.to_radians() * 0.5).tan() * 0.5;
    let target = camera
        .position
        .add(camera.forward.mul(depth))
        .add(
            camera
                .right
                .mul((screen_x as f32 - WIDTH as f32 * 0.5) * depth / focal),
        )
        .add(
            camera
                .up
                .mul((HEIGHT as f32 * 0.5 - screen_y as f32) * depth / focal),
        );
    let heading = Vec3::new(angle.sin(), 0.0, -angle.cos());
    let side = Vec3::new(angle.cos(), 0.0, angle.sin());
    let scale = 2.1 / model_radius(model, center).max(0.001);
    render_model(
        model,
        framebuffer,
        camera,
        |vertex| {
            target.add(spec.model_to_world(
                vertex.sub(center).mul(scale),
                heading,
                side,
                Vec3::new(0.0, 1.0, 0.0),
            ))
        },
        None,
        None,
    );
    let left = screen_x - CARD_WIDTH / 2;
    let top = screen_y - CARD_HEIGHT / 2;
    let mut thumbnail = vec![TRANSPARENT; CARD_WIDTH * CARD_HEIGHT];
    for y in 0..CARD_HEIGHT {
        let source = (top + y) * WIDTH + left;
        let target = y * CARD_WIDTH;
        thumbnail[target..target + CARD_WIDTH]
            .copy_from_slice(&framebuffer.pixels[source..source + CARD_WIDTH]);
    }
    thumbnail
}

fn draw_hangar_card(
    framebuffer: &mut Framebuffer,
    left: usize,
    top: usize,
    width: usize,
    height: usize,
    selected: bool,
) {
    let tint = if selected { 0x071a25 } else { 0x06111c };
    let alpha = if selected { 120 } else { 145 };
    for y in top..top + height {
        for x in left..left + width {
            let index = y * WIDTH + x;
            let pixel = framebuffer.pixels[index];
            let blend = |shift: u32| {
                let foreground = (tint >> shift) & 0xff;
                let background = (pixel >> shift) & 0xff;
                (foreground * alpha + background * (255 - alpha)) / 255
            };
            framebuffer.pixels[index] = (blend(16) << 16) | (blend(8) << 8) | blend(0);
        }
    }
    let border = if selected { 0x42dff0 } else { 0x35658c };
    let thickness = if selected { 3 } else { 2 };
    for inset in 0..thickness {
        for x in left + inset..left + width - inset {
            framebuffer.pixels[(top + inset) * WIDTH + x] = border;
            framebuffer.pixels[(top + height - 1 - inset) * WIDTH + x] = border;
        }
        for y in top + inset..top + height - inset {
            framebuffer.pixels[y * WIDTH + left + inset] = border;
            framebuffer.pixels[y * WIDTH + left + width - 1 - inset] = border;
        }
    }
}

fn select_ship(
    window: &mut Window,
    framebuffer: &mut Framebuffer,
    choices: &[(&Model, &ShipSpec)],
    ship_forms: &[Vec<(&Model, &ShipSpec, &str)>],
    initial_selection: usize,
) -> Result<Option<(usize, usize)>, minifb::Error> {
    crate::input::clear();
    window.update();
    let mut selected = initial_selection % choices.len();
    let mut variants = vec![0; choices.len()];
    let start = Instant::now();
    let forward = Vec3::new(0.0, -0.13, 1.0).normalize();
    let right = forward.cross(Vec3::new(0.0, 1.0, 0.0)).normalize();
    let camera = Camera {
        position: Vec3::new(0.0, 2.0, -15.0),
        forward,
        right,
        up: right.cross(forward).normalize(),
    };
    let card_positions = [
        (48, 145),
        (238, 145),
        (428, 145),
        (48, 385),
        (238, 385),
        (428, 385),
    ];
    let mut thumbnails = Vec::with_capacity(choices.len());
    for (index, forms) in ship_forms.iter().enumerate() {
        let (model, spec, _) = forms[0];
        let (left, top) = card_positions[index];
        thumbnails.push(hangar_thumbnail(
            model,
            spec,
            framebuffer,
            &camera,
            left + 90,
            top + 105,
            0.36,
        ));
    }
    while window.is_open() {
        if crate::input::pressed(Key::Escape) {
            return Ok(None);
        }
        if crate::input::pressed(Key::Left) || crate::input::pressed(Key::A) {
            selected = (selected + choices.len() - 1) % choices.len();
        }
        if crate::input::pressed(Key::Right) || crate::input::pressed(Key::D) {
            selected = (selected + 1) % choices.len();
        }
        if crate::input::pressed(Key::V) {
            variants[selected] = (variants[selected] + 1) % ship_forms[selected].len();
            let (model, spec, _) = ship_forms[selected][variants[selected]];
            let (left, top) = card_positions[selected];
            thumbnails[selected] = hangar_thumbnail(
                model,
                spec,
                framebuffer,
                &camera,
                left + 90,
                top + 105,
                0.36,
            );
        }
        if crate::input::pressed(Key::Enter) {
            return Ok(Some((selected, variants[selected])));
        }
        window.set_title("Hangar | A/D: nave | V: cambiar forma | Enter: jugar | Esc: menu");
        menu::draw_backdrop(framebuffer);
        draw_selection_label(framebuffer, "SELECCIONA", 194, 34, 4, 0xe9f2ff);
        draw_selection_label(framebuffer, "TU NAVE", 438, 34, 4, 0x42dff0);
        for x in 48..626 {
            framebuffer.pixels[92 * WIDTH + x] = if x < 610 { 0x1389bd } else { 0x42dff0 };
        }
        draw_selection_label(framebuffer, "SEIS NAVES DISPONIBLES", 235, 108, 2, 0x91a9bd);
        for (index, ((_, spec), (left, top))) in choices.iter().zip(card_positions).enumerate() {
            draw_hangar_card(framebuffer, left, top, 180, 220, index == selected);
            let thumbnail = &thumbnails[index];
            for y in 0..220 {
                for x in 0..180 {
                    let pixel = thumbnail[y * 180 + x];
                    if pixel != 0x0100_0000 {
                        framebuffer.pixels[(top + y) * WIDTH + left + x] = pixel;
                    }
                }
            }
            draw_selection_label(
                framebuffer,
                &spec.name.to_ascii_uppercase(),
                left as i32 + 90,
                top as i32 + 185,
                2,
                if index == selected {
                    0xf0f3fa
                } else {
                    0xa9bbd2
                },
            );
        }
        draw_hangar_card(framebuffer, 638, 135, 540, 520, true);
        let (model, spec, form_label) = ship_forms[selected][variants[selected]];
        let center = model_center(model);
        let angle = start.elapsed().as_secs_f32() * 0.50;
        let heading = Vec3::new(angle.sin(), 0.0, -angle.cos());
        let side = Vec3::new(angle.cos(), 0.0, angle.sin());
        let depth = 18.0;
        let focal = HEIGHT as f32 / (60.0_f32.to_radians() * 0.5).tan() * 0.5;
        let target = camera
            .position
            .add(camera.forward.mul(depth))
            .add(
                camera
                    .right
                    .mul((900.0 - WIDTH as f32 * 0.5) * depth / focal),
            )
            .add(camera.up.mul((HEIGHT as f32 * 0.5 - 360.0) * depth / focal));
        let scale = 4.2 / model_radius(model, center).max(0.001);
        render_model(
            model,
            framebuffer,
            &camera,
            |v| {
                target.add(spec.model_to_world(
                    v.sub(center).mul(scale),
                    heading,
                    side,
                    Vec3::new(0.0, 1.0, 0.0),
                ))
            },
            None,
            None,
        );
        draw_selection_label(
            framebuffer,
            &spec.name.to_ascii_uppercase(),
            908,
            570,
            4,
            0xffd329,
        );
        draw_selection_label(framebuffer, form_label, 908, 622, 2, 0x42dff0);
        draw_selection_panel(framebuffer, 0, 690, WIDTH, 800, false);
        draw_selection_label(framebuffer, "A / D: SELECCIONAR", 150, 724, 2, 0xc9d9ea);
        draw_selection_label(framebuffer, "V: CAMBIAR FORMA", 430, 724, 2, 0xc9d9ea);
        draw_selection_label(framebuffer, "ENTER: CONFIRMAR", 740, 724, 2, 0xc9d9ea);
        draw_selection_label(framebuffer, "ESC: VOLVER", 1050, 724, 2, 0xc9d9ea);
        crate::input::clear();
        window.update_with_buffer(&framebuffer.pixels, WIDTH, HEIGHT)?;
    }
    Ok(None)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    while run_game()? {}
    Ok(())
}

fn run_game() -> Result<bool, Box<dyn std::error::Error>> {
    let arwing_path = "assets/vehicles/arwing/arwing2.obj";
    let kirby_path = "assets/vehicles/kirby/kirby_detallado.obj";
    let astra_path = "assets/vehicles/Astra.obj";
    let astra_car_path = "assets/vehicles/astra/astra_carro_detallado.obj";
    let cowboyb_path = "assets/vehicles/Swordfish II.obj";
    let cowboyb_car_path = "assets/vehicles/cowboy/cb_carro_detallado.obj";
    let astra_submarine_path = "assets/vehicles/astra/astra_submarino_detallado.obj";
    let cowboyb_submarine_path = "assets/vehicles/cowboy/cb_submarino_detallado.obj";
    let arwing_submarine_path = "assets/vehicles/arwing/arwing2_submarino.obj";
    let kirby_submarine_path = "assets/vehicles/kirby/kirbysub_detallado.obj";
    let arwing_model =
        load_obj(arwing_path).map_err(|error| format!("Error al cargar {arwing_path}: {error}"))?;
    let kirby_model =
        load_obj(kirby_path).map_err(|error| format!("Error al cargar {kirby_path}: {error}"))?;
    let astra_model =
        load_obj(astra_path).map_err(|error| format!("Error al cargar {astra_path}: {error}"))?;
    let astra_car_model = load_obj(astra_car_path)
        .map_err(|error| format!("Error al cargar {astra_car_path}: {error}"))?;
    let cowboyb_model = load_obj(cowboyb_path)
        .map_err(|error| format!("Error al cargar {cowboyb_path}: {error}"))?;
    let mut cowboyb_car_model = load_obj(cowboyb_car_path)
        .map_err(|error| format!("Error al cargar {cowboyb_car_path}: {error}"))?;
    move_swordfish_wheels_outward(&mut cowboyb_car_model);
    let astra_submarine_model = load_obj(astra_submarine_path)
        .map_err(|error| format!("Error al cargar {astra_submarine_path}: {error}"))?;
    let cowboyb_submarine_model = load_obj(cowboyb_submarine_path)
        .map_err(|error| format!("Error al cargar {cowboyb_submarine_path}: {error}"))?;
    let arwing_submarine_model = load_obj(arwing_submarine_path)
        .map_err(|error| format!("Error al cargar {arwing_submarine_path}: {error}"))?;
    let kirby_submarine_model = load_obj(kirby_submarine_path)
        .map_err(|error| format!("Error al cargar {kirby_submarine_path}: {error}"))?;

    let mut fullscreen = false;
    let mut window = create_window(fullscreen)?;
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
    let buzz_variants = [
        load_obj("assets/vehicles/buzz/buzz_nave_detallada.obj")?,
        load_obj("assets/vehicles/buzz/buzz_carro_detallado.obj")?,
        load_obj("assets/vehicles/buzz/buzz_submarino_detallado.obj")?,
    ];
    let mcqueen_model = load_obj("assets/vehicles/mcqueen/mcqueen_nave_detallada.obj")?;
    let mcqueen_car_model = load_obj("assets/vehicles/mcqueen/mcqueen_carro_detallado.obj")?;
    let mcqueen_submarine_model =
        load_obj("assets/vehicles/mcqueen/mcqueen_submarino_detallado.obj")?;
    let mcqueen_car_center = model_center(&mcqueen_car_model);
    let choices = [
        (&arwing_model, &ARWING_SPEC),
        (&kirby_model, &KIRBY_SPEC),
        (&astra_model, &ASTRA_SPEC),
        (&cowboyb_model, &COWBOYB_SPEC),
        (&buzz_variants[0], &BUZZ_SPEC),
        (&mcqueen_model, &MCQUEEN_SPEC),
    ];
    let ship_forms = [
        vec![
            (&arwing_model, &ARWING_SPEC, "MODO ESPACIAL"),
            (&arwing_submarine_model, &ARWING_SUBMARINE_SPEC, "SUBMARINO"),
        ],
        vec![
            (&kirby_model, &KIRBY_SPEC, "MODO ESPACIAL"),
            (&kirby_submarine_model, &KIRBY_SUBMARINE_SPEC, "SUBMARINO"),
        ],
        vec![
            (&astra_model, &ASTRA_SPEC, "MODO ESPACIAL"),
            (&astra_car_model, &ASTRA_CAR_SPEC, "MODO CARRO"),
            (&astra_submarine_model, &ASTRA_SUBMARINE_SPEC, "SUBMARINO"),
        ],
        vec![
            (&cowboyb_model, &COWBOYB_SPEC, "MODO ESPACIAL"),
            (&cowboyb_car_model, &COWBOYB_CAR_SPEC, "MODO CARRO"),
            (
                &cowboyb_submarine_model,
                &COWBOYB_SUBMARINE_SPEC,
                "SUBMARINO",
            ),
        ],
        vec![
            (&buzz_variants[0], &BUZZ_SPEC, "MODO ESPACIAL"),
            (&buzz_variants[1], &BUZZ_CAR_SPEC, "MODO CARRO"),
            (&buzz_variants[2], &BUZZ_SUBMARINE_SPEC, "SUBMARINO"),
        ],
        vec![
            (&mcqueen_model, &MCQUEEN_SPEC, "MODO ESPACIAL"),
            (&mcqueen_car_model, &MCQUEEN_CAR_SPEC, "MODO CARRO"),
            (
                &mcqueen_submarine_model,
                &MCQUEEN_SUBMARINE_SPEC,
                "SUBMARINO",
            ),
        ],
    ];
    let (mut selected_ship, initial_form) = loop {
        if menu::show(&mut window, &mut framebuffer, false)? == menu::Action::Quit {
            return Ok(false);
        }
        if let Some(index) = select_ship(&mut window, &mut framebuffer, &choices, &ship_forms, 0)? {
            break index;
        }
        if !window.is_open() {
            return Ok(false);
        }
    };
    let (mut ship_model, mut ship_spec) = choices[selected_ship];
    if selected_ship == 5 && initial_form == 1 {
        ship_spec = &MCQUEEN_CAR_SPEC;
    }
    let mut base_center = model_center(ship_model);
    let buzz_centers = [
        model_center(&buzz_variants[0]),
        model_center(&buzz_variants[1]),
        model_center(&buzz_variants[2]),
    ];
    let mut car_mode = selected_ship == 5 && initial_form == 1;
    let (mut level_ship_model, mut level_ship_spec) = match selected_ship {
        0 => (&arwing_submarine_model, &ARWING_SUBMARINE_SPEC),
        1 => (&kirby_submarine_model, &KIRBY_SUBMARINE_SPEC),
        2 => (&astra_submarine_model, &ASTRA_SUBMARINE_SPEC),
        3 => (&cowboyb_submarine_model, &COWBOYB_SUBMARINE_SPEC),
        4 => (&buzz_variants[2], &BUZZ_SUBMARINE_SPEC),
        5 => (&mcqueen_submarine_model, &MCQUEEN_SUBMARINE_SPEC),
        _ => unreachable!(),
    };
    let mut level_ship_center = model_center(level_ship_model);
    window.set_title("Sistema solar | Vuelo libre 3D | F11: fullscreen");

    let planet_path = "assets/planets/water_planet.obj";
    let planet =
        load_obj(planet_path).map_err(|error| format!("Error al cargar {planet_path}: {error}"))?;
    let spiky_planet_path = "assets/planets/planeta_espinas.obj";
    let spiky_planet = load_obj(spiky_planet_path)
        .map_err(|error| format!("Error al cargar {spiky_planet_path}: {error}"))?;
    let water_level_path = "assets/levels/waterlevel_extendido.obj";
    let mut water_level = load_obj(water_level_path)
        .map_err(|error| format!("Error al cargar {water_level_path}: {error}"))?;
    water_level
        .faces
        .retain(|face| !face.group.starts_with("CAVES_AND_ARCHES"));
    stretch_model_group_height(&mut water_level, "TERRAIN_WALLS", -5.0, 3.6);
    let enemy_path = "assets/enemies/acuaticenemies.obj";
    let enemy_source =
        load_obj(enemy_path).map_err(|error| format!("Error al cargar {enemy_path}: {error}"))?;
    let enemy_models = [
        extract_model_group(&enemy_source, "ENEMY_PUFFER_MINE")?,
        extract_model_group(&enemy_source, "ENEMY_SHARK_DRONE")?,
        extract_model_group(&enemy_source, "ENEMY_JELLY_BOT")?,
    ];
    let enemy_centers = [
        model_center(&enemy_models[0]),
        model_center(&enemy_models[1]),
        model_center(&enemy_models[2]),
    ];
    let mut enemy_instance_buffers: [Vec<Vec3>; 3] =
        std::array::from_fn(|index| Vec::with_capacity(enemy_models[index].vertices.len()));
    let mut scene_framebuffer = Framebuffer::new(WIDTH, HEIGHT);
    let mut triangle_batch = TriangleBatch::new(WIDTH, HEIGHT);
    let mut dynamic_resolution = DynamicResolution::new();
    let level_colliders = build_level_colliders(&water_level);
    let water_level_spatial = LevelSpatialIndex::new(&water_level, level_colliders);
    drop(water_level);
    let planet_center = model_center(&planet);
    let planet_scale = WATER_PLANET_RADIUS / model_radius(&planet, planet_center).max(0.001);
    let spiky_planet_center = model_center(&spiky_planet);
    let spiky_planet_scale =
        SPIKY_PLANET_RADIUS / model_radius(&spiky_planet, spiky_planet_center).max(0.001);
    let planet_surface_radius = material_radius(&planet, planet_center, "Oceano") * planet_scale;
    let object_colliders = build_object_colliders(&planet, planet_center, planet_scale);
    let sun = make_sphere(55.0, 18, 28, true);
    let turbo_plume = make_turbo_plume(12);
    let laser_bolt = make_laser_bolt();
    let bubble_model = make_bubble_model();

    println!("Vuelo libre 3D iniciado: {}", ship_spec.name);
    println!("W/S o flechas: subir/bajar | Espacio: acelerar | Ctrl: frenar");
    println!("A/D: direccion | Q/E: alabeo | Enter: disparar/cargar | R: reiniciar | P: captura");
    println!("1: warp al planeta acuatico | Shift: velocidad de luz");
    println!("Backspace: abandonar la zona orbital");
    println!("Enter cerca del planeta: iniciar nivel submarino");
    println!(
        "Colisionadores alrededor del planeta: {}",
        object_colliders.len()
    );
    println!(
        "Nivel submarino dividido en {} sectores espaciales",
        water_level_spatial.render_chunks.len()
    );

    let start_underwater = selected_ship == 5 && initial_form == 2;
    let mut ship_controller = ShipController::new(if start_underwater {
        level_ship_spec
    } else {
        ship_spec
    });
    let mut game_mode = if start_underwater {
        GameMode::WaterLevel
    } else {
        GameMode::FreeWorld
    };
    if start_underwater {
        window.set_title("Nivel submarino | Backspace: volver al sistema solar");
    }
    let mut water_level_state = WaterLevelState::new();
    let mut bubble_emitter = BubbleEmitter::new();
    let mut guardian_fight = CoreGuardian::new();
    let mut previous = Instant::now();
    let mut world_time = 0.0_f32;
    let mut distance = 0.0_f32;

    while window.is_open() {
        if crate::input::pressed(Key::Escape) {
            match menu::show(&mut window, &mut framebuffer, true)? {
                menu::Action::Quit => return Ok(false),
                menu::Action::Hangar => {
                    if let Some((new_selection, new_form)) = select_ship(
                        &mut window,
                        &mut framebuffer,
                        &choices,
                        &ship_forms,
                        selected_ship,
                    )? {
                        selected_ship = new_selection;
                        (ship_model, ship_spec) = choices[selected_ship];
                        if selected_ship == 5 && new_form == 1 {
                            ship_spec = &MCQUEEN_CAR_SPEC;
                        }
                        base_center = model_center(ship_model);
                        (level_ship_model, level_ship_spec) = match selected_ship {
                            0 => (&arwing_submarine_model, &ARWING_SUBMARINE_SPEC),
                            1 => (&kirby_submarine_model, &KIRBY_SUBMARINE_SPEC),
                            2 => (&astra_submarine_model, &ASTRA_SUBMARINE_SPEC),
                            3 => (&cowboyb_submarine_model, &COWBOYB_SUBMARINE_SPEC),
                            4 => (&buzz_variants[2], &BUZZ_SUBMARINE_SPEC),
                            5 => (&mcqueen_submarine_model, &MCQUEEN_SUBMARINE_SPEC),
                            _ => unreachable!(),
                        };
                        level_ship_center = model_center(level_ship_model);
                        let start_underwater = selected_ship == 5 && new_form == 2;
                        ship_controller = ShipController::new(if start_underwater {
                            level_ship_spec
                        } else {
                            ship_spec
                        });
                        game_mode = if start_underwater {
                            GameMode::WaterLevel
                        } else {
                            GameMode::FreeWorld
                        };
                        if start_underwater {
                            window
                                .set_title("Nivel submarino | Backspace: volver al sistema solar");
                        }
                        water_level_state = WaterLevelState::new();
                        bubble_emitter.clear();
                        car_mode = selected_ship == 5 && new_form == 1;
                        distance = 0.0;
                    }
                }
                menu::Action::Play => {}
            }
            previous = Instant::now();
            continue;
        }
        if crate::input::pressed(Key::F1) {
            menu::instructions(&mut window, &mut framebuffer)?;
            previous = Instant::now();
            continue;
        }
        if matches!(selected_ship, 4 | 5)
            && game_mode == GameMode::FreeWorld
            && crate::input::pressed(Key::T)
        {
            car_mode = !car_mode;
            if selected_ship == 5 {
                ship_spec = if car_mode {
                    &MCQUEEN_CAR_SPEC
                } else {
                    &MCQUEEN_SPEC
                };
                ship_controller.ship.spec = ship_spec;
            }
        }
        let now = Instant::now();
        let frame_time = now.duration_since(previous).as_secs_f32();
        let delta_time = frame_time.min(0.05);
        previous = now;
        world_time += delta_time;

        let resolution_changed = dynamic_resolution.update(frame_time);
        let toggled_resolution = if crate::input::pressed(Key::F2) {
            dynamic_resolution.toggle()
        } else {
            false
        };
        if resolution_changed || toggled_resolution {
            let (render_width, render_height) = dynamic_resolution.dimensions();
            scene_framebuffer = Framebuffer::new(render_width, render_height);
            triangle_batch.resize(render_width, render_height);
        }

        if crate::input::pressed(Key::F11) {
            fullscreen = !fullscreen;
            window = create_window(fullscreen)?;
        }
        if crate::input::pressed(Key::R) {
            if game_mode == GameMode::CoreGuardian {
                guardian_fight = CoreGuardian::new();
            } else {
                ship_controller.reset();
                if game_mode == GameMode::WaterLevel {
                    water_level_state = WaterLevelState::new();
                    bubble_emitter.clear();
                }
            }
            distance = 0.0;
        }
        if game_mode == GameMode::FreeWorld && crate::input::pressed(Key::Key1) {
            place_ship_near_water_planet(&mut ship_controller, world_time);
            println!("Warp completado: planeta acuatico");
        }
        if game_mode == GameMode::FreeWorld && crate::input::pressed(Key::Key2) {
            place_ship_near_spiky_planet(&mut ship_controller, world_time);
            println!("Warp completado: planeta rosado | Enter: Guardián del Núcleo");
        }

        let pitch_input = (window.is_key_down(Key::W) || window.is_key_down(Key::Up)) as i8
            - (window.is_key_down(Key::S) || window.is_key_down(Key::Down)) as i8;
        let yaw_input = window.is_key_down(Key::D) as i8 - window.is_key_down(Key::A) as i8;
        let roll_input = window.is_key_down(Key::E) as i8 - window.is_key_down(Key::Q) as i8;
        let active_yaw_input = if game_mode == GameMode::WaterLevel {
            -yaw_input
        } else {
            yaw_input
        };
        let mut entered_level = false;
        if game_mode == GameMode::FreeWorld
            && ship_controller.ship.planet_locked
            && crate::input::pressed(Key::Enter)
        {
            if ship_controller.ship.spiky_planet_locked {
                guardian_fight = CoreGuardian::new();
                game_mode = GameMode::CoreGuardian;
                entered_level = true;
                window.set_title("Guardian del Nucleo | Space: disparar | Backspace: salir");
                println!("Minijuego Guardián del Núcleo iniciado");
            } else {
                ship_controller = ShipController::new(level_ship_spec);
                water_level_state = WaterLevelState::new();
                bubble_emitter.clear();
                game_mode = GameMode::WaterLevel;
                entered_level = true;
                window.set_title("Nivel submarino | Backspace: volver al sistema solar");
                println!("Nivel submarino iniciado");
            }
        } else if game_mode == GameMode::WaterLevel && crate::input::pressed(Key::Backspace) {
            game_mode = GameMode::FreeWorld;
            bubble_emitter.clear();
            ship_controller = ShipController::new(ship_spec);
            place_ship_near_water_planet(&mut ship_controller, world_time);
            window.set_title("Sistema solar | Vuelo libre 3D | F11: fullscreen");
            println!("Regreso al sistema solar");
        }

        let input = ShipInput {
            pitch: pitch_input,
            yaw: active_yaw_input,
            roll: roll_input,
            accelerating: window.is_key_down(Key::Space),
            braking: window.is_key_down(Key::LeftCtrl) || window.is_key_down(Key::RightCtrl),
            firing: window.is_key_down(Key::Enter) && !entered_level,
        };

        let active_model = if selected_ship == 4 {
            &buzz_variants[if game_mode == GameMode::WaterLevel {
                2
            } else {
                usize::from(car_mode)
            }]
        } else if selected_ship == 5 && car_mode {
            &mcqueen_car_model
        } else {
            ship_model
        };
        let ship_center = if selected_ship == 4 {
            buzz_centers[if game_mode == GameMode::WaterLevel {
                2
            } else {
                usize::from(car_mode)
            }]
        } else if selected_ship == 5 && car_mode {
            mcqueen_car_center
        } else {
            base_center
        };
        match game_mode {
            GameMode::FreeWorld => {
                ship_controller.update(&input, delta_time);
                let light_speed =
                    window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift);
                let active_speed = if light_speed {
                    LIGHT_SPEED
                } else {
                    ship_controller.ship.speed
                };
                let turbo_power = ship_controller.turbo_power(active_speed);
                let movement = active_speed * delta_time;
                let ship = &mut ship_controller.ship;

                if crate::input::pressed(Key::Backspace) && ship.planet_locked {
                    ship.planet_locked = false;
                    ship.spiky_planet_locked = false;
                    ship.orbit_release_active = true;
                    ship.orbit_plane_normal = None;
                    println!("Orbita liberada");
                }

                let empty_colliders: [ObjectCollider; 0] = [];
                let water_center = water_planet_position(world_time);
                let spiky_center = spiky_planet_position(world_time);
                if !ship.planet_locked {
                    ship.spiky_planet_locked =
                        spiky_planet_is_closer(ship.position, water_center, spiky_center);
                }
                let (orbit_center, orbit_radius, orbit_rotation, orbit_colliders) =
                    if ship.spiky_planet_locked {
                        (
                            spiky_center,
                            SPIKY_PLANET_RADIUS,
                            world_time * SPIKY_PLANET_ROTATION_SPEED,
                            empty_colliders.as_slice(),
                        )
                    } else {
                        (
                            water_center,
                            planet_surface_radius,
                            world_time * 0.25,
                            object_colliders.as_slice(),
                        )
                    };
                move_around_planet(
                    ship,
                    movement,
                    orbit_center,
                    orbit_radius,
                    orbit_rotation,
                    orbit_colliders,
                    yaw_input,
                    pitch_input,
                );
                let camera_target = if ship.orbit_plane_normal.is_some() { 1.0 } else { 0.0 };
                let camera_response = 1.0 - (-2.4 * delta_time).exp();
                ship.orbit_camera_blend +=
                    (camera_target - ship.orbit_camera_blend) * camera_response;
                if ship.orbit_plane_normal.is_none() && ship.orbit_camera_blend < 0.001 {
                    ship.orbit_center = None;
                    ship.orbit_camera_blend = 0.0;
                }
                distance += movement;

                if ship.planet_locked {
                    if ship.spiky_planet_locked {
                        window.set_title(
                            "Planeta rosado | Enter: Guardián del Núcleo | Backspace: salir",
                        );
                    } else {
                        window.set_title(
                            "Planeta acuatico | Enter: iniciar nivel | Backspace: salir",
                        );
                    }
                } else {
                    window.set_title("Sistema solar | Vuelo libre 3D | F11: fullscreen");
                }
                render_scene(
                    active_model,
                    ship_center,
                    &turbo_plume,
                    &laser_bolt,
                    &ship_controller.projectiles,
                    &planet,
                    planet_center,
                    planet_scale,
                    &spiky_planet,
                    spiky_planet_center,
                    spiky_planet_scale,
                    &sun,
                    &mut scene_framebuffer,
                    &mut triangle_batch,
                    &ship_controller.ship,
                    world_time,
                    (distance * 0.0025).rem_euclid(1.0),
                    turbo_power,
                    ship_controller.charge_ratio(),
                );
            }
            GameMode::WaterLevel => {
                ship_controller.update(&input, delta_time);
                water_level_state.update(&mut ship_controller.ship, &input, delta_time);
                resolve_level_collisions(
                    &mut water_level_state,
                    &mut ship_controller.ship,
                    &water_level_spatial,
                );
                resolve_water_enemy_collisions(
                    &mut water_level_state,
                    &mut ship_controller.ship,
                    &mut ship_controller.projectiles,
                );
                bubble_emitter.update(&ship_controller.ship, delta_time);
                let turbo_power = ship_controller.turbo_power(ship_controller.ship.speed);
                render_water_level(
                    &water_level_spatial,
                    &enemy_models,
                    &enemy_centers,
                    &water_level_state.enemies,
                    water_level_state.elapsed,
                    &mut enemy_instance_buffers,
                    &bubble_model,
                    &bubble_emitter.particles,
                    level_ship_model,
                    level_ship_center,
                    &turbo_plume,
                    &laser_bolt,
                    &ship_controller.projectiles,
                    &mut scene_framebuffer,
                    &mut triangle_batch,
                    &ship_controller.ship,
                    world_time,
                    water_level_state.progress,
                    turbo_power,
                    ship_controller.charge_ratio(),
                );
                if water_level_state.progress >= WATER_LEVEL_LENGTH {
                    game_mode = GameMode::FreeWorld;
                    bubble_emitter.clear();
                    ship_controller = ShipController::new(ship_spec);
                    place_ship_near_water_planet(&mut ship_controller, world_time);
                    window.set_title("Sistema solar | Nivel completado");
                    println!("Nivel submarino completado");
                }
            }
            GameMode::CoreGuardian => {
                let input = GuardianInput {
                    horizontal: window.is_key_down(Key::D) as i8 as f32
                        - window.is_key_down(Key::A) as i8 as f32,
                    vertical: window.is_key_down(Key::S) as i8 as f32
                        + window.is_key_down(Key::Down) as i8 as f32
                        - window.is_key_down(Key::W) as i8 as f32
                        - window.is_key_down(Key::Up) as i8 as f32,
                    fire: window.is_key_down(Key::Space) || window.is_key_down(Key::Enter),
                    confirm: crate::input::pressed(Key::Enter),
                    exit: crate::input::pressed(Key::Backspace),
                };
                if guardian_fight.update(
                    delta_time,
                    input,
                    scene_framebuffer.width,
                    scene_framebuffer.height,
                ) == GuardianAction::Exit
                {
                    game_mode = GameMode::FreeWorld;
                    place_ship_near_spiky_planet(&mut ship_controller, world_time);
                    window.set_title(
                        "Planeta rosado | Enter: Guardián del Núcleo | Backspace: salir",
                    );
                }
                guardian_fight.render_scene(
                    &mut scene_framebuffer,
                    active_model,
                    ship_center,
                    ship_controller.ship.spec,
                );
            }
        }

        upscale_scene(&scene_framebuffer, &mut framebuffer);

        if game_mode == GameMode::CoreGuardian {
            guardian_fight.draw_hud(&mut framebuffer);
            draw_selection_label(&mut framebuffer, "GUARDIAN DEL NUCLEO", 600, 8, 2, 0xffe2ed);
            let message = if guardian_fight.is_victory() {
                "VICTORIA! ENTER PARA VOLVER"
            } else if guardian_fight.is_defeat() {
                "DERROTA  R: REINTENTAR  BACKSPACE: SALIR"
            } else if guardian_fight.is_exposed() {
                "NUCLEO EXPUESTO - DISPARA!"
            } else {
                "WASD / FLECHAS: MOVER  ESPACIO: DISPARAR"
            };
            draw_selection_label(
                &mut framebuffer,
                message,
                600,
                64,
                2,
                if guardian_fight.is_exposed() {
                    0xffe68b
                } else {
                    0xffd5df
                },
            );
            draw_selection_label(
                &mut framebuffer,
                "ESC: PAUSA   BACKSPACE: SALIR",
                600,
                766,
                2,
                0xffffff,
            );
        } else {
            let form = if selected_ship == 4 {
                if game_mode == GameMode::WaterLevel {
                    "BUZZ - SUBMARINO"
                } else if car_mode {
                    "BUZZ - CARRO"
                } else {
                    "BUZZ - NAVE"
                }
            } else if selected_ship == 5 {
                if game_mode == GameMode::WaterLevel {
                    "RAYO MCQUEEN - SUBMARINO"
                } else if car_mode {
                    "RAYO MCQUEEN - CARRO"
                } else {
                    "RAYO MCQUEEN - NAVE"
                }
            } else {
                ship_spec.name
            };
            draw_selection_label(
                &mut framebuffer,
                &form.to_ascii_uppercase(),
                600,
                22,
                2,
                0x99e226,
            );
            if game_mode == GameMode::WaterLevel {
                let enemy_counter = format!(
                    "ENEMIGOS: {}/{}",
                    water_level_state.defeated_enemies,
                    water_level_state.enemies.len()
                );
                draw_selection_label(&mut framebuffer, &enemy_counter, 1035, 24, 2, 0x65dfea);
            }
            draw_selection_label(
                &mut framebuffer,
                "ESC: PAUSA   F1: CONTROLES",
                600,
                766,
                2,
                0xffffff,
            );
            let resolution_label = if dynamic_resolution.enabled {
                format!("F2: RES AUTO {}%", dynamic_resolution.percentage())
            } else {
                "F2: RES AUTO OFF".to_string()
            };
            draw_selection_label(&mut framebuffer, &resolution_label, 1040, 766, 2, 0x9fc5df);
        }
        if crate::input::pressed(Key::P) {
            framebuffer.save_png("captura.png")?;
            println!("Captura guardada: captura.png");
        }
        crate::input::clear();
        window.update_with_buffer(&framebuffer.pixels, WIDTH, HEIGHT)?;
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_flight_selects_the_nearest_planet_for_orbit_collision() {
        let ship = Vec3::new(95.0, 0.0, 0.0);
        let water = Vec3::new(0.0, 0.0, 0.0);
        let spiky = Vec3::new(100.0, 0.0, 0.0);
        assert!(spiky_planet_is_closer(ship, water, spiky));
        assert!(!spiky_planet_is_closer(ship, spiky, water));
    }

    #[test]
    fn approaching_spiky_planet_cannot_cross_its_surface() {
        let center = Vec3::default();
        let mut ship = Ship::new(&ASTRA_SPEC);
        ship.position = Vec3::new(100.0, 0.0, 0.0);
        ship.yaw = 0.0;
        move_around_planet(&mut ship, 80.0, center, SPIKY_PLANET_RADIUS, 0.0, &[], 0, 0);
        let offset = ship.position.sub(center);
        let distance = offset.dot(offset).sqrt();
        let safe_radius = SPIKY_PLANET_RADIUS + ship.spec.collision_radius + COLLISION_MARGIN;
        assert!(ship.planet_locked);
        assert!(
            distance >= safe_radius - 0.001,
            "ship reached radius {distance}"
        );
    }

    #[test]
    fn mcqueen_submarine_has_aquatic_parts_and_renders_from_multiple_angles() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let model =
            load_obj(root.join("assets/vehicles/mcqueen/mcqueen_submarino_detallado.obj")).unwrap();
        assert!(
            model
                .vertices
                .iter()
                .all(|v| v.x.is_finite() && v.y.is_finite() && v.z.is_finite())
        );
        assert!(model.faces.iter().all(|face| {
            face.indices
                .iter()
                .all(|&index| index < model.vertices.len())
        }));
        for group in [
            "Periscopio",
            "Palas_helice",
            "Aro_helice",
            "Faro_submarino",
            "Aleta_dorsal",
            "Flotador",
            "Numero_95",
            "Rayo_lateral",
        ] {
            assert!(
                model.faces.iter().any(|face| face.group == group),
                "missing {group}"
            );
        }
        assert!(!model.faces.iter().any(|face| face.group == "Neumatico"));
        for side in [-1.0_f32, 1.0] {
            assert!(
                model
                    .faces
                    .iter()
                    .filter(|face| face.group == "Aro_helice")
                    .any(|face| face
                        .indices
                        .iter()
                        .all(|&i| model.vertices[i].x * side > 1.0))
            );
        }
        for (material, rgb) in [
            ("mcqueen_red", [225, 26, 35]),
            ("mcqueen_yellow", [255, 211, 36]),
            ("mcqueen_blue", [32, 105, 220]),
            ("mcqueen_white", [247, 249, 252]),
        ] {
            assert!(
                model.faces.iter().any(
                    |face| face.material == material && face.color == MaterialColor::Fixed(rgb)
                )
            );
        }
        let center = model_center(&model);
        assert!(
            (1.4..2.5)
                .contains(&(model_radius(&model, center) * MCQUEEN_SUBMARINE_SPEC.model_scale))
        );
        let mut frame = Framebuffer::new(WIDTH, HEIGHT);
        frame.clear(0x080e1d);
        for (index, angle) in [0.35_f32, 1.57, 3.5].into_iter().enumerate() {
            let forward = Vec3::new(angle.sin(), 0.0, -angle.cos());
            let side = Vec3::new(angle.cos(), 0.0, angle.sin());
            let before = frame.pixels.iter().filter(|&&p| p != 0x080e1d).count();
            render_model(
                &model,
                &mut frame,
                &test_camera(),
                |v| {
                    let local = MCQUEEN_SUBMARINE_SPEC.model_to_world(
                        v.sub(center).mul(0.80),
                        forward,
                        side,
                        Vec3::new(0.0, 1.0, 0.0),
                    );
                    Vec3::new(
                        local.x + (index as f32 - 1.0) * 7.0,
                        local.y * 0.90 + local.z * 0.43,
                        14.0 + local.z * 0.90 - local.y * 0.43,
                    )
                },
                None,
                None,
            );
            assert!(frame.pixels.iter().filter(|&&p| p != 0x080e1d).count() > before + 1000);
        }
        if let Ok(path) = std::env::var("MCQUEEN_SUB_PREVIEW") {
            frame.save_png(&path).unwrap();
        }
    }

    #[test]
    fn mcqueen_car_has_four_exposed_wheels_and_renders_from_multiple_angles() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let model =
            load_obj(root.join("assets/vehicles/mcqueen/mcqueen_carro_detallado.obj")).unwrap();
        let spaceship =
            load_obj(root.join("assets/vehicles/mcqueen/mcqueen_nave_detallada.obj")).unwrap();
        assert_ne!(model.faces.len(), spaceship.faces.len());
        assert!(
            model
                .vertices
                .iter()
                .all(|v| v.x.is_finite() && v.y.is_finite() && v.z.is_finite())
        );
        let mut wheel_vertices = HashSet::new();
        for face in &model.faces {
            assert!(
                face.indices
                    .iter()
                    .all(|&index| index < model.vertices.len())
            );
            if face.group == "Neumatico" {
                wheel_vertices.extend(face.indices);
            }
        }
        for x_sign in [-1.0_f32, 1.0] {
            for z_sign in [-1.0_f32, 1.0] {
                let count = wheel_vertices
                    .iter()
                    .filter(|&&index| {
                        let v = model.vertices[index];
                        v.x * x_sign > 1.5 && v.z * z_sign > 1.0
                    })
                    .count();
                assert_eq!(count, 64, "missing wheel at {x_sign}, {z_sign}");
            }
        }
        for (group, color) in [
            ("Numero_95", [255, 211, 36]),
            ("Carroceria", [225, 26, 35]),
            ("Iris", [32, 105, 220]),
            ("Neumatico", [26, 29, 38]),
        ] {
            assert!(
                model
                    .faces
                    .iter()
                    .any(|face| face.group == group && face.color == MaterialColor::Fixed(color))
            );
        }
        let center = model_center(&model);
        assert!(
            (1.5..2.5).contains(&(model_radius(&model, center) * MCQUEEN_CAR_SPEC.model_scale))
        );
        let mut frame = Framebuffer::new(WIDTH, HEIGHT);
        frame.clear(0x080e1d);
        for (index, angle) in [0.35_f32, 1.57, 3.5].into_iter().enumerate() {
            let forward = Vec3::new(angle.sin(), 0.0, -angle.cos());
            let side = Vec3::new(angle.cos(), 0.0, angle.sin());
            let before = frame.pixels.iter().filter(|&&p| p != 0x080e1d).count();
            render_model(
                &model,
                &mut frame,
                &test_camera(),
                |v| {
                    let local = MCQUEEN_CAR_SPEC.model_to_world(
                        v.sub(center).mul(0.80),
                        forward,
                        side,
                        Vec3::new(0.0, 1.0, 0.0),
                    );
                    Vec3::new(
                        local.x + (index as f32 - 1.0) * 7.0,
                        local.y * 0.90 + local.z * 0.43,
                        14.0 + local.z * 0.90 - local.y * 0.43,
                    )
                },
                None,
                None,
            );
            assert!(frame.pixels.iter().filter(|&&p| p != 0x080e1d).count() > before + 1000);
        }
        if let Ok(path) = std::env::var("MCQUEEN_CAR_PREVIEW") {
            frame.save_png(&path).unwrap();
        }
    }

    #[test]
    fn mcqueen_spaceship_loads_and_renders_from_multiple_angles() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let model =
            load_obj(root.join("assets/vehicles/mcqueen/mcqueen_nave_detallada.obj")).unwrap();
        assert!(
            model
                .vertices
                .iter()
                .all(|v| v.x.is_finite() && v.y.is_finite() && v.z.is_finite())
        );
        assert!(
            model
                .faces
                .iter()
                .all(|face| face.indices.iter().all(|&i| i < model.vertices.len()))
        );
        for (material, rgb) in [
            ("mcqueen_red", [225, 26, 35]),
            ("mcqueen_yellow", [255, 211, 36]),
            ("mcqueen_blue", [32, 105, 220]),
            ("mcqueen_white", [247, 249, 252]),
        ] {
            assert!(
                model.faces.iter().any(
                    |face| face.material == material && face.color == MaterialColor::Fixed(rgb)
                )
            );
        }
        for group in [
            "Numero_95",
            "Rayo_ala",
            "Ojo_blanco",
            "Reactor_rojo",
            "Sonrisa",
        ] {
            assert!(model.faces.iter().any(|face| face.group == group));
        }
        let center = model_center(&model);
        assert!((1.5..2.5).contains(&(model_radius(&model, center) * MCQUEEN_SPEC.model_scale)));
        let mut frame = Framebuffer::new(WIDTH, HEIGHT);
        frame.clear(0x080e1d);
        for (index, angle) in [0.35_f32, 1.57, 3.5].into_iter().enumerate() {
            let forward = Vec3::new(angle.sin(), 0.0, -angle.cos());
            let side = Vec3::new(angle.cos(), 0.0, angle.sin());
            let before = frame.pixels.iter().filter(|&&p| p != 0x080e1d).count();
            render_model(
                &model,
                &mut frame,
                &test_camera(),
                |v| {
                    let local = MCQUEEN_SPEC.model_to_world(
                        v.sub(center).mul(0.66),
                        forward,
                        side,
                        Vec3::new(0.0, 1.0, 0.0),
                    );
                    Vec3::new(
                        local.x + (index as f32 - 1.0) * 7.0,
                        local.y * 0.90 + local.z * 0.43,
                        14.0 + local.z * 0.90 - local.y * 0.43,
                    )
                },
                None,
                None,
            );
            let after = frame.pixels.iter().filter(|&&p| p != 0x080e1d).count();
            assert!(
                after > before + 1000,
                "invisible spaceship at angle {angle}"
            );
        }
        if let Ok(path) = std::env::var("MCQUEEN_PREVIEW") {
            frame.save_png(&path).unwrap();
        }
    }

    #[test]
    fn detailed_mcqueen_assets_have_metallic_materials_and_aligned_thrusters() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let variants = [
            (
                load_obj(root.join("assets/vehicles/mcqueen/mcqueen_nave_detallada.obj")).unwrap(),
                "McQueen_Ship_Core_L",
                "McQueen_Ship_Guard_L",
                &MCQUEEN_SPEC,
                (9_000, 15_400),
            ),
            (
                load_obj(root.join("assets/vehicles/mcqueen/mcqueen_carro_detallado.obj")).unwrap(),
                "McQueen_Car_Core_L",
                "McQueen_Car_Rim_Front_L",
                &MCQUEEN_CAR_SPEC,
                (8_900, 15_200),
            ),
            (
                load_obj(root.join("assets/vehicles/mcqueen/mcqueen_submarino_detallado.obj"))
                    .unwrap(),
                "McQueen_Sub_Core_L",
                "McQueen_Sub_Guard_L",
                &MCQUEEN_SUBMARINE_SPEC,
                (9_500, 16_300),
            ),
        ];

        for (model, core_group, detail_group, spec, minimum) in variants {
            assert!(model.vertices.len() > minimum.0 && model.faces.len() > minimum.1);
            assert!(model.faces.iter().any(|face| face.smooth));
            assert!(model.faces.iter().any(|face| face.group == detail_group));
            assert!(model.faces.iter().any(|face| {
                face.material == "mcqueen_red" && face.shader == SurfaceShader::McQueenRedMetal
            }));
            assert!(model.faces.iter().any(|face| {
                face.material == "mcqueen_emission"
                    && face.shader == SurfaceShader::McQueenEmission
                    && face.color == MaterialColor::Fixed([255, 82, 14])
            }));
            assert!(
                model
                    .faces
                    .iter()
                    .any(|face| { face.group.contains("Ojo") || face.group.contains("Iris") })
            );
            assert!(
                model
                    .faces
                    .iter()
                    .any(|face| { face.group == "Pupila" && face.material == "mcqueen_dark" })
            );
            for glass_group in ["Cabina_azul", "Parabrisas", "Cupula"] {
                assert!(
                    !model.faces.iter().any(|face| {
                        face.group == glass_group && face.material == "mcqueen_glass"
                    })
                );
            }

            let mut points = model
                .faces
                .iter()
                .filter(|face| face.group == core_group)
                .flat_map(|face| face.indices)
                .map(|index| model.vertices[index]);
            let first = points.next().expect("missing McQueen emitter geometry");
            let (mut min, mut max) = (first, first);
            for point in points {
                min.x = min.x.min(point.x);
                min.y = min.y.min(point.y);
                min.z = min.z.min(point.z);
                max.x = max.x.max(point.x);
                max.y = max.y.max(point.y);
                max.z = max.z.max(point.z);
            }
            let core = min.add(max).mul(0.5).sub(model_center(&model));
            assert!((-core.z * spec.model_scale - spec.turbo_back).abs() < 0.002);
            assert!((core.x.abs() * spec.model_scale - spec.turbo_side).abs() < 0.002);
            assert!((-core.y * spec.model_scale - spec.turbo_down).abs() < 0.002);
        }
    }

    fn test_camera() -> Camera {
        Camera {
            position: Vec3::default(),
            forward: Vec3::new(0.0, 0.0, 1.0),
            right: Vec3::new(1.0, 0.0, 0.0),
            up: Vec3::new(0.0, 1.0, 0.0),
        }
    }

    #[test]
    fn water_planet_atmosphere_draws_a_halo_without_covering_the_center() {
        let mut framebuffer = Framebuffer::new(320, 200);
        framebuffer.clear(0x030718);
        draw_water_planet_atmosphere(
            &mut framebuffer,
            &test_camera(),
            Vec3::new(0.0, 0.0, 100.0),
            WATER_PLANET_RADIUS,
            1.5,
        );
        assert_eq!(framebuffer.pixels[100 * 320 + 160], 0x030718);
        assert!(framebuffer.pixels.iter().any(|&pixel| pixel != 0x030718));
        assert!(framebuffer.pixels.iter().any(|&pixel| {
            let green = (pixel >> 8) & 0xff;
            let blue = pixel & 0xff;
            pixel != 0x030718 && blue > green
        }));
    }

    #[test]
    fn refractive_water_distorts_and_tints_the_background() {
        let width = 16;
        let height = 16;
        let background: Vec<u32> = (0..width * height)
            .map(|index| {
                let x = (index % width) as u32;
                let y = (index / width) as u32;
                (x * 12 << 16) | (y * 10 << 8) | 0x24
            })
            .collect();
        let surface = 0x168dcc;
        let refracted = refract_water_color(
            surface,
            &background,
            width,
            height,
            8,
            8,
            Vec3::new(0.3, -0.2, 0.7),
            1.25,
            0.18,
        );
        assert_ne!(refracted, surface);
        assert_ne!(refracted, background[8 * width + 8]);
        assert!((refracted & 0xff) > ((refracted >> 16) & 0xff));
    }

    fn test_triangle(z: f32) -> Model {
        Model::new(
            vec![
                Vec3::new(-1.0, -1.0, z),
                Vec3::new(1.0, -1.0, z),
                Vec3::new(0.0, 1.0, z),
            ],
            vec![Face::new([0, 1, 2], "metal", "test")],
        )
    }

    #[test]
    fn frustum_and_distance_culling_keep_visible_geometry_only() {
        let camera = test_camera();
        let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
        render_model(
            &test_triangle(5.0),
            &mut framebuffer,
            &camera,
            |vertex| vertex,
            None,
            None,
        );
        assert!(framebuffer.pixels.iter().any(|&pixel| pixel != 0));

        framebuffer.clear(0);
        render_model(
            &test_triangle(-5.0),
            &mut framebuffer,
            &camera,
            |vertex| vertex,
            None,
            None,
        );
        render_model(
            &test_triangle(FAR_PLANE + 10.0),
            &mut framebuffer,
            &camera,
            |vertex| vertex,
            None,
            None,
        );
        assert!(framebuffer.pixels.iter().all(|&pixel| pixel == 0));
    }

    #[test]
    fn instanced_rendering_reuses_the_vertex_buffer() {
        let camera = test_camera();
        let model = test_triangle(5.0);
        let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
        let mut camera_vertices = Vec::new();
        render_model_with_scratch(
            &model,
            &mut framebuffer,
            &camera,
            |vertex| vertex,
            None,
            None,
            &mut camera_vertices,
        );
        let allocation = camera_vertices.as_ptr();
        render_model_with_scratch(
            &model,
            &mut framebuffer,
            &camera,
            |vertex| vertex.add(Vec3::new(0.25, 0.0, 0.0)),
            None,
            None,
            &mut camera_vertices,
        );
        assert_eq!(camera_vertices.len(), model.vertices.len());
        assert_eq!(camera_vertices.as_ptr(), allocation);
    }

    #[test]
    fn tiled_rasterizer_preserves_depth_across_tile_boundaries() {
        let mut framebuffer = Framebuffer::new(64, 64);
        let vertices = |z| {
            [
                ScreenVertex { x: 4.0, y: 4.0, z },
                ScreenVertex { x: 60.0, y: 4.0, z },
                ScreenVertex {
                    x: 32.0,
                    y: 60.0,
                    z,
                },
            ]
        };
        let mut batch = TriangleBatch::new(framebuffer.width, framebuffer.height);
        batch.commands.push(TriangleCommand {
            vertices: vertices(0.5),
            style: TriangleStyle::Flat(0xff0000),
        });
        batch.commands.push(TriangleCommand {
            vertices: vertices(1.0),
            style: TriangleStyle::Flat(0x00ff00),
        });
        batch.flush_with_workers(&mut framebuffer, 2);

        assert_eq!(framebuffer.pixels[31 * 64 + 32], 0x00ff00);
        assert_eq!(framebuffer.pixels[32 * 64 + 32], 0x00ff00);
        assert!(batch.commands.is_empty());
    }

    #[test]
    fn dynamic_resolution_reacts_only_to_sustained_frame_rates() {
        let mut resolution = DynamicResolution::new();
        for _ in 0..80 {
            resolution.update(1.0 / 40.0);
        }
        assert_eq!(resolution.scale_index, 1);
        assert_eq!(resolution.dimensions(), (1080, 720));

        for _ in 0..600 {
            resolution.update(1.0 / 100.0);
        }
        assert_eq!(resolution.scale_index, 0);
        assert_eq!(resolution.dimensions(), (WIDTH, HEIGHT));
    }

    #[test]
    fn disabling_dynamic_resolution_restores_full_size() {
        let mut resolution = DynamicResolution::new();
        resolution.scale_index = RESOLUTION_SCALES.len() - 1;
        assert!(resolution.toggle());
        assert!(!resolution.enabled);
        assert_eq!(resolution.dimensions(), (WIDTH, HEIGHT));
    }

    #[test]
    fn scene_upscaling_covers_the_full_presentation_buffer() {
        let mut source = Framebuffer::new(2, 2);
        source.pixels = vec![1, 2, 3, 4];
        let mut target = Framebuffer::new(WIDTH, HEIGHT);
        upscale_scene(&source, &mut target);
        assert_eq!(target.pixels[0], 1);
        assert_eq!(target.pixels[WIDTH - 1], 2);
        assert_eq!(target.pixels[(HEIGHT - 1) * WIDTH], 3);
        assert_eq!(target.pixels[HEIGHT * WIDTH - 1], 4);
    }

    #[test]
    fn spatial_level_preserves_faces_and_selects_nearby_chunks() {
        let source_x = |progress: f32| progress - WATER_LEVEL_SOURCE_START;
        let model = Model::new(
            vec![
                Vec3::new(source_x(5.0), 0.0, 0.0),
                Vec3::new(source_x(5.0), 1.0, 0.0),
                Vec3::new(source_x(5.0), 0.0, 1.0),
                Vec3::new(source_x(300.0), 0.0, 0.0),
                Vec3::new(source_x(300.0), 1.0, 0.0),
                Vec3::new(source_x(300.0), 0.0, 1.0),
            ],
            vec![
                Face::new([0, 1, 2], "metal", "near"),
                Face::new([3, 4, 5], "metal", "far"),
            ],
        );
        let spatial = LevelSpatialIndex::new(&model, Vec::new());
        assert_eq!(
            spatial
                .render_chunks
                .iter()
                .map(|chunk| chunk.model.faces.len())
                .sum::<usize>(),
            model.faces.len()
        );
        assert_eq!(spatial.visible_chunks(5.0).count(), 1);
        assert_eq!(spatial.visible_chunks(300.0).count(), 1);
    }

    #[test]
    fn extended_water_level_is_long_chunked_and_has_collision_gates() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut level = load_obj(root.join("assets/levels/waterlevel_extendido.obj")).unwrap();
        assert!(level.vertices.len() > 35_000);
        assert!(level.faces.len() > 50_000);
        let min_x = level
            .vertices
            .iter()
            .map(|vertex| vertex.x)
            .fold(f32::INFINITY, f32::min);
        let max_x = level
            .vertices
            .iter()
            .map(|vertex| vertex.x)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(max_x - min_x >= 540.0);
        let min_y = level
            .vertices
            .iter()
            .map(|vertex| vertex.y)
            .fold(f32::INFINITY, f32::min);
        let max_y = level
            .vertices
            .iter()
            .map(|vertex| vertex.y)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(max_y - min_y >= 34.0);
        assert!(!level.faces.iter().any(|face| {
            face.group.starts_with("CAVES_AND_ARCHES")
                || face.group.starts_with("obstacle_arch_top")
        }));
        for sector in 1..=3 {
            assert!(
                level
                    .faces
                    .iter()
                    .any(|face| { face.group == format!("TERRAIN_MAIN_SECTOR_{sector}") })
            );
        }
        assert!(level.faces.iter().any(|face| {
            face.group.starts_with("LONG_LEVEL_CRYSTAL_") && face.material == "Crystal_Cyan"
        }));

        stretch_model_group_height(&mut level, "TERRAIN_WALLS", -5.0, 3.6);
        let wall_top = level
            .faces
            .iter()
            .filter(|face| face.group.starts_with("TERRAIN_WALLS"))
            .flat_map(|face| face.indices)
            .map(|index| level.vertices[index].z)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(wall_top > 13.0);

        let colliders = build_level_colliders(&level);
        assert!(colliders.len() >= 7);
        let spatial = LevelSpatialIndex::new(&level, colliders);
        assert!(spatial.render_chunks.len() >= 17);
        assert!(!spatial.nearby_colliders(70.0).is_empty());
        assert!(!spatial.nearby_colliders(510.0).is_empty());

        let state = WaterLevelState::new();
        assert_eq!(state.enemies.len(), 30);
        assert!(state.enemies.last().unwrap().progress < WATER_LEVEL_LENGTH);
    }

    #[test]
    fn spatial_collision_index_covers_chunk_boundaries() {
        let model = test_triangle(5.0);
        let collider = LevelCollider {
            min: Vec3::new(-33.0, 0.0, 0.0),
            max: Vec3::new(-32.0, 1.0, 1.0),
        };
        let spatial = LevelSpatialIndex::new(&model, vec![collider]);
        assert!(!spatial.nearby_colliders(31.5).is_empty());
        assert!(!spatial.nearby_colliders(32.5).is_empty());
    }

    #[test]
    fn collision_grid_handles_negative_cells_and_sphere_boundaries() {
        let mut grid = SpatialCollisionGrid::new(10.0);
        grid.insert_sphere(7, Vec3::new(-0.5, 0.0, 0.0), 1.0);
        assert_eq!(grid.candidates(Vec3::new(-1.0, 0.0, 0.0)), &[7]);
        assert_eq!(grid.candidates(Vec3::new(0.25, 0.0, 0.0)), &[7]);
        assert!(grid.candidates(Vec3::new(25.0, 0.0, 0.0)).is_empty());
    }

    #[test]
    fn collision_grid_returns_only_local_candidates() {
        let mut grid = SpatialCollisionGrid::new(8.0);
        grid.insert_sphere(0, Vec3::new(1.0, 1.0, 1.0), 2.0);
        grid.insert_sphere(1, Vec3::new(100.0, 100.0, 100.0), 2.0);
        assert!(grid.candidates(Vec3::new(1.0, 1.0, 1.0)).contains(&0));
        assert!(!grid.candidates(Vec3::new(1.0, 1.0, 1.0)).contains(&1));
    }

    fn test_bubble(lifetime: f32) -> BubbleParticle {
        BubbleParticle {
            position: Vec3::default(),
            velocity: Vec3::default(),
            lifetime,
            scale: 1.0,
        }
    }

    #[test]
    fn bubble_pool_reuses_slots_and_preserves_capacity_on_clear() {
        let mut emitter = BubbleEmitter::new();
        emitter.spawn(test_bubble(0.0));
        emitter.spawn(test_bubble(2.0));
        assert_eq!(emitter.particles.len(), 1);
        emitter.clear();
        assert_eq!(emitter.particles.len(), 1);
        assert_eq!(emitter.particles[0].lifetime, 0.0);
    }

    #[test]
    fn bubble_pool_never_exceeds_safety_limit() {
        let mut emitter = BubbleEmitter::new();
        for _ in 0..MAX_BUBBLE_PARTICLES + 20 {
            emitter.spawn(test_bubble(2.0));
        }
        assert_eq!(emitter.particles.len(), MAX_BUBBLE_PARTICLES);
    }

    #[test]
    fn buzz_assets_load_with_valid_geometry_and_recognized_materials() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut sizes = Vec::new();
        for (form, filename, minimum, core_group, spec) in [
            (
                "nave",
                "buzz_nave_detallada.obj",
                (5_700, 9_700),
                "Buzz_Ship_Core_L",
                &BUZZ_SPEC,
            ),
            (
                "carro",
                "buzz_carro_detallado.obj",
                (9_700, 17_100),
                "Buzz_Car_Core_L",
                &BUZZ_CAR_SPEC,
            ),
            (
                "submarino",
                "buzz_submarino_detallado.obj",
                (7_100, 12_100),
                "Buzz_Sub_Core_L",
                &BUZZ_SUBMARINE_SPEC,
            ),
        ] {
            let model = load_obj(root.join("assets/vehicles/buzz").join(filename)).unwrap();
            assert!(model.vertices.len() > minimum.0 && model.faces.len() > minimum.1);
            assert!(
                model
                    .vertices
                    .iter()
                    .all(|v| v.x.is_finite() && v.y.is_finite() && v.z.is_finite())
            );
            assert!(
                model
                    .faces
                    .iter()
                    .all(|face| face.indices.iter().all(|&i| i < model.vertices.len()))
            );
            for material in [
                "buzz_white",
                "buzz_green",
                "buzz_purple",
                "buzz_red",
                "buzz_glass",
                "buzz_emission",
            ] {
                assert!(
                    model.faces.iter().any(|f| f.material == material),
                    "{form}: {material}"
                );
            }
            assert!(model.faces.iter().any(|face| {
                face.material == "buzz_green" && face.shader == SurfaceShader::BuzzGreenMetal
            }));
            assert!(model.faces.iter().any(|face| {
                face.material == "buzz_emission"
                    && face.shader == SurfaceShader::BuzzEmission
                    && face.color == MaterialColor::Fixed([128, 255, 30])
            }));
            assert!(model.faces.iter().any(|face| face.smooth));
            let radius = model_radius(&model, model_center(&model)) * spec.model_scale;
            assert!(
                (1.0..3.0).contains(&radius),
                "{form}: unexpected size {radius}"
            );

            let mut points = model
                .faces
                .iter()
                .filter(|face| face.group == core_group)
                .flat_map(|face| face.indices)
                .map(|index| model.vertices[index]);
            let first = points.next().expect("missing Buzz emitter geometry");
            let (mut min, mut max) = (first, first);
            for point in points {
                min.x = min.x.min(point.x);
                min.y = min.y.min(point.y);
                min.z = min.z.min(point.z);
                max.x = max.x.max(point.x);
                max.y = max.y.max(point.y);
                max.z = max.z.max(point.z);
            }
            let core = min.add(max).mul(0.5).sub(model_center(&model));
            assert!((-core.z * spec.model_scale - spec.turbo_back).abs() < 0.002);
            assert!((core.x.abs() * spec.model_scale - spec.turbo_side).abs() < 0.002);
            assert!((-core.y * spec.model_scale - spec.turbo_down).abs() < 0.002);
            sizes.push((model.vertices.len(), model.faces.len()));
        }
        assert_ne!(sizes[0], sizes[1]);
        assert_ne!(sizes[0], sizes[2]);
        assert_ne!(sizes[1], sizes[2]);
    }

    #[test]
    fn detailed_arwing_asset_has_expected_scale_and_axes() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let model = load_obj(root.join("assets/vehicles/arwing/arwing2.obj")).unwrap();
        assert!(model.vertices.len() > 600);
        assert!(model.faces.len() > 1_200);
        let radius = model_radius(&model, model_center(&model)) * ARWING_SPEC.model_scale;
        assert!((1.5..3.5).contains(&radius), "unexpected radius {radius}");
        assert!(model.faces.iter().any(|face| {
            face.material.to_ascii_lowercase().contains("engine_cyan")
                && face.color == MaterialColor::Fixed([40, 225, 242])
        }));

        let forward = Vec3::new(1.0, 0.0, 0.0);
        let right = Vec3::new(0.0, 0.0, 1.0);
        let up = Vec3::new(0.0, 1.0, 0.0);
        let transformed = ARWING_SPEC.model_to_world(Vec3::new(1.0, 2.0, 3.0), forward, right, up);
        assert_eq!(
            (transformed.x, transformed.y, transformed.z),
            (-1.0, 3.0, -2.0)
        );

        let submarine =
            load_obj(root.join("assets/vehicles/arwing/arwing2_submarino.obj")).unwrap();
        assert!(submarine.vertices.len() > 4_000);
        assert!(submarine.faces.len() > 8_000);
        let submarine_radius =
            model_radius(&submarine, model_center(&submarine)) * ARWING_SUBMARINE_SPEC.model_scale;
        assert!(
            (1.5..2.5).contains(&submarine_radius),
            "unexpected submarine radius {submarine_radius}"
        );
        assert!(
            submarine
                .faces
                .iter()
                .any(|face| { face.group.contains("aro_helice") && face.material == "fin_blue" })
        );
        for candidate in [&model, &submarine] {
            for material in [
                "hull_white",
                "wing_metal",
                "fin_blue",
                "cockpit_blue",
                "dark_metal",
                "engine_cyan",
                "warning_yellow",
            ] {
                assert!(
                    candidate.faces.iter().any(|face| face.material == material),
                    "missing Arwing material {material}"
                );
            }
        }
    }

    #[test]
    fn detailed_kirby_asset_has_expected_scale_axes_and_palette() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let model = load_obj(root.join("assets/vehicles/kirby/kirby_detallado.obj")).unwrap();
        assert!(model.vertices.len() > 14_000);
        assert!(model.faces.len() > 29_000);
        assert!(model.faces.iter().any(|face| face.smooth));
        assert!(
            model
                .vertex_normals
                .iter()
                .any(|normal| normal.dot(*normal) > 0.9)
        );
        let radius = model_radius(&model, model_center(&model)) * KIRBY_SPEC.model_scale;
        assert!((1.5..2.0).contains(&radius), "unexpected radius {radius}");

        let forward = Vec3::new(1.0, 0.0, 0.0);
        let right = Vec3::new(0.0, 0.0, 1.0);
        let up = Vec3::new(0.0, 1.0, 0.0);
        let transformed = KIRBY_SPEC.model_to_world(Vec3::new(2.0, -1.0, 3.0), forward, right, up);
        assert_eq!(
            (transformed.x, transformed.y, transformed.z),
            (1.0, 3.0, 2.0)
        );
        assert!(model.faces.iter().any(|face| {
            face.material.to_ascii_lowercase().contains("cyan_engine")
                && face.color == MaterialColor::Fixed([40, 225, 242])
        }));
        assert!(model.faces.iter().any(|face| {
            face.material
                .to_ascii_lowercase()
                .contains("kirby_smooth_pink")
                && face.color == MaterialColor::Fixed([255, 158, 198])
        }));
        assert!(
            model
                .faces
                .iter()
                .any(|face| face.group == "Kirby_Star_Emblem")
        );
        assert!(
            model
                .faces
                .iter()
                .any(|face| face.group == "Thruster_Guard_L")
        );

        let submarine =
            load_obj(root.join("assets/vehicles/kirby/kirbysub_detallado.obj")).unwrap();
        assert!(submarine.vertices.len() > 10_000);
        assert!(submarine.faces.len() > 21_000);
        assert!(submarine.faces.iter().any(|face| face.smooth));
        assert!(
            submarine
                .faces
                .iter()
                .any(|face| face.group == "Submarine_Star_Emblem")
        );
        assert!(
            submarine
                .faces
                .iter()
                .any(|face| face.group == "Sub_Propeller_Guard_L")
        );

        let emitter_offset = |candidate: &Model, group: &str, scale: f32| {
            let mut points = candidate
                .faces
                .iter()
                .filter(|face| face.group == group)
                .flat_map(|face| face.indices)
                .map(|index| candidate.vertices[index]);
            let first = points.next().expect("missing emitter geometry");
            let (mut min, mut max) = (first, first);
            for point in points {
                min.x = min.x.min(point.x);
                min.y = min.y.min(point.y);
                min.z = min.z.min(point.z);
                max.x = max.x.max(point.x);
                max.y = max.y.max(point.y);
                max.z = max.z.max(point.z);
            }
            let relative = min.add(max).mul(0.5).sub(model_center(candidate));
            (
                relative.y * scale,
                relative.x.abs() * scale,
                -relative.z * scale,
            )
        };
        let assert_emitter = |actual: (f32, f32, f32), spec: &ShipSpec| {
            assert!((actual.0 - spec.turbo_back).abs() < 0.002);
            assert!((actual.1 - spec.turbo_side).abs() < 0.002);
            assert!((actual.2 - spec.turbo_down).abs() < 0.002);
        };
        assert_emitter(
            emitter_offset(&model, "Rear_Core_L", KIRBY_SPEC.model_scale),
            &KIRBY_SPEC,
        );
        assert_emitter(
            emitter_offset(
                &submarine,
                "Sub_Propeller_Core_L",
                KIRBY_SUBMARINE_SPEC.model_scale,
            ),
            &KIRBY_SUBMARINE_SPEC,
        );
    }

    #[test]
    fn detailed_astra_assets_have_geometry_materials_and_thrusters() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let ship = load_obj(root.join("assets/vehicles/astra/nave_astra_detallada.obj")).unwrap();
        let car = load_obj(root.join("assets/vehicles/astra/astra_carro_detallado.obj")).unwrap();
        let submarine =
            load_obj(root.join("assets/vehicles/astra/astra_submarino_detallado.obj")).unwrap();

        assert!(ship.vertices.len() > 2_300 && ship.faces.len() > 4_300);
        assert!(car.vertices.len() > 2_700 && car.faces.len() > 5_100);
        assert!(submarine.vertices.len() > 2_700 && submarine.faces.len() > 5_100);

        for (name, model) in [("nave", &ship), ("carro", &car), ("submarino", &submarine)] {
            assert!(
                model.faces.iter().any(|face| face.smooth),
                "{name}: missing smooth shading"
            );
            for material in [
                "Casco_blanco",
                "Azul_cobalto",
                "Azul_profundo",
                "Vidrio_oscuro",
                "Luz_cian",
                "Titanio",
                "Grafito",
            ] {
                assert!(
                    model.faces.iter().any(|face| face.material == material),
                    "{name}: missing ASTRA material {material}"
                );
            }
        }

        for (model, groups) in [
            (&ship, ["Astra_Engine_Core_L", "Astra_Wingtip_L"]),
            (&car, ["Astra_Car_Core_L", "Astra_Rim_Front_L"]),
            (&submarine, ["Astra_Sub_Core_L", "Astra_Sub_Guard_L"]),
        ] {
            for group in groups {
                assert!(
                    model.faces.iter().any(|face| face.group == group),
                    "missing ASTRA group {group}"
                );
            }
        }

        let emitter_offset = |candidate: &Model, group: &str, scale: f32| {
            let mut points = candidate
                .faces
                .iter()
                .filter(|face| face.group == group)
                .flat_map(|face| face.indices)
                .map(|index| candidate.vertices[index]);
            let first = points.next().expect("missing ASTRA emitter geometry");
            let (mut min, mut max) = (first, first);
            for point in points {
                min.x = min.x.min(point.x);
                min.y = min.y.min(point.y);
                min.z = min.z.min(point.z);
                max.x = max.x.max(point.x);
                max.y = max.y.max(point.y);
                max.z = max.z.max(point.z);
            }
            let relative = min.add(max).mul(0.5).sub(model_center(candidate));
            (
                -relative.z * scale,
                relative.x.abs() * scale,
                -relative.y * scale,
            )
        };
        let assert_emitter = |actual: (f32, f32, f32), spec: &ShipSpec| {
            assert!((actual.0 - spec.turbo_back).abs() < 0.002);
            assert!((actual.1 - spec.turbo_side).abs() < 0.002);
            assert!((actual.2 - spec.turbo_down).abs() < 0.002);
        };
        assert_emitter(
            emitter_offset(&ship, "Astra_Engine_Core_L", ASTRA_SPEC.model_scale),
            &ASTRA_SPEC,
        );
        assert_emitter(
            emitter_offset(&car, "Astra_Car_Core_L", ASTRA_CAR_SPEC.model_scale),
            &ASTRA_CAR_SPEC,
        );
        assert_emitter(
            emitter_offset(
                &submarine,
                "Astra_Sub_Core_L",
                ASTRA_SUBMARINE_SPEC.model_scale,
            ),
            &ASTRA_SUBMARINE_SPEC,
        );
    }

    #[test]
    fn detailed_cowboy_assets_have_geometry_materials_and_aligned_thrusters() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let ship =
            load_obj(root.join("assets/vehicles/cowboy/nave_cowboyb_detallada.obj")).unwrap();
        let car = load_obj(root.join("assets/vehicles/cowboy/cb_carro_detallado.obj")).unwrap();
        let submarine =
            load_obj(root.join("assets/vehicles/cowboy/cb_submarino_detallado.obj")).unwrap();

        assert!(ship.vertices.len() > 1_900 && ship.faces.len() > 2_700);
        assert!(car.vertices.len() > 3_300 && car.faces.len() > 5_400);
        assert!(submarine.vertices.len() > 3_100 && submarine.faces.len() > 5_000);

        for (name, model) in [("nave", &ship), ("carro", &car), ("submarino", &submarine)] {
            assert!(
                model.faces.iter().any(|face| face.smooth),
                "{name}: no smooth shading"
            );
            for material in [
                "Cowboy_Metal",
                "Cowboy_Graphite",
                "Cowboy_Glass",
                "Cowboy_OrangeGlow",
                "Cowboy_Cream",
            ] {
                assert!(
                    model.faces.iter().any(|face| face.material == material),
                    "{name}: missing Cowboy material {material}"
                );
            }
        }
        for material in ["Cowboy_Yellow", "Cowboy_Red", "Cowboy_DarkRed"] {
            assert!(
                [&ship, &car, &submarine]
                    .iter()
                    .any(|model| model.faces.iter().any(|face| face.material == material)),
                "missing Cowboy paint {material}"
            );
        }
        assert!(ship.faces.iter().any(|face| {
            face.material == "Cowboy_Yellow"
                && face.shader == SurfaceShader::CowboyYellowPaint
                && face.color == MaterialColor::Fixed([244, 158, 20])
        }));
        assert!(ship.faces.iter().any(|face| {
            face.material == "Cowboy_OrangeGlow"
                && face.shader == SurfaceShader::CowboyOrangeEmission
                && face.color == MaterialColor::Fixed([255, 72, 12])
        }));
        for (material, shader) in [
            ("Amarillo", SurfaceShader::CowboyYellowPaint),
            ("Rojo", SurfaceShader::CowboyRedPaint),
            ("Gris_oscuro", SurfaceShader::ArwingMetal),
        ] {
            assert!(
                ship.faces
                    .iter()
                    .any(|face| face.material == material && face.shader == shader),
                "main ship surface {material} is not metallic"
            );
        }

        for (model, groups) in [
            (&ship, ["Cowboy_Ship_Core_L", "Cowboy_Ship_Canopy"]),
            (&car, ["Cowboy_Car_Core_L", "Cowboy_Car_Rim_Front_L"]),
            (&submarine, ["Cowboy_Sub_Core_L", "Cowboy_Sub_Guard_L"]),
        ] {
            for group in groups {
                assert!(
                    model.faces.iter().any(|face| face.group == group),
                    "missing {group}"
                );
            }
        }

        let group_center = |candidate: &Model, group: &str| {
            let mut points = candidate
                .faces
                .iter()
                .filter(|face| face.group == group)
                .flat_map(|face| face.indices)
                .map(|index| candidate.vertices[index]);
            let first = points.next().expect("missing Cowboy emitter geometry");
            let (mut min, mut max) = (first, first);
            for point in points {
                min.x = min.x.min(point.x);
                min.y = min.y.min(point.y);
                min.z = min.z.min(point.z);
                max.x = max.x.max(point.x);
                max.y = max.y.max(point.y);
                max.z = max.z.max(point.z);
            }
            min.add(max).mul(0.5).sub(model_center(candidate))
        };
        let assert_emitter = |actual: (f32, f32, f32), spec: &ShipSpec| {
            assert!((actual.0 - spec.turbo_back).abs() < 0.002);
            assert!((actual.1 - spec.turbo_side).abs() < 0.002);
            assert!((actual.2 - spec.turbo_down).abs() < 0.002);
        };
        let ship_core = group_center(&ship, "Cowboy_Ship_Core_L");
        assert_emitter(
            (
                ship_core.y * COWBOYB_SPEC.model_scale,
                ship_core.x.abs() * COWBOYB_SPEC.model_scale,
                -ship_core.z * COWBOYB_SPEC.model_scale,
            ),
            &COWBOYB_SPEC,
        );
        for (model, group, spec) in [
            (&car, "Cowboy_Car_Core_L", &COWBOYB_CAR_SPEC),
            (&submarine, "Cowboy_Sub_Core_L", &COWBOYB_SUBMARINE_SPEC),
        ] {
            let core = group_center(model, group);
            assert_emitter(
                (
                    -core.z * spec.model_scale,
                    core.x.abs() * spec.model_scale,
                    -core.y * spec.model_scale,
                ),
                spec,
            );
        }
    }

    #[test]
    fn aquatic_enemy_asset_contains_all_enemy_groups() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let source = load_obj(root.join("assets/enemies/acuaticenemies.obj")).unwrap();
        for group in ["ENEMY_PUFFER_MINE", "ENEMY_SHARK_DRONE", "ENEMY_JELLY_BOT"] {
            let enemy = extract_model_group(&source, group).unwrap();
            assert!(!enemy.vertices.is_empty(), "{group} sin vertices");
            assert!(!enemy.faces.is_empty(), "{group} sin caras");
        }
    }
}
