mod black_hole;
mod core_guardian;
mod framebuffer;
mod galaxy;
mod hourglass_rescue;
mod input;
mod lava_castle;
mod lava_castle_interior;
mod menu;
mod minigame_audio;
mod obj_loader;
mod race_track;
mod ship;
mod space_audio;
mod space_battle;
mod space_decor;
mod story_guide;
mod tower_guard;
mod triangle;

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::f32::consts::{PI, TAU};
use std::fs::File;
use std::sync::OnceLock;
use std::thread;
use std::time::Instant;

use core_guardian::{CoreGuardian, GuardianAction, GuardianInput};
use framebuffer::Framebuffer;
use galaxy::{
    Galaxy, GalaxyTransit, crossed_wormhole, draw_hyperspace_transition, nova_portal_position,
    wormhole_position,
};
use minifb::{Key, ScaleMode, Window, WindowOptions};
use obj_loader::{Face, MaterialColor, Model, SurfaceShader, Vec3, load_obj};
use ship::{
    ARWING_CAR_SPEC, ARWING_SPEC, ARWING_SUBMARINE_SPEC, ASTRA_CAR_SPEC, ASTRA_SPEC,
    ASTRA_SUBMARINE_SPEC, BUZZ_CAR_SPEC, BUZZ_SPEC, BUZZ_SUBMARINE_SPEC, COWBOYB_CAR_SPEC,
    COWBOYB_SPEC, COWBOYB_SUBMARINE_SPEC, KIRBY_CAR_SPEC, KIRBY_SPEC, KIRBY_SUBMARINE_SPEC,
    MCQUEEN_CAR_SPEC, MCQUEEN_SPEC, MCQUEEN_SUBMARINE_SPEC, Projectile, Ship, ShipController,
    ShipInput, ShipSpec,
};
use triangle::{ScreenVertex, draw_triangle_alpha_region, draw_triangle_shaded_region};

const WIDTH: usize = 1200;
const HEIGHT: usize = 800;
const BENCHMARK_WARMUP_FRAMES: usize = 180;
const BENCHMARK_SAMPLE_FRAMES: usize = 600;
const MIN_VERTICES_PER_TRANSFORM_WORKER: usize = 2_048;

#[derive(Clone, Copy, Default)]
struct BenchmarkSample {
    frame_ms: f32,
    update_scene_ms: f32,
    geometry_ms: f32,
    sort_ms: f32,
    raster_ms: f32,
    post_ms: f32,
    copy_ms: f32,
    present_ms: f32,
    models_visible: usize,
    models_culled: usize,
    vertices: usize,
    triangles: usize,
    race_geometry_ms: [f32; 7],
    race_vertices: [usize; 7],
    race_triangles: [usize; 7],
}

struct BenchmarkRun {
    scene: String,
    warmup_left: usize,
    samples: Vec<BenchmarkSample>,
}

impl BenchmarkRun {
    fn from_environment() -> Option<Self> {
        std::env::var("GRAPH_BENCH_SCENE").ok().map(|scene| Self {
            scene,
            warmup_left: BENCHMARK_WARMUP_FRAMES,
            samples: Vec::with_capacity(BENCHMARK_SAMPLE_FRAMES),
        })
    }

    fn record(&mut self, sample: BenchmarkSample) -> bool {
        if self.warmup_left > 0 {
            self.warmup_left -= 1;
            return false;
        }
        self.samples.push(sample);
        self.samples.len() == BENCHMARK_SAMPLE_FRAMES
    }

    fn print(&self, workers: usize, advanced_lighting: bool) {
        fn percentile(mut values: Vec<f32>, fraction: f32) -> f32 {
            values.sort_unstable_by(f32::total_cmp);
            let index = ((values.len() - 1) as f32 * fraction).ceil() as usize;
            values[index]
        }
        fn mean(values: impl Iterator<Item = f32>, count: usize) -> f32 {
            values.sum::<f32>() / count as f32
        }
        let count = self.samples.len();
        let frames: Vec<f32> = self.samples.iter().map(|s| s.frame_ms).collect();
        let mean_frame = mean(frames.iter().copied(), count);
        println!(
            "BENCH scene={} frames={} resolution={}x{} workers={} advanced={}",
            self.scene, count, WIDTH, HEIGHT, workers, advanced_lighting
        );
        if self.scene == "race" {
            const LABELS: [&str; 7] = [
                "cielo", "terreno", "bosques", "arcos", "pista", "jugador", "rivales",
            ];
            for (index, label) in LABELS.iter().enumerate() {
                println!(
                    "BENCH race_category={label} geometry={:.4} vertices={:.1} triangles={:.1}",
                    mean(
                        self.samples.iter().map(|s| s.race_geometry_ms[index]),
                        count
                    ),
                    mean(
                        self.samples.iter().map(|s| s.race_vertices[index] as f32),
                        count
                    ),
                    mean(
                        self.samples.iter().map(|s| s.race_triangles[index] as f32),
                        count
                    ),
                );
            }
        }
        println!(
            "BENCH frame avg={mean_frame:.4} min={:.4} max={:.4} p95={:.4} p99={:.4} fps={:.3}",
            percentile(frames.clone(), 0.0),
            percentile(frames.clone(), 1.0),
            percentile(frames.clone(), 0.95),
            percentile(frames, 0.99),
            1_000.0 / mean_frame
        );
        println!(
            "BENCH stages update_scene={:.4} geometry={:.4} sort={:.4} raster={:.4} post={:.4} copy={:.4} present={:.4}",
            mean(self.samples.iter().map(|s| s.update_scene_ms), count),
            mean(self.samples.iter().map(|s| s.geometry_ms), count),
            mean(self.samples.iter().map(|s| s.sort_ms), count),
            mean(self.samples.iter().map(|s| s.raster_ms), count),
            mean(self.samples.iter().map(|s| s.post_ms), count),
            mean(self.samples.iter().map(|s| s.copy_ms), count),
            mean(self.samples.iter().map(|s| s.present_ms), count)
        );
        println!(
            "BENCH counters visible={:.1} culled={:.1} vertices={:.1} triangles={:.1}",
            mean(self.samples.iter().map(|s| s.models_visible as f32), count),
            mean(self.samples.iter().map(|s| s.models_culled as f32), count),
            mean(self.samples.iter().map(|s| s.vertices as f32), count),
            mean(self.samples.iter().map(|s| s.triangles as f32), count)
        );
    }
}
const START_FULLSCREEN: bool = true;
const NEAR_PLANE: f32 = 0.2;
const FAR_PLANE: f32 = 750.0;
const VERTICAL_FOV: f32 = 60.0_f32.to_radians();
const LIGHT_SPEED: f32 = 600.0;
const PLANET_ORBIT_SPEED: f32 = 0.075;
const SUN_POSITION_X: f32 = -520.0;
const SUN_VISUAL_SCALE: f32 = 1.35;
const SOLAR_SHIP_EXPLOSION_DURATION: f32 = 1.35;
const PLANET_ORBIT_RADIUS: f32 = 260.0;
const SPIKY_PLANET_ORBIT_SPEED: f32 = -0.075;
const SPIKY_PLANET_ORBIT_RADIUS: f32 = 440.0;
const SPIKY_PLANET_ORBIT_PHASE: f32 = PI * 0.5;
const SPIKY_PLANET_ORBIT_INCLINATION: f32 = 0.30;
const RACE_PLANET_ORBIT_SPEED: f32 = 0.075;
const RACE_PLANET_ORBIT_RADIUS: f32 = 620.0;
const RACE_PLANET_ORBIT_PHASE: f32 = PI * 1.18;
const RACE_PLANET_ORBIT_INCLINATION: f32 = -0.16;
const SPIKY_PLANET_RADIUS: f32 = 39.0;
const WATER_PLANET_ROTATION_SPEED: f32 = 0.55;
const SPIKY_PLANET_ROTATION_SPEED: f32 = 0.48;
const RACE_PLANET_ROTATION_SPEED: f32 = 0.38;
const WATER_PLANET_RADIUS: f32 = 36.0;
const COLLISION_MARGIN: f32 = 4.0;
const AVOIDANCE_DISTANCE: f32 = 52.0;
const ORBIT_LIMIT_RADIUS: f32 = 60.0;
const ORBIT_FLIGHT_ALTITUDE: f32 = 8.0;
// Each authored Vesper structure has a different visual center and footprint.
// Use structure-centered horizontal orbits instead of the complete OBJ bounds.
const VESPER_ORBIT_HEIGHTS: [f32; 2] = [0.0, 12.0];
const VESPER_ORBIT_RADII: [f32; 2] = [30.0, 20.0];
const VESPER_SYSTEM_CENTER: Vec3 = Vec3::new(0.0, 48.0, 0.0);
const VESPER_SYSTEM_ORBIT_RADII: [f32; 2] = [275.0, 440.0];
const VESPER_SYSTEM_ORBIT_SPEEDS: [f32; 2] = [0.075, -0.075];
const VESPER_SYSTEM_ORBIT_PHASES: [f32; 2] = [0.45, 3.35];
const VESPER_BLACK_HOLE_GRAVITY_RADIUS: f32 = 175.0;
const VESPER_BLACK_HOLE_EVENT_HORIZON: f32 = 25.0;
const VESPER_BLACK_HOLE_ABSORPTION_DURATION: f32 = 1.55;
// The castle asset's 58-unit bounding sphere includes its tall flag and portal;
// its rocky equator is much narrower and is the surface used for free-flight orbit.
const NOVA_CASTLE_ORBIT_RADIUS: f32 = 34.0;
const ORBIT_CAMERA_DISTANCE: f32 = 100.0;
const ORBIT_CAMERA_HEIGHT: f32 = 28.0;
const HOME_ORBIT_TARGET_HEIGHT: f32 = 3.0;
const VESPER_ORBIT_CAMERA_DISTANCE: f32 = 115.0;
const VESPER_ORBIT_CAMERA_HEIGHT: f32 = 32.0;
#[cfg(test)]
const WATER_LEVEL_LENGTH: f32 = 562.0;
const WATER_BOSS_ARENA_START: f32 = 450.0;
const WATER_BOSS_POSITION_PROGRESS: f32 = 486.0;
const WATER_BOSS_MAX_HEALTH: i32 = 60;
const WATER_BOSS_EXPLOSION_DURATION: f32 = 2.8;
const WATER_REWARD_PROGRESS: f32 = 532.0;
const WATER_REWARD_DOOR_PROGRESS: f32 = 502.0;
const WATER_REWARD_DOOR_OPEN_TIME: f32 = 1.8;
const WATER_REWARD_DOOR_PASSABLE: f32 = 0.98;
const WATER_MISSION_COMPLETE_TIME: f32 = 3.4;
const WATER_SUBMARINE_MAX_HEALTH: i32 = 6;
const WATER_ENEMY_SHOT_SPEED: f32 = 24.0;
const WATER_LEVEL_WIDTH_SCALE: f32 = 1.45;
const WATER_LEVEL_HALF_WIDTH: f32 = 14.5;
const WATER_LEVEL_SOFT_BORDER: f32 = 12.2;
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
const MIN_SURFACE_TEXTURE_SIZE: usize = 512;
// Keep the optimized renderer at the native presentation resolution. Dynamic
// downscaling from main conflicts with optimization3's visual contract.
const RESOLUTION_SCALES: [f32; 1] = [1.0];

fn water_reward_position() -> Vec3 {
    Vec3::new(-WATER_REWARD_PROGRESS, 7.2, 0.0)
}

fn render_worker_count() -> usize {
    static WORKER_COUNT: OnceLock<usize> = OnceLock::new();
    *WORKER_COUNT.get_or_init(|| {
        std::env::var("GRAPH_BENCH_WORKERS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or_else(|| thread::available_parallelism().map_or(1, usize::from))
    })
}

fn transform_worker_count(vertex_count: usize, available_workers: usize) -> usize {
    if vertex_count < 4_096 || available_workers <= 1 {
        1
    } else {
        available_workers.min(vertex_count.div_ceil(MIN_VERTICES_PER_TRANSFORM_WORKER))
    }
}

struct DynamicResolution {
    enabled: bool,
    scale_index: usize,
    fps_average: f32,
    frame_ms_average: f32,
    render_ms_average: f32,
    jitter_ms_average: f32,
    low_time: f32,
    high_time: f32,
    cooldown: f32,
    status: &'static str,
}

impl DynamicResolution {
    fn new() -> Self {
        Self {
            enabled: true,
            scale_index: 0,
            fps_average: 60.0,
            frame_ms_average: 1_000.0 / 60.0,
            render_ms_average: 0.0,
            jitter_ms_average: 0.0,
            low_time: 0.0,
            high_time: 0.0,
            cooldown: 0.0,
            status: "ESTABLE",
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
        self.status = if self.enabled { "ESTABLE" } else { "OFF" };
        if !self.enabled && self.scale_index != 0 {
            self.scale_index = 0;
            return true;
        }
        false
    }

    fn prefer_racing_resolution(&mut self) -> bool {
        self.prefer_native_resolution("HD NATIVO")
    }

    fn prefer_native_resolution(&mut self, label: &'static str) -> bool {
        let changed = self.scale_index != 0;
        self.scale_index = 0;
        self.low_time = 0.0;
        self.high_time = 0.0;
        self.cooldown = 0.0;
        self.status = if self.enabled { label } else { "OFF" };
        changed
    }

    fn update(&mut self, frame_time: f32, previous_render: RenderStats) -> bool {
        // Ignore extreme pause/debugger spikes without hiding sustained load.
        let frame_time = frame_time.clamp(0.000_1, 0.1);
        let control_dt = frame_time.min(1.0 / 30.0);
        let frame_ms = (frame_time * 1_000.0).min(self.frame_ms_average + 12.0);
        let fps = 1_000.0 / frame_ms;
        let smoothing = 1.0 - (-frame_time * 2.5).exp();
        self.fps_average += (fps - self.fps_average) * smoothing;
        let previous_frame_average = self.frame_ms_average;
        self.frame_ms_average += (frame_ms - self.frame_ms_average) * smoothing;
        self.jitter_ms_average +=
            ((frame_ms - previous_frame_average).abs() - self.jitter_ms_average) * smoothing;
        let render_ms =
            previous_render.geometry_ms + previous_render.sort_ms + previous_render.raster_ms;
        if render_ms > 0.0 {
            self.render_ms_average += (render_ms - self.render_ms_average) * smoothing;
        }
        self.cooldown = (self.cooldown - frame_time).max(0.0);
        if !self.enabled {
            self.status = "OFF";
            return false;
        }
        if self.cooldown > 0.0 {
            self.status = "COOLDOWN";
            return false;
        }

        let overloaded = self.frame_ms_average > 19.0 || self.render_ms_average > 16.5;
        let comfortable = self.frame_ms_average < 14.8
            && (self.render_ms_average == 0.0 || self.render_ms_average < 11.5)
            && self.jitter_ms_average < 3.5;
        if overloaded {
            self.low_time += control_dt;
            self.high_time = (self.high_time - control_dt * 2.0).max(0.0);
            self.status = "BAJANDO";
        } else if comfortable {
            self.high_time += control_dt;
            self.low_time = (self.low_time - control_dt).max(0.0);
            self.status = "SUBIENDO";
        } else {
            self.low_time = (self.low_time - control_dt * 0.5).max(0.0);
            self.high_time = (self.high_time - control_dt * 0.5).max(0.0);
            self.status = "ESTABLE";
        }

        if self.low_time >= 0.8 && self.scale_index + 1 < RESOLUTION_SCALES.len() {
            self.scale_index += 1;
            self.low_time = 0.0;
            self.high_time = 0.0;
            self.cooldown = 2.5;
            self.status = "REDUCIDA";
            true
        } else if self.high_time >= 3.0 && self.scale_index > 0 {
            self.scale_index -= 1;
            self.high_time = 0.0;
            self.low_time = 0.0;
            self.cooldown = 4.0;
            self.status = "AUMENTADA";
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
    let scale_x =
        (source.width.saturating_sub(1)) as f32 / (target.width.saturating_sub(1)).max(1) as f32;
    let scale_y =
        (source.height.saturating_sub(1)) as f32 / (target.height.saturating_sub(1)).max(1) as f32;
    for y in 0..target.height {
        let source_y = y as f32 * scale_y;
        let y0 = source_y.floor() as usize;
        let y1 = (y0 + 1).min(source.height - 1);
        let fy = source_y - y0 as f32;
        let target_row = y * target.width;
        for x in 0..target.width {
            let source_x = x as f32 * scale_x;
            let x0 = source_x.floor() as usize;
            let x1 = (x0 + 1).min(source.width - 1);
            let fx = source_x - x0 as f32;
            let colors = [
                source.pixels[y0 * source.width + x0],
                source.pixels[y0 * source.width + x1],
                source.pixels[y1 * source.width + x0],
                source.pixels[y1 * source.width + x1],
            ];
            let interpolate = |shift: u32| {
                let channel = |color: u32| ((color >> shift) & 255_u32) as f32;
                let top = channel(colors[0]) + (channel(colors[1]) - channel(colors[0])) * fx;
                let bottom = channel(colors[2]) + (channel(colors[3]) - channel(colors[2])) * fx;
                let bilinear = top + (bottom - top) * fy;
                // Mild reconstruction sharpening offsets bilinear blur without
                // reintroducing the blockiness of nearest-neighbor scaling.
                let neighborhood = colors.iter().map(|&color| channel(color)).sum::<f32>() * 0.25;
                (bilinear + (bilinear - neighborhood) * 0.14)
                    .clamp(0.0, 255.0)
                    .round() as u32
            };
            target.pixels[target_row + x] =
                (interpolate(16) << 16) | (interpolate(8) << 8) | interpolate(0);
        }
    }
}

fn upscale_split_scene(left: &Framebuffer, right: &Framebuffer, target: &mut Framebuffer) {
    let split = target.width / 2;
    for y in 0..target.height {
        let source_y = y * left.height / target.height;
        for x in 0..target.width {
            let (source, local_x, viewport_width) = if x < split {
                (left, x, split)
            } else {
                (right, x - split, target.width - split)
            };
            let source_x = local_x * source.width / viewport_width.max(1);
            target.pixels[y * target.width + x] = source.pixels
                [source_y.min(source.height - 1) * source.width + source_x.min(source.width - 1)];
        }
        target.pixels[y * target.width + split.saturating_sub(1)] = 0x46e6ff;
        target.pixels[y * target.width + split.min(target.width - 1)] = 0x46e6ff;
    }
}

struct HybridRayTracing {
    history: Vec<u32>,
    width: usize,
    height: usize,
}

impl HybridRayTracing {
    fn new() -> Self {
        Self {
            history: Vec::new(),
            width: 0,
            height: 0,
        }
    }

    fn apply_high(&mut self, framebuffer: &mut Framebuffer) {
        let width = framebuffer.width;
        let height = framebuffer.height;
        if width < 4 || height < 4 {
            return;
        }
        if self.width != width || self.height != height || self.history.len() != width * height {
            self.width = width;
            self.height = height;
            self.history.clone_from(&framebuffer.pixels);
        }

        let history = &self.history;
        let (pixels, depth) = framebuffer.buffers_mut();
        // Half-resolution rays (one sample per 2x2 block), then spatially
        // resolve the result across the block. Depth remains full resolution.
        for y in (2..height - 2).step_by(2) {
            for x in (2..width - 2).step_by(2) {
                let index = y * width + x;
                let surface_depth = depth[index];
                if !surface_depth.is_finite() || surface_depth <= 0.0 {
                    continue;
                }

                let gradient_x = depth[index + 1] - depth[index - 1];
                let gradient_y = depth[index + width] - depth[index - width];
                let reflection_x = if gradient_x > 0.0 { -1 } else { 1 };
                let reflection_y = if gradient_y > 0.0 { -1 } else { 1 };

                let mut shadow = 0.0_f32;
                for step in 1..=8_i32 {
                    let ray_x = x as i32 - step * 2;
                    let ray_y = y as i32 - step;
                    if ray_x < 0 || ray_y < 0 {
                        break;
                    }
                    let ray_depth = depth[ray_y as usize * width + ray_x as usize];
                    let expected = surface_depth * (1.0 + step as f32 * 0.012);
                    if ray_depth.is_finite() && ray_depth > expected + 0.0015 {
                        shadow = (0.34 - step as f32 * 0.025).max(0.08);
                        break;
                    }
                }

                let mut reflection = None;
                for step in 2..=10_i32 {
                    let ray_x = x as i32 + reflection_x * step * 2;
                    let ray_y = y as i32 + reflection_y * step;
                    if ray_x < 0 || ray_y < 0 || ray_x >= width as i32 || ray_y >= height as i32 {
                        break;
                    }
                    let ray_index = ray_y as usize * width + ray_x as usize;
                    let ray_depth = depth[ray_index];
                    let expected = surface_depth * (1.0 - step as f32 * 0.014);
                    if ray_depth.is_finite() && ray_depth > expected + 0.001 {
                        reflection = Some(history[ray_index]);
                        break;
                    }
                }

                let bounce_index =
                    (y.saturating_sub(2) * width + (x + 2).min(width - 1)).min(history.len() - 1);
                let bounce = history[bounce_index];
                let source = pixels[index];
                let channel = |color: u32, shift: u32| ((color >> shift) & 255_u32) as f32;
                let shade_channel = |shift: u32| {
                    let base = channel(source, shift) * (1.0 - shadow);
                    let indirect = channel(bounce, shift) * 0.055;
                    let reflected = reflection.map_or(0.0, |color| channel(color, shift) * 0.16);
                    (base + indirect + reflected).clamp(0.0, 255.0) as u32
                };
                let traced = (shade_channel(16) << 16) | (shade_channel(8) << 8) | shade_channel(0);
                for block_y in y..(y + 2).min(height) {
                    for block_x in x..(x + 2).min(width) {
                        let block_index = block_y * width + block_x;
                        if depth[block_index].is_finite() && depth[block_index] > 0.0 {
                            pixels[block_index] = traced;
                        }
                    }
                }
            }
        }
        self.history.copy_from_slice(pixels);
    }
}

#[derive(Clone, Copy)]
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
    HourglassRescue,
    Racing,
    TowerGuard,
    LavaCastle,
    LavaCastleInterior,
}

impl GameMode {
    fn music_track(self) -> Option<minigame_audio::Track> {
        use minigame_audio::Track;
        match self {
            Self::CoreGuardian => Some(Track::CoreGuardian),
            Self::WaterLevel => Some(Track::Aquatic),
            Self::Racing => Some(Track::Racing),
            Self::TowerGuard => Some(Track::TowerGuard),
            Self::HourglassRescue => Some(Track::Hourglass),
            _ => None,
        }
    }

    fn is_planet_minigame(self) -> bool {
        self.music_track().is_some()
    }
}

struct WaterLevelState {
    progress: f32,
    lateral: f32,
    height: f32,
    elapsed: f32,
    enemies: Vec<WaterEnemy>,
    defeated_enemies: usize,
    boss_health: i32,
    boss_defeated: bool,
    boss_hit_flash: f32,
    boss_explosion_timer: f32,
    boss_explosion_position: Vec3,
    reward_door_open: f32,
    enemy_projectiles: Vec<Projectile>,
    enemy_fire_timer: f32,
    attack_serial: usize,
    hull_health: i32,
    hit_invulnerability: f32,
    dragon_ball_collected: bool,
    mission_complete_timer: f32,
    mission_complete_position: Vec3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WaterStage {
    Entrance,
    Ruins,
    Minefield,
    SpeedTunnel,
    OpenArena,
    SharkBoss,
    RewardChamber,
}

impl WaterStage {
    fn label(self) -> &'static str {
        match self {
            Self::Entrance => "1 CAVERNA DE ENTRADA",
            Self::Ruins => "2 CORREDOR DE RUINAS",
            Self::Minefield => "3 CAMPO MINADO",
            Self::SpeedTunnel => "4 TUNEL DE ALTA VELOCIDAD",
            Self::OpenArena => "5 ARENA ABIERTA",
            Self::SharkBoss => "6 TIBURON MECANICO",
            Self::RewardChamber => "7 SANTUARIO DE LA ESFERA",
        }
    }
}

struct WaterEnemy {
    kind: usize,
    progress: f32,
    lateral: f32,
    height: f32,
    phase: f32,
    formation: u8,
    formation_slot: u8,
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
        let mut enemies = Vec::new();
        let mut add_wave =
            |count: usize, start: f32, group_spacing: f32, kinds: &[usize], formation: u8| {
                let base = enemies.len();
                enemies.extend((0..count).map(|index| WaterEnemy {
                    kind: kinds[index % kinds.len()],
                    progress: start + (index / 4) as f32 * group_spacing,
                    lateral: 0.0,
                    height: 6.5,
                    phase: (base + index) as f32 * 1.37,
                    formation,
                    formation_slot: (index % 4) as u8,
                    alive: true,
                }));
            };
        add_wave(4, 32.0, 18.0, &[2, 1], 0);
        add_wave(8, 92.0, 34.0, &[1, 2, 1], 1);
        add_wave(16, 178.0, 23.0, &[0], 3);
        add_wave(6, 290.0, 38.0, &[0, 2], 2);
        add_wave(10, 370.0, 28.0, &[1, 2, 0], 0);
        Self {
            progress: 3.0,
            lateral: 0.0,
            height: 4.5,
            elapsed: 0.0,
            enemies,
            defeated_enemies: 0,
            boss_health: WATER_BOSS_MAX_HEALTH,
            boss_defeated: false,
            boss_hit_flash: 0.0,
            boss_explosion_timer: 0.0,
            boss_explosion_position: Vec3::default(),
            reward_door_open: 0.0,
            enemy_projectiles: Vec::new(),
            enemy_fire_timer: 1.4,
            attack_serial: 0,
            hull_health: WATER_SUBMARINE_MAX_HEALTH,
            hit_invulnerability: 0.0,
            dragon_ball_collected: false,
            mission_complete_timer: 0.0,
            mission_complete_position: Vec3::default(),
        }
    }

    fn stage(&self) -> WaterStage {
        if self.boss_defeated
            && self.reward_door_open >= WATER_REWARD_DOOR_PASSABLE
            && self.progress >= 475.0
        {
            return WaterStage::RewardChamber;
        }
        match self.progress {
            progress if progress < 75.0 => WaterStage::Entrance,
            progress if progress < 170.0 => WaterStage::Ruins,
            progress if progress < 270.0 => WaterStage::Minefield,
            progress if progress < 360.0 => WaterStage::SpeedTunnel,
            progress if progress < WATER_BOSS_ARENA_START => WaterStage::OpenArena,
            _ => WaterStage::SharkBoss,
        }
    }

    fn boss_position(&self) -> Vec3 {
        let lunge = ((self.elapsed * 0.72).sin().max(0.0)).powf(5.0) * 9.0;
        Vec3::new(
            -(WATER_BOSS_POSITION_PROGRESS - lunge),
            7.2 + (self.elapsed * 1.15).sin() * 2.8,
            (self.elapsed * 0.78).sin() * 10.4,
        )
    }

    fn update(&mut self, ship: &mut Ship, input: &ShipInput, delta_time: f32) {
        self.elapsed += delta_time;
        self.boss_hit_flash = (self.boss_hit_flash - delta_time).max(0.0);
        let explosion_time_before_update = self.boss_explosion_timer;
        self.boss_explosion_timer = (self.boss_explosion_timer - delta_time).max(0.0);
        if self.boss_defeated && self.boss_explosion_timer <= 0.0 {
            let door_delta = if explosion_time_before_update > 0.0 {
                (delta_time - explosion_time_before_update).max(0.0)
            } else {
                delta_time
            };
            self.reward_door_open =
                (self.reward_door_open + door_delta / WATER_REWARD_DOOR_OPEN_TIME).min(1.0);
        }
        self.hit_invulnerability = (self.hit_invulnerability - delta_time).max(0.0);
        if self.dragon_ball_collected {
            self.mission_complete_timer = (self.mission_complete_timer - delta_time).max(0.0);
            ship.speed = 0.0;
            ship.position = self.mission_complete_position;
            ship.yaw = 0.0;
            ship.pitch = 0.0;
            ship.roll = 0.0;
            return;
        }
        let speed_boost = if self.stage() == WaterStage::SpeedTunnel {
            1.75
        } else {
            1.0
        };
        self.progress += ship.speed * WATER_LEVEL_SPEED_FACTOR * speed_boost * delta_time;
        if !self.boss_defeated || self.boss_explosion_timer > 0.0 {
            self.progress = self.progress.min(WATER_BOSS_ARENA_START);
        } else if self.reward_door_open < WATER_REWARD_DOOR_PASSABLE {
            self.progress = self.progress.min(WATER_REWARD_DOOR_PROGRESS - 4.0);
        } else if !self.dragon_ball_collected {
            self.progress = self.progress.min(WATER_REWARD_PROGRESS);
        }
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

        // Funnel the on-rails movement through the physical doorway. If the
        // submarine reaches the frame while outside the safe aperture, hold it
        // in front of the door and gently recenter instead of clipping/sticking.
        let approaching_door = self.progress >= WATER_REWARD_DOOR_PROGRESS - 7.0
            && self.progress <= WATER_REWARD_DOOR_PROGRESS + 3.0;
        if approaching_door {
            let aperture_clear = self.lateral.abs() <= 3.7 && self.height <= 8.0;
            if self.reward_door_open < WATER_REWARD_DOOR_PASSABLE || !aperture_clear {
                self.progress = self.progress.min(WATER_REWARD_DOOR_PROGRESS - 4.0);
                let recenter = (delta_time * 3.8).clamp(0.0, 1.0);
                self.lateral += (0.0 - self.lateral) * recenter;
                self.height += (5.8 - self.height) * recenter;
            }
        }

        // The reward chamber has narrower walls than the outdoor arena.
        if self.progress > WATER_REWARD_DOOR_PROGRESS + 2.0 {
            self.lateral = self.lateral.clamp(-11.7, 11.7);
            self.height = self.height.clamp(2.4, 11.4);
        }

        let (route_curve, route_height) = water_route(self.progress);
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

        if self.boss_defeated && !self.dragon_ball_collected {
            let reward_offset = ship.position.sub(water_reward_position());
            if reward_offset.dot(reward_offset) <= 6.0 * 6.0 {
                self.dragon_ball_collected = true;
                self.mission_complete_timer = WATER_MISSION_COMPLETE_TIME;
                self.mission_complete_position = ship.position;
                ship.speed = 0.0;
            }
        }

        for projectile in self
            .enemy_projectiles
            .iter_mut()
            .filter(|shot| shot.lifetime > 0.0)
        {
            projectile.position = projectile.position.add(
                projectile
                    .direction
                    .mul(WATER_ENEMY_SHOT_SPEED * delta_time),
            );
            projectile.lifetime -= delta_time;
        }
        self.update_enemy_attacks(ship, delta_time);
    }

    fn spawn_enemy_projectile(&mut self, position: Vec3, target: Vec3, scale: f32) {
        let direction = target.sub(position).normalize();
        let projectile = Projectile {
            position,
            direction,
            lifetime: 4.5,
            scale,
        };
        if let Some(slot) = self
            .enemy_projectiles
            .iter_mut()
            .find(|shot| shot.lifetime <= 0.0)
        {
            *slot = projectile;
        } else if self.enemy_projectiles.len() < 48 {
            self.enemy_projectiles.push(projectile);
        }
    }

    fn update_enemy_attacks(&mut self, ship: &Ship, delta_time: f32) {
        self.enemy_fire_timer -= delta_time;
        if self.enemy_fire_timer > 0.0 {
            return;
        }

        let boss_active = self.stage() == WaterStage::SharkBoss && !self.boss_defeated;
        if boss_active {
            let boss_position = self.boss_position();
            let mouth = boss_position.add(Vec3::new(3.5, -0.2, 0.0));
            match self.attack_serial % 4 {
                // A compact aimed burst from the shark's mouth.
                0 => {
                    for lateral in [-1.8, 0.0, 1.8] {
                        self.spawn_enemy_projectile(
                            mouth.add(Vec3::new(0.0, 0.0, lateral)),
                            ship.position,
                            0.9,
                        );
                    }
                }
                // A broad fan forces the player to move through a visible gap.
                1 => {
                    for lateral in [-10.0, -6.5, -3.2, 0.0, 3.2, 6.5, 10.0] {
                        self.spawn_enemy_projectile(
                            mouth,
                            ship.position.add(Vec3::new(0.0, 0.0, lateral)),
                            0.72,
                        );
                    }
                }
                // Alternating missile-rack salvo from both sides of the body.
                2 => {
                    for side in [-1.0_f32, 1.0] {
                        let rack = boss_position.add(Vec3::new(0.5, 3.2, side * 5.8));
                        for offset in [-2.8, 0.0, 2.8] {
                            self.spawn_enemy_projectile(
                                rack,
                                ship.position.add(Vec3::new(0.0, offset, side * 1.8)),
                                1.05,
                            );
                        }
                    }
                }
                // Cross pattern closes the center and pressures vertical dodges.
                _ => {
                    for (vertical, lateral) in
                        [(0.0, 0.0), (-5.5, 0.0), (5.5, 0.0), (0.0, -7.0), (0.0, 7.0)]
                    {
                        self.spawn_enemy_projectile(
                            mouth,
                            ship.position.add(Vec3::new(0.0, vertical, lateral)),
                            0.82,
                        );
                    }
                }
            }
            self.attack_serial = self.attack_serial.wrapping_add(1);
            self.enemy_fire_timer = if self.boss_health <= WATER_BOSS_MAX_HEALTH / 2 {
                0.82
            } else {
                1.08
            };
            return;
        }

        let mut attackers: Vec<(usize, Vec3)> = self
            .enemies
            .iter()
            .enumerate()
            .filter(|(_, enemy)| enemy.alive && enemy.kind != 0)
            .map(|(index, enemy)| (index, enemy.position(self.elapsed)))
            .filter(|(_, position)| {
                let ahead = self.progress + position.x;
                (-105.0..4.0).contains(&ahead)
            })
            .collect();
        attackers.sort_by_key(|(index, _)| *index);
        if !attackers.is_empty() {
            let start = self.attack_serial % attackers.len();
            for offset in 0..attackers.len().min(2) {
                let (_, origin) = attackers[(start + offset) % attackers.len()];
                let travel_time = origin
                    .sub(ship.position)
                    .dot(origin.sub(ship.position))
                    .sqrt()
                    / WATER_ENEMY_SHOT_SPEED;
                let predicted_target =
                    ship.position
                        .add(Vec3::new(-ship.speed * 0.18 * travel_time, 0.0, 0.0));
                self.spawn_enemy_projectile(origin, predicted_target, 0.55);
            }
            self.attack_serial = self.attack_serial.wrapping_add(2);
        }
        self.enemy_fire_timer = 1.25;
    }
}

fn water_route(progress: f32) -> (f32, f32) {
    match progress {
        p if p < 75.0 => ((p * 0.075).sin() * 1.4, (p * 0.035).sin() * 0.8),
        p if p < 170.0 => (
            (p * 0.115).sin() * 4.2 + (p * 0.041).sin() * 1.6,
            (p * 0.052).sin() * 1.8,
        ),
        p if p < 270.0 => ((p * 0.082).sin() * 3.0, (p * 0.071).sin() * 2.0),
        p if p < 360.0 => ((p * 0.028).sin() * 1.1, (p * 0.032).sin() * 0.7),
        p if p < WATER_BOSS_ARENA_START => ((p * 0.055).sin() * 2.0, (p * 0.024).sin() * 1.2),
        _ => (0.0, 0.0),
    }
}

impl WaterEnemy {
    fn position(&self, time: f32) -> Vec3 {
        let (route_curve, route_height) = water_route(self.progress);
        let slot = self.formation_slot as usize;
        let (longitudinal, formation_lateral, formation_height) = match self.formation {
            // Arrowhead: a leader followed by two wings and a rear guard.
            0 => (
                [0.0, 4.0, 4.0, 8.0][slot],
                [0.0, -5.2, 5.2, 0.0][slot],
                [1.8, 0.0, 0.0, -1.5][slot],
            ),
            // Diamond patrol formation.
            1 => (
                [0.0, 3.5, 3.5, 7.0][slot],
                [0.0, -5.8, 5.8, 0.0][slot],
                [2.3, 0.0, 0.0, -2.0][slot],
            ),
            // Horizontal attack line used by the high-speed tunnel.
            2 => (
                [0.0, 0.8, 0.8, 0.0][slot],
                [-8.0, -2.7, 2.7, 8.0][slot],
                [0.0, 1.2, -1.2, 0.0][slot],
            ),
            // Mine wall: fixed staggered square that forces a deliberate gap choice.
            _ => (
                [0.0, 0.0, 4.5, 4.5][slot],
                [-5.8, 5.8, -5.8, 5.8][slot],
                [2.2, 2.2, -1.9, -1.9][slot],
            ),
        };
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
            -(self.progress + longitudinal),
            self.height + formation_height + route_height + vertical_motion,
            self.lateral + formation_lateral + route_curve + lateral_motion,
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

fn stretch_water_level_width(model: &mut Model, factor: f32) {
    for vertex in &mut model.vertices {
        vertex.y *= factor;
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
    // The final chamber supplies its own walls and doorway constraints. The
    // extended source level may still contain legacy obstacle colliders here,
    // which otherwise push the submarine into the new door frame.
    if state.progress >= WATER_REWARD_DOOR_PROGRESS - 8.0 {
        return;
    }
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
    let mut submarine_destroyed = false;
    if state.hit_invulnerability <= 0.0 {
        for shot in state
            .enemy_projectiles
            .iter_mut()
            .filter(|shot| shot.lifetime > 0.0)
        {
            let offset = shot.position.sub(ship.position);
            let hit_radius = ship.spec.collision_radius + 0.75 * shot.scale;
            if offset.dot(offset) <= hit_radius * hit_radius {
                shot.lifetime = 0.0;
                state.hull_health -= 1;
                state.hit_invulnerability = 0.85;
                state.lateral = (state.lateral + offset.z.signum() * -2.2)
                    .clamp(-WATER_LEVEL_HALF_WIDTH, WATER_LEVEL_HALF_WIDTH);
                submarine_destroyed = state.hull_health <= 0;
                break;
            }
        }
    }
    if submarine_destroyed {
        state.hull_health = WATER_SUBMARINE_MAX_HEALTH;
        state.progress = (state.progress - 42.0).max(3.0);
        state.lateral = 0.0;
        state.height = 5.5;
        state.hit_invulnerability = 2.0;
        for shot in &mut state.enemy_projectiles {
            shot.lifetime = 0.0;
        }
        submarine_destroyed = false;
    }

    let boss_active = state.stage() == WaterStage::SharkBoss && !state.boss_defeated;
    let boss_position = state.boss_position();
    if boss_active {
        for projectile in projectiles.iter_mut().filter(|shot| shot.lifetime > 0.0) {
            let offset = projectile.position.sub(boss_position);
            let hit_radius = 8.5 + projectile.scale * 0.45;
            if offset.dot(offset) <= hit_radius * hit_radius {
                let damage = if projectile.scale > 2.0 { 3 } else { 1 };
                state.boss_health = (state.boss_health - damage).max(0);
                state.boss_hit_flash = 0.16;
                projectile.lifetime = 0.0;
                if state.boss_health == 0 {
                    state.boss_defeated = true;
                    state.boss_explosion_timer = WATER_BOSS_EXPLOSION_DURATION;
                    state.boss_explosion_position = boss_position;
                    state.defeated_enemies += 1;
                    for shot in &mut state.enemy_projectiles {
                        shot.lifetime = 0.0;
                    }
                }
            }
        }

        let offset = ship.position.sub(boss_position);
        let separation = ship.spec.collision_radius + 8.0;
        let distance_squared = offset.dot(offset);
        if distance_squared < separation * separation {
            let lateral = Vec3::new(0.0, offset.y, offset.z);
            let lateral_length = lateral.dot(lateral).sqrt();
            let normal = if lateral_length > 0.001 {
                lateral.mul(1.0 / lateral_length)
            } else {
                Vec3::new(0.0, 1.0, 0.0)
            };
            let correction = normal.mul(separation - distance_squared.sqrt() + 0.5);
            state.height =
                (state.height + correction.y).clamp(WATER_LEVEL_MIN_HEIGHT, WATER_LEVEL_MAX_HEIGHT);
            state.lateral = (state.lateral + correction.z)
                .clamp(-WATER_LEVEL_HALF_WIDTH, WATER_LEVEL_HALF_WIDTH);
            ship.position = ship.position.add(correction);
            if state.hit_invulnerability <= 0.0 {
                state.hull_health -= 2;
                state.hit_invulnerability = 1.15;
                submarine_destroyed = state.hull_health <= 0;
            }
        }
    }

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
        if !state.enemies[enemy_index].alive {
            continue;
        }
        let enemy_position = enemy_positions[enemy_index];
        let offset = ship.position.sub(enemy_position);
        let separation = ship.spec.collision_radius + state.enemies[enemy_index].collision_radius();
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

        if state.hit_invulnerability <= 0.0 {
            state.hull_health -= 1;
            state.hit_invulnerability = 0.95;
            // Small enemies are destroyed by the impact, preventing the same
            // body from trapping the submarine after its invulnerability ends.
            state.enemies[enemy_index].alive = false;
            state.defeated_enemies += 1;
            submarine_destroyed = state.hull_health <= 0;
            break;
        }
    }

    if submarine_destroyed {
        state.hull_health = WATER_SUBMARINE_MAX_HEALTH;
        state.progress = (state.progress - 42.0).max(3.0);
        state.lateral = 0.0;
        state.height = 5.5;
        state.hit_invulnerability = 2.0;
        ship.position = Vec3::new(-state.progress, state.height, state.lateral);
        for shot in &mut state.enemy_projectiles {
            shot.lifetime = 0.0;
        }
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
        #[link(name = "user32")]
        unsafe extern "system" {
            fn GetSystemMetrics(index: i32) -> i32;
            fn SetProcessDPIAware() -> i32;
        }

        const SM_CXSCREEN: i32 = 0;
        const SM_CYSCREEN: i32 = 1;
        // Request physical monitor pixels before querying its size. Without
        // this, Windows can expose a smaller DPI-virtualized resolution and
        // then enlarge the finished frame, which makes fullscreen look soft.
        static DPI_AWARE: OnceLock<()> = OnceLock::new();
        DPI_AWARE.get_or_init(|| unsafe {
            SetProcessDPIAware();
        });
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
            // Fill the complete physical display. This keeps the existing
            // render resolution and cost while avoiding fullscreen side bars.
            scale_mode: ScaleMode::Stretch,
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

struct SurfaceTexture {
    width: usize,
    height: usize,
    pixels: Vec<[u8; 3]>,
}

impl SurfaceTexture {
    fn load_png(path: &str, target_size: usize) -> Option<Self> {
        let file = File::open(path).ok()?;
        let mut decoder = png::Decoder::new(file);
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info().ok()?;
        let mut bytes = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut bytes).ok()?;
        let channels = match info.color_type {
            png::ColorType::Rgb => 3,
            png::ColorType::Rgba => 4,
            png::ColorType::Grayscale => 1,
            png::ColorType::GrayscaleAlpha => 2,
            png::ColorType::Indexed => return None,
        };
        let source_width = info.width as usize;
        let source_height = info.height as usize;
        let size = target_size
            .max(MIN_SURFACE_TEXTURE_SIZE)
            .min(source_width)
            .min(source_height)
            .max(1);
        let mut pixels = Vec::with_capacity(size * size);
        for y in 0..size {
            let source_y = y * source_height / size;
            for x in 0..size {
                let source_x = x * source_width / size;
                let offset = (source_y * source_width + source_x) * channels;
                let color = if channels >= 3 {
                    [bytes[offset], bytes[offset + 1], bytes[offset + 2]]
                } else {
                    [bytes[offset]; 3]
                };
                pixels.push(color);
            }
        }
        Some(Self {
            width: size,
            height: size,
            pixels,
        })
    }

    fn sample(&self, u: f32, v: f32) -> [f32; 3] {
        let u = u.rem_euclid(1.0) * self.width as f32;
        let v = v.rem_euclid(1.0) * self.height as f32;
        let x0 = u.floor() as usize % self.width;
        let y0 = v.floor() as usize % self.height;
        let x1 = (x0 + 1) % self.width;
        let y1 = (y0 + 1) % self.height;
        let fx = u.fract();
        let fy = v.fract();
        let colors = [
            self.pixels[y0 * self.width + x0],
            self.pixels[y0 * self.width + x1],
            self.pixels[y1 * self.width + x0],
            self.pixels[y1 * self.width + x1],
        ];
        std::array::from_fn(|channel| {
            let top = colors[0][channel] as f32
                + (colors[1][channel] as f32 - colors[0][channel] as f32) * fx;
            let bottom = colors[2][channel] as f32
                + (colors[3][channel] as f32 - colors[2][channel] as f32) * fx;
            top + (bottom - top) * fy
        })
    }

    fn sample_triplanar(&self, point: Vec3, scale: f32) -> [f32; 3] {
        let xy = self.sample(point.x * scale, point.y * scale);
        let xz = self.sample(point.x * scale, point.z * scale);
        let zy = self.sample(point.z * scale, point.y * scale);
        std::array::from_fn(|channel| (xy[channel] + xz[channel] + zy[channel]) / 3.0)
    }
}

fn blue_metal_texture() -> Option<&'static SurfaceTexture> {
    static TEXTURE: OnceLock<Option<SurfaceTexture>> = OnceLock::new();
    TEXTURE
        .get_or_init(|| {
            SurfaceTexture::load_png(
                "assets/textures/blue_metal_plate_1k.blend/textures/blue_metal_plate_diff_1k.png",
                256,
            )
        })
        .as_ref()
}

struct RaceAsphaltTextures {
    clean_color: SurfaceTexture,
    clean_height: SurfaceTexture,
    asphalt04_color: SurfaceTexture,
    asphalt04_height: SurfaceTexture,
}

fn race_asphalt_textures() -> Option<&'static RaceAsphaltTextures> {
    static TEXTURES: OnceLock<Option<RaceAsphaltTextures>> = OnceLock::new();
    TEXTURES
        .get_or_init(|| {
            let clean = "assets/textures/clean_asphalt_1k.blend/textures";
            let asphalt04 = "assets/textures/asphalt_04_1k.blend/textures";
            Some(RaceAsphaltTextures {
                clean_color: SurfaceTexture::load_png(
                    &format!("{clean}/clean_asphalt_diff_1k.optimized.png"),
                    256,
                )?,
                clean_height: SurfaceTexture::load_png(
                    &format!("{clean}/clean_asphalt_disp_1k.png"),
                    256,
                )?,
                asphalt04_color: SurfaceTexture::load_png(
                    &format!("{asphalt04}/asphalt_04_diff_1k.optimized.png"),
                    256,
                )?,
                asphalt04_height: SurfaceTexture::load_png(
                    &format!("{asphalt04}/asphalt_04_disp_1k.png"),
                    256,
                )?,
            })
        })
        .as_ref()
}

struct GrassTextures {
    color: SurfaceTexture,
    normal: SurfaceTexture,
    roughness: SurfaceTexture,
    ao: SurfaceTexture,
}

fn grass_textures() -> Option<&'static GrassTextures> {
    static TEXTURES: OnceLock<Option<GrassTextures>> = OnceLock::new();
    TEXTURES
        .get_or_init(|| {
            let root = "assets/textures/Grass005_1K-PNG";
            Some(GrassTextures {
                color: SurfaceTexture::load_png(&format!("{root}/Grass005_1K-PNG_Color.png"), 256)?,
                normal: SurfaceTexture::load_png(
                    &format!("{root}/Grass005_1K-PNG_NormalGL.png"),
                    256,
                )?,
                roughness: SurfaceTexture::load_png(
                    &format!("{root}/Grass005_1K-PNG_Roughness.png"),
                    256,
                )?,
                ao: SurfaceTexture::load_png(
                    &format!("{root}/Grass005_1K-PNG_AmbientOcclusion.png"),
                    256,
                )?,
            })
        })
        .as_ref()
}

struct GroundTextures {
    color: SurfaceTexture,
    normal: SurfaceTexture,
    roughness: SurfaceTexture,
    ao: SurfaceTexture,
}

fn ground_textures() -> Option<&'static GroundTextures> {
    static TEXTURES: OnceLock<Option<GroundTextures>> = OnceLock::new();
    TEXTURES
        .get_or_init(|| {
            let root = "assets/textures/Ground030_1K-PNG";
            Some(GroundTextures {
                color: SurfaceTexture::load_png(
                    &format!("{root}/Ground030_1K-PNG_Color.png"),
                    256,
                )?,
                normal: SurfaceTexture::load_png(
                    &format!("{root}/Ground030_1K-PNG_NormalGL.png"),
                    256,
                )?,
                roughness: SurfaceTexture::load_png(
                    &format!("{root}/Ground030_1K-PNG_Roughness.png"),
                    256,
                )?,
                ao: SurfaceTexture::load_png(
                    &format!("{root}/Ground030_1K-PNG_AmbientOcclusion.png"),
                    256,
                )?,
            })
        })
        .as_ref()
}

struct WaterSandTextures {
    color: SurfaceTexture,
    normal: SurfaceTexture,
    roughness: SurfaceTexture,
    ao: SurfaceTexture,
}

fn water_sand_textures() -> Option<&'static WaterSandTextures> {
    static TEXTURES: OnceLock<Option<WaterSandTextures>> = OnceLock::new();
    TEXTURES
        .get_or_init(|| {
            let root = "assets/textures/Ground054_1K-PNG";
            Some(WaterSandTextures {
                color: SurfaceTexture::load_png(
                    &format!("{root}/Ground054_1K-PNG_Color.png"),
                    256,
                )?,
                normal: SurfaceTexture::load_png(
                    &format!("{root}/Ground054_1K-PNG_NormalGL.png"),
                    256,
                )?,
                roughness: SurfaceTexture::load_png(
                    &format!("{root}/Ground054_1K-PNG_Roughness.png"),
                    256,
                )?,
                ao: SurfaceTexture::load_png(
                    &format!("{root}/Ground054_1K-PNG_AmbientOcclusion.png"),
                    256,
                )?,
            })
        })
        .as_ref()
}

struct AsphaltTextures {
    color: SurfaceTexture,
    normal: SurfaceTexture,
    roughness: SurfaceTexture,
    ao: SurfaceTexture,
}

fn asphalt_textures() -> Option<&'static AsphaltTextures> {
    static TEXTURES: OnceLock<Option<AsphaltTextures>> = OnceLock::new();
    TEXTURES
        .get_or_init(|| {
            let root = "assets/textures/Asphalt031_1K-PNG";
            Some(AsphaltTextures {
                color: SurfaceTexture::load_png(
                    &format!("{root}/Asphalt031_1K-PNG_Color.png"),
                    256,
                )?,
                normal: SurfaceTexture::load_png(
                    &format!("{root}/Asphalt031_1K-PNG_NormalGL.png"),
                    256,
                )?,
                roughness: SurfaceTexture::load_png(
                    &format!("{root}/Asphalt031_1K-PNG_Roughness.png"),
                    256,
                )?,
                ao: SurfaceTexture::load_png(
                    &format!("{root}/Asphalt031_1K-PNG_AmbientOcclusion.png"),
                    256,
                )?,
            })
        })
        .as_ref()
}

struct WaterEarthTextures {
    color: SurfaceTexture,
    normal: SurfaceTexture,
    roughness: SurfaceTexture,
    ao: SurfaceTexture,
}

fn water_earth_textures() -> Option<&'static WaterEarthTextures> {
    static TEXTURES: OnceLock<Option<WaterEarthTextures>> = OnceLock::new();
    TEXTURES
        .get_or_init(|| {
            let root = "assets/textures/Ground048_1K-PNG";
            Some(WaterEarthTextures {
                color: SurfaceTexture::load_png(
                    &format!("{root}/Ground048_1K-PNG_Color.png"),
                    256,
                )?,
                normal: SurfaceTexture::load_png(
                    &format!("{root}/Ground048_1K-PNG_NormalGL.png"),
                    256,
                )?,
                roughness: SurfaceTexture::load_png(
                    &format!("{root}/Ground048_1K-PNG_Roughness.png"),
                    256,
                )?,
                ao: SurfaceTexture::load_png(
                    &format!("{root}/Ground048_1K-PNG_AmbientOcclusion.png"),
                    256,
                )?,
            })
        })
        .as_ref()
}

struct AlienEggTextures {
    color: SurfaceTexture,
    normal: SurfaceTexture,
    roughness: SurfaceTexture,
    ao: SurfaceTexture,
    emissive: SurfaceTexture,
}

fn alien_egg_textures() -> Option<&'static AlienEggTextures> {
    static TEXTURES: OnceLock<Option<AlienEggTextures>> = OnceLock::new();
    TEXTURES
        .get_or_init(|| {
            let root = "assets/textures/AlienEggs";
            Some(AlienEggTextures {
                color: SurfaceTexture::load_png(
                    &format!("{root}/AlienEggs_4K_basecolor.optimized.png"),
                    256,
                )?,
                normal: SurfaceTexture::load_png(
                    &format!("{root}/AlienEggs_4K_normal.optimized.png"),
                    256,
                )?,
                roughness: SurfaceTexture::load_png(
                    &format!("{root}/AlienEggs_4K_roughness.optimized.png"),
                    256,
                )?,
                ao: SurfaceTexture::load_png(
                    &format!("{root}/AlienEggs_4K_ambientocclusion.optimized.png"),
                    256,
                )?,
                emissive: SurfaceTexture::load_png(
                    &format!("{root}/AlienEggs_4K_emissive.optimized.png"),
                    256,
                )?,
            })
        })
        .as_ref()
}

struct AlienTissueTextures {
    color: SurfaceTexture,
    normal: SurfaceTexture,
    roughness: SurfaceTexture,
    ao: SurfaceTexture,
}

fn alien_tissue_textures() -> Option<&'static AlienTissueTextures> {
    static TEXTURES: OnceLock<Option<AlienTissueTextures>> = OnceLock::new();
    TEXTURES
        .get_or_init(|| {
            let root = "assets/textures/AlienSkinTissue";
            Some(AlienTissueTextures {
                color: SurfaceTexture::load_png(
                    &format!("{root}/AlienSkinTissue_4K_basecolor.optimized.png"),
                    256,
                )?,
                normal: SurfaceTexture::load_png(
                    &format!("{root}/AlienSkinTissue_4K_normal.optimized.png"),
                    256,
                )?,
                roughness: SurfaceTexture::load_png(
                    &format!("{root}/AlienSkinTissue_4K_roughness.optimized.png"),
                    256,
                )?,
                ao: SurfaceTexture::load_png(
                    &format!("{root}/AlienSkinTissue_4K_ambientocclusion.optimized.png"),
                    256,
                )?,
            })
        })
        .as_ref()
}

struct AlienVeinTextures {
    color: SurfaceTexture,
    normal: SurfaceTexture,
    roughness: SurfaceTexture,
    ao: SurfaceTexture,
    emissive: SurfaceTexture,
}

fn alien_vein_textures() -> Option<&'static AlienVeinTextures> {
    static TEXTURES: OnceLock<Option<AlienVeinTextures>> = OnceLock::new();
    TEXTURES
        .get_or_init(|| {
            let root = "assets/textures/AlienVeinSkin";
            Some(AlienVeinTextures {
                color: SurfaceTexture::load_png(
                    &format!("{root}/AlienVeinSkin_4K_basecolor.optimized.png"),
                    256,
                )?,
                normal: SurfaceTexture::load_png(
                    &format!("{root}/AlienVeinSkin_4K_normal.optimized.png"),
                    256,
                )?,
                roughness: SurfaceTexture::load_png(
                    &format!("{root}/AlienVeinSkin_4K_roughness.optimized.png"),
                    256,
                )?,
                ao: SurfaceTexture::load_png(
                    &format!("{root}/AlienVeinSkin_4K_ambientocclusion.optimized.png"),
                    256,
                )?,
                emissive: SurfaceTexture::load_png(
                    &format!("{root}/AlienVeinSkin_4K_emissive.optimized.png"),
                    256,
                )?,
            })
        })
        .as_ref()
}

struct BarkTextures {
    color: SurfaceTexture,
    normal: SurfaceTexture,
    roughness: SurfaceTexture,
    ao: SurfaceTexture,
}

fn bark_textures() -> Option<&'static BarkTextures> {
    static TEXTURES: OnceLock<Option<BarkTextures>> = OnceLock::new();
    TEXTURES
        .get_or_init(|| {
            let root = "assets/textures/Bark014_1K-PNG";
            Some(BarkTextures {
                color: SurfaceTexture::load_png(&format!("{root}/Bark014_1K-PNG_Color.png"), 256)?,
                normal: SurfaceTexture::load_png(
                    &format!("{root}/Bark014_1K-PNG_NormalGL.png"),
                    256,
                )?,
                roughness: SurfaceTexture::load_png(
                    &format!("{root}/Bark014_1K-PNG_Roughness.png"),
                    256,
                )?,
                ao: SurfaceTexture::load_png(
                    &format!("{root}/Bark014_1K-PNG_AmbientOcclusion.png"),
                    256,
                )?,
            })
        })
        .as_ref()
}

struct BricksTextures {
    color: SurfaceTexture,
    normal: SurfaceTexture,
    roughness: SurfaceTexture,
    ao: SurfaceTexture,
}

fn bricks_textures() -> Option<&'static BricksTextures> {
    static TEXTURES: OnceLock<Option<BricksTextures>> = OnceLock::new();
    TEXTURES
        .get_or_init(|| {
            let root = "assets/textures/Bricks113A_1K-PNG";
            Some(BricksTextures {
                color: SurfaceTexture::load_png(
                    &format!("{root}/Bricks113A_1K-PNG_Color.png"),
                    512,
                )?,
                normal: SurfaceTexture::load_png(
                    &format!("{root}/Bricks113A_1K-PNG_NormalGL.png"),
                    512,
                )?,
                roughness: SurfaceTexture::load_png(
                    &format!("{root}/Bricks113A_1K-PNG_Roughness.png"),
                    512,
                )?,
                ao: SurfaceTexture::load_png(
                    &format!("{root}/Bricks113A_1K-PNG_AmbientOcclusion.png"),
                    512,
                )?,
            })
        })
        .as_ref()
}

struct LavaTextures {
    color: SurfaceTexture,
    normal: SurfaceTexture,
    roughness: SurfaceTexture,
    emission: SurfaceTexture,
    displacement: SurfaceTexture,
}

fn lava_textures() -> Option<&'static LavaTextures> {
    static TEXTURES: OnceLock<Option<LavaTextures>> = OnceLock::new();
    TEXTURES
        .get_or_init(|| {
            let root = "assets/textures/Lava005_1K-PNG";
            Some(LavaTextures {
                color: SurfaceTexture::load_png(&format!("{root}/Lava005_1K-PNG_Color.png"), 512)?,
                normal: SurfaceTexture::load_png(
                    &format!("{root}/Lava005_1K-PNG_NormalGL.png"),
                    512,
                )?,
                roughness: SurfaceTexture::load_png(
                    &format!("{root}/Lava005_1K-PNG_Roughness.png"),
                    512,
                )?,
                emission: SurfaceTexture::load_png(
                    &format!("{root}/Lava005_1K-PNG_Emission.png"),
                    512,
                )?,
                displacement: SurfaceTexture::load_png(
                    &format!("{root}/Lava005_1K-PNG_Displacement.png"),
                    512,
                )?,
            })
        })
        .as_ref()
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
    if matches!(
        shader,
        SurfaceShader::RaceAsphaltClean | SurfaceShader::RaceAsphalt04
    ) {
        if let Some(textures) = race_asphalt_textures() {
            let (color_map, height_map, scale, contrast) = match shader {
                SurfaceShader::RaceAsphaltClean => {
                    (&textures.clean_color, &textures.clean_height, 0.095, 0.15)
                }
                SurfaceShader::RaceAsphalt04 => (
                    &textures.asphalt04_color,
                    &textures.asphalt04_height,
                    0.082,
                    0.23,
                ),
                _ => unreachable!(),
            };
            let color = color_map.sample(point.x * scale, point.z * scale);
            let height = height_map.sample(point.x * scale, point.z * scale)[0] / 255.0;
            let grain = 1.0 + (height - 0.5) * contrast;
            let edge_sheen = 1.0 + fresnel * (0.07 + (1.0 - height) * 0.05);
            let rgb =
                std::array::from_fn(|channel| (color[channel] * grain).clamp(0.0, 255.0) as u8);
            return pack(rgb, brightness * edge_sheen);
        }
    }
    if shader == SurfaceShader::DragonBallGlass {
        let inner_flow = (point.x * 3.7 + point.y * 4.9 - point.z * 2.8 + time * 1.4).sin();
        let caustic = (point.x * 8.3 - point.y * 5.1 + time * 2.2)
            .sin()
            .max(0.0)
            .powf(5.0);
        let rim = fresnel.clamp(0.0, 1.0).powf(0.62);
        let light = brightness * (0.82 + rim * 0.38);
        return pack(
            [
                (218.0 + inner_flow * 12.0 + caustic * 25.0) as u8,
                (91.0 + inner_flow * 9.0 + caustic * 34.0 + rim * 38.0) as u8,
                (14.0 + caustic * 18.0 + rim * 22.0) as u8,
            ],
            light,
        );
    }
    if shader == SurfaceShader::CastlePortalGlass {
        let energy = (point.x * 4.1 + point.y * 5.7 - point.z * 3.3 + time * 2.4).sin();
        let caustic = (point.x * 9.2 - point.y * 6.4 + time * 3.1)
            .sin()
            .max(0.0)
            .powf(4.0);
        let rim = fresnel.clamp(0.0, 1.0).powf(0.55);
        return pack(
            [
                (118.0 + energy * 18.0 + caustic * 54.0 + rim * 65.0) as u8,
                (18.0 + energy * 7.0 + caustic * 18.0 + rim * 20.0) as u8,
                (196.0 + energy * 20.0 + caustic * 42.0 + rim * 52.0) as u8,
            ],
            brightness * (0.86 + rim * 0.42),
        );
    }
    if shader == SurfaceShader::CoreGuardianEnvironmentTexture {
        if let Some(textures) = alien_vein_textures() {
            let scale = 0.11;
            let color = textures.color.sample_triplanar(point, scale);
            let normal = textures.normal.sample_triplanar(point, scale);
            let roughness = textures.roughness.sample_triplanar(point, scale)[0] / 255.0;
            let ao = textures.ao.sample_triplanar(point, scale)[0] / 255.0;
            let emissive = textures.emissive.sample_triplanar(point, scale);
            let relief = ((normal[0] - 128.0).abs() + (normal[1] - 128.0).abs()) / 255.0;
            let pulse = 0.68 + (time * 1.8 + point.z * 0.18).sin() * 0.22;
            let surface_light = brightness
                * (0.65 + ao * 0.35)
                * (0.92 + relief * 0.20)
                * (1.0 + fresnel * (1.0 - roughness) * 0.10);
            let tint = [0.90, 0.72, 0.92];
            let channels: [u32; 3] = std::array::from_fn(|channel| {
                (color[channel] * tint[channel] * surface_light + emissive[channel] * pulse * 0.55)
                    .clamp(0.0, 255.0) as u32
            });
            return (channels[0] << 16) | (channels[1] << 8) | channels[2];
        }
    }
    if shader == SurfaceShader::WaterMonsterCoreTexture {
        if let Some(textures) = alien_egg_textures() {
            let scale = 0.22;
            let color = textures.color.sample_triplanar(point, scale);
            let normal = textures.normal.sample_triplanar(point, scale);
            let roughness = textures.roughness.sample_triplanar(point, scale)[0] / 255.0;
            let ao = textures.ao.sample_triplanar(point, scale)[0] / 255.0;
            let emissive = textures.emissive.sample_triplanar(point, scale);
            let relief = ((normal[0] - 128.0).abs() + (normal[1] - 128.0).abs()) / 255.0;
            let pulse = 0.78 + (time * 4.5 + point.y * 1.7).sin() * 0.22;
            let surface_light = brightness
                * (0.72 + ao * 0.28)
                * (0.94 + relief * 0.16)
                * (1.0 + fresnel * (1.0 - roughness) * 0.12);
            let channels: [u32; 3] = std::array::from_fn(|channel| {
                (color[channel] * surface_light + emissive[channel] * pulse * 0.72)
                    .clamp(0.0, 255.0) as u32
            });
            return (channels[0] << 16) | (channels[1] << 8) | channels[2];
        }
    }
    if shader == SurfaceShader::WaterMonsterTissueTexture {
        if let Some(textures) = alien_tissue_textures() {
            let scale = 0.20;
            let color = textures.color.sample_triplanar(point, scale);
            let normal = textures.normal.sample_triplanar(point, scale);
            let roughness = textures.roughness.sample_triplanar(point, scale)[0] / 255.0;
            let ao = textures.ao.sample_triplanar(point, scale)[0] / 255.0;
            let relief = ((normal[0] - 128.0).abs() + (normal[1] - 128.0).abs()) / 255.0;
            let surface_light = brightness
                * (0.70 + ao * 0.30)
                * (0.93 + relief * 0.18)
                * (1.0 + fresnel * (1.0 - roughness) * 0.14);
            let tint = [1.12, 0.82, 0.86];
            let rgb = std::array::from_fn(|channel| {
                (color[channel] * tint[channel]).clamp(0.0, 255.0) as u8
            });
            return pack(rgb, surface_light);
        }
    }
    if shader == SurfaceShader::WaterPlanetEarthTexture {
        if let Some(textures) = water_earth_textures() {
            let scale = 0.08;
            let color = textures.color.sample_triplanar(point, scale);
            let normal = textures.normal.sample_triplanar(point, scale);
            let roughness = textures.roughness.sample_triplanar(point, scale)[0] / 255.0;
            let ao = textures.ao.sample_triplanar(point, scale)[0] / 255.0;
            let relief = ((normal[0] - 128.0).abs() + (normal[1] - 128.0).abs()) / 255.0;
            let surface_light = brightness
                * (0.73 + ao * 0.27)
                * (0.94 + relief * 0.15)
                * (1.0 + fresnel * (1.0 - roughness) * 0.07);
            let tint = [0.94, 0.90, 0.82];
            let rgb = std::array::from_fn(|channel| {
                (color[channel] * tint[channel]).clamp(0.0, 255.0) as u8
            });
            return pack(rgb, surface_light);
        }
    }
    if matches!(
        shader,
        SurfaceShader::WaterPlanetPillarTexture
            | SurfaceShader::WaterLevelRockTexture
            | SurfaceShader::TimePlanetRockTexture
            | SurfaceShader::CastleRockTexture
    ) {
        if let Some(textures) = asphalt_textures() {
            let scale = 0.12;
            let color = textures.color.sample_triplanar(point, scale);
            let normal = textures.normal.sample_triplanar(point, scale);
            let roughness = textures.roughness.sample_triplanar(point, scale)[0] / 255.0;
            let ao = textures.ao.sample_triplanar(point, scale)[0] / 255.0;
            let relief = ((normal[0] - 128.0).abs() + (normal[1] - 128.0).abs()) / 255.0;
            let surface_light = brightness
                * (0.69 + ao * 0.31)
                * (0.93 + relief * 0.18)
                * (1.0 + fresnel * (1.0 - roughness) * 0.07);
            let tint = match shader {
                SurfaceShader::WaterLevelRockTexture => [0.76, 0.92, 1.08],
                SurfaceShader::TimePlanetRockTexture => [1.04, 0.66, 1.08],
                SurfaceShader::CastleRockTexture => [0.90, 0.78, 0.70],
                SurfaceShader::WaterPlanetPillarTexture => [0.88, 0.94, 1.02],
                _ => unreachable!(),
            };
            let rgb = std::array::from_fn(|channel| {
                (color[channel] * tint[channel]).clamp(0.0, 255.0) as u8
            });
            return pack(rgb, surface_light);
        }
    }
    if matches!(
        shader,
        SurfaceShader::WaterPlanetSandTexture
            | SurfaceShader::WaterLevelSandTexture
            | SurfaceShader::TimePlanetSandTexture
    ) {
        if let Some(textures) = water_sand_textures() {
            let scale = 0.075;
            let color = textures.color.sample_triplanar(point, scale);
            let normal = textures.normal.sample_triplanar(point, scale);
            let roughness = textures.roughness.sample_triplanar(point, scale)[0] / 255.0;
            let ao = textures.ao.sample_triplanar(point, scale)[0] / 255.0;
            let relief = ((normal[0] - 128.0).abs() + (normal[1] - 128.0).abs()) / 255.0;
            let surface_light = brightness
                * (0.77 + ao * 0.23)
                * (0.95 + relief * 0.12)
                * (1.0 + fresnel * (1.0 - roughness) * 0.08);
            let tint = match shader {
                SurfaceShader::WaterLevelSandTexture => [0.84, 0.98, 1.04],
                SurfaceShader::TimePlanetSandTexture => [0.88, 0.58, 1.12],
                SurfaceShader::WaterPlanetSandTexture => [1.06, 1.01, 0.90],
                _ => unreachable!(),
            };
            let rgb = std::array::from_fn(|channel| {
                (color[channel] * tint[channel]).clamp(0.0, 255.0) as u8
            });
            return pack(rgb, surface_light);
        }
    }
    if shader == SurfaceShader::RaceBricksTexture {
        if let Some(textures) = bricks_textures() {
            let scale = 0.14;
            let color = textures.color.sample_triplanar(point, scale);
            let normal = textures.normal.sample_triplanar(point, scale);
            let roughness = textures.roughness.sample_triplanar(point, scale)[0] / 255.0;
            let ao = textures.ao.sample_triplanar(point, scale)[0] / 255.0;
            let relief = ((normal[0] - 128.0).abs() + (normal[1] - 128.0).abs()) / 255.0;
            let surface_light = brightness
                * (0.70 + ao * 0.30)
                * (0.93 + relief * 0.17)
                * (1.0 + fresnel * (1.0 - roughness) * 0.09);
            let tint = [0.92, 0.94, 0.86];
            let rgb = std::array::from_fn(|channel| {
                (color[channel] * tint[channel]).clamp(0.0, 255.0) as u8
            });
            return pack(rgb, surface_light);
        }
    }
    if shader == SurfaceShader::CastleLavaTexture {
        if let Some(textures) = lava_textures() {
            let scale = 0.16;
            let drift = Vec3::new(time * 0.18, 0.0, -time * 0.11);
            let animated_point = point.add(drift);
            let color = textures.color.sample_triplanar(animated_point, scale);
            let normal = textures.normal.sample_triplanar(animated_point, scale);
            let roughness = textures.roughness.sample_triplanar(animated_point, scale)[0] / 255.0;
            let emission = textures.emission.sample_triplanar(animated_point, scale);
            let height = textures
                .displacement
                .sample_triplanar(animated_point, scale)[0]
                / 255.0;
            let relief = ((normal[0] - 128.0).abs() + (normal[1] - 128.0).abs()) / 255.0;
            let pulse = 0.84 + (time * 2.8 + point.x * 0.8 + point.z * 0.6).sin() * 0.16;
            let surface_light = brightness
                * (0.88 + relief * 0.18)
                * (0.96 + height * 0.12)
                * (1.0 + fresnel * (1.0 - roughness) * 0.10);
            let rgb: [u32; 3] = std::array::from_fn(|channel| {
                let glow = [1.12, 0.72, 0.34][channel];
                (color[channel] * surface_light + emission[channel] * pulse * glow * 0.72)
                    .clamp(0.0, 255.0) as u32
            });
            return (rgb[0] << 16) | (rgb[1] << 8) | rgb[2];
        }
    }
    if shader == SurfaceShader::RaceBarkTexture {
        if let Some(textures) = bark_textures() {
            let scale = 0.18;
            let color = textures.color.sample_triplanar(point, scale);
            let normal = textures.normal.sample_triplanar(point, scale);
            let roughness = textures.roughness.sample_triplanar(point, scale)[0] / 255.0;
            let ao = textures.ao.sample_triplanar(point, scale)[0] / 255.0;
            let relief = ((normal[0] - 128.0).abs() + (normal[1] - 128.0).abs()) / 255.0;
            let surface_light = brightness
                * (0.72 + ao * 0.28)
                * (0.94 + relief * 0.16)
                * (1.0 + fresnel * (1.0 - roughness) * 0.08);
            let tint = [0.90, 0.84, 0.76];
            let rgb = std::array::from_fn(|channel| {
                (color[channel] * tint[channel]).clamp(0.0, 255.0) as u8
            });
            return pack(rgb, surface_light);
        }
    }
    if shader == SurfaceShader::RaceGroundTexture {
        if let Some(textures) = ground_textures() {
            // Triplanar projection prevents stretching on the steep sides and
            // avoids requiring UVs on the procedural mountain geometry.
            let scale = 0.065;
            let color = textures.color.sample_triplanar(point, scale);
            let normal = textures.normal.sample_triplanar(point, scale);
            let roughness = textures.roughness.sample_triplanar(point, scale)[0] / 255.0;
            let ao = textures.ao.sample_triplanar(point, scale)[0] / 255.0;
            let relief = ((normal[0] - 128.0).abs() + (normal[1] - 128.0).abs()) / 255.0;
            let surface_light = brightness
                * (0.76 + ao * 0.24)
                * (0.95 + relief * 0.12)
                * (1.0 + fresnel * (1.0 - roughness) * 0.10);
            let tint = [0.92, 1.0, 0.90];
            let rgb = std::array::from_fn(|channel| {
                (color[channel] * tint[channel]).clamp(0.0, 255.0) as u8
            });
            return pack(rgb, surface_light);
        }
    }
    if matches!(
        shader,
        SurfaceShader::RaceGrassTexture
            | SurfaceShader::RacePlanetGrassTexture
            | SurfaceShader::WaterPlanetGrassTexture
    ) {
        if let Some(textures) = grass_textures() {
            let scale = 0.085;
            let color = textures.color.sample(point.x * scale, point.z * scale);
            let normal = textures.normal.sample(point.x * scale, point.z * scale);
            let roughness = textures.roughness.sample(point.x * scale, point.z * scale)[0] / 255.0;
            let ao = textures.ao.sample(point.x * scale, point.z * scale)[0] / 255.0;
            let relief = ((normal[0] - 128.0).abs() + (normal[1] - 128.0).abs()) / 255.0;
            let planet_tint = match shader {
                SurfaceShader::RacePlanetGrassTexture => [0.62, 0.78, 0.67],
                SurfaceShader::WaterPlanetGrassTexture => [0.82, 0.94, 0.84],
                _ => [1.0, 1.0, 1.0],
            };
            let surface_light = brightness
                * (0.78 + ao * 0.22)
                * (0.96 + relief * 0.10)
                * (1.0 + fresnel * (1.0 - roughness) * 0.12);
            let rgb = std::array::from_fn(|channel| {
                (color[channel] * planet_tint[channel]).clamp(0.0, 255.0) as u8
            });
            return pack(rgb, surface_light);
        }
    }
    if matches!(
        shader,
        SurfaceShader::ArwingHull
            | SurfaceShader::BuzzWhiteMetal
            | SurfaceShader::McQueenWhiteMetal
    ) {
        if let Some(texture) = blue_metal_texture() {
            let sample = texture.sample_triplanar(point, 0.16);
            let grain = sample[0] * 0.2126 + sample[1] * 0.7152 + sample[2] * 0.0722;
            let white_metal = 176.0 + grain * 0.31;
            let tint = match shader {
                SurfaceShader::ArwingHull => [0.98, 1.01, 1.04],
                SurfaceShader::BuzzWhiteMetal => [0.96, 0.99, 1.03],
                SurfaceShader::McQueenWhiteMetal => [1.00, 1.01, 1.03],
                _ => unreachable!(),
            };
            let metal_light = brightness * (0.88 + fresnel * 0.52);
            let rgb = std::array::from_fn(|channel| {
                (white_metal * tint[channel]).clamp(0.0, 255.0) as u8
            });
            return pack(rgb, metal_light);
        }
    }
    if matches!(
        shader,
        SurfaceShader::ArwingBlue
            | SurfaceShader::AstraCobalt
            | SurfaceShader::AstraDeepBlue
            | SurfaceShader::SharkBlueMetal
    ) {
        if let Some(texture) = blue_metal_texture() {
            let sample = texture.sample_triplanar(point, 0.16);
            let tint = match shader {
                SurfaceShader::ArwingBlue => [0.92, 1.04, 1.22],
                SurfaceShader::AstraCobalt => [0.78, 1.05, 1.34],
                SurfaceShader::AstraDeepBlue => [0.52, 0.68, 0.94],
                SurfaceShader::SharkBlueMetal => [0.72, 1.02, 1.18],
                _ => unreachable!(),
            };
            let metal_light = brightness * (0.88 + fresnel * 0.52);
            let rgb = std::array::from_fn(|channel| {
                (sample[channel] * tint[channel]).clamp(0.0, 255.0) as u8
            });
            return pack(rgb, metal_light);
        }
    }
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
        SurfaceShader::SharkBlueMetal => (
            35.0 + fresnel * 34.0,
            96.0 + fresnel * 55.0,
            142.0 + fresnel * 66.0,
            0.82 + fresnel * 0.40,
        ),
        SurfaceShader::RaceGrassTexture => (
            48.0 + noise * 9.0,
            126.0 + noise * 18.0,
            42.0 + noise * 8.0,
            1.0,
        ),
        SurfaceShader::RacePlanetGrassTexture => (
            34.0 + noise * 8.0,
            101.0 + noise * 16.0,
            62.0 + noise * 9.0,
            1.0,
        ),
        SurfaceShader::RaceGroundTexture => (
            126.0 + noise * 18.0,
            112.0 + noise * 15.0,
            78.0 + noise * 12.0,
            1.0,
        ),
        SurfaceShader::RaceBarkTexture => (
            91.0 + noise * 16.0,
            68.0 + noise * 12.0,
            45.0 + noise * 9.0,
            1.0,
        ),
        SurfaceShader::RaceBricksTexture => (
            122.0 + noise * 17.0,
            112.0 + noise * 14.0,
            91.0 + noise * 11.0,
            1.0,
        ),
        SurfaceShader::WaterPlanetSandTexture => (
            205.0 + noise * 18.0,
            178.0 + noise * 14.0,
            112.0 + noise * 10.0,
            1.0,
        ),
        SurfaceShader::TimePlanetSandTexture => (
            166.0 + noise * 15.0,
            105.0 + noise * 12.0,
            184.0 + noise * 16.0,
            1.0,
        ),
        SurfaceShader::WaterPlanetGrassTexture => (
            45.0 + noise * 9.0,
            128.0 + noise * 17.0,
            58.0 + noise * 8.0,
            1.0,
        ),
        SurfaceShader::WaterPlanetPillarTexture => (
            82.0 + noise * 16.0,
            87.0 + noise * 15.0,
            92.0 + noise * 14.0,
            1.0,
        ),
        SurfaceShader::WaterPlanetEarthTexture => (
            112.0 + noise * 18.0,
            82.0 + noise * 14.0,
            52.0 + noise * 11.0,
            1.0,
        ),
        SurfaceShader::WaterLevelRockTexture => (
            66.0 + noise * 15.0,
            83.0 + noise * 15.0,
            96.0 + noise * 15.0,
            1.0,
        ),
        SurfaceShader::TimePlanetRockTexture => (
            132.0 + noise * 18.0,
            82.0 + noise * 13.0,
            142.0 + noise * 19.0,
            1.0,
        ),
        SurfaceShader::CastleRockTexture => (
            105.0 + noise * 17.0,
            88.0 + noise * 14.0,
            78.0 + noise * 12.0,
            1.0,
        ),
        SurfaceShader::CastleLavaTexture => (
            242.0 + noise * 13.0,
            66.0 + noise * 18.0,
            8.0 + noise * 5.0,
            1.18,
        ),
        SurfaceShader::WaterLevelSandTexture => (
            170.0 + noise * 16.0,
            169.0 + noise * 14.0,
            126.0 + noise * 11.0,
            1.0,
        ),
        SurfaceShader::WaterMonsterCoreTexture => (
            112.0 + noise * 20.0,
            188.0 + noise * 18.0,
            98.0 + noise * 14.0,
            1.0,
        ),
        SurfaceShader::WaterMonsterTissueTexture => (
            172.0 + noise * 22.0,
            42.0 + noise * 10.0,
            58.0 + noise * 12.0,
            1.0,
        ),
        SurfaceShader::CoreGuardianEnvironmentTexture => (
            88.0 + noise * 20.0,
            28.0 + noise * 10.0,
            76.0 + noise * 17.0,
            1.0,
        ),
        SurfaceShader::DragonBallGlass => (244.0, 132.0, 22.0, 1.0),
        SurfaceShader::CastlePortalGlass => (154.0, 32.0, 238.0, 1.0),
        SurfaceShader::RaceAsphaltClean => (
            48.0 + noise * 8.0,
            49.0 + noise * 8.0,
            50.0 + noise * 8.0,
            1.0,
        ),
        SurfaceShader::RaceAsphalt04 => (
            40.0 + noise * 12.0,
            42.0 + noise * 12.0,
            44.0 + noise * 12.0,
            1.0,
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
        SurfaceShader::Sun => {
            let direction = point.normalize();
            let flow_a = (direction.x * 17.0 + direction.z * 9.0 + time * 0.72).sin();
            let flow_b = (direction.y * 23.0 - direction.x * 11.0 - time * 0.93).sin();
            let granulation = ((direction.x + direction.y - direction.z) * 61.0
                + (time * 1.4).sin() * 2.0)
                .sin()
                .abs();
            let plasma = (flow_a * 0.5 + flow_b * 0.35 + granulation * 0.4).clamp(-0.7, 1.0);
            let pulse = 0.94 + (time * 2.1 + direction.y * 8.0).sin() * 0.06;
            (
                255.0 * pulse,
                (126.0 + plasma * 72.0 + granulation * 38.0) * pulse,
                (18.0 + plasma.max(0.0) * 38.0 + fresnel * 32.0) * pulse,
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

fn refract_dragon_ball_color(
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
    let lens = (1.0 - fresnel).clamp(0.0, 1.0);
    let shimmer = (point.x * 5.8 + point.y * 4.2 + time * 1.6).sin();
    let offset_x = (point.y * 4.4 + shimmer * 1.4) * lens;
    let offset_y = (-point.z * 3.8 + (point.x * 6.1 - time).cos()) * lens;
    let sample = |dx: f32, dy: f32| {
        let sx = (x as f32 + offset_x + dx)
            .round()
            .clamp(0.0, width.saturating_sub(1) as f32) as usize;
        let sy = (y as f32 + offset_y + dy)
            .round()
            .clamp(0.0, height.saturating_sub(1) as f32) as usize;
        background[sy * width + sx]
    };
    // Slight channel separation imitates dispersion through the curved shell.
    let red_sample = sample(1.6 * lens, 0.0);
    let green_sample = sample(0.0, 0.0);
    let blue_sample = sample(-1.6 * lens, 0.0);
    let transmitted = [
        ((red_sample >> 16) & 0xff) as f32,
        ((green_sample >> 8) & 0xff) as f32,
        (blue_sample & 0xff) as f32,
    ];
    let shell = [
        ((surface >> 16) & 0xff) as f32,
        ((surface >> 8) & 0xff) as f32,
        (surface & 0xff) as f32,
    ];
    let transmission = 0.38 + lens * 0.30;
    let rim_glow = fresnel.powf(0.55) * 62.0;
    let rgb: [u32; 3] = std::array::from_fn(|channel| {
        let amber_filter = [1.04, 0.72, 0.32][channel];
        (transmitted[channel] * transmission * amber_filter
            + shell[channel] * (1.0 - transmission)
            + rim_glow * [1.0, 0.58, 0.18][channel])
            .clamp(0.0, 255.0) as u32
    });
    (rgb[0] << 16) | (rgb[1] << 8) | rgb[2]
}

fn refract_castle_portal_color(
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
    let lens = (1.0 - fresnel).clamp(0.08, 1.0);
    let vortex = (point.y * 6.8 + point.x * 4.7 + time * 2.5).sin();
    let offset_x = (vortex * 5.5 + (point.z * 7.1 - time * 1.8).cos() * 2.4) * lens;
    let offset_y = ((point.x * 5.9 + time * 2.1).sin() * 4.2 - point.z * 1.6) * lens;
    let sample = |dx: f32, dy: f32| {
        let sx = (x as f32 + offset_x + dx)
            .round()
            .clamp(0.0, width.saturating_sub(1) as f32) as usize;
        let sy = (y as f32 + offset_y + dy)
            .round()
            .clamp(0.0, height.saturating_sub(1) as f32) as usize;
        background[sy * width + sx]
    };
    let red_sample = sample(2.4 * lens, -0.8 * lens);
    let green_sample = sample(0.0, 0.0);
    let blue_sample = sample(-2.4 * lens, 0.8 * lens);
    let transmitted = [
        ((red_sample >> 16) & 0xff) as f32,
        ((green_sample >> 8) & 0xff) as f32,
        (blue_sample & 0xff) as f32,
    ];
    let portal = [
        ((surface >> 16) & 0xff) as f32,
        ((surface >> 8) & 0xff) as f32,
        (surface & 0xff) as f32,
    ];
    let transmission = 0.48 + lens * 0.28;
    let rim_glow = fresnel.powf(0.48) * 78.0;
    let pulse = 0.86 + (time * 3.4 + point.y * 2.8).sin() * 0.14;
    let rgb: [u32; 3] = std::array::from_fn(|channel| {
        let violet_filter = [0.86, 0.48, 1.18][channel];
        (transmitted[channel] * transmission * violet_filter
            + portal[channel] * (1.0 - transmission) * pulse
            + rim_glow * [0.82, 0.24, 1.0][channel])
            .clamp(0.0, 255.0) as u32
    });
    (rgb[0] << 16) | (rgb[1] << 8) | rgb[2]
}

fn hash(mut value: u32) -> u32 {
    value ^= value >> 16;
    value = value.wrapping_mul(0x7feb_352d);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846c_a68b);
    value ^ (value >> 16)
}

fn draw_skybox(
    framebuffer: &mut Framebuffer,
    travel: f32,
    galaxy: Galaxy,
    camera: Option<&Camera>,
    world_time: f32,
) {
    let (base, horizon_glow, star_tint) = match galaxy {
        Galaxy::Home => ([4.0, 7.0, 20.0], [8.0, 11.0, 24.0], None),
        Galaxy::Vesper => ([7.0, 3.0, 25.0], [18.0, 11.0, 48.0], Some([130, 198, 255])),
        Galaxy::Nova => ([12.0, 5.0, 13.0], [28.0, 10.0, 22.0], Some([255, 154, 178])),
    };
    for y in 0..framebuffer.height {
        let t = y as f32 / framebuffer.height as f32;
        let horizon = (1.0 - (t - 0.48).abs() * 2.0).max(0.0);
        let color = (((base[0] + horizon * horizon_glow[0]) as u32) << 16)
            | (((base[1] + horizon * horizon_glow[1]) as u32) << 8)
            | (base[2] + horizon * horizon_glow[2]) as u32;
        let start = y * framebuffer.width;
        framebuffer.pixels[start..start + framebuffer.width].fill(color);
    }

    if galaxy == Galaxy::Nova {
        let cx = framebuffer.width as f32 * 0.58;
        let cy = framebuffer.height as f32 * 0.44;
        let scale = framebuffer.width.min(framebuffer.height) as f32;
        for y in 0..framebuffer.height {
            for x in 0..framebuffer.width {
                let dx = (x as f32 - cx) / (scale * 0.43);
                let dy = (y as f32 - cy) / (scale * 0.28);
                let radius = (dx * dx + dy * dy).sqrt();
                if radius > 1.0 {
                    continue;
                }
                let angle = dy.atan2(dx);
                let arm = ((angle - radius * 5.0).sin() * 0.5 + 0.5).powf(5.0);
                let glow = ((1.0 - radius) * 0.18 + arm * (1.0 - radius) * 0.26).clamp(0.0, 0.38);
                let index = y * framebuffer.width + x;
                let old = framebuffer.pixels[index];
                let red = ((old >> 16) & 255) as f32;
                let green = ((old >> 8) & 255) as f32;
                let blue = (old & 255) as f32;
                let (core_red, core_green, core_blue) = if galaxy == Galaxy::Nova {
                    (150.0, 30.0, 54.0)
                } else {
                    (80.0, 30.0, 130.0)
                };
                framebuffer.pixels[index] = (((red + core_red * glow) as u32) << 16)
                    | (((green + core_green * glow) as u32) << 8)
                    | (blue + core_blue * glow) as u32;
            }
        }
    }

    // Use the supplied transparent nebula art as a low-resolution sky overlay.
    if let Some(camera) = camera {
        space_decor::draw_nebula(framebuffer, galaxy, camera.forward, camera.right, camera.up);
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
        let color = star_tint.map_or_else(
            || (brightness << 16) | (brightness << 8) | (brightness + 18).min(255),
            |tint| {
                ((tint[0] as u32 * brightness / 255) << 16)
                    | ((tint[1] as u32 * brightness / 255) << 8)
                    | (tint[2] as u32 * brightness / 255)
            },
        );
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
    if let Some(camera) = camera {
        space_decor::draw_star_bits(
            framebuffer,
            galaxy,
            camera.forward,
            camera.right,
            camera.up,
            world_time,
        );
    }
    space_decor::draw_dust(framebuffer, travel, galaxy);
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

fn make_dragon_ball(star_centers: &[(f32, f32)]) -> Model {
    let radius = 2.2;
    let mut model = make_sphere(radius, 14, 22, false);
    for face in &mut model.faces {
        *face = Face::new(face.indices, "race_rgb_244_132_22", "dragon_ball");
        face.shader = SurfaceShader::DragonBallGlass;
        face.smooth = true;
    }

    for (star_index, &(y, z)) in star_centers.iter().enumerate() {
        let x = (radius * radius - y * y - z * z).sqrt() + 0.055;
        let center_index = model.vertices.len();
        model.vertices.push(Vec3::new(x, y, z));
        for point in 0..10 {
            let angle = PI * 0.5 + point as f32 * PI / 5.0;
            let point_radius = if point % 2 == 0 { 0.43 } else { 0.19 };
            model.vertices.push(Vec3::new(
                x,
                y + angle.sin() * point_radius,
                z + angle.cos() * point_radius,
            ));
        }
        for point in 0..10 {
            model.faces.push(Face::new(
                [
                    center_index,
                    center_index + 1 + point,
                    center_index + 1 + (point + 1) % 10,
                ],
                "race_rgb_190_25_25",
                format!("dragon_star_{star_index}"),
            ));
        }
    }
    model.recalculate_bounds();
    model.recalculate_normals();
    model
}

fn make_four_star_dragon_ball() -> Model {
    make_dragon_ball(&[(-0.62, -0.62), (0.62, -0.62), (-0.62, 0.62), (0.62, 0.62)])
}

fn make_five_star_dragon_ball() -> Model {
    make_dragon_ball(&[
        (-0.62, -0.62),
        (0.62, -0.62),
        (0.0, 0.0),
        (-0.62, 0.62),
        (0.62, 0.62),
    ])
}

fn make_six_star_dragon_ball() -> Model {
    make_dragon_ball(&[
        (-0.62, -0.70),
        (0.62, -0.70),
        (-0.62, 0.0),
        (0.62, 0.0),
        (-0.62, 0.70),
        (0.62, 0.70),
    ])
}

fn make_seven_star_dragon_ball() -> Model {
    make_dragon_ball(&[
        (-0.62, -0.70),
        (0.62, -0.70),
        (-0.62, 0.0),
        (0.62, 0.0),
        (-0.62, 0.70),
        (0.62, 0.70),
        (0.0, 0.0),
    ])
}

fn make_one_star_dragon_ball() -> Model {
    make_dragon_ball(&[(0.0, 0.0)])
}

fn make_two_star_dragon_ball() -> Model {
    make_dragon_ball(&[(-0.52, -0.38), (0.52, 0.38)])
}

fn make_three_star_dragon_ball() -> Model {
    make_dragon_ball(&[(-0.62, -0.48), (0.62, -0.48), (0.0, 0.62)])
}

fn make_reward_pedestal() -> Model {
    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    let tiers = [
        (0.0, 2.8, 0.55),
        (0.55, 2.15, 0.38),
        (0.93, 1.45, 0.30),
        (1.23, 0.68, 2.05),
    ];
    let segments = 18;
    for (tier, (base_y, radius, height)) in tiers.into_iter().enumerate() {
        let start = vertices.len();
        for ring_y in [base_y, base_y + height] {
            for segment in 0..segments {
                let angle = TAU * segment as f32 / segments as f32;
                vertices.push(Vec3::new(0.0, ring_y, 0.0).add(Vec3::new(
                    angle.cos() * radius,
                    0.0,
                    angle.sin() * radius,
                )));
            }
        }
        for segment in 0..segments {
            let next = (segment + 1) % segments;
            let material = if tier == 2 {
                "race_rgb_35_225_255"
            } else {
                "race_rgb_45_62_78"
            };
            faces.push(Face::new(
                [start + segment, start + next, start + segments + segment],
                material,
                "reward_pedestal",
            ));
            faces.push(Face::new(
                [
                    start + next,
                    start + segments + next,
                    start + segments + segment,
                ],
                material,
                "reward_pedestal",
            ));
        }
    }
    Model::new(vertices, faces)
}

fn append_box_geometry(
    vertices: &mut Vec<Vec3>,
    faces: &mut Vec<Face>,
    min: Vec3,
    max: Vec3,
    material: &str,
    group: &str,
) {
    let start = vertices.len();
    vertices.extend([
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(max.x, max.y, max.z),
        Vec3::new(min.x, max.y, max.z),
    ]);
    for indices in [
        [0, 2, 1],
        [0, 3, 2],
        [4, 5, 6],
        [4, 6, 7],
        [0, 1, 5],
        [0, 5, 4],
        [3, 7, 6],
        [3, 6, 2],
        [0, 4, 7],
        [0, 7, 3],
        [1, 2, 6],
        [1, 6, 5],
    ] {
        faces.push(Face::new(
            [start + indices[0], start + indices[1], start + indices[2]],
            material,
            group,
        ));
    }
}

fn make_reward_chamber() -> Model {
    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    let stone = "race_rgb_34_55_72";
    let trim = "race_rgb_35_225_255";
    // The chamber shell deliberately extends well past its playable bounds.
    // The camera trails behind the ship and has a wide field of view, so a
    // wall sized only to the collision volume exposes the space backdrop at
    // its edges once the ship crosses the doorway.
    let visual_half_width = 25.0;
    let visual_height = 25.0;
    append_box_geometry(
        &mut vertices,
        &mut faces,
        Vec3::new(-27.0, 0.0, -visual_half_width),
        Vec3::new(30.0, 1.0, visual_half_width),
        stone,
        "reward_room_floor",
    );
    append_box_geometry(
        &mut vertices,
        &mut faces,
        Vec3::new(-27.0, visual_height - 1.0, -visual_half_width),
        Vec3::new(30.0, visual_height, visual_half_width),
        stone,
        "reward_room_ceiling",
    );
    for side in [-1.0_f32, 1.0] {
        let z0 = if side < 0.0 {
            -visual_half_width - 1.0
        } else {
            visual_half_width
        };
        append_box_geometry(
            &mut vertices,
            &mut faces,
            Vec3::new(-27.0, 0.0, z0),
            Vec3::new(30.0, visual_height, z0 + 1.0),
            stone,
            "reward_room_side_wall",
        );
    }
    append_box_geometry(
        &mut vertices,
        &mut faces,
        Vec3::new(-28.0, 0.0, -visual_half_width),
        Vec3::new(-27.0, visual_height, visual_half_width),
        stone,
        "reward_room_back_wall",
    );
    // Front wall leaves a 10x9 doorway in the center.
    append_box_geometry(
        &mut vertices,
        &mut faces,
        Vec3::new(29.0, 0.0, -visual_half_width),
        Vec3::new(30.0, visual_height, -5.0),
        stone,
        "reward_room_front_left",
    );
    append_box_geometry(
        &mut vertices,
        &mut faces,
        Vec3::new(29.0, 0.0, 5.0),
        Vec3::new(30.0, visual_height, visual_half_width),
        stone,
        "reward_room_front_right",
    );
    append_box_geometry(
        &mut vertices,
        &mut faces,
        Vec3::new(29.0, 10.0, -5.0),
        Vec3::new(30.0, visual_height, 5.0),
        trim,
        "reward_room_door_lintel",
    );
    Model::new(vertices, faces)
}

fn make_reward_door() -> Model {
    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    append_box_geometry(
        &mut vertices,
        &mut faces,
        Vec3::new(-0.55, 0.0, -4.85),
        Vec3::new(0.55, 9.5, 4.85),
        "race_rgb_50_78_96",
        "reward_door",
    );
    // Bright central seam makes the opening motion readable from the arena.
    append_box_geometry(
        &mut vertices,
        &mut faces,
        Vec3::new(-0.60, 0.3, -0.14),
        Vec3::new(0.60, 9.2, 0.14),
        "race_rgb_35_225_255",
        "reward_door_energy_seam",
    );
    Model::new(vertices, faces)
}

#[derive(Clone, Copy)]
enum PlanetVisual {
    Water,
    Spiky,
    Racing,
    VesperClock,
    VesperShell,
    Castle,
}

struct PlanetLodModels {
    medium: Model,
    low: Model,
}

struct VesperPlanet {
    model: Model,
    medium: Option<Model>,
    medium_center: Option<Vec3>,
    low: Model,
    source_center: Vec3,
    scale: f32,
    center: Vec3,
    radius: f32,
    rotation_y: f32,
}

impl VesperPlanet {
    fn load(
        path: &str,
        center: Vec3,
        radius: f32,
        rotation_y: f32,
        low_visual: PlanetVisual,
    ) -> Result<Self, String> {
        let model = load_obj(path).map_err(|error| format!("Error al cargar {path}: {error}"))?;
        let medium_path = path
            .strip_suffix(".obj")
            .map(|base| format!("{base}_medium.obj"));
        let medium = medium_path
            .as_deref()
            .and_then(|candidate| load_obj(candidate).ok());
        let medium_center = medium.as_ref().map(model_center);
        let source_center = model_center(&model);
        let scale = radius / model_radius(&model, source_center).max(0.001);
        let low = make_planet_lod(radius, 6, 10, low_visual);
        Ok(Self {
            model,
            medium,
            medium_center,
            low,
            source_center,
            scale,
            center,
            radius,
            rotation_y,
        })
    }
}

#[derive(Clone, Copy, Default)]
struct PlanetLodState {
    level: usize,
}

impl PlanetLodState {
    fn update(&mut self, projected_radius: f32) -> usize {
        self.level = match self.level {
            0 if projected_radius < 24.0 => 1,
            1 if projected_radius > 34.0 => 0,
            level => level,
        };
        self.level
    }

    fn update_detailed(&mut self, projected_radius: f32, fast_flight: bool) -> usize {
        if fast_flight {
            // Keep the authored model whenever the planet is close enough to
            // be a major part of the view, even during a turbo pass.
            self.level = if projected_radius >= 110.0 {
                0
            } else if projected_radius < 24.0 {
                2
            } else {
                1
            };
            return self.level;
        }
        self.level = match self.level {
            0 if projected_radius < 90.0 => 1,
            1 if projected_radius < 24.0 => 2,
            1 if projected_radius > 110.0 => 0,
            2 if projected_radius > 34.0 => 1,
            level => level,
        };
        self.level
    }
}

fn make_planet_lod(radius: f32, stacks: usize, slices: usize, visual: PlanetVisual) -> Model {
    let mut model = make_sphere(radius, stacks, slices, false);
    for (index, face) in model.faces.iter_mut().enumerate() {
        let band = index / (slices * 2).max(1);
        let material = match visual {
            PlanetVisual::Water if band < 2 || band + 2 >= stacks => "planet_ice",
            PlanetVisual::Water if hash(index as u32 * 31) % 5 == 0 => "water_planet_sand_texture",
            PlanetVisual::Water if hash(index as u32 * 31) % 5 < 2 => "water_planet_grass_texture",
            PlanetVisual::Water => "planet_ocean",
            // The distant LOD contains only the spherical shell; authored
            // tentacles are restored by the detailed model at close range.
            PlanetVisual::Spiky => "spiky_planet_vein_texture",
            PlanetVisual::Racing if band == stacks / 2 => "race_rgb_245_208_42",
            PlanetVisual::Racing if index % 3 == 0 => "race_rgb_58_160_107",
            PlanetVisual::Racing => "race_rgb_44_133_88",
            PlanetVisual::VesperClock if band == stacks / 2 => "race_rgb_228_174_72",
            PlanetVisual::VesperClock if index % 3 == 0 => "race_rgb_159_110_58",
            PlanetVisual::VesperClock => "race_rgb_112_79_49",
            PlanetVisual::VesperShell if band < 2 => "race_rgb_239_226_194",
            PlanetVisual::VesperShell if index % 3 == 0 => "race_rgb_24_112_119",
            PlanetVisual::VesperShell => "race_rgb_192_204_184",
            PlanetVisual::Castle if band == stacks / 2 => "Castle_Lava",
            PlanetVisual::Castle if index % 4 == 0 => "Castle_Bricks_Dark",
            PlanetVisual::Castle => "Castle_Rock",
        };
        *face = Face::new(face.indices, material, "planet_lod");
        if matches!(visual, PlanetVisual::Spiky) {
            face.shader = SurfaceShader::CoreGuardianEnvironmentTexture;
        }
        face.smooth = true;
    }
    model.recalculate_normals();
    model
}

fn make_planet_lods(radius: f32, visual: PlanetVisual) -> PlanetLodModels {
    PlanetLodModels {
        medium: make_planet_lod(radius, 24, 40, visual),
        low: make_planet_lod(radius, 6, 10, visual),
    }
}

fn projected_planet_radius(camera: &Camera, center: Vec3, radius: f32, focal: f32) -> f32 {
    let depth = center.sub(camera.position).dot(camera.forward);
    if depth <= NEAR_PLANE {
        f32::INFINITY
    } else {
        radius * focal / depth
    }
}

fn animate_spiky_tentacle(vertex: Vec3, time: f32) -> Vec3 {
    let radius = vertex.dot(vertex).sqrt();
    // The authored planet body ends around radius 1.49. Fade deformation in
    // above that boundary so every tentacle stays firmly rooted in the shell.
    let extension = ((radius - 1.48) / 0.82).clamp(0.0, 1.0);
    if extension <= 0.0 {
        return vertex;
    }

    let radial = vertex.normalize();
    let reference = if radial.y.abs() < 0.82 {
        Vec3::new(0.0, 1.0, 0.0)
    } else {
        Vec3::new(1.0, 0.0, 0.0)
    };
    let tangent_a = radial.cross(reference).normalize();
    let tangent_b = radial.cross(tangent_a).normalize();
    let phase = radial.z.atan2(radial.x) * 2.7 + radial.y * 4.1;
    // Keep the first third nearly rigid, then exaggerate the long outer arcs.
    let bend = extension.powf(2.15);
    let wave_a = (time * 1.78 + phase + extension * 3.2).sin();
    let wave_b = (time * 1.46 - phase * 0.73 + extension * 4.2).cos();
    let tip_flutter = (time * 2.65 + phase * 1.4 + extension * 5.8).sin();
    let breathing = (time * 1.05 + phase).sin() * 0.065 * bend;
    vertex
        .add(tangent_a.mul((wave_a * 0.285 + tip_flutter * 0.065 * extension) * bend))
        .add(tangent_b.mul((wave_b * 0.205 - tip_flutter * 0.048 * extension) * bend))
        .add(radial.mul(breathing))
}

fn make_wormhole_sphere(radius: f32, stacks: usize, slices: usize) -> Model {
    let mut model = make_sphere(radius, stacks, slices, false);
    for vertex in &mut model.vertices {
        vertex.y *= 0.12;
    }
    model.recalculate_bounds();
    model.recalculate_normals();
    for face in &mut model.faces {
        face.shader = SurfaceShader::Flat;
        face.color = MaterialColor::Fixed([2, 1, 7]);
    }
    model
}

fn make_vesper_black_hole() -> black_hole::BlackHole {
    black_hole::build()
}

fn make_home_orbit_rings() -> Model {
    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    let segments = 160;
    let ring_width = 1.15;
    let rings = [
        (PLANET_ORBIT_RADIUS, 0.0_f32, [58, 174, 255]),
        (
            SPIKY_PLANET_ORBIT_RADIUS,
            SPIKY_PLANET_ORBIT_INCLINATION,
            [226, 75, 255],
        ),
        (
            RACE_PLANET_ORBIT_RADIUS,
            RACE_PLANET_ORBIT_INCLINATION,
            [255, 205, 68],
        ),
    ];
    for (ring_index, (radius, inclination, color)) in rings.into_iter().enumerate() {
        let base = vertices.len();
        let (sin_inclination, cos_inclination) = inclination.sin_cos();
        for segment in 0..segments {
            let angle = TAU * segment as f32 / segments as f32;
            let (sin, cos) = angle.sin_cos();
            for radial in [radius - ring_width, radius + ring_width] {
                vertices.push(Vec3::new(
                    cos * radial,
                    sin * radial * sin_inclination - 20.0,
                    sin * radial * cos_inclination,
                ));
            }
        }
        for segment in 0..segments {
            let next = (segment + 1) % segments;
            let a = base + segment * 2;
            let b = base + next * 2;
            let c = a + 1;
            let d = b + 1;
            for indices in [[a, b, c], [c, b, d]] {
                let mut face = Face::new(indices, "home_orbit_ring", format!("orbit_{ring_index}"));
                face.shader = SurfaceShader::Flat;
                face.color = MaterialColor::Fixed(color);
                faces.push(face);
            }
        }
    }
    Model::new(vertices, faces)
}

fn make_vesper_orbit_rings() -> Model {
    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    let segments = 160;
    let ring_width = 1.25;
    let colors = [[255, 184, 72], [78, 225, 255]];
    for index in 0..2 {
        let base = vertices.len();
        let radius = VESPER_SYSTEM_ORBIT_RADII[index];
        for segment in 0..segments {
            let angle = TAU * segment as f32 / segments as f32;
            let (sin, cos) = angle.sin_cos();
            for radial in [radius - ring_width, radius + ring_width] {
                vertices.push(Vec3::new(
                    cos * radial,
                    VESPER_ORBIT_HEIGHTS[index],
                    sin * radial,
                ));
            }
        }
        for segment in 0..segments {
            let next = (segment + 1) % segments;
            let a = base + segment * 2;
            let b = base + next * 2;
            let c = a + 1;
            let d = b + 1;
            for indices in [[a, b, c], [c, b, d]] {
                let mut face = Face::new(
                    indices,
                    "vesper_orbit_ring",
                    format!("vesper_orbit_{index}"),
                );
                face.shader = SurfaceShader::Flat;
                face.color = MaterialColor::Fixed(colors[index]);
                faces.push(face);
            }
        }
    }
    Model::new(vertices, faces)
}

fn vesper_planet_position(index: usize, world_time: f32) -> Vec3 {
    let angle = VESPER_SYSTEM_ORBIT_PHASES[index] + VESPER_SYSTEM_ORBIT_SPEEDS[index] * world_time;
    let (sin, cos) = angle.sin_cos();
    VESPER_SYSTEM_CENTER.add(Vec3::new(
        cos * VESPER_SYSTEM_ORBIT_RADII[index],
        VESPER_ORBIT_HEIGHTS[index],
        sin * VESPER_SYSTEM_ORBIT_RADII[index],
    ))
}

fn apply_vesper_black_hole_gravity(position: Vec3, delta_time: f32) -> Vec3 {
    let offset = position.sub(VESPER_SYSTEM_CENTER);
    let distance = offset.dot(offset).sqrt();
    if distance <= VESPER_BLACK_HOLE_EVENT_HORIZON || distance >= VESPER_BLACK_HOLE_GRAVITY_RADIUS {
        return position;
    }
    let influence = 1.0
        - (distance - VESPER_BLACK_HOLE_EVENT_HORIZON)
            / (VESPER_BLACK_HOLE_GRAVITY_RADIUS - VESPER_BLACK_HOLE_EVENT_HORIZON);
    let pull_speed = 8.0 + influence * influence * 58.0;
    position.sub(offset.mul(pull_speed * delta_time / distance.max(0.001)))
}

fn vesper_absorption_position(start: Vec3, progress: f32) -> Vec3 {
    let offset = start.sub(VESPER_SYSTEM_CENTER);
    let radius_scale = (1.0 - progress).max(0.0).powf(1.65);
    let angle = progress * TAU * 2.4;
    let (sin, cos) = angle.sin_cos();
    let rotated = Vec3::new(
        offset.x * cos - offset.z * sin,
        offset.y * (1.0 - progress),
        offset.x * sin + offset.z * cos,
    );
    VESPER_SYSTEM_CENTER.add(rotated.mul(radius_scale))
}

fn make_underwater_boost_ring() -> Model {
    let major_segments = 32;
    let tube_segments = 6;
    let major_radius = 7.8;
    let tube_radius = 0.34;
    let mut vertices = Vec::new();
    for major in 0..=major_segments {
        let angle = TAU * major as f32 / major_segments as f32;
        let (sin_angle, cos_angle) = angle.sin_cos();
        for tube in 0..=tube_segments {
            let around = TAU * tube as f32 / tube_segments as f32;
            let (sin_around, cos_around) = around.sin_cos();
            let ring = major_radius + tube_radius * cos_around;
            vertices.push(Vec3::new(
                tube_radius * sin_around,
                ring * cos_angle,
                ring * sin_angle,
            ));
        }
    }
    let mut faces = Vec::new();
    for major in 0..major_segments {
        for tube in 0..tube_segments {
            let a = major * (tube_segments + 1) + tube;
            let b = a + tube_segments + 1;
            let c = a + 1;
            let d = b + 1;
            for indices in [[a, b, c], [c, b, d]] {
                let mut face = Face::new(indices, "energy_submarina", "boost_ring");
                face.shader = SurfaceShader::Flat;
                face.color = MaterialColor::Fixed([30, 220, 240]);
                faces.push(face);
            }
        }
    }
    Model::new(vertices, faces)
}

fn draw_wormhole_energy(framebuffer: &mut Framebuffer, camera: &Camera, center: Vec3, time: f32) {
    let relative = center.sub(camera.position);
    let camera_x = relative.dot(camera.right);
    let camera_y = relative.dot(camera.up);
    let camera_z = relative.dot(camera.forward);
    if camera_z <= NEAR_PLANE || camera_z >= FAR_PLANE {
        return;
    }

    let focal = framebuffer.height as f32 / (VERTICAL_FOV * 0.5).tan() * 0.5;
    let center_x = framebuffer.width as f32 * 0.5 + camera_x * focal / camera_z;
    let center_y = framebuffer.height as f32 * 0.5 - camera_y * focal / camera_z;
    let radius = 30.0 * focal / camera_z;
    if radius < 1.0 {
        return;
    }
    let vertical_scale = 0.28;
    let extent_x = radius * 1.55;
    let extent_y = extent_x * vertical_scale;
    if center_x + extent_x < 0.0
        || center_x - extent_x >= framebuffer.width as f32
        || center_y + extent_y < 0.0
        || center_y - extent_y >= framebuffer.height as f32
    {
        return;
    }
    let width = framebuffer.width;
    let height = framebuffer.height;
    let min_x = (center_x - extent_x).floor().max(0.0) as usize;
    let max_x = (center_x + extent_x)
        .ceil()
        .min(width.saturating_sub(1) as f32) as usize;
    let min_y = (center_y - extent_y).floor().max(0.0) as usize;
    let max_y = (center_y + extent_y)
        .ceil()
        .min(height.saturating_sub(1) as f32) as usize;
    if min_x > max_x || min_y > max_y {
        return;
    }

    let portal_depth = 1.0 / camera_z;
    let (pixels, depth) = framebuffer.buffers_mut();
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let dx = x as f32 + 0.5 - center_x;
            let dy = y as f32 + 0.5 - center_y;
            let flattened_dy = dy / vertical_scale;
            let distance = (dx * dx + flattened_dy * flattened_dy).sqrt();
            let angle = flattened_dy.atan2(dx);
            let turbulence = (angle * 7.0 + time * 2.4).sin() * 0.055
                + (angle * 13.0 - time * 3.1).sin() * 0.028
                + (angle * 3.0 + time * 1.3).cos() * 0.035;
            let energy_radius = radius * (1.0 + turbulence);
            let arc_mask = ((angle * 5.0 - time * 2.8).sin() * 0.5 + 0.5).powf(2.2);
            let filament =
                ((distance - energy_radius).abs() / (radius * 0.075).max(1.0)).clamp(0.0, 1.0);
            let core_energy = (1.0 - filament).powf(2.4) * (0.45 + arc_mask * 0.85);
            let halo_distance =
                ((distance - energy_radius).abs() / (radius * 0.30).max(1.0)).clamp(0.0, 1.0);
            let halo = (1.0 - halo_distance).powf(3.0) * 0.28;
            let sparks = ((angle * 29.0 + time * 5.6).sin() * 0.5 + 0.5).powf(18.0)
                * (1.0 - ((distance - radius * 1.18).abs() / (radius * 0.24)).clamp(0.0, 1.0));
            let intensity = (core_energy + halo + sparks * 1.25).clamp(0.0, 1.5);
            if intensity < 0.015 {
                continue;
            }
            let index = y * width + x;
            if depth[index] > portal_depth + 0.001 {
                continue;
            }
            let old = pixels[index];
            let old_r = ((old >> 16) & 0xff) as f32;
            let old_g = ((old >> 8) & 0xff) as f32;
            let old_b = (old & 0xff) as f32;
            let violet = (angle * 4.0 + time * 1.7).sin() * 0.5 + 0.5;
            let red = (old_r + intensity * (32.0 + violet * 72.0)).min(255.0) as u32;
            let green = (old_g + intensity * (126.0 + (1.0 - violet) * 70.0)).min(255.0) as u32;
            let blue = (old_b + intensity * 245.0).min(255.0) as u32;
            pixels[index] = (red << 16) | (green << 8) | blue;
        }
    }
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
        Vec3::new(1.8, 0.0, 0.0),
        Vec3::new(-1.8, 0.0, 0.0),
        Vec3::new(0.0, 0.24, 0.0),
        Vec3::new(0.0, 0.0, 0.24),
        Vec3::new(0.0, -0.24, 0.0),
        Vec3::new(0.0, 0.0, -0.24),
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

fn make_enemy_laser_bolt() -> Model {
    let vertices = vec![
        Vec3::new(2.1, 0.0, 0.0),
        Vec3::new(-2.1, 0.0, 0.0),
        Vec3::new(0.0, 0.34, 0.0),
        Vec3::new(0.0, 0.0, 0.34),
        Vec3::new(0.0, -0.34, 0.0),
        Vec3::new(0.0, 0.0, -0.34),
    ];
    let mut faces = Vec::new();
    for side in 0..4 {
        let current = 2 + side;
        let next = 2 + (side + 1) % 4;
        let material = if side % 2 == 0 {
            "enemy_laser_core"
        } else {
            "enemy_laser_glow"
        };
        faces.push(Face::new([0, current, next], material, "enemy_laser"));
        faces.push(Face::new([1, next, current], material, "enemy_laser"));
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

fn chase_camera(ship: &Ship, vesper_orbit: bool) -> Camera {
    let (forward, ship_right, ship_up) = ship.camera_basis();
    let chase_position = ship
        .position
        .sub(forward.mul(ship.spec.chase_distance))
        .add(ship_up.mul(ship.spec.chase_height));
    let chase_target = ship.position.add(forward.mul(ship.spec.chase_look_ahead));
    let world_up = Vec3::new(0.0, 1.0, 0.0);

    let (orbit_position, orbit_target) = if let Some(planet_center) = ship.orbit_center {
        let radial = ship.position.sub(planet_center).normalize();
        let orbit_distance = if vesper_orbit {
            VESPER_ORBIT_CAMERA_DISTANCE
        } else {
            ORBIT_CAMERA_DISTANCE
        };
        let orbit_height = if vesper_orbit {
            VESPER_ORBIT_CAMERA_HEIGHT
        } else {
            ORBIT_CAMERA_HEIGHT
        };
        // Aim at the planetary body instead of tightly following the ship, so
        // the complete planet remains framed while the ship crosses in front.
        let target_height = if vesper_orbit {
            4.0
        } else {
            HOME_ORBIT_TARGET_HEIGHT
        };
        let target = planet_center.add(world_up.mul(target_height));
        (
            planet_center
                .add(radial.mul(orbit_distance))
                .add(world_up.mul(orbit_height)),
            target,
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

fn race_planet_position(world_time: f32) -> Vec3 {
    let sun_position = Vec3::new(SUN_POSITION_X, 20.0, 0.0);
    let orbit = world_time * RACE_PLANET_ORBIT_SPEED + RACE_PLANET_ORBIT_PHASE;
    let (sin, cos) = orbit.sin_cos();
    let (sin_inclination, cos_inclination) = RACE_PLANET_ORBIT_INCLINATION.sin_cos();
    sun_position.add(Vec3::new(
        cos * RACE_PLANET_ORBIT_RADIUS,
        sin * RACE_PLANET_ORBIT_RADIUS * sin_inclination - 20.0,
        sin * RACE_PLANET_ORBIT_RADIUS * cos_inclination,
    ))
}

fn ship_touches_sun(previous: Vec3, current: Vec3, ship_radius: f32, sun_radius: f32) -> bool {
    crossed_wormhole(
        previous,
        current,
        Vec3::new(SUN_POSITION_X, 20.0, 0.0),
        sun_radius + ship_radius,
    )
}

fn spiky_planet_is_closer(ship_position: Vec3, water_center: Vec3, spiky_center: Vec3) -> bool {
    let water_offset = ship_position.sub(water_center);
    let spiky_offset = ship_position.sub(spiky_center);
    spiky_offset.dot(spiky_offset) < water_offset.dot(water_offset)
}

fn race_planet_is_closest(
    ship_position: Vec3,
    water_center: Vec3,
    spiky_center: Vec3,
    race_center: Vec3,
) -> bool {
    let race_offset = ship_position.sub(race_center);
    let water_offset = ship_position.sub(water_center);
    let spiky_offset = ship_position.sub(spiky_center);
    let race_distance = race_offset.dot(race_offset);
    race_distance < water_offset.dot(water_offset) && race_distance < spiky_offset.dot(spiky_offset)
}

fn place_ship_near_water_planet(controller: &mut ShipController, world_time: f32) {
    controller.reset();
    let ship = &mut controller.ship;
    ship.position = water_planet_position(world_time).add(Vec3::new(45.0, 8.0, 0.0));
    ship.spiky_planet_locked = false;
    ship.race_planet_locked = false;
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
    ship.race_planet_locked = false;
    ship.orbit_center = Some(center);
    let radial = offset.normalize();
    let (_, right, _) = ship.movement_basis();
    ship.orbit_plane_normal = Some(radial.cross(right).normalize());
    ship.orbit_camera_blend = 1.0;
}

fn place_ship_near_race_planet(controller: &mut ShipController, world_time: f32) {
    controller.reset();
    let center = race_planet_position(world_time);
    let offset = Vec3::new(45.0, 8.0, 0.0);
    let ship = &mut controller.ship;
    ship.position = center.add(offset);
    (ship.yaw, ship.pitch) = direction_to_angles(offset.mul(-1.0));
    ship.speed = ship.spec.cruise_speed;
    ship.planet_locked = true;
    ship.spiky_planet_locked = false;
    ship.race_planet_locked = true;
    ship.orbit_center = Some(center);
    let radial = offset.normalize();
    let (_, right, _) = ship.movement_basis();
    ship.orbit_plane_normal = Some(radial.cross(right).normalize());
    ship.orbit_camera_blend = 1.0;
}

fn launch_ship_from_planet(controller: &mut ShipController, center: Vec3, radius: f32) {
    controller.reset();
    let radial = Vec3::new(1.0, 0.24, -0.12).normalize();
    let ship = &mut controller.ship;
    ship.position = center.add(radial.mul(radius + ORBIT_LIMIT_RADIUS + 20.0));
    (ship.yaw, ship.pitch) = direction_to_angles(radial);
    ship.speed = ship.spec.cruise_speed;
    ship.planet_locked = false;
    ship.spiky_planet_locked = false;
    ship.race_planet_locked = false;
    ship.orbit_center = None;
    ship.orbit_plane_normal = None;
    ship.orbit_camera_blend = 0.0;
    ship.orbit_release_active = false;
}

fn launch_ship_after_guardian_exit(controller: &mut ShipController, world_time: f32) {
    launch_ship_from_planet(
        controller,
        spiky_planet_position(world_time),
        SPIKY_PLANET_RADIUS,
    );
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
    force_horizontal_orbit: bool,
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
            // Vesper always uses the world's vertical axis so its planets are
            // orbited around the equator instead of from pole to pole.
            let orbit_normal = if force_horizontal_orbit {
                let horizontal = Vec3::new(offset.x, 0.0, offset.z);
                if horizontal.dot(horizontal) > 0.0001 {
                    horizontal.normalize()
                } else {
                    Vec3::new(1.0, 0.0, 0.0)
                }
            } else {
                normal
            };
            let plane_normal = if force_horizontal_orbit {
                Vec3::new(0.0, 1.0, 0.0)
            } else {
                ship.orbit_plane_normal.unwrap_or_else(|| {
                    let initial_tangent = right
                        .sub(orbit_normal.mul(right.dot(orbit_normal)))
                        .normalize();
                    orbit_normal.cross(initial_tangent).normalize()
                })
            };
            ship.orbit_plane_normal = Some(plane_normal);
            let tangent = plane_normal.cross(orbit_normal).normalize();
            let target_radius = safe_radius + ORBIT_FLIGHT_ALTITUDE;
            let angle = step_distance / target_radius;
            let (sin_angle, cos_angle) = angle.sin_cos();
            let next_normal = orbit_normal
                .mul(cos_angle)
                .add(tangent.mul(sin_angle))
                .normalize();
            let orbit_forward = tangent
                .mul(cos_angle)
                .sub(orbit_normal.mul(sin_angle))
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
    BlackHolePlasma {
        color: u32,
        alpha: u8,
    },
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

const TRIANGLE_DEPTH_BUCKETS: usize = 256;

fn depth_bucket(depth: f32) -> usize {
    // Reciprocal depth spans several orders of magnitude. Logarithmic buckets
    // keep useful front-to-back order without sorting every triangle exactly.
    let exponent = depth.max(f32::MIN_POSITIVE).log2();
    (((exponent + 12.0) / 16.0).clamp(0.0, 1.0) * (TRIANGLE_DEPTH_BUCKETS - 1) as f32) as usize
}

const OCCLUSION_BLOCK_SIZE: usize = 16;

struct CoarseOcclusion {
    minimum_depth: Vec<f32>,
    blocks_x: usize,
    blocks_y: usize,
    region_y: usize,
    region_height: usize,
}

impl CoarseOcclusion {
    fn build(depth: &[f32], width: usize, region_y: usize, region_height: usize) -> Self {
        let blocks_x = width.div_ceil(OCCLUSION_BLOCK_SIZE);
        let blocks_y = region_height.div_ceil(OCCLUSION_BLOCK_SIZE);
        let mut minimum_depth = vec![f32::INFINITY; blocks_x * blocks_y];
        for local_y in 0..region_height {
            let block_y = local_y / OCCLUSION_BLOCK_SIZE;
            for x in 0..width {
                let block_x = x / OCCLUSION_BLOCK_SIZE;
                let block = block_y * blocks_x + block_x;
                minimum_depth[block] = minimum_depth[block].min(depth[local_y * width + x]);
            }
        }
        Self {
            minimum_depth,
            blocks_x,
            blocks_y,
            region_y,
            region_height,
        }
    }

    fn hides(&self, vertices: [ScreenVertex; 3], width: usize) -> bool {
        let min_x = vertices
            .iter()
            .map(|vertex| vertex.x)
            .fold(f32::INFINITY, f32::min)
            .floor()
            .max(0.0) as usize;
        let max_x = vertices
            .iter()
            .map(|vertex| vertex.x)
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .min(width.saturating_sub(1) as f32) as usize;
        let min_y = vertices
            .iter()
            .map(|vertex| vertex.y)
            .fold(f32::INFINITY, f32::min)
            .floor()
            .max(self.region_y as f32) as usize;
        let region_end = self.region_y + self.region_height;
        let max_y = vertices
            .iter()
            .map(|vertex| vertex.y)
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .min(region_end.saturating_sub(1) as f32) as usize;
        if min_x > max_x || min_y > max_y || min_y >= region_end || max_y < self.region_y {
            return true;
        }

        let closest_depth = vertices
            .iter()
            .map(|vertex| vertex.z)
            .fold(f32::NEG_INFINITY, f32::max);
        let first_block_x = min_x / OCCLUSION_BLOCK_SIZE;
        let last_block_x = max_x / OCCLUSION_BLOCK_SIZE;
        let first_block_y = (min_y - self.region_y) / OCCLUSION_BLOCK_SIZE;
        let last_block_y = (max_y - self.region_y) / OCCLUSION_BLOCK_SIZE;
        for block_y in first_block_y..=last_block_y.min(self.blocks_y.saturating_sub(1)) {
            for block_x in first_block_x..=last_block_x.min(self.blocks_x.saturating_sub(1)) {
                if self.minimum_depth[block_y * self.blocks_x + block_x] < closest_depth {
                    return false;
                }
            }
        }
        true
    }
}

#[derive(Clone, Copy, Default)]
struct RenderStats {
    models_visible: usize,
    models_culled: usize,
    vertices_transformed: usize,
    triangles_submitted: usize,
    geometry_ms: f32,
    sort_ms: f32,
    raster_ms: f32,
}

thread_local! {
    static LOCAL_RENDER_STATS: Cell<(RenderStats, usize)> = const {
        Cell::new((RenderStats {
            models_visible: 0,
            models_culled: 0,
            vertices_transformed: 0,
            triangles_submitted: 0,
            geometry_ms: 0.0,
            sort_ms: 0.0,
            raster_ms: 0.0,
        }, 0))
    };
}

thread_local! {
    static RACE_BENCHMARK_CATEGORIES: Cell<([f32; 7], [usize; 7], [usize; 7])> =
        const { Cell::new(([0.0; 7], [0; 7], [0; 7])) };
}

fn reset_race_benchmark_categories() {
    RACE_BENCHMARK_CATEGORIES.with(|categories| categories.set(([0.0; 7], [0; 7], [0; 7])));
}

fn take_race_benchmark_categories() -> ([f32; 7], [usize; 7], [usize; 7]) {
    RACE_BENCHMARK_CATEGORIES.with(|categories| categories.replace(([0.0; 7], [0; 7], [0; 7])))
}

fn record_race_benchmark_category(index: usize, before: RenderStats, after: RenderStats) {
    if std::env::var_os("GRAPH_BENCH_SCENE").is_none() {
        return;
    }
    RACE_BENCHMARK_CATEGORIES.with(|categories| {
        let (mut geometry, mut vertices, mut triangles) = categories.get();
        geometry[index] += after.geometry_ms - before.geometry_ms;
        vertices[index] += after.vertices_transformed - before.vertices_transformed;
        triangles[index] += after.triangles_submitted - before.triangles_submitted;
        categories.set((geometry, vertices, triangles));
    });
}

fn reset_local_render_stats() {
    LOCAL_RENDER_STATS.with(|stats| stats.set((RenderStats::default(), 0)));
}

fn take_local_render_stats() -> (RenderStats, usize) {
    LOCAL_RENDER_STATS.with(|stats| stats.replace((RenderStats::default(), 0)))
}

fn accumulate_local_render_stats(frame: RenderStats) {
    LOCAL_RENDER_STATS.with(|stats| {
        let (mut total, calls) = stats.get();
        total.models_visible += frame.models_visible;
        total.models_culled += frame.models_culled;
        total.vertices_transformed += frame.vertices_transformed;
        total.triangles_submitted += frame.triangles_submitted;
        total.geometry_ms += frame.geometry_ms;
        total.sort_ms += frame.sort_ms;
        total.raster_ms += frame.raster_ms;
        stats.set((total, calls + 1));
    });
}

#[derive(Clone, Copy)]
struct ProjectionCache {
    tan_half_vertical: f32,
    tan_half_horizontal: f32,
    horizontal_plane_length: f32,
    vertical_plane_length: f32,
    focal: f32,
    light: Vec3,
}

impl ProjectionCache {
    fn new(width: usize, height: usize) -> Self {
        let tan_half_vertical = (VERTICAL_FOV * 0.5).tan();
        let tan_half_horizontal = tan_half_vertical * width as f32 / height as f32;
        Self {
            tan_half_vertical,
            tan_half_horizontal,
            horizontal_plane_length: (1.0 + tan_half_horizontal * tan_half_horizontal).sqrt(),
            vertical_plane_length: (1.0 + tan_half_vertical * tan_half_vertical).sqrt(),
            focal: height as f32 / tan_half_vertical * 0.5,
            light: Vec3::new(-0.3, 0.8, -0.5).normalize(),
        }
    }
}

struct TriangleBatch {
    commands: Vec<TriangleCommand>,
    depth_buckets: Vec<Vec<TriangleCommand>>,
    camera_vertex_scratch: Vec<Vec3>,
    camera_normal_scratch: Vec<Vec3>,
    viewport_width: usize,
    viewport_height: usize,
    projection: ProjectionCache,
    stats: RenderStats,
}

thread_local! {
    static MODEL_RENDER_BATCH: RefCell<TriangleBatch> =
        RefCell::new(TriangleBatch::new(1, 1));
}

impl TriangleBatch {
    fn new(viewport_width: usize, viewport_height: usize) -> Self {
        Self {
            commands: Vec::new(),
            depth_buckets: (0..TRIANGLE_DEPTH_BUCKETS).map(|_| Vec::new()).collect(),
            camera_vertex_scratch: Vec::new(),
            camera_normal_scratch: Vec::new(),
            viewport_width,
            viewport_height,
            projection: ProjectionCache::new(viewport_width, viewport_height),
            stats: RenderStats::default(),
        }
    }

    fn begin_frame(&mut self) {
        debug_assert!(self.commands.is_empty());
        self.stats = RenderStats::default();
    }

    fn stats(&self) -> RenderStats {
        self.stats
    }

    fn resize(&mut self, width: usize, height: usize) {
        debug_assert!(self.commands.is_empty());
        self.viewport_width = width;
        self.viewport_height = height;
        self.projection = ProjectionCache::new(width, height);
    }

    fn flush(&mut self, framebuffer: &mut Framebuffer) {
        self.flush_with_workers(framebuffer, render_worker_count());
    }

    fn flush_with_workers(&mut self, framebuffer: &mut Framebuffer, worker_count: usize) {
        if self.commands.is_empty() {
            return;
        }
        let sort_start = Instant::now();
        if self.commands.len() > 128 {
            for command in self.commands.drain(..) {
                let average_depth =
                    (command.vertices[0].z + command.vertices[1].z + command.vertices[2].z) / 3.0;
                let bucket = depth_bucket(average_depth);
                self.depth_buckets[bucket].push(command);
            }
            for bucket in self.depth_buckets.iter_mut().rev() {
                self.commands.append(bucket);
            }
        }
        self.stats.sort_ms += sort_start.elapsed().as_secs_f32() * 1_000.0;
        let raster_start = Instant::now();
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
                        shader: SurfaceShader::Ocean
                            | SurfaceShader::DragonBallGlass
                            | SurfaceShader::CastlePortalGlass,
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
                    let region_height = tile_depth.len() / width;
                    let region_end_y = region_y + region_height;
                    let mut occlusion = None;
                    for (command_index, command) in commands.iter().enumerate() {
                        let [a, b, c] = command.vertices;
                        let triangle_min_y = a.y.min(b.y).min(c.y).floor().max(0.0) as usize;
                        let triangle_max_y = a.y.max(b.y).max(c.y).ceil().max(0.0) as usize;
                        if triangle_max_y < region_y || triangle_min_y >= region_end_y {
                            continue;
                        }
                        if command_index == 256 || command_index == 1024 {
                            occlusion = Some(CoarseOcclusion::build(
                                tile_depth,
                                width,
                                region_y,
                                region_height,
                            ));
                        }
                        if occlusion
                            .as_ref()
                            .is_some_and(|grid| grid.hides([a, b, c], width))
                        {
                            continue;
                        }
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
                            TriangleStyle::BlackHolePlasma { color, alpha } => {
                                draw_triangle_alpha_region(
                                    tile_pixels,
                                    tile_depth,
                                    width,
                                    height,
                                    region_y,
                                    a,
                                    b,
                                    c,
                                    *color,
                                    *alpha,
                                )
                            }
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
                                    } else if *shader == SurfaceShader::DragonBallGlass {
                                        if let Some(background) = refractive_background {
                                            refract_dragon_ball_color(
                                                surface, background, width, height, x, y, point,
                                                *time, *fresnel,
                                            )
                                        } else {
                                            surface
                                        }
                                    } else if *shader == SurfaceShader::CastlePortalGlass {
                                        if let Some(background) = refractive_background {
                                            refract_castle_portal_color(
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
        self.stats.raster_ms += raster_start.elapsed().as_secs_f32() * 1_000.0;
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
    MODEL_RENDER_BATCH.with_borrow_mut(|batch| {
        batch.begin_frame();
        if (batch.viewport_width, batch.viewport_height) != (framebuffer.width, framebuffer.height)
        {
            batch.resize(framebuffer.width, framebuffer.height);
        }
        queue_model(model, batch, camera, transform, water_time, effect_spec);
        batch.flush(framebuffer);
        if std::env::var_os("GRAPH_BENCH_SCENE").is_some() {
            accumulate_local_render_stats(batch.stats());
        }
    });
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
    // Reuse the largest vertex buffer seen by this batch. With several planets
    // in the home galaxy this avoids multiple large allocations every frame.
    let mut camera_vertices = std::mem::take(&mut batch.camera_vertex_scratch);
    let mut camera_normals = std::mem::take(&mut batch.camera_normal_scratch);
    queue_model_with_scratch(
        model,
        batch,
        camera,
        transform,
        water_time,
        effect_spec,
        &mut camera_vertices,
        &mut camera_normals,
    );
    batch.camera_vertex_scratch = camera_vertices;
    batch.camera_normal_scratch = camera_normals;
}

fn queue_model_with_scratch<F>(
    model: &Model,
    batch: &mut TriangleBatch,
    camera: &Camera,
    transform: F,
    water_time: Option<f32>,
    effect_spec: Option<&ShipSpec>,
    camera_vertices: &mut Vec<Vec3>,
    camera_normals: &mut Vec<Vec3>,
) where
    F: Fn(Vec3) -> Vec3 + Sync,
{
    let geometry_start = Instant::now();
    let projection = batch.projection;
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
    if camera_center.z + transformed_radius < NEAR_PLANE
        || camera_center.z - transformed_radius > FAR_PLANE
        || camera_center.z * projection.tan_half_horizontal + camera_center.x
            < -transformed_radius * projection.horizontal_plane_length
        || camera_center.z * projection.tan_half_horizontal - camera_center.x
            < -transformed_radius * projection.horizontal_plane_length
        || camera_center.z * projection.tan_half_vertical + camera_center.y
            < -transformed_radius * projection.vertical_plane_length
        || camera_center.z * projection.tan_half_vertical - camera_center.y
            < -transformed_radius * projection.vertical_plane_length
    {
        batch.stats.models_culled += 1;
        batch.stats.geometry_ms += geometry_start.elapsed().as_secs_f32() * 1_000.0;
        return;
    }

    batch.stats.models_visible += 1;
    batch.stats.vertices_transformed += model.vertices.len();
    let commands_before = batch.commands.len();

    let transform_vertex = |vertex: &Vec3| {
        let relative = transform(*vertex).sub(camera.position);
        Vec3::new(
            relative.dot(camera.right),
            relative.dot(camera.up),
            relative.dot(camera.forward),
        )
    };
    let worker_count = transform_worker_count(model.vertices.len(), render_worker_count());
    if worker_count > 1 {
        let chunk_size = model.vertices.len().div_ceil(worker_count);
        camera_vertices.clear();
        camera_vertices.resize(model.vertices.len(), Vec3::default());
        thread::scope(|scope| {
            for (output, vertices) in camera_vertices
                .chunks_mut(chunk_size)
                .zip(model.vertices.chunks(chunk_size))
            {
                let transform_vertex = &transform_vertex;
                scope.spawn(move || {
                    for (slot, vertex) in output.iter_mut().zip(vertices) {
                        *slot = transform_vertex(vertex);
                    }
                });
            }
        });
    } else {
        camera_vertices.clear();
        camera_vertices.reserve(model.vertices.len());
        camera_vertices.extend(model.vertices.iter().map(transform_vertex));
    }
    camera_normals.clear();
    if model.faces.iter().any(|face| face.smooth) {
        camera_normals.reserve(model.vertex_normals.len());
        camera_normals.extend(model.vertex_normals.iter().map(|normal| {
            let world_direction = transform(local_center.add(*normal)).sub(world_center);
            Vec3::new(
                world_direction.dot(camera.right),
                world_direction.dot(camera.up),
                world_direction.dot(camera.forward),
            )
            .normalize()
        }));
    } else {
        camera_normals.clear();
    }
    for face in &model.faces {
        let [i0, i1, i2] = face.indices;
        let v0 = camera_vertices[i0];
        let v1 = camera_vertices[i1];
        let v2 = camera_vertices[i2];
        if v0.z <= NEAR_PLANE || v1.z <= NEAR_PLANE || v2.z <= NEAR_PLANE {
            continue;
        }
        if (v0.z > FAR_PLANE && v1.z > FAR_PLANE && v2.z > FAR_PLANE)
            || (v0.x < -v0.z * projection.tan_half_horizontal
                && v1.x < -v1.z * projection.tan_half_horizontal
                && v2.x < -v2.z * projection.tan_half_horizontal)
            || (v0.x > v0.z * projection.tan_half_horizontal
                && v1.x > v1.z * projection.tan_half_horizontal
                && v2.x > v2.z * projection.tan_half_horizontal)
            || (v0.y < -v0.z * projection.tan_half_vertical
                && v1.y < -v1.z * projection.tan_half_vertical
                && v2.y < -v2.z * projection.tan_half_vertical)
            || (v0.y > v0.z * projection.tan_half_vertical
                && v1.y > v1.z * projection.tan_half_vertical
                && v2.y > v2.z * projection.tan_half_vertical)
        {
            continue;
        }
        let project = |vertex: Vec3| ScreenVertex {
            x: batch.viewport_width as f32 * 0.5 + vertex.x * projection.focal / vertex.z,
            y: batch.viewport_height as f32 * 0.5 - vertex.y * projection.focal / vertex.z,
            z: 1.0 / vertex.z,
        };
        let normal = v1.sub(v0).cross(v2.sub(v0)).normalize();
        let brightness = 0.34 + normal.dot(projection.light).abs() * 0.66;
        let a = project(v0);
        let b = project(v1);
        let c = project(v2);
        let shader = face.shader;
        let flat_color = color_for_material(face.color, effect_spec);
        let is_black_hole_face = matches!(
            face.group.as_str(),
            "accretion_disk"
                | "accretion_filament"
                | "lensing_arc_upper"
                | "lensing_arc_lower"
                | "lensing_arc_upper_glow"
                | "lensing_arc_lower_glow"
        );
        if is_black_hole_face {
            let p0 = model.vertices[i0];
            let p1 = model.vertices[i1];
            let p2 = model.vertices[i2];
            let center = p0.add(p1).add(p2).mul(1.0 / 3.0);
            let radius = (center.x * center.x + center.z * center.z).sqrt();
            let alpha = match face.group.as_str() {
                "accretion_disk" => crate::black_hole::disk_alpha(radius),
                "accretion_filament" => crate::black_hole::filament_alpha(radius),
                _ => crate::black_hole::lensing_arc_alpha(&face.material),
            };
            let angle = center.z.atan2(center.x);
            let pulse = 0.94
                + 0.06
                    * (water_time.unwrap_or_default() * 0.8 + angle * 3.0 - radius * 0.025).sin();
            let color = flat_color.map(|channel| (channel as f32 * pulse).clamp(0.0, 255.0) as u8);
            batch.commands.push(TriangleCommand {
                vertices: [a, b, c],
                style: TriangleStyle::BlackHolePlasma {
                    color: pack(color, 1.0),
                    alpha,
                },
            });
            continue;
        }

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
            let brightness = [i0, i1, i2]
                .map(|index| 0.34 + camera_normals[index].dot(projection.light).abs() * 0.66);
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
    batch.stats.triangles_submitted += batch.commands.len() - commands_before;
    batch.stats.geometry_ms += geometry_start.elapsed().as_secs_f32() * 1_000.0;
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
    let mut camera_normals = Vec::new();
    queue_model_with_scratch(
        model,
        &mut batch,
        camera,
        transform,
        water_time,
        effect_spec,
        camera_vertices,
        &mut camera_normals,
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
    sun_center: Vec3,
    home_orbit_rings: &Model,
    wormhole: &Model,
    wormhole_center: Vec3,
    galaxy: Galaxy,
    vesper_planets: &[VesperPlanet; 2],
    vesper_black_hole: &black_hole::BlackHole,
    vesper_orbit_rings: &Model,
    castle_planet: &VesperPlanet,
    race_planet: &Model,
    ship_choices: &[(&Model, &'static ShipSpec)],
    space_battle: &space_battle::SpaceBattle,
    planet_lods: [&PlanetLodModels; 3],
    planet_lod_states: &mut [PlanetLodState; 3],
    vesper_lod_states: &mut [PlanetLodState; 2],
    castle_lod_state: &mut PlanetLodState,
    framebuffer: &mut Framebuffer,
    batch: &mut TriangleBatch,
    ship: &Ship,
    world_time: f32,
    travel: f32,
    turbo_power: f32,
    fast_flight: bool,
    fire_charge: f32,
    show_ship: bool,
    tower_reward: Option<(&Model, f32, Vec3)>,
) {
    framebuffer.clear(0);
    let camera = chase_camera(ship, matches!(galaxy, Galaxy::Vesper | Galaxy::Nova));
    draw_skybox(framebuffer, travel, galaxy, Some(&camera), world_time);
    debug_assert!(batch.commands.is_empty());
    let sun_position = Vec3::new(SUN_POSITION_X, 20.0, 0.0);
    let planet_position = water_planet_position(world_time);
    let planet_rotation = world_time * WATER_PLANET_ROTATION_SPEED;

    if galaxy == Galaxy::Home {
        draw_water_planet_atmosphere(
            framebuffer,
            &camera,
            planet_position,
            WATER_PLANET_RADIUS,
            world_time,
        );

        queue_model(
            home_orbit_rings,
            batch,
            &camera,
            |vertex| vertex.add(sun_position),
            None,
            None,
        );

        queue_model(
            sun,
            batch,
            &camera,
            |vertex| {
                vertex
                    .sub(sun_center)
                    .mul(SUN_VISUAL_SCALE)
                    .add(sun_position)
            },
            Some(world_time),
            None,
        );
        let water_lod = planet_lod_states[0].update_detailed(
            projected_planet_radius(
                &camera,
                planet_position,
                WATER_PLANET_RADIUS,
                batch.projection.focal,
            ),
            fast_flight,
        );
        let water_model = match water_lod {
            0 => planet,
            1 => &planet_lods[0].medium,
            _ => &planet_lods[0].low,
        };
        queue_model(
            water_model,
            batch,
            &camera,
            |vertex| {
                let vertex = if water_lod == 0 {
                    vertex.sub(planet_center).mul(planet_scale)
                } else {
                    vertex
                };
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
        let spiky_lod = planet_lod_states[1].update_detailed(
            projected_planet_radius(
                &camera,
                spiky_position,
                SPIKY_PLANET_RADIUS,
                batch.projection.focal,
            ),
            fast_flight,
        );
        let spiky_model = match spiky_lod {
            0 => spiky_planet,
            1 => &planet_lods[1].medium,
            _ => &planet_lods[1].low,
        };
        queue_model(
            spiky_model,
            batch,
            &camera,
            |vertex| {
                let vertex = if spiky_lod == 0 {
                    animate_spiky_tentacle(vertex.sub(spiky_planet_center), world_time)
                        .mul(spiky_planet_scale)
                } else {
                    vertex
                };
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
        let race_position = race_planet_position(world_time);
        let race_lod = planet_lod_states[2].update_detailed(
            projected_planet_radius(
                &camera,
                race_position,
                race_track::PLANET_RADIUS,
                batch.projection.focal,
            ),
            fast_flight,
        );
        let race_model = match race_lod {
            0 => race_planet,
            1 => &planet_lods[2].medium,
            _ => &planet_lods[2].low,
        };
        queue_model(
            race_model,
            batch,
            &camera,
            |vertex| {
                let rotation = world_time * RACE_PLANET_ROTATION_SPEED;
                let (sin, cos) = rotation.sin_cos();
                Vec3::new(
                    vertex.x * cos - vertex.z * sin,
                    vertex.y,
                    vertex.x * sin + vertex.z * cos,
                )
                .add(race_position)
            },
            None,
            None,
        );
    } else if galaxy == Galaxy::Vesper {
        queue_model(
            vesper_orbit_rings,
            batch,
            &camera,
            |vertex| vertex.add(VESPER_SYSTEM_CENTER),
            None,
            None,
        );
        queue_model(
            &vesper_black_hole.core,
            batch,
            &camera,
            |vertex| {
                vertex
                    .mul(black_hole::VISUAL_SCALE)
                    .add(VESPER_SYSTEM_CENTER)
            },
            None,
            None,
        );
        queue_model(
            &vesper_black_hole.disk,
            batch,
            &camera,
            |vertex| {
                black_hole::differential_disk_transform(vertex, world_time)
                    .mul(black_hole::VISUAL_SCALE)
                    .add(VESPER_SYSTEM_CENTER)
            },
            Some(world_time),
            None,
        );
        queue_model(
            &vesper_black_hole.filaments,
            batch,
            &camera,
            |vertex| {
                black_hole::differential_disk_transform(vertex, world_time)
                    .mul(black_hole::VISUAL_SCALE)
                    .add(VESPER_SYSTEM_CENTER)
            },
            Some(world_time),
            None,
        );
        let arc_transform = black_hole::camera_arc_transform(
            VESPER_SYSTEM_CENTER,
            camera.position,
            camera.right,
            Vec3::new(0.0, 1.0, 0.0),
        );
        queue_model(
            &vesper_black_hole.lensing_arcs,
            batch,
            &camera,
            |vertex| black_hole::scale_about_center(arc_transform(vertex), VESPER_SYSTEM_CENTER),
            Some(world_time),
            None,
        );
        for (index, body) in vesper_planets.iter().enumerate() {
            let body_position = vesper_planet_position(index, world_time);
            let (sin, cos) = (body.rotation_y + world_time * 0.035).sin_cos();
            let lod = vesper_lod_states[index].update(projected_planet_radius(
                &camera,
                body_position,
                body.radius,
                batch.projection.focal,
            ));
            let use_low = lod != 0;
            queue_model(
                if use_low { &body.low } else { &body.model },
                batch,
                &camera,
                |vertex| {
                    let local = if use_low {
                        vertex
                    } else {
                        vertex.sub(body.source_center).mul(body.scale)
                    };
                    Vec3::new(
                        local.x * cos - local.z * sin,
                        local.y,
                        local.x * sin + local.z * cos,
                    )
                    .add(body_position)
                },
                None,
                None,
            );
        }
    } else if galaxy == Galaxy::Nova {
        let body = castle_planet;
        let (sin, cos) = (body.rotation_y + world_time * 0.018).sin_cos();
        let lod = castle_lod_state.update_detailed(
            projected_planet_radius(&camera, body.center, body.radius, batch.projection.focal),
            false,
        );
        let (visible, authored_center) = match (lod, body.medium.as_ref()) {
            (0, _) => (&body.model, Some(body.source_center)),
            (1, Some(medium)) => (medium, body.medium_center),
            _ => (&body.low, None),
        };
        queue_model(
            visible,
            batch,
            &camera,
            |vertex| {
                let local = if let Some(center) = authored_center {
                    vertex.sub(center).mul(body.scale)
                } else {
                    vertex
                };
                Vec3::new(
                    local.x * cos - local.z * sin,
                    local.y,
                    local.x * sin + local.z * cos,
                )
                .add(body.center)
            },
            None,
            None,
        );
    }

    // Keep fine debris and collectible-size rocks in the same orbital lanes.
    let asteroid_orbit = match galaxy {
        Galaxy::Home => Some((sun_position, 350.0)),
        Galaxy::Vesper => Some((VESPER_SYSTEM_CENTER, 350.0)),
        // Nova has no star system center; its three belts circle the region
        // between the return portal and the castle world.
        Galaxy::Nova => Some((Vec3::new(0.0, 70.0, 0.0), 300.0)),
    };
    if let Some((asteroid_center, asteroid_radius)) = asteroid_orbit {
        let dust_rotation = world_time
            * if galaxy == Galaxy::Nova {
                0.0035
            } else {
                0.006
            };
        let (dust_sin, dust_cos) = dust_rotation.sin_cos();
        queue_model(
            space_decor::asteroid_belt(galaxy),
            batch,
            &camera,
            |vertex| {
                let local = vertex.mul(asteroid_radius);
                Vec3::new(
                    local.x * dust_cos - local.z * dust_sin,
                    local.y,
                    local.x * dust_sin + local.z * dust_cos,
                )
                .add(asteroid_center)
            },
            None,
            None,
        );

        for asteroid in space_decor::asteroids(galaxy) {
            let position = space_decor::asteroid_position(
                asteroid,
                asteroid_center,
                asteroid_radius,
                world_time,
            );
            let spin = asteroid.spin_phase + world_time * asteroid.spin_speed;
            let (spin_sin, spin_cos) = spin.sin_cos();
            queue_model(
                &asteroid.model,
                batch,
                &camera,
                |vertex| {
                    Vec3::new(
                        vertex.x * spin_cos - vertex.z * spin_sin,
                        vertex.y,
                        vertex.x * spin_sin + vertex.z * spin_cos,
                    )
                    .add(position)
                },
                None,
                None,
            );
        }
    }

    queue_model(
        wormhole,
        batch,
        &camera,
        |vertex| vertex.add(wormhole_center),
        None,
        None,
    );
    if galaxy == Galaxy::Vesper {
        queue_model(
            wormhole,
            batch,
            &camera,
            |vertex| vertex.add(nova_portal_position()),
            None,
            None,
        );
    }

    // Four background fighters are enough to sell a battle without crowding
    // the view. Their ship indices are assigned from the non-player choices.
    if matches!(galaxy, Galaxy::Home | Galaxy::Vesper) {
        for fighter in &space_battle.fighters {
            let Some((model, spec)) = ship_choices.get(fighter.ship_index) else {
                continue;
            };
            let mut npc = Ship::new(spec);
            npc.position = fighter.position;
            npc.yaw = fighter.yaw;
            npc.pitch = fighter.pitch;
            render_ship_components(
                model,
                model_center(model),
                turbo_plume,
                laser_bolt,
                &fighter.shots,
                batch,
                &camera,
                &npc,
                world_time,
                0.18,
                0.0,
                true,
                0.72,
            );
        }
    }

    if show_ship {
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
            1.0,
        );
    }
    if let Some((dragon_ball, reward_elapsed, reward_position)) = tower_reward {
        let descent = (reward_elapsed / 2.2).clamp(0.0, 1.0);
        let descent = descent * descent * (3.0 - 2.0 * descent);
        let hover = (reward_elapsed * 3.2).sin() * 0.22;
        let position = reward_position.add(Vec3::new(0.0, 13.0 - descent * 8.0 + hover, 0.0));
        let center = model_center(dragon_ball);
        let rotation = reward_elapsed * 1.8;
        let (sin, cos) = rotation.sin_cos();
        let collect_scale = (1.0 - (reward_elapsed - 2.8).max(0.0) / 0.55).clamp(0.0, 1.0);
        if collect_scale > 0.0 {
            queue_model(
                dragon_ball,
                batch,
                &camera,
                |vertex| {
                    let local = vertex.sub(center).mul(collect_scale);
                    Vec3::new(
                        local.x * cos - local.z * sin,
                        local.y,
                        local.x * sin + local.z * cos,
                    )
                    .add(position)
                },
                None,
                None,
            );
        }
    }
    batch.flush(framebuffer);
    draw_wormhole_energy(framebuffer, &camera, wormhole_center, world_time);
    if galaxy == Galaxy::Vesper {
        draw_wormhole_energy(
            framebuffer,
            &camera,
            nova_portal_position(),
            world_time + 1.7,
        );
    }
    draw_projectile_glow(
        framebuffer,
        projectiles,
        &camera,
        world_time,
        ship.spec.laser_color,
        1.65,
    );
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
    model_scale_multiplier: f32,
) {
    let (forward, right, up) = ship.basis();
    let pulse = 0.9 + (world_time * 28.0).sin() * 0.1;
    let plume_length = (1.3 + turbo_power * 5.5) * pulse * model_scale_multiplier;
    let plume_radius = (0.16 + turbo_power * 0.16) * model_scale_multiplier;
    if show_turbo {
        for side in ship.spec.turbo_sides {
            let emitter = ship
                .position
                .sub(forward.mul(ship.spec.turbo_back * model_scale_multiplier))
                .add(right.mul(*side * ship.spec.turbo_side * model_scale_multiplier))
                .sub(up.mul(ship.spec.turbo_down * model_scale_multiplier));
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
            let local = vertex
                .sub(ship_center)
                .mul(ship.spec.model_scale * model_scale_multiplier);
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
    let mut camera_normals = Vec::new();
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
                &mut camera_normals,
            );
        }
    }
}

fn render_enemy_projectiles(
    projectiles: &[Projectile],
    laser_bolt: &Model,
    batch: &mut TriangleBatch,
    camera: &Camera,
) {
    for projectile in projectiles.iter().filter(|shot| shot.lifetime > 0.0) {
        let mut right = projectile
            .direction
            .cross(Vec3::new(0.0, 1.0, 0.0))
            .normalize();
        if right.dot(right) < 0.001 {
            right = Vec3::new(0.0, 0.0, 1.0);
        }
        let up = right.cross(projectile.direction).normalize();
        queue_model(
            laser_bolt,
            batch,
            camera,
            |vertex| {
                projectile
                    .position
                    .add(projectile.direction.mul(vertex.x * projectile.scale * 1.4))
                    .add(up.mul(vertex.y * projectile.scale * 0.8))
                    .add(right.mul(vertex.z * projectile.scale * 0.8))
            },
            None,
            None,
        );
    }
}

fn draw_projectile_glow(
    framebuffer: &mut Framebuffer,
    projectiles: &[Projectile],
    camera: &Camera,
    time: f32,
    glow_color: [u8; 3],
    radius_multiplier: f32,
) {
    let focal = framebuffer.height as f32 / (VERTICAL_FOV * 0.5).tan() * 0.5;
    for projectile in projectiles.iter().filter(|shot| shot.lifetime > 0.0) {
        let relative = projectile.position.sub(camera.position);
        let depth = relative.dot(camera.forward);
        if depth <= NEAR_PLANE {
            continue;
        }
        let center_x = framebuffer.width as f32 * 0.5 + relative.dot(camera.right) * focal / depth;
        let center_y = framebuffer.height as f32 * 0.5 - relative.dot(camera.up) * focal / depth;
        let pulse = 0.9 + (time * 13.0 + projectile.position.x).sin() * 0.1;
        let radius =
            (focal * projectile.scale * radius_multiplier / depth).clamp(5.0, 32.0) * pulse;
        if !circle_intersects_frame(framebuffer, center_x, center_y, radius) {
            continue;
        }
        let min_x = (center_x - radius).floor().max(0.0) as usize;
        let max_x = (center_x + radius)
            .ceil()
            .min(framebuffer.width.saturating_sub(1) as f32) as usize;
        let min_y = (center_y - radius).floor().max(0.0) as usize;
        let max_y = (center_y + radius)
            .ceil()
            .min(framebuffer.height.saturating_sub(1) as f32) as usize;
        if min_x > max_x || min_y > max_y {
            continue;
        }
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let dx = x as f32 + 0.5 - center_x;
                let dy = y as f32 + 0.5 - center_y;
                let distance = (dx * dx + dy * dy).sqrt();
                if distance > radius {
                    continue;
                }
                let intensity = (1.0 - distance / radius).powf(2.2);
                let index = y * framebuffer.width + x;
                let old = framebuffer.pixels[index];
                let red = (((old >> 16) & 255) as f32 + glow_color[0] as f32 * intensity * 0.72)
                    .min(255.0) as u32;
                let green = (((old >> 8) & 255) as f32 + glow_color[1] as f32 * intensity * 0.72)
                    .min(255.0) as u32;
                let blue = ((old & 255) as f32 + glow_color[2] as f32 * intensity * 0.72).min(255.0)
                    as u32;
                framebuffer.pixels[index] = (red << 16) | (green << 8) | blue;
            }
        }
    }
}

fn draw_boss_explosion_glow(
    framebuffer: &mut Framebuffer,
    camera: &Camera,
    state: &WaterLevelState,
) {
    if state.boss_explosion_timer <= 0.0 {
        return;
    }
    let progress = 1.0 - state.boss_explosion_timer / WATER_BOSS_EXPLOSION_DURATION;
    let relative = state.boss_explosion_position.sub(camera.position);
    let depth = relative.dot(camera.forward);
    if depth <= NEAR_PLANE {
        return;
    }
    let focal = framebuffer.height as f32 / (VERTICAL_FOV * 0.5).tan() * 0.5;
    let center_x = framebuffer.width as f32 * 0.5 + relative.dot(camera.right) * focal / depth;
    let center_y = framebuffer.height as f32 * 0.5 - relative.dot(camera.up) * focal / depth;
    let envelope = (progress * PI).sin().max(0.0);
    let radius = (18.0 + progress * 105.0) * (0.72 + envelope * 0.28);
    if !circle_intersects_frame(framebuffer, center_x, center_y, radius) {
        return;
    }
    let min_x = (center_x - radius).floor().max(0.0) as usize;
    let max_x = (center_x + radius)
        .ceil()
        .min(framebuffer.width.saturating_sub(1) as f32) as usize;
    let min_y = (center_y - radius).floor().max(0.0) as usize;
    let max_y = (center_y + radius)
        .ceil()
        .min(framebuffer.height.saturating_sub(1) as f32) as usize;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let dx = x as f32 + 0.5 - center_x;
            let dy = y as f32 + 0.5 - center_y;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance > radius {
                continue;
            }
            let core = (1.0 - distance / radius).powf(2.4) * envelope;
            let ring_center = radius * (0.55 + progress * 0.32);
            let ring = (1.0 - ((distance - ring_center).abs() / (radius * 0.11)).min(1.0))
                .powf(2.0)
                * (1.0 - progress);
            let strength = (core + ring * 0.9).clamp(0.0, 1.25);
            let index = y * framebuffer.width + x;
            let old = framebuffer.pixels[index];
            let red = (((old >> 16) & 255) as f32 + 255.0 * strength).min(255.0) as u32;
            let green = (((old >> 8) & 255) as f32 + 128.0 * strength).min(255.0) as u32;
            let blue = ((old & 255) as f32 + 45.0 * strength).min(255.0) as u32;
            framebuffer.pixels[index] = (red << 16) | (green << 8) | blue;
        }
    }
}

fn draw_solar_ship_explosion(
    framebuffer: &mut Framebuffer,
    camera: &Camera,
    position: Vec3,
    timer: f32,
) {
    if timer <= 0.0 {
        return;
    }
    let progress = 1.0 - timer / SOLAR_SHIP_EXPLOSION_DURATION;
    let relative = position.sub(camera.position);
    let depth = relative.dot(camera.forward);
    if depth <= NEAR_PLANE {
        return;
    }
    let focal = framebuffer.height as f32 / (VERTICAL_FOV * 0.5).tan() * 0.5;
    let center_x = framebuffer.width as f32 * 0.5 + relative.dot(camera.right) * focal / depth;
    let center_y = framebuffer.height as f32 * 0.5 - relative.dot(camera.up) * focal / depth;
    let envelope = (progress * PI).sin().max(0.0);
    let radius = 20.0 + progress * 125.0;
    if !circle_intersects_frame(framebuffer, center_x, center_y, radius) {
        return;
    }
    let min_x = (center_x - radius).floor().max(0.0) as usize;
    let max_x = (center_x + radius)
        .ceil()
        .min(framebuffer.width.saturating_sub(1) as f32) as usize;
    let min_y = (center_y - radius).floor().max(0.0) as usize;
    let max_y = (center_y + radius)
        .ceil()
        .min(framebuffer.height.saturating_sub(1) as f32) as usize;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let dx = x as f32 + 0.5 - center_x;
            let dy = y as f32 + 0.5 - center_y;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance > radius {
                continue;
            }
            let core = (1.0 - distance / radius).powf(2.0) * envelope;
            let shock_radius = radius * (0.42 + progress * 0.46);
            let shock = (1.0 - ((distance - shock_radius).abs() / (radius * 0.09)).min(1.0))
                .powf(2.0)
                * (1.0 - progress);
            let sparks = ((dx * 0.23 + dy * 0.17 + progress * 18.0).sin().max(0.0)).powf(9.0)
                * envelope
                * 0.7;
            let strength = (core + shock + sparks).clamp(0.0, 1.3);
            let index = y * framebuffer.width + x;
            let old = framebuffer.pixels[index];
            let red = (((old >> 16) & 255) as f32 + 255.0 * strength).min(255.0) as u32;
            let green = (((old >> 8) & 255) as f32 + 150.0 * strength).min(255.0) as u32;
            let blue = ((old & 255) as f32 + 42.0 * strength).min(255.0) as u32;
            framebuffer.pixels[index] = (red << 16) | (green << 8) | blue;
        }
    }
}

fn circle_intersects_frame(
    framebuffer: &Framebuffer,
    center_x: f32,
    center_y: f32,
    radius: f32,
) -> bool {
    center_x + radius >= 0.0
        && center_y + radius >= 0.0
        && center_x - radius < framebuffer.width as f32
        && center_y - radius < framebuffer.height as f32
}

fn render_water_level(
    level: &LevelSpatialIndex,
    enemy_models: &[Model; 3],
    enemy_centers: &[Vec3; 3],
    state: &WaterLevelState,
    enemy_instance_buffers: &mut [Vec<Vec3>; 3],
    bubble_model: &Model,
    boost_ring: &Model,
    dragon_ball: &Model,
    reward_pedestal: &Model,
    reward_chamber: &Model,
    reward_door: &Model,
    bubbles: &[BubbleParticle],
    ship_model: &Model,
    ship_center: Vec3,
    turbo_plume: &Model,
    laser_bolt: &Model,
    enemy_laser_bolt: &Model,
    projectiles: &[Projectile],
    framebuffer: &mut Framebuffer,
    batch: &mut TriangleBatch,
    ship: &Ship,
    world_time: f32,
    turbo_power: f32,
    fire_charge: f32,
) {
    framebuffer.clear(0);
    draw_underwater_background(framebuffer, state.progress * 0.002);
    let camera = chase_camera(ship, false);
    debug_assert!(batch.commands.is_empty());
    for chunk in level.visible_chunks(state.progress) {
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
        &state.enemies,
        state.elapsed,
        state.progress,
        batch,
        &camera,
        enemy_instance_buffers,
    );
    for gate_progress in [286.0_f32, 310.0, 334.0, 356.0] {
        if (gate_progress - state.progress).abs() < 135.0 {
            let (route_curve, route_height) = water_route(gate_progress);
            let pulse = 0.92 + (world_time * 4.0 + gate_progress).sin() * 0.08;
            queue_model(
                boost_ring,
                batch,
                &camera,
                |vertex| {
                    Vec3::new(-gate_progress, 7.0 + route_height, route_curve)
                        .add(vertex.mul(pulse))
                },
                None,
                None,
            );
        }
    }
    if (state.progress - WATER_REWARD_PROGRESS).abs() < 175.0 {
        queue_model(
            reward_chamber,
            batch,
            &camera,
            |vertex| Vec3::new(-WATER_REWARD_PROGRESS, 0.0, 0.0).add(vertex),
            None,
            None,
        );
        let door_lift = state.reward_door_open * 11.2;
        queue_model(
            reward_door,
            batch,
            &camera,
            |vertex| Vec3::new(-WATER_REWARD_DOOR_PROGRESS, door_lift, 0.0).add(vertex),
            None,
            None,
        );
    }
    if state.stage() == WaterStage::SharkBoss
        && (!state.boss_defeated || state.boss_explosion_timer > 0.0)
    {
        let model = &enemy_models[1];
        let center = enemy_centers[1];
        let exploding = state.boss_explosion_timer > 0.0;
        let explosion_progress = if exploding {
            1.0 - state.boss_explosion_timer / WATER_BOSS_EXPLOSION_DURATION
        } else {
            0.0
        };
        let base_position = if exploding {
            state.boss_explosion_position
        } else {
            state.boss_position()
        };
        let shake = if exploding {
            Vec3::new(
                0.0,
                (state.elapsed * 48.0).sin() * (1.0 - explosion_progress) * 0.65,
                (state.elapsed * 61.0).cos() * (1.0 - explosion_progress) * 0.85,
            )
        } else {
            Vec3::default()
        };
        let position = base_position.add(shake);
        let scale = if exploding {
            3.55 * (1.0 + explosion_progress * 0.32)
        } else if state.boss_hit_flash > 0.0 {
            3.75
        } else {
            3.55
        };
        let sway = (state.elapsed * 1.6).sin() * 0.08;
        let forward = Vec3::new(sway.cos(), 0.0, sway.sin());
        let right = Vec3::new(-sway.sin(), 0.0, sway.cos());
        let up = Vec3::new(0.0, 1.0, 0.0);
        if !exploding || explosion_progress < 0.74 {
            queue_model(
                model,
                batch,
                &camera,
                |vertex| {
                    let local = vertex.sub(center).mul(WATER_ENEMY_SCALE * scale);
                    position
                        .add(forward.mul(local.x))
                        .add(right.mul(local.y))
                        .add(up.mul(local.z))
                },
                None,
                None,
            );
        }
        if exploding {
            for fragment in 0..18_u32 {
                let angle = fragment as f32 * 2.399_963;
                let vertical = (hash(fragment * 71 + 9) as f32 / u32::MAX as f32) * 2.0 - 1.0;
                let radial = (1.0 - vertical * vertical).sqrt();
                let direction = Vec3::new(angle.cos() * radial, vertical, angle.sin() * radial);
                let distance = explosion_progress.powf(0.72) * (10.0 + fragment as f32 % 5.0 * 2.2);
                let fragment_position = state
                    .boss_explosion_position
                    .add(direction.mul(distance))
                    .add(Vec3::new(
                        0.0,
                        -explosion_progress * explosion_progress * 3.0,
                        0.0,
                    ));
                let fragment_scale = 0.28 + (fragment % 4) as f32 * 0.09;
                queue_model(
                    enemy_laser_bolt,
                    batch,
                    &camera,
                    |vertex| fragment_position.add(vertex.mul(fragment_scale)),
                    None,
                    None,
                );
            }
        }
    }
    if state.boss_defeated && (state.progress - WATER_REWARD_PROGRESS).abs() < 145.0 {
        queue_model(
            reward_pedestal,
            batch,
            &camera,
            |vertex| Vec3::new(-WATER_REWARD_PROGRESS, 2.4, 0.0).add(vertex),
            None,
            None,
        );
        if !state.dragon_ball_collected {
            let bob = (world_time * 2.2).sin() * 0.24;
            let pulse = 1.0 + (world_time * 3.1).sin() * 0.035;
            queue_model(
                dragon_ball,
                batch,
                &camera,
                |vertex| {
                    water_reward_position()
                        .add(Vec3::new(0.0, bob, 0.0))
                        .add(vertex.mul(pulse))
                },
                None,
                None,
            );
        }
    }
    render_enemy_projectiles(&state.enemy_projectiles, enemy_laser_bolt, batch, &camera);
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
        1.0,
    );
    batch.flush(framebuffer);
    apply_underwater_lighting(framebuffer, world_time);
    draw_projectile_glow(
        framebuffer,
        &state.enemy_projectiles,
        &camera,
        world_time,
        [255, 68, 24],
        1.45,
    );
    draw_projectile_glow(
        framebuffer,
        projectiles,
        &camera,
        world_time,
        ship.spec.laser_color,
        1.65,
    );
    draw_boss_explosion_glow(framebuffer, &camera, state);
}

fn hud_rect(framebuffer: &mut Framebuffer, left: usize, top: usize, width: usize, height: usize) {
    let right = (left + width).min(framebuffer.width);
    let bottom = (top + height).min(framebuffer.height);
    for y in top..bottom {
        for x in left..right {
            let border = x < left + 3 || x + 3 >= right || y < top + 3 || y + 3 >= bottom;
            let corner_cut = (x < left + 15 && y < top + 15 - (x - left))
                || (x + 15 >= right && y + 15 >= bottom + (x + 15 - right));
            if corner_cut {
                continue;
            }
            let index = y * framebuffer.width + x;
            if border {
                framebuffer.pixels[index] = 0x46e6ff;
            } else {
                let old = framebuffer.pixels[index];
                let r = ((old >> 16) & 255) / 5;
                let g = (((old >> 8) & 255) / 5 + 10).min(255);
                let b = ((old & 255) / 4 + 25).min(255);
                framebuffer.pixels[index] = (r << 16) | (g << 8) | b;
            }
        }
    }
}

fn hud_bar(
    framebuffer: &mut Framebuffer,
    left: usize,
    top: usize,
    width: usize,
    height: usize,
    ratio: f32,
    color: u32,
) {
    let filled = (width as f32 * ratio.clamp(0.0, 1.0)) as usize;
    for y in top..(top + height).min(framebuffer.height) {
        for x in left..(left + width).min(framebuffer.width) {
            framebuffer.pixels[y * framebuffer.width + x] =
                if x - left < filled { color } else { 0x173448 };
        }
    }
}

fn draw_unified_combat_hud(
    framebuffer: &mut Framebuffer,
    objective: &str,
    shield_ratio: f32,
    score: u32,
    boost_ratio: f32,
    charge_ratio: f32,
    boss: Option<(&str, f32)>,
    aim_position: (f32, f32),
) {
    let width = framebuffer.width as f32;
    let height = framebuffer.height as f32;
    let unit = width.min(height);
    let scale = ((height / 900.0).round() as i32).clamp(1, 2);
    let x = |ratio: f32| (ratio * width).round() as usize;
    let y = |ratio: f32| (ratio * height).round() as usize;
    let label = |frame: &mut Framebuffer, text: &str, xr: f32, yr: f32, color: u32| {
        draw_selection_label(frame, text, x(xr) as i32, y(yr) as i32, scale, color);
    };
    let panel = |frame: &mut Framebuffer, l: f32, t: f32, r: f32, b: f32| {
        hud_rect(frame, x(l), y(t), x(r - l).max(1), y(b - t).max(1));
    };
    let bar = |frame: &mut Framebuffer, l: f32, t: f32, r: f32, b: f32, ratio: f32, color| {
        hud_bar(
            frame,
            x(l),
            y(t),
            x(r - l).max(1),
            y(b - t).max(3),
            ratio,
            color,
        );
    };

    // Compact corner cards leave the center and horizon open for navigation.
    panel(framebuffer, 0.018, 0.020, 0.226, 0.091);
    label(framebuffer, "INTEGRIDAD", 0.072, 0.034, 0xffffff);
    bar(
        framebuffer,
        0.032,
        0.060,
        0.211,
        0.073,
        shield_ratio,
        0x22e8f4,
    );
    label(
        framebuffer,
        &format!("{:03}%", (shield_ratio.clamp(0.0, 1.0) * 100.0) as u32),
        0.055,
        0.081,
        0xbfefff,
    );

    panel(framebuffer, 0.018, 0.858, 0.318, 0.980);
    label(framebuffer, "OBJETIVO", 0.168, 0.883, 0xffffff);
    label(framebuffer, objective, 0.168, 0.936, 0xcaf5ff);

    panel(framebuffer, 0.774, 0.020, 0.982, 0.091);
    label(framebuffer, "PUNTAJE", 0.878, 0.034, 0xffffff);
    label(
        framebuffer,
        &format!("{:07}", score),
        0.878,
        0.068,
        0xe9f8ff,
    );

    panel(framebuffer, 0.797, 0.826, 0.982, 0.982);
    label(framebuffer, "ENERGIA", 0.889, 0.842, 0xffffff);
    bar(
        framebuffer,
        0.814,
        0.862,
        0.965,
        0.872,
        boost_ratio,
        0x20dcf2,
    );
    label(
        framebuffer,
        &format!("{:03}%", (boost_ratio * 100.0) as u32),
        0.889,
        0.888,
        0xdafcff,
    );
    label(framebuffer, "ARMA", 0.889, 0.914, 0xffffff);
    bar(
        framebuffer,
        0.814,
        0.934,
        0.965,
        0.944,
        charge_ratio,
        0x26e7f2,
    );
    label(framebuffer, "ENTER: DISPARAR", 0.889, 0.963, 0xcaf5ff);

    // Small, centered reticle scales with the viewport and remains inside it.
    let arm = (unit * 0.022).clamp(10.0, 22.0) as i32;
    let cx = (aim_position.0.clamp(0.08, 0.92) * width) as i32;
    let cy = (aim_position.1.clamp(0.12, 0.82) * height) as i32;
    for offset in 0..=arm {
        if offset < arm * 48 / 100 || offset > arm * 78 / 100 {
            for (px, py) in [
                (cx - arm + offset, cy - arm),
                (cx + arm - offset, cy + arm),
                (cx + arm, cy - arm + offset),
                (cx - arm, cy + arm - offset),
            ] {
                if px >= 0
                    && py >= 0
                    && px < framebuffer.width as i32
                    && py < framebuffer.height as i32
                {
                    framebuffer.pixels[py as usize * framebuffer.width + px as usize] = 0x56eaff;
                }
            }
        }
    }
    for offset in -5..=5 {
        let px = cx + offset;
        let py = cy + offset;
        if px >= 0 && px < framebuffer.width as i32 {
            framebuffer.pixels[cy as usize * framebuffer.width + px as usize] = 0xff685c;
        }
        if py >= 0 && py < framebuffer.height as i32 {
            framebuffer.pixels[py as usize * framebuffer.width + cx as usize] = 0xff685c;
        }
    }

    if let Some((name, health)) = boss {
        panel(framebuffer, 0.30, 0.105, 0.70, 0.158);
        label(framebuffer, name, 0.50, 0.117, 0xffffff);
        bar(framebuffer, 0.32, 0.140, 0.68, 0.151, health, 0xff4d47);
    }
}

fn draw_split_coop_hud(
    framebuffer: &mut Framebuffer,
    first: &ShipController,
    second: &ShipController,
    dragon_balls: u8,
) {
    let half = framebuffer.width / 2;
    let scale = ((framebuffer.height as f32 / 900.0).round() as i32).clamp(1, 2);
    for (index, controller) in [first, second].into_iter().enumerate() {
        let left = index * half;
        let center = left + half / 2;
        hud_rect(framebuffer, left + 14, 16, 190.min(half - 28), 58);
        draw_selection_label(
            framebuffer,
            if index == 0 { "JUGADOR 1" } else { "JUGADOR 2" },
            (left + 105.min(half / 2)) as i32,
            25,
            scale,
            if index == 0 { 0x56eaff } else { 0xffd15c },
        );
        draw_selection_label(
            framebuffer,
            &format!(
                "ENERGIA {:03}%",
                (controller.turbo_power(controller.ship.speed) * 100.0) as u32
            ),
            (left + 105.min(half / 2)) as i32,
            49,
            scale,
            0xffffff,
        );
        draw_selection_label(
            framebuffer,
            &format!("ESFERAS {}/7", dragon_balls.count_ones()),
            (left + 105.min(half / 2)) as i32,
            72,
            scale,
            0xffc95c,
        );
        hud_rect(
            framebuffer,
            left + 14,
            framebuffer.height - 66,
            half.saturating_sub(28),
            50,
        );
        draw_selection_label(
            framebuffer,
            if index == 1 {
                "SELECT: HANGAR  //  C: SALIR"
            } else {
                "CRUCETA: NAVE  //  C: SALIR"
            },
            center as i32,
            framebuffer.height as i32 - 49,
            scale,
            0xcaf5ff,
        );
        let cy = framebuffer.height as i32 / 2;
        let cx = center as i32;
        for offset in -10_i32..=10 {
            if offset.abs() > 3 {
                let x = cx + offset;
                let y = cy + offset;
                framebuffer.pixels[cy as usize * framebuffer.width + x as usize] = 0x56eaff;
                framebuffer.pixels[y as usize * framebuffer.width + cx as usize] = 0x56eaff;
            }
        }
    }
}

fn draw_mission_complete_overlay(framebuffer: &mut Framebuffer, remaining: f32) {
    let fade = ((WATER_MISSION_COMPLETE_TIME - remaining) / 0.55).clamp(0.0, 1.0);
    for pixel in &mut framebuffer.pixels {
        let red = (((*pixel >> 16) & 255) as f32 * (1.0 - fade * 0.68)) as u32;
        let green = (((*pixel >> 8) & 255) as f32 * (1.0 - fade * 0.62)) as u32;
        let blue = ((*pixel & 255) as f32 * (1.0 - fade * 0.52)) as u32;
        *pixel = (red << 16) | (green << 8) | blue;
    }

    hud_rect(framebuffer, 250, 245, 700, 285);
    hud_rect(framebuffer, 275, 270, 650, 235);
    draw_selection_label(framebuffer, "MISION CUMPLIDA", 600, 305, 5, 0xffd34e);
    draw_selection_label(
        framebuffer,
        "TIBURON MECANICO DESTRUIDO",
        600,
        385,
        2,
        0xd8f8ff,
    );
    draw_selection_label(
        framebuffer,
        "ESFERA DE 4 ESTRELLAS RECUPERADA",
        600,
        420,
        2,
        0xffa62b,
    );
    let seconds = remaining.ceil().max(0.0) as u32;
    draw_selection_label(
        framebuffer,
        &format!("REGRESO AL MUNDO LIBRE EN {}", seconds),
        600,
        468,
        2,
        0x75efff,
    );
}

fn draw_story_epilogue(framebuffer: &mut Framebuffer, remaining: f32) {
    let width = framebuffer.width;
    let height = framebuffer.height;
    let panel_width = (width as f32 * 0.58) as usize;
    let panel_height = (height as f32 * 0.115) as usize;
    let left = (width - panel_width) / 2;
    let top = (height as f32 * 0.16) as usize;
    hud_rect(framebuffer, left, top, panel_width, panel_height);
    let pulse = (remaining * 2.0).sin().abs();
    let accent = if pulse > 0.5 { 0xffd27b } else { 0x68e8ff };
    draw_selection_label(
        framebuffer,
        "NEXO ESTABILIZADO // LAS GALAXIAS ESTAN A SALVO",
        (width / 2) as i32,
        (top + 10) as i32,
        2,
        accent,
    );
    draw_selection_label(
        framebuffer,
        "TU VUELO CONTINUA. NUEVOS HORIZONTES TE ESPERAN.",
        (width / 2) as i32,
        (top + 42) as i32,
        1,
        0xd7efff,
    );
}

fn draw_guardian_reward_overlay(framebuffer: &mut Framebuffer, remaining: u32) {
    hud_rect(framebuffer, 250, 245, 700, 285);
    hud_rect(framebuffer, 275, 270, 650, 235);
    draw_selection_label(framebuffer, "GUARDIAN DERROTADO", 600, 305, 4, 0xffd34e);
    draw_selection_label(
        framebuffer,
        "ESFERA DE 3 ESTRELLAS OBTENIDA",
        600,
        400,
        2,
        0xffa62b,
    );
    draw_selection_label(
        framebuffer,
        &format!("REGRESO AL MUNDO LIBRE EN {remaining}"),
        600,
        468,
        2,
        0x75efff,
    );
}

fn draw_tower_reward_hud(framebuffer: &mut Framebuffer, remaining: f32) {
    let elapsed = 5.5 - remaining;
    if elapsed < 3.35 {
        draw_selection_label(
            framebuffer,
            "RECOMPENSA DE LA MANSION",
            600,
            110,
            2,
            0xffd15c,
        );
        draw_selection_label(
            framebuffer,
            "RECIBIENDO ESFERA DEL DRAGON",
            600,
            142,
            1,
            0xffa62b,
        );
        return;
    }
    draw_selection_panel(framebuffer, 300, 250, 900, 555, true);
    draw_selection_label(framebuffer, "GANASTE", 600, 294, 3, 0xffdf6b);
    draw_selection_label(
        framebuffer,
        "OBJETIVO DE LA MANSION COMPLETADO",
        600,
        350,
        2,
        0x9fffd2,
    );
    draw_selection_label(
        framebuffer,
        "ESFERA DE 2 ESTRELLAS OBTENIDA",
        600,
        400,
        2,
        0xffa62b,
    );
    draw_selection_label(
        framebuffer,
        &format!("REGRESO AL MUNDO LIBRE EN {}", remaining.ceil() as u32),
        600,
        495,
        1,
        0x8feaff,
    );
}

#[allow(dead_code)]
fn draw_tower_reward_animation(framebuffer: &mut Framebuffer, remaining: f32) {
    let progress = (1.0 - remaining / 5.0).clamp(0.0, 1.0);
    let eased = 1.0 - (1.0 - progress).powi(3);
    let center_x = WIDTH as i32 / 2;
    let center_y = (HEIGHT as f32 * (0.88 - eased * 0.43)) as i32;
    let radius = (18.0 + eased * 72.0 + (progress * 10.0).sin().abs() * 4.0) as i32;

    for glow in (radius..=radius + 24).rev() {
        let strength = (radius + 24 - glow) as f32 / 24.0;
        let color = ((75.0 * strength) as u32) << 16
            | ((38.0 * strength) as u32) << 8
            | (8.0 * strength) as u32;
        for step in 0..180 {
            let angle = step as f32 * std::f32::consts::TAU / 180.0;
            let x = center_x + (angle.cos() * glow as f32) as i32;
            let y = center_y + (angle.sin() * glow as f32) as i32;
            if x >= 0 && y >= 0 && x < WIDTH as i32 && y < HEIGHT as i32 {
                framebuffer.pixels[y as usize * WIDTH + x as usize] = color;
            }
        }
    }
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let distance = (dx * dx + dy * dy) as f32;
            if distance > (radius * radius) as f32 {
                continue;
            }
            let normal_x = dx as f32 / radius as f32;
            let normal_y = dy as f32 / radius as f32;
            let light = (0.42 + (-normal_x * 0.45 - normal_y * 0.55).max(0.0)).min(1.0);
            let edge = (1.0 - distance.sqrt() / radius as f32).sqrt();
            let red = (255.0 * light).min(255.0) as u32;
            let green = (118.0 * light + 42.0 * edge) as u32;
            let blue = (18.0 * light) as u32;
            let x = center_x + dx;
            let y = center_y + dy;
            if x >= 0 && y >= 0 && x < WIDTH as i32 && y < HEIGHT as i32 {
                framebuffer.pixels[y as usize * WIDTH + x as usize] =
                    (red << 16) | (green << 8) | blue;
            }
        }
    }
    let rotation = progress * std::f32::consts::TAU * 1.5;
    for side in [-1.0_f32, 1.0] {
        let local_x = side * radius as f32 * 0.28;
        let local_y = radius as f32 * 0.04;
        let star_x = center_x + (local_x * rotation.cos() - local_y * rotation.sin()) as i32;
        let star_y = center_y + (local_x * rotation.sin() + local_y * rotation.cos()) as i32;
        for ray in 0..5 {
            let angle = ray as f32 * std::f32::consts::TAU / 5.0 - std::f32::consts::FRAC_PI_2;
            let end_x = star_x + (angle.cos() * radius as f32 * 0.17) as i32;
            let end_y = star_y + (angle.sin() * radius as f32 * 0.17) as i32;
            reward_line(framebuffer, star_x, star_y, end_x, end_y, 0xb51419);
        }
    }
    draw_selection_label(framebuffer, "MISION CUMPLIDA", center_x, 72, 4, 0xffd34e);
    draw_selection_label(
        framebuffer,
        "ESFERA DE 2 ESTRELLAS OBTENIDA",
        center_x,
        130,
        2,
        0xffa62b,
    );
}

#[allow(dead_code)]
fn reward_line(
    framebuffer: &mut Framebuffer,
    mut x0: i32,
    mut y0: i32,
    x1: i32,
    y1: i32,
    color: u32,
) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut error = dx + dy;
    loop {
        if x0 >= 0 && y0 >= 0 && x0 < WIDTH as i32 && y0 < HEIGHT as i32 {
            framebuffer.pixels[y0 as usize * WIDTH + x0 as usize] = color;
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let twice = error * 2;
        if twice >= dy {
            error += dy;
            x0 += sx;
        }
        if twice <= dx {
            error += dx;
            y0 += sy;
        }
    }
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
    let width = text.chars().count() as i32 * 6 * scale - scale;
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
            '·' => [0, 0, 0, 4, 0, 0, 0],
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
    gamepad_index: u32,
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
    let mut gamepad = crate::input::Gamepad::new(gamepad_index);
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
        gamepad.update();
        if crate::input::pressed(Key::Escape) || gamepad.pressed(crate::input::Gamepad::B) {
            return Ok(None);
        }
        if crate::input::pressed(Key::Left)
            || crate::input::pressed(Key::A)
            || gamepad.pressed(crate::input::Gamepad::DPAD_LEFT)
        {
            selected = (selected + choices.len() - 1) % choices.len();
        }
        if crate::input::pressed(Key::Right)
            || crate::input::pressed(Key::D)
            || gamepad.pressed(crate::input::Gamepad::DPAD_RIGHT)
        {
            selected = (selected + 1) % choices.len();
        }
        if crate::input::pressed(Key::V) || gamepad.pressed(crate::input::Gamepad::X) {
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
        if crate::input::pressed(Key::Enter) || gamepad.pressed(crate::input::Gamepad::A) {
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
        if gamepad.connected {
            draw_selection_label(
                framebuffer,
                "CONTROL BLUETOOTH / USB DETECTADO",
                WIDTH as i32 / 2,
                765,
                2,
                0x42dff0,
            );
        }
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
    let arwing_car_path = "assets/vehicles/arwing/arwing2_carro_detallado.obj";
    let kirby_path = "assets/vehicles/kirby/kirby_detallado.obj";
    let kirby_car_path = "assets/vehicles/kirby/kirby_carro_detallado.obj";
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
    let arwing_car_model = load_obj(arwing_car_path)
        .map_err(|error| format!("Error al cargar {arwing_car_path}: {error}"))?;
    let kirby_model =
        load_obj(kirby_path).map_err(|error| format!("Error al cargar {kirby_path}: {error}"))?;
    let kirby_car_model = load_obj(kirby_car_path)
        .map_err(|error| format!("Error al cargar {kirby_car_path}: {error}"))?;
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

    let mut fullscreen = START_FULLSCREEN;
    let mut window = create_window(fullscreen)?;
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
    let tower_sprites = tower_guard::TowerSprites::load().map_err(|error| {
        std::io::Error::other(format!(
            "No se pudieron cargar los sprites de Torre: {error}"
        ))
    })?;
    let story_guide = story_guide::Guide::load();
    let tower_audio = tower_guard::TowerAudio::load();
    let mut minigame_music = minigame_audio::MusicPlayer::new();
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
    let mut race_track = race_track::RaceTrack::load()?;
    let mut coop_race_track = race_track::RaceTrack::load()?;
    let race_vehicles = [
        Some((&arwing_car_model, &ARWING_CAR_SPEC)),
        Some((&kirby_car_model, &KIRBY_CAR_SPEC)),
        Some((&astra_car_model, &ASTRA_CAR_SPEC)),
        Some((&cowboyb_car_model, &COWBOYB_CAR_SPEC)),
        Some((&buzz_variants[1], &BUZZ_CAR_SPEC)),
        Some((&mcqueen_car_model, &MCQUEEN_CAR_SPEC)),
    ];
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
            (&arwing_car_model, &ARWING_CAR_SPEC, "MODO CARRO"),
            (&arwing_submarine_model, &ARWING_SUBMARINE_SPEC, "SUBMARINO"),
        ],
        vec![
            (&kirby_model, &KIRBY_SPEC, "MODO ESPACIAL"),
            (&kirby_car_model, &KIRBY_CAR_SPEC, "MODO CARRO"),
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
    let mut benchmark = BenchmarkRun::from_environment();
    let (mut selected_ship, initial_form) = if benchmark.is_some() {
        (2, 0)
    } else {
        loop {
            if menu::show(
                &mut window,
                &mut framebuffer,
                false,
                false,
                &mut minigame_music,
            )? == menu::Action::Quit
            {
                return Ok(false);
            }
            if let Some(index) =
                select_ship(&mut window, &mut framebuffer, &choices, &ship_forms, 0, 0)?
            {
                break index;
            }
            if !window.is_open() {
                return Ok(false);
            }
        }
    };
    // Selection is local to each player. P2 starts with P1's choice but can
    // change it at any time while flying freely.
    let mut coop_selected_ship = selected_ship;
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
    let planet_lods = [
        make_planet_lods(WATER_PLANET_RADIUS, PlanetVisual::Water),
        make_planet_lods(SPIKY_PLANET_RADIUS, PlanetVisual::Spiky),
        make_planet_lods(race_track::PLANET_RADIUS, PlanetVisual::Racing),
    ];
    let mut planet_lod_states = [PlanetLodState::default(); 3];
    let mut vesper_lod_states = [PlanetLodState::default(); 2];
    let mut castle_lod_state = PlanetLodState::default();
    let water_level_path = "assets/levels/waterlevel_extendido.obj";
    let mut water_level = load_obj(water_level_path)
        .map_err(|error| format!("Error al cargar {water_level_path}: {error}"))?;
    water_level
        .faces
        .retain(|face| !face.group.starts_with("CAVES_AND_ARCHES"));
    stretch_water_level_width(&mut water_level, WATER_LEVEL_WIDTH_SCALE);
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
    let mut coop_enemy_instance_buffers: [Vec<Vec3>; 3] =
        std::array::from_fn(|index| Vec::with_capacity(enemy_models[index].vertices.len()));
    let mut dynamic_resolution = DynamicResolution::new();
    let (initial_render_width, initial_render_height) = dynamic_resolution.dimensions();
    let mut scene_framebuffer = Framebuffer::new(initial_render_width, initial_render_height);
    let mut triangle_batch = TriangleBatch::new(initial_render_width, initial_render_height);
    let mut coop_framebuffer = Framebuffer::new(initial_render_width, initial_render_height);
    let mut coop_triangle_batch = TriangleBatch::new(initial_render_width, initial_render_height);
    let mut hybrid_ray_tracing = HybridRayTracing::new();
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
    let sun_path = "assets/planets/sol_detallado.obj";
    let sun = load_obj(sun_path).map_err(|error| format!("Error al cargar {sun_path}: {error}"))?;
    let sun_center = model_center(&sun);
    let sun_collision_radius = model_radius(&sun, sun_center) * SUN_VISUAL_SCALE;
    let home_orbit_rings = make_home_orbit_rings();
    let vesper_planets = [
        VesperPlanet::load(
            "assets/planets/reloj_arena.obj",
            vesper_planet_position(0, 0.0),
            42.0,
            0.24,
            PlanetVisual::VesperClock,
        )?,
        VesperPlanet::load(
            "assets/planets/caracola.obj",
            vesper_planet_position(1, 0.0),
            34.0,
            0.0,
            PlanetVisual::VesperShell,
        )?,
    ];
    let vesper_black_hole = make_vesper_black_hole();
    let vesper_orbit_rings = make_vesper_orbit_rings();
    let castle_planet = VesperPlanet::load(
        "assets/planets/planeta_castillo_lava_optimizado.obj",
        Vec3::new(260.0, 90.0, 230.0),
        58.0,
        0.0,
        PlanetVisual::Castle,
    )?;
    let race_sky = race_track::RaceSky {
        water_planet: &planet,
        water_center: planet_center,
        water_scale: planet_scale,
        pink_planet: &spiky_planet,
        pink_center: spiky_planet_center,
        pink_scale: spiky_planet_scale,
        sun: &sun,
        sun_center,
    };
    let wormhole = make_wormhole_sphere(22.0, 18, 28);
    let turbo_plume = make_turbo_plume(12);
    let laser_bolt = make_laser_bolt();
    let enemy_laser_bolt = make_enemy_laser_bolt();
    let bubble_model = make_bubble_model();
    let underwater_boost_ring = make_underwater_boost_ring();
    let four_star_dragon_ball = make_four_star_dragon_ball();
    let one_star_dragon_ball = make_one_star_dragon_ball();
    let two_star_dragon_ball = make_two_star_dragon_ball();
    let three_star_dragon_ball = make_three_star_dragon_ball();
    let six_star_dragon_ball = make_six_star_dragon_ball();
    let seven_star_dragon_ball = make_seven_star_dragon_ball();
    let reward_pedestal = make_reward_pedestal();
    let reward_chamber = make_reward_chamber();
    let reward_door = make_reward_door();

    println!("Vuelo libre 3D iniciado: {}", ship_spec.name);
    println!("W/S o flechas: subir/bajar | Espacio: acelerar | Ctrl: frenar");
    println!("A/D: direccion | Q/E: alabeo | Enter: disparar/cargar | R: reiniciar | P: captura");
    println!("F4: iluminacion avanzada (desactivada por defecto para mejor rendimiento)");
    println!("1: warp al Abismo Azul | Shift: velocidad de luz");
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
    let mut coop_controller = ShipController::new(ship_spec);
    coop_controller.ship.position = ship_controller.ship.position.add(Vec3::new(8.0, 0.0, 0.0));
    let mut coop_enabled = false;
    let mut coop_game_mode = GameMode::FreeWorld;
    let mut coop_water_level_state = WaterLevelState::new();
    let mut coop_bubble_emitter = BubbleEmitter::new();
    let mut coop_guardian_fight = CoreGuardian::new();
    let mut coop_hourglass_rescue = hourglass_rescue::HourglassRescue::new();
    let mut coop_lava_castle = lava_castle::LavaCastle::new();
    let mut coop_lava_castle_interior = lava_castle_interior::LavaCastleInterior::new();
    let mut coop_vesper_orbit_index: Option<usize> = None;
    let mut coop_planet_lod_states = [PlanetLodState::default(); 3];
    let mut coop_vesper_lod_states = [PlanetLodState::default(); 2];
    let mut coop_castle_lod_state = PlanetLodState::default();
    let mut game_mode = if start_underwater {
        GameMode::WaterLevel
    } else {
        GameMode::FreeWorld
    };
    let mut active_galaxy = Galaxy::Home;
    // The guest is a separate local session: its portal travel must never
    // change the galaxy shown to player one.
    let mut coop_active_galaxy = Galaxy::Home;
    let mut vesper_orbit_index: Option<usize> = None;
    let mut galaxy_transition: Option<GalaxyTransit> = None;
    if start_underwater {
        window.set_title("Nivel submarino | Backspace: volver al sistema solar");
    }
    let mut water_level_state = WaterLevelState::new();
    let mut bubble_emitter = BubbleEmitter::new();
    let mut guardian_fight = CoreGuardian::new();
    let mut hourglass_rescue = hourglass_rescue::HourglassRescue::new();
    let mut lava_castle = lava_castle::LavaCastle::new();
    let mut lava_castle_interior = lava_castle_interior::LavaCastleInterior::new();
    let mut tower_game = tower_guard::TowerGame::new(&tower_sprites);
    let mut coop_tower_game = tower_guard::TowerGame::new(&tower_sprites);
    let mut ambient_battle = space_battle::SpaceBattle::new();
    let mut previous = Instant::now();
    let mut world_time = 0.0_f32;
    let mut distance = 0.0_f32;
    let mut performance_hud = false;
    let mut advanced_lighting = std::env::var_os("GRAPH_BENCH_ADVANCED").is_some();
    let mut last_tower_cursor = (0.5_f32, 0.52_f32);
    let mut tower_mouse_down = false;
    let mut tower_dragon_ball_collected = false;
    // One bit per Dragon Ball. Both local players read and write this shared
    // progression, so a completed planet cannot be farmed twice.
    let mut collected_dragon_balls: u8 = 0;
    let mut story_epilogue_timer = 0.0_f32;
    let mut tower_reward_timer = 0.0_f32;
    let mut tower_reward_position = Vec3::default();
    let mut solar_explosion_timer = 0.0_f32;
    let mut solar_explosion_position = Vec3::default();
    let mut black_hole_absorption_timer = 0.0_f32;
    let mut black_hole_absorption_start = Vec3::default();
    let mut gamepad = crate::input::Gamepad::default();
    let mut gamepad_two = crate::input::Gamepad::new(1);
    let mut gamepad_was_connected = false;
    let mut gamepad_two_was_connected = false;
    let mut motion_was_active = false;

    while window.is_open() {
        minigame_music.update(game_mode.music_track());
        gamepad.update();
        gamepad_two.update();
        if gamepad.connected != gamepad_was_connected {
            if gamepad.connected {
                println!("Control detectado y listo para jugar");
            } else {
                println!("Control desconectado; teclado y mouse siguen activos");
            }
            gamepad_was_connected = gamepad.connected;
        }
        let motion_active = gamepad.motion_active();
        if motion_active && !motion_was_active {
            println!("Control de movimiento detectado");
        }
        motion_was_active = motion_active;
        if gamepad_two.connected != gamepad_two_was_connected {
            println!(
                "Control 2 {}",
                if gamepad_two.connected {
                    "conectado"
                } else {
                    "desconectado"
                }
            );
            gamepad_two_was_connected = gamepad_two.connected;
        }
        let benchmark_frame_start = Instant::now();
        if game_mode == GameMode::HourglassRescue && !window.is_active() {
            // Do not let Alt+Tab or a transient Windows focus change eat the
            // mission timer or accumulate stale key presses.
            window.update();
            previous = Instant::now();
            crate::input::clear();
            continue;
        }
        if benchmark.is_some() {
            reset_local_render_stats();
            reset_race_benchmark_categories();
        }
        if game_mode == GameMode::Racing
            && (crate::input::pressed(Key::Escape)
                || crate::input::pressed(Key::P)
                || gamepad.pressed(crate::input::Gamepad::START))
        {
            race_track.pause();
            match menu::show(
                &mut window,
                &mut framebuffer,
                true,
                true,
                &mut minigame_music,
            )? {
                menu::Action::Quit => return Ok(false),
                menu::Action::Play => race_track.resume(),
                menu::Action::ExitToSpace => {
                    game_mode = GameMode::FreeWorld;
                    ship_controller = ShipController::new(ship_spec);
                    race_track.leave(&mut ship_controller, race_planet_position(world_time));
                    window.set_title("Sistema solar | Vuelo libre 3D | F11: fullscreen");
                    previous = Instant::now();
                    crate::input::clear();
                    continue;
                }
                menu::Action::Hangar => {
                    if let Some((new_selection, _)) = select_ship(
                        &mut window,
                        &mut framebuffer,
                        &choices,
                        &ship_forms,
                        selected_ship,
                        0,
                    )? {
                        if let Some((_, car_spec)) = race_vehicles[new_selection] {
                            selected_ship = new_selection;
                            (ship_model, ship_spec) = choices[new_selection];
                            base_center = model_center(ship_model);
                            car_mode = false;
                            ship_controller = ShipController::new(car_spec);
                            race_track.enter(&mut ship_controller, selected_ship, &race_vehicles);
                        } else {
                            println!(
                                "No hay variante de carro para {}. Agrega el OBJ de carro antes de seleccionarla.",
                                choices[new_selection].1.name
                            );
                        }
                    }
                }
            }
            previous = Instant::now();
            crate::input::clear();
            continue;
        }
        if game_mode == GameMode::TowerGuard
            && (crate::input::pressed(Key::Escape)
                || crate::input::pressed(Key::P)
                || gamepad.pressed(crate::input::Gamepad::START))
        {
            tower_game.pause();
            window.set_cursor_visibility(true);
            match menu::show(
                &mut window,
                &mut framebuffer,
                true,
                true,
                &mut minigame_music,
            )? {
                menu::Action::Quit => return Ok(false),
                menu::Action::Play => tower_game.resume(),
                menu::Action::ExitToSpace => {
                    tower_audio.stop_music();
                    tower_game.resume();
                    game_mode = GameMode::FreeWorld;
                    launch_ship_from_planet(
                        &mut ship_controller,
                        vesper_planet_position(1, world_time),
                        vesper_planets[1].radius,
                    );
                    window.set_cursor_visibility(true);
                    window.set_title("Galaxia Vesper | Vuelo libre 3D");
                    previous = Instant::now();
                    crate::input::clear();
                    continue;
                }
                menu::Action::Hangar => {
                    if let Some((new_selection, new_form)) = select_ship(
                        &mut window,
                        &mut framebuffer,
                        &choices,
                        &ship_forms,
                        selected_ship,
                        0,
                    )? {
                        selected_ship = new_selection;
                        (ship_model, ship_spec) = choices[selected_ship];
                        car_mode = selected_ship == 5 && new_form == 1;
                        if car_mode {
                            ship_spec = &MCQUEEN_CAR_SPEC;
                        }
                        base_center = model_center(ship_model);
                        ship_controller.ship.spec = ship_spec;
                    }
                    tower_game.resume();
                }
            }
            window.set_cursor_visibility(!matches!(
                tower_game.phase(),
                tower_guard::Phase::Encounter
                    | tower_guard::Phase::Boss
                    | tower_guard::Phase::Intro
                    | tower_guard::Phase::Resolving
                    | tower_guard::Phase::Transition
            ));
            previous = Instant::now();
            crate::input::clear();
            continue;
        }
        if game_mode != GameMode::TowerGuard
            && game_mode != GameMode::HourglassRescue
            && (crate::input::pressed(Key::Escape) || gamepad.pressed(crate::input::Gamepad::START))
        {
            match menu::show(
                &mut window,
                &mut framebuffer,
                true,
                game_mode.is_planet_minigame(),
                &mut minigame_music,
            )? {
                menu::Action::Quit => return Ok(false),
                menu::Action::ExitToSpace => {
                    let leaving = game_mode;
                    game_mode = GameMode::FreeWorld;
                    match leaving {
                        GameMode::CoreGuardian => {
                            launch_ship_after_guardian_exit(&mut ship_controller, world_time);
                        }
                        GameMode::WaterLevel => {
                            bubble_emitter.clear();
                            ship_controller = ShipController::new(ship_spec);
                            launch_ship_from_planet(
                                &mut ship_controller,
                                water_planet_position(world_time),
                                WATER_PLANET_RADIUS,
                            );
                        }
                        _ => {}
                    }
                    window.set_cursor_visibility(true);
                    window.set_title("Sistema solar | Vuelo libre 3D | F11: fullscreen");
                    previous = Instant::now();
                    crate::input::clear();
                    continue;
                }
                menu::Action::Hangar => {
                    if let Some((new_selection, new_form)) = select_ship(
                        &mut window,
                        &mut framebuffer,
                        &choices,
                        &ship_forms,
                        selected_ship,
                        0,
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
        if game_mode == GameMode::FreeWorld
            && (crate::input::pressed(Key::C)
                || (!coop_enabled
                    && gamepad_two.connected
                    && gamepad_two.pressed(crate::input::Gamepad::START)))
        {
            coop_enabled = !coop_enabled;
            if coop_enabled {
                coop_controller = ShipController::new(choices[coop_selected_ship].1);
                coop_controller.ship.position =
                    ship_controller.ship.position.add(Vec3::new(8.0, 0.0, 0.0));
                coop_controller.ship.yaw = ship_controller.ship.yaw;
                window.set_title("Cooperativo local | P1: WASD | P2: IJKL | C: salir");
            } else {
                window.set_title("Sistema solar | Vuelo libre 3D | C: cooperativo");
            }
        }
        if coop_enabled
            && coop_game_mode == GameMode::FreeWorld
            && !coop_controller.ship.planet_locked
            && gamepad_two.pressed(crate::input::Gamepad::SELECT)
        {
            if let Some((new_selection, _)) = select_ship(
                &mut window,
                &mut framebuffer,
                &choices,
                &ship_forms,
                coop_selected_ship,
                1,
            )? {
                coop_selected_ship = new_selection;
                coop_controller.ship.spec = choices[coop_selected_ship].1;
            }
            previous = Instant::now();
            crate::input::clear();
            continue;
        }
        if game_mode == GameMode::FreeWorld
            && !ship_controller.ship.planet_locked
            && (gamepad.pressed(crate::input::Gamepad::DPAD_LEFT)
                || gamepad.pressed(crate::input::Gamepad::DPAD_RIGHT))
        {
            let direction = if gamepad.pressed(crate::input::Gamepad::DPAD_LEFT) {
                choices.len() - 1
            } else {
                1
            };
            selected_ship = (selected_ship + direction) % choices.len();
            (ship_model, ship_spec) = choices[selected_ship];
            car_mode = false;
            base_center = model_center(ship_model);
            ship_controller.ship.spec = ship_spec;
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
        }
        if coop_enabled
            && coop_game_mode == GameMode::FreeWorld
            && !coop_controller.ship.planet_locked
            && (gamepad_two.pressed(crate::input::Gamepad::DPAD_LEFT)
                || gamepad_two.pressed(crate::input::Gamepad::DPAD_RIGHT))
        {
            let direction = if gamepad_two.pressed(crate::input::Gamepad::DPAD_LEFT) {
                choices.len() - 1
            } else {
                1
            };
            coop_selected_ship = (coop_selected_ship + direction) % choices.len();
            coop_controller.ship.spec = choices[coop_selected_ship].1;
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
        let delta_time = if benchmark.is_some() {
            1.0 / 60.0
        } else {
            frame_time.min(0.05)
        };
        tower_reward_timer = (tower_reward_timer - delta_time).max(0.0);
        let solar_explosion_was_active = solar_explosion_timer > 0.0;
        solar_explosion_timer = (solar_explosion_timer - delta_time).max(0.0);
        if solar_explosion_was_active && solar_explosion_timer <= 0.0 {
            launch_ship_from_planet(
                &mut ship_controller,
                Vec3::new(SUN_POSITION_X, 20.0, 0.0),
                sun_collision_radius,
            );
            window.set_title("Sistema solar | Nave restaurada | Evita el Sol");
        }
        let black_hole_was_absorbing = black_hole_absorption_timer > 0.0;
        black_hole_absorption_timer = (black_hole_absorption_timer - delta_time).max(0.0);
        if black_hole_was_absorbing && black_hole_absorption_timer <= 0.0 {
            ship_controller.reset();
            let ship = &mut ship_controller.ship;
            ship.position = wormhole_position(Galaxy::Vesper).add(Vec3::new(-90.0, 22.0, -125.0));
            (ship.yaw, ship.pitch) = direction_to_angles(VESPER_SYSTEM_CENTER.sub(ship.position));
            ship.speed = ship.spec.cruise_speed;
            window.set_title("Galaxia Vesper | La nave escapo del agujero negro");
        }
        previous = now;
        world_time += delta_time;
        story_epilogue_timer = (story_epilogue_timer - delta_time).max(0.0);

        if let Some(transit) = &mut galaxy_transition {
            transit.elapsed += delta_time;
            if !transit.switched && transit.elapsed < 0.82 {
                let t = (transit.elapsed / 0.82).clamp(0.0, 1.0);
                let eased = t * t * (3.0 - 2.0 * t);
                let offset = transit.portal_position.sub(transit.start_position);
                ship_controller.ship.position = transit.start_position.add(offset.mul(eased));
            }
            if !transit.switched && transit.elapsed >= 0.82 {
                if transit.destination == Galaxy::Vesper
                    && collected_dragon_balls & 0b0111_1111 == 0b0111_1111
                {
                    story_epilogue_timer = 7.0;
                }
                active_galaxy = transit.destination;
                let portal = wormhole_position(active_galaxy);
                let (exit_offset, exit_yaw) = match active_galaxy {
                    Galaxy::Vesper => {
                        // Spawn facing the planet cluster with the portal behind.
                        (Vec3::new(-420.0, 0.0, 0.0), std::f32::consts::FRAC_PI_2)
                    }
                    Galaxy::Nova => {
                        // Enter the green Nova nebula clear of its gateway.
                        (Vec3::new(0.0, 20.0, -150.0), std::f32::consts::PI)
                    }
                    Galaxy::Home => {
                        // Arrive beside the solar portal and fly away from it.
                        (Vec3::new(150.0, 0.0, 80.0), 0.0)
                    }
                };
                ship_controller.ship.position = portal.add(exit_offset);
                ship_controller.ship.yaw = exit_yaw;
                ship_controller.ship.pitch = 0.0;
                ship_controller.ship.planet_locked = false;
                ship_controller.ship.spiky_planet_locked = false;
                ship_controller.ship.race_planet_locked = false;
                vesper_orbit_index = None;
                ship_controller.ship.orbit_plane_normal = None;
                ship_controller.ship.orbit_center = None;
                ship_controller.ship.orbit_camera_blend = 0.0;
                ship_controller.ship.orbit_release_active = false;
                black_hole_absorption_timer = 0.0;
                transit.switched = true;
            }
        }

        let resolution_changed = dynamic_resolution.update(frame_time, triangle_batch.stats());
        let toggled_resolution = if crate::input::pressed(Key::F2) {
            dynamic_resolution.toggle()
        } else {
            false
        };
        if crate::input::pressed(Key::F3) {
            performance_hud = !performance_hud;
        }
        if crate::input::pressed(Key::F4) {
            advanced_lighting = !advanced_lighting;
        }
        if resolution_changed || toggled_resolution {
            let (render_width, render_height) = dynamic_resolution.dimensions();
            scene_framebuffer = Framebuffer::new(render_width, render_height);
            triangle_batch.resize(render_width, render_height);
        }

        // Recreating a fullscreen window during the rescue mission can drop
        // keyboard focus on Windows. Keep the active mission window stable.
        if game_mode != GameMode::HourglassRescue && crate::input::pressed(Key::F11) {
            fullscreen = !fullscreen;
            window = create_window(fullscreen)?;
            if game_mode == GameMode::TowerGuard {
                window.set_cursor_visibility(false);
            }
        }
        if game_mode != GameMode::TowerGuard
            && game_mode != GameMode::HourglassRescue
            && crate::input::pressed(Key::R)
        {
            if game_mode == GameMode::Racing {
                race_track.recover(&mut ship_controller);
            } else if game_mode == GameMode::CoreGuardian {
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
        if game_mode == GameMode::FreeWorld
            && active_galaxy == Galaxy::Home
            && crate::input::pressed(Key::Key1)
        {
            place_ship_near_water_planet(&mut ship_controller, world_time);
            println!("Warp completado: Abismo Azul");
        }
        if game_mode == GameMode::FreeWorld
            && active_galaxy == Galaxy::Home
            && crate::input::pressed(Key::Key2)
        {
            place_ship_near_spiky_planet(&mut ship_controller, world_time);
            println!("Warp completado: Nova Rosa | Enter: Guardián del Núcleo");
        }

        if game_mode == GameMode::FreeWorld
            && active_galaxy == Galaxy::Home
            && crate::input::pressed(Key::Key3)
        {
            place_ship_near_race_planet(&mut ship_controller, world_time);
        }

        let keyboard_pitch = (window.is_key_down(Key::W) || window.is_key_down(Key::Up)) as i8
            - (window.is_key_down(Key::S) || window.is_key_down(Key::Down)) as i8;
        let keyboard_yaw = (window.is_key_down(Key::D) || window.is_key_down(Key::Right)) as i8
            - (window.is_key_down(Key::A) || window.is_key_down(Key::Left)) as i8;
        let keyboard_roll = window.is_key_down(Key::E) as i8 - window.is_key_down(Key::Q) as i8;
        let pitch_input = if keyboard_pitch != 0 {
            keyboard_pitch
        } else {
            gamepad.vertical()
        };
        let yaw_input = if keyboard_yaw != 0 {
            keyboard_yaw
        } else {
            gamepad.horizontal()
        };
        let roll_input = if keyboard_roll != 0 {
            keyboard_roll
        } else {
            gamepad.roll_right as i8 - gamepad.roll_left as i8
        };
        let active_yaw_input = if game_mode == GameMode::WaterLevel {
            -yaw_input
        } else {
            yaw_input
        };
        if coop_enabled
            && coop_game_mode == GameMode::FreeWorld
            && coop_controller.ship.planet_locked
            && gamepad_two.pressed(crate::input::Gamepad::A)
        {
            if coop_active_galaxy == Galaxy::Home
                && coop_controller.ship.race_planet_locked
                && collected_dragon_balls & 0b0000001 == 0
            {
                if let Some((_, car_spec)) = race_vehicles[coop_selected_ship] {
                    coop_controller = ShipController::new(car_spec);
                    if coop_race_track.enter(
                        &mut coop_controller,
                        coop_selected_ship,
                        &race_vehicles,
                    ) {
                        coop_game_mode = GameMode::Racing;
                    }
                }
            } else if coop_active_galaxy == Galaxy::Home
                && coop_controller.ship.spiky_planet_locked
                && collected_dragon_balls & 0b0000100 == 0
            {
                coop_guardian_fight = CoreGuardian::new();
                coop_game_mode = GameMode::CoreGuardian;
            } else if coop_active_galaxy == Galaxy::Home
                && !coop_controller.ship.race_planet_locked
                && collected_dragon_balls & 0b0001000 == 0
            {
                let coop_level_spec = match coop_selected_ship {
                    0 => &ARWING_SUBMARINE_SPEC,
                    1 => &KIRBY_SUBMARINE_SPEC,
                    2 => &ASTRA_SUBMARINE_SPEC,
                    3 => &COWBOYB_SUBMARINE_SPEC,
                    4 => &BUZZ_SUBMARINE_SPEC,
                    5 => &MCQUEEN_SUBMARINE_SPEC,
                    _ => unreachable!(),
                };
                coop_controller = ShipController::new(coop_level_spec);
                coop_water_level_state = WaterLevelState::new();
                coop_bubble_emitter.clear();
                coop_game_mode = GameMode::WaterLevel;
            } else if coop_active_galaxy == Galaxy::Vesper && coop_vesper_orbit_index == Some(0) {
                coop_hourglass_rescue = hourglass_rescue::HourglassRescue::new();
                coop_hourglass_rescue.place_ship_at_start(&mut coop_controller.ship);
                coop_game_mode = GameMode::HourglassRescue;
            } else if coop_active_galaxy == Galaxy::Vesper
                && coop_vesper_orbit_index == Some(1)
                && collected_dragon_balls & 0b0000010 == 0
            {
                coop_tower_game = tower_guard::TowerGame::new(&tower_sprites);
                coop_game_mode = GameMode::TowerGuard;
            } else if coop_active_galaxy == Galaxy::Nova {
                coop_lava_castle.enter(&mut coop_controller);
                coop_game_mode = GameMode::LavaCastle;
            }
        }
        let mut entered_level = false;
        if game_mode == GameMode::FreeWorld
            && ship_controller.ship.planet_locked
            && (crate::input::pressed(Key::Enter) || gamepad.pressed(crate::input::Gamepad::A))
        {
            if ship_controller.ship.race_planet_locked && collected_dragon_balls & 0b0000001 == 0 {
                if let Some((_, car_spec)) = race_vehicles[selected_ship] {
                    ship_controller = ShipController::new(car_spec);
                    if race_track.enter(&mut ship_controller, selected_ship, &race_vehicles) {
                        game_mode = GameMode::Racing;
                        if dynamic_resolution.prefer_racing_resolution() {
                            let (render_width, render_height) = dynamic_resolution.dimensions();
                            scene_framebuffer = Framebuffer::new(render_width, render_height);
                            triangle_batch.resize(render_width, render_height);
                        }
                        entered_level = true;
                        window.set_title(
                            "Gran Premio Orbital | W acelerar | A/D girar | Backspace: espacio",
                        );
                    }
                } else {
                    let missing_model = match selected_ship {
                        0 => "assets/vehicles/arwing/arwing2_carro.obj",
                        1 => "assets/vehicles/kirby/kirby_carro.obj",
                        _ => "variante carro no disponible",
                    };
                    window.set_title("Carrera: esta nave aún no tiene modelo de carro");
                    println!("Falta un OBJ de carro para esta nave: {missing_model}");
                }
            } else if active_galaxy == Galaxy::Vesper && vesper_orbit_index == Some(0) {
                hourglass_rescue = hourglass_rescue::HourglassRescue::new();
                hourglass_rescue.place_ship_at_start(&mut ship_controller.ship);
                game_mode = GameMode::HourglassRescue;
                entered_level = true;
                window.set_title("Planeta del Reloj | controles de vuelo libre | Enter dispara");
            } else if active_galaxy == Galaxy::Vesper && vesper_orbit_index == Some(1) {
                window.set_cursor_visibility(true);
                let resident_directory = std::env::current_dir()?.join("resident_level1");
                let resident_executable =
                    resident_directory.join("target/release/pacman_polygon.exe");
                if !resident_executable.is_file() {
                    let build = std::process::Command::new("cargo")
                        .args([
                            "build",
                            "--release",
                            "--manifest-path",
                            "resident_level1/Cargo.toml",
                        ])
                        .status();
                    if !build.is_ok_and(|status| status.success()) {
                        eprintln!("No se pudo compilar Resident Evil nivel 1");
                    }
                }
                let resident_result = std::process::Command::new(&resident_executable)
                    .current_dir(&resident_directory)
                    .status();
                match resident_result {
                    Ok(status) if status.code() == Some(10) => {
                        tower_dragon_ball_collected = true;
                        tower_reward_timer = 5.5;
                    }
                    Ok(_) => {}
                    Err(error) => {
                        eprintln!("No se pudo iniciar Resident Evil nivel 1: {error}");
                    }
                }
                game_mode = GameMode::FreeWorld;
                launch_ship_after_guardian_exit(&mut ship_controller, world_time);
                if tower_dragon_ball_collected {
                    tower_reward_position = ship_controller.ship.position;
                }
                entered_level = true;
                window.set_title(if tower_dragon_ball_collected {
                    "Galaxia Vesper | Esfera de 2 estrellas obtenida | Vuelo libre"
                } else {
                    "Galaxia Vesper | Resident Evil | Vuelo libre"
                });
            } else if active_galaxy == Galaxy::Vesper {
                // Vesper's other bodies have no minigame; keep the player in
                // orbit instead of accidentally launching the water level.
            } else if active_galaxy == Galaxy::Nova {
                lava_castle.enter(&mut ship_controller);
                game_mode = GameMode::LavaCastle;
                entered_level = true;
                window.set_title("Castillo de Lava | Guardian de Lava | Enter: disparar");
            } else if ship_controller.ship.spiky_planet_locked {
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
        } else if game_mode == GameMode::Racing && crate::input::pressed(Key::Backspace) {
            game_mode = GameMode::FreeWorld;
            ship_controller = ShipController::new(ship_spec);
            race_track.leave(&mut ship_controller, race_planet_position(world_time));
            window.set_title("Sistema solar | Vuelo libre 3D | F11: fullscreen");
        } else if game_mode == GameMode::WaterLevel && crate::input::pressed(Key::Backspace) {
            game_mode = GameMode::FreeWorld;
            bubble_emitter.clear();
            ship_controller = ShipController::new(ship_spec);
            launch_ship_from_planet(
                &mut ship_controller,
                water_planet_position(world_time),
                WATER_PLANET_RADIUS,
            );
            window.set_title("Sistema solar | Vuelo libre 3D | F11: fullscreen");
            println!("Regreso al sistema solar");
        }

        let input = ShipInput {
            pitch: pitch_input,
            yaw: active_yaw_input,
            roll: roll_input,
            accelerating: window.is_key_down(Key::Space) || gamepad.accelerate,
            braking: window.is_key_down(Key::LeftCtrl)
                || window.is_key_down(Key::RightCtrl)
                || gamepad.brake,
            firing: (window.is_key_down(Key::Enter) || gamepad.fire) && !entered_level,
        };
        let coop_input = ShipInput {
            pitch: (window.is_key_down(Key::K) as i8 - window.is_key_down(Key::I) as i8)
                + gamepad_two.vertical(),
            yaw: (window.is_key_down(Key::L) as i8 - window.is_key_down(Key::J) as i8)
                + gamepad_two.horizontal(),
            roll: (window.is_key_down(Key::O) as i8 - window.is_key_down(Key::U) as i8)
                + gamepad_two.roll_right as i8
                - gamepad_two.roll_left as i8,
            accelerating: window.is_key_down(Key::N) || gamepad_two.accelerate,
            braking: window.is_key_down(Key::M) || gamepad_two.brake,
            firing: window.is_key_down(Key::H) || gamepad_two.fire,
        };
        if coop_enabled && coop_game_mode == GameMode::Racing {
            if gamepad_two.pressed(crate::input::Gamepad::B) || coop_race_track.reward_complete() {
                if coop_race_track.reward_complete() {
                    collected_dragon_balls |= 0b0000001;
                }
                coop_game_mode = GameMode::FreeWorld;
                coop_controller = ShipController::new(ship_spec);
                coop_race_track.leave(&mut coop_controller, race_planet_position(world_time));
            } else {
                coop_race_track.update(
                    &mut coop_controller,
                    race_track::RaceInput {
                        accelerate: gamepad_two.accelerate,
                        reverse: gamepad_two.brake,
                        brake: gamepad_two.brake,
                        steering: (-gamepad_two.move_x * i8::MAX as f32)
                            .round()
                            .clamp(i8::MIN as f32, i8::MAX as f32)
                            as i8,
                        drift: gamepad_two.down(crate::input::Gamepad::X),
                        boost: gamepad_two.boost,
                        pause: gamepad_two.pressed(crate::input::Gamepad::START),
                        confirm: gamepad_two.pressed(crate::input::Gamepad::A),
                    },
                    delta_time,
                );
            }
        } else if coop_enabled && coop_game_mode == GameMode::TowerGuard {
            let action = coop_tower_game.update(
                delta_time,
                tower_guard::InputFrame {
                    cursor: (0.5, 0.5),
                    cursor_moved: false,
                    fire: gamepad_two.fire,
                    fire_held: gamepad_two.fire,
                    mouse_clicked: false,
                    confirm: gamepad_two.pressed(crate::input::Gamepad::A),
                    exit: gamepad_two.pressed(crate::input::Gamepad::B),
                    reload: gamepad_two.pressed(crate::input::Gamepad::Y),
                    arrow_x: gamepad_two.horizontal(),
                    arrow_y: -gamepad_two.vertical(),
                    move_x: gamepad_two.horizontal(),
                    move_y: gamepad_two.vertical(),
                    interact: gamepad_two.pressed(crate::input::Gamepad::A),
                    look_x: gamepad_two.motion_x * 0.035,
                    look_y: -gamepad_two.motion_y * 0.035,
                    aiming: gamepad_two.brake,
                    weapon: 0,
                },
                coop_framebuffer.width,
                coop_framebuffer.height,
                &tower_sprites,
            );
            if matches!(
                action,
                tower_guard::TowerAction::Exit | tower_guard::TowerAction::Restart
            ) {
                if action == tower_guard::TowerAction::Restart {
                    coop_tower_game = tower_guard::TowerGame::new(&tower_sprites);
                } else {
                    coop_game_mode = GameMode::FreeWorld;
                    launch_ship_from_planet(
                        &mut coop_controller,
                        vesper_planet_position(1, world_time),
                        VESPER_ORBIT_RADII[1],
                    );
                }
            }
        } else if coop_enabled && coop_game_mode == GameMode::WaterLevel {
            if gamepad_two.pressed(crate::input::Gamepad::B) {
                coop_game_mode = GameMode::FreeWorld;
                coop_bubble_emitter.clear();
                coop_controller = ShipController::new(choices[coop_selected_ship].1);
                launch_ship_from_planet(
                    &mut coop_controller,
                    water_planet_position(world_time),
                    WATER_PLANET_RADIUS,
                );
            } else {
                coop_controller.update(&coop_input, delta_time);
                coop_water_level_state.update(&mut coop_controller.ship, &coop_input, delta_time);
                resolve_level_collisions(
                    &mut coop_water_level_state,
                    &mut coop_controller.ship,
                    &water_level_spatial,
                );
                resolve_water_enemy_collisions(
                    &mut coop_water_level_state,
                    &mut coop_controller.ship,
                    &mut coop_controller.projectiles,
                );
                coop_bubble_emitter.update(&coop_controller.ship, delta_time);
                if coop_water_level_state.dragon_ball_collected {
                    collected_dragon_balls |= 0b0001000;
                }
            }
        } else if coop_enabled && coop_game_mode == GameMode::CoreGuardian {
            let action = coop_guardian_fight.update(
                delta_time,
                GuardianInput {
                    horizontal: gamepad_two.move_x,
                    vertical: -gamepad_two.move_y,
                    fire: gamepad_two.fire,
                    confirm: gamepad_two.pressed(crate::input::Gamepad::A),
                    exit: gamepad_two.pressed(crate::input::Gamepad::B),
                },
                coop_framebuffer.width,
                coop_framebuffer.height,
            );
            if action == GuardianAction::Exit {
                coop_game_mode = GameMode::FreeWorld;
                launch_ship_after_guardian_exit(&mut coop_controller, world_time);
            }
        } else if coop_enabled && coop_game_mode == GameMode::HourglassRescue {
            let rescue_input = hourglass_rescue::Input {
                forward: gamepad_two.accelerate,
                brake: gamepad_two.brake,
                left: gamepad_two.horizontal() < 0,
                right: gamepad_two.horizontal() > 0,
                rise: gamepad_two.vertical() > 0,
                descend: gamepad_two.vertical() < 0,
                roll_left: gamepad_two.roll_left,
                roll_right: gamepad_two.roll_right,
                boost: gamepad_two.boost,
                firing: gamepad_two.fire,
                pause: gamepad_two.pressed(crate::input::Gamepad::START),
                exit: gamepad_two.pressed(crate::input::Gamepad::B),
                restart: gamepad_two.pressed(crate::input::Gamepad::Y),
                recover: gamepad_two.pressed(crate::input::Gamepad::Y),
            };
            if coop_hourglass_rescue.update(&mut coop_controller.ship, rescue_input, delta_time)
                == hourglass_rescue::Action::Exit
            {
                if coop_hourglass_rescue.mission_won() {
                    collected_dragon_balls |= 0b0010000;
                }
                coop_game_mode = GameMode::FreeWorld;
                launch_ship_from_planet(
                    &mut coop_controller,
                    vesper_planet_position(0, world_time),
                    VESPER_ORBIT_RADII[0],
                );
            } else {
                coop_controller.update_weapon_and_projectiles(rescue_input.firing, delta_time);
                coop_hourglass_rescue.update_enemies(
                    &mut coop_controller.ship,
                    &mut coop_controller.projectiles,
                    delta_time,
                );
            }
        } else if coop_enabled && coop_game_mode == GameMode::LavaCastle {
            let action = coop_lava_castle.update(
                &mut coop_controller,
                lava_castle::Input {
                    pitch: coop_input.pitch,
                    yaw: coop_input.yaw,
                    roll: coop_input.roll,
                    accelerate: coop_input.accelerating,
                    brake: coop_input.braking,
                    boost: gamepad_two.boost,
                    fire: coop_input.firing,
                    restart: gamepad_two.pressed(crate::input::Gamepad::Y),
                    exit: gamepad_two.pressed(crate::input::Gamepad::B),
                },
                delta_time,
            );
            match action {
                lava_castle::Action::Continue => {}
                lava_castle::Action::InteriorTransition => {
                    collected_dragon_balls |= 0b0100000;
                    coop_game_mode = GameMode::LavaCastleInterior;
                    coop_lava_castle_interior.enter(&mut coop_controller);
                }
                lava_castle::Action::Exit => {
                    coop_game_mode = GameMode::FreeWorld;
                    launch_ship_from_planet(
                        &mut coop_controller,
                        castle_planet.center,
                        castle_planet.radius,
                    );
                }
            }
        } else if coop_enabled && coop_game_mode == GameMode::LavaCastleInterior {
            let action = coop_lava_castle_interior.update(
                &mut coop_controller,
                &coop_input,
                gamepad_two.pressed(crate::input::Gamepad::Y),
                gamepad_two.pressed(crate::input::Gamepad::B),
                delta_time,
            );
            if matches!(
                action,
                lava_castle_interior::Action::Exit | lava_castle_interior::Action::Victory
            ) {
                if action == lava_castle_interior::Action::Victory {
                    collected_dragon_balls |= 0b1000000;
                }
                coop_game_mode = GameMode::FreeWorld;
                launch_ship_from_planet(
                    &mut coop_controller,
                    castle_planet.center,
                    castle_planet.radius,
                );
            }
        }

        if let Some(run) = benchmark.as_ref() {
            match run.scene.as_str() {
                "free" if game_mode != GameMode::FreeWorld => game_mode = GameMode::FreeWorld,
                "race" if game_mode != GameMode::Racing => {
                    ship_controller = ShipController::new(&ASTRA_CAR_SPEC);
                    race_track.enter(&mut ship_controller, selected_ship, &race_vehicles);
                    game_mode = GameMode::Racing;
                }
                "water" if game_mode != GameMode::WaterLevel => {
                    ship_controller = ShipController::new(level_ship_spec);
                    game_mode = GameMode::WaterLevel;
                }
                "guardian" if game_mode != GameMode::CoreGuardian => {
                    game_mode = GameMode::CoreGuardian
                }
                "tower" if game_mode != GameMode::TowerGuard => game_mode = GameMode::TowerGuard,
                _ => {}
            }
            if run.scene == "free" && !ship_controller.ship.planet_locked {
                place_ship_near_race_planet(&mut ship_controller, world_time);
            }
        }
        if guardian_fight.reward_received() || coop_guardian_fight.reward_received() {
            collected_dragon_balls |= 0b0000100;
        }
        if tower_dragon_ball_collected {
            collected_dragon_balls |= 0b0000010;
        }

        let update_scene_start = Instant::now();
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
        let coop_active_model = match coop_selected_ship {
            0 => {
                if coop_game_mode == GameMode::WaterLevel {
                    &arwing_submarine_model
                } else {
                    choices[0].0
                }
            }
            1 => {
                if coop_game_mode == GameMode::WaterLevel {
                    &kirby_submarine_model
                } else {
                    choices[1].0
                }
            }
            2 => {
                if coop_game_mode == GameMode::WaterLevel {
                    &astra_submarine_model
                } else {
                    choices[2].0
                }
            }
            3 => {
                if coop_game_mode == GameMode::WaterLevel {
                    &cowboyb_submarine_model
                } else {
                    choices[3].0
                }
            }
            4 => {
                &buzz_variants[if coop_game_mode == GameMode::WaterLevel {
                    2
                } else {
                    0
                }]
            }
            5 => {
                if coop_game_mode == GameMode::WaterLevel {
                    &mcqueen_submarine_model
                } else {
                    &mcqueen_model
                }
            }
            _ => unreachable!(),
        };
        let coop_ship_center = model_center(coop_active_model);
        triangle_batch.begin_frame();
        match game_mode {
            GameMode::FreeWorld => {
                let portal_approach_active = galaxy_transition
                    .as_ref()
                    .is_some_and(|transit| !transit.switched);
                let tower_reward_active = tower_reward_timer > 0.0;
                let solar_explosion_active = solar_explosion_timer > 0.0;
                let black_hole_absorption_active = black_hole_absorption_timer > 0.0;
                if !portal_approach_active
                    && !tower_reward_active
                    && !solar_explosion_active
                    && !black_hole_absorption_active
                {
                    ship_controller.update(&input, delta_time);
                }
                // P2's free-flight simulation is deliberately outside P1's
                // transition lockout: each viewport owns its travel state.
                if coop_enabled && coop_game_mode == GameMode::FreeWorld {
                    coop_controller.update(&coop_input, delta_time);
                    coop_controller.rebase_far_flight();
                }
                let light_speed =
                    window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift);
                let active_speed = if tower_reward_active
                    || solar_explosion_active
                    || black_hole_absorption_active
                {
                    0.0
                } else if light_speed {
                    LIGHT_SPEED
                } else {
                    ship_controller.ship.speed
                };
                let turbo_power = ship_controller.turbo_power(active_speed);
                let movement = if !portal_approach_active {
                    active_speed * delta_time
                } else {
                    0.0
                };
                // Avoid f32 precision loss after prolonged travel. Do this
                // before capturing the segment used for portal collision.
                ship_controller.rebase_far_flight();
                let ship = &mut ship_controller.ship;
                let previous_ship_position = ship.position;

                if (crate::input::pressed(Key::Backspace)
                    || gamepad.pressed(crate::input::Gamepad::B))
                    && ship.planet_locked
                {
                    ship.planet_locked = false;
                    ship.spiky_planet_locked = false;
                    ship.race_planet_locked = false;
                    ship.orbit_release_active = true;
                    ship.orbit_plane_normal = None;
                    vesper_orbit_index = None;
                    println!("Orbita liberada");
                }

                if black_hole_absorption_active {
                    let progress =
                        1.0 - black_hole_absorption_timer / VESPER_BLACK_HOLE_ABSORPTION_DURATION;
                    ship.position =
                        vesper_absorption_position(black_hole_absorption_start, progress);
                    (ship.yaw, ship.pitch) =
                        direction_to_angles(VESPER_SYSTEM_CENTER.sub(ship.position));
                    ship.speed = 0.0;
                    ship.planet_locked = false;
                    ship.orbit_center = None;
                    ship.orbit_plane_normal = None;
                    vesper_orbit_index = None;
                } else if !portal_approach_active {
                    if active_galaxy == Galaxy::Home {
                        let empty_colliders: [ObjectCollider; 0] = [];
                        let water_center = water_planet_position(world_time);
                        let spiky_center = spiky_planet_position(world_time);
                        let race_center = race_planet_position(world_time);
                        if !ship.planet_locked {
                            ship.race_planet_locked = race_planet_is_closest(
                                ship.position,
                                water_center,
                                spiky_center,
                                race_center,
                            );
                            ship.spiky_planet_locked = !ship.race_planet_locked
                                && spiky_planet_is_closer(
                                    ship.position,
                                    water_center,
                                    spiky_center,
                                );
                        }
                        let (orbit_center, orbit_radius, orbit_rotation, orbit_colliders) =
                            if ship.race_planet_locked {
                                (
                                    race_center,
                                    race_track::PLANET_RADIUS,
                                    world_time * RACE_PLANET_ROTATION_SPEED,
                                    empty_colliders.as_slice(),
                                )
                            } else if ship.spiky_planet_locked {
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
                                    world_time * WATER_PLANET_ROTATION_SPEED,
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
                            false,
                        );
                    } else if active_galaxy == Galaxy::Vesper {
                        ship.spiky_planet_locked = false;
                        ship.race_planet_locked = false;
                        let nearest = vesper_orbit_index.unwrap_or_else(|| {
                            vesper_planets
                                .iter()
                                .enumerate()
                                .min_by(|(a_index, _), (b_index, _)| {
                                    let a_offset = ship
                                        .position
                                        .sub(vesper_planet_position(*a_index, world_time));
                                    let b_offset = ship
                                        .position
                                        .sub(vesper_planet_position(*b_index, world_time));
                                    a_offset.dot(a_offset).total_cmp(&b_offset.dot(b_offset))
                                })
                                .map(|(index, _)| index)
                                .unwrap_or(0)
                        });
                        let body = &vesper_planets[nearest];
                        let orbit_center = vesper_planet_position(nearest, world_time);
                        let orbit_radius = VESPER_ORBIT_RADII[nearest];
                        move_around_planet(
                            ship,
                            movement,
                            orbit_center,
                            orbit_radius,
                            body.rotation_y + world_time * 0.035,
                            &[],
                            yaw_input,
                            pitch_input,
                            true,
                        );
                        vesper_orbit_index = ship.planet_locked.then_some(nearest);
                        if !ship.planet_locked {
                            ship.position =
                                apply_vesper_black_hole_gravity(ship.position, delta_time);
                            let black_hole_offset = ship.position.sub(VESPER_SYSTEM_CENTER);
                            if black_hole_offset.dot(black_hole_offset).sqrt()
                                <= VESPER_BLACK_HOLE_EVENT_HORIZON
                            {
                                black_hole_absorption_timer = VESPER_BLACK_HOLE_ABSORPTION_DURATION;
                                black_hole_absorption_start = ship.position;
                                ship.speed = 0.0;
                                window.set_title("Agujero negro | La nave esta siendo absorbida");
                            }
                        }
                    } else if active_galaxy == Galaxy::Nova {
                        ship.spiky_planet_locked = false;
                        ship.race_planet_locked = false;
                        move_around_planet(
                            ship,
                            movement,
                            castle_planet.center,
                            NOVA_CASTLE_ORBIT_RADIUS,
                            castle_planet.rotation_y + world_time * 0.018,
                            &[],
                            yaw_input,
                            pitch_input,
                            true,
                        );
                        vesper_orbit_index = None;
                    } else {
                        unreachable!("all galaxies are handled above");
                    }
                }
                if active_galaxy == Galaxy::Home
                    && solar_explosion_timer <= 0.0
                    && ship_touches_sun(
                        previous_ship_position,
                        ship.position,
                        ship.spec.collision_radius,
                        sun_collision_radius,
                    )
                {
                    solar_explosion_timer = SOLAR_SHIP_EXPLOSION_DURATION;
                    solar_explosion_position = ship.position;
                    ship.speed = 0.0;
                    ship.planet_locked = false;
                    ship.spiky_planet_locked = false;
                    ship.race_planet_locked = false;
                    ship.orbit_center = None;
                    ship.orbit_plane_normal = None;
                    ship.orbit_camera_blend = 0.0;
                    window.set_title("La nave fue destruida por el Sol");
                }
                if !portal_approach_active && !black_hole_absorption_active && !ship.planet_locked {
                    let asteroid_field = match active_galaxy {
                        Galaxy::Home => Some((Vec3::new(SUN_POSITION_X, 20.0, 0.0), 350.0)),
                        Galaxy::Vesper => Some((VESPER_SYSTEM_CENTER, 350.0)),
                        Galaxy::Nova => Some((Vec3::new(0.0, 70.0, 0.0), 300.0)),
                    };
                    if let Some((center, radius)) = asteroid_field {
                        if let Some(safe_position) = space_decor::asteroid_collision_position(
                            previous_ship_position,
                            ship.position,
                            ship.spec.collision_radius,
                            active_galaxy,
                            center,
                            radius,
                            world_time,
                        ) {
                            ship.position = safe_position;
                            ship.speed = ship.speed.min(ship.spec.cruise_speed * 0.38);
                        }
                    }
                }
                let camera_target = if ship.orbit_plane_normal.is_some() {
                    1.0
                } else {
                    0.0
                };
                let camera_response = 1.0 - (-2.4 * delta_time).exp();
                ship.orbit_camera_blend +=
                    (camera_target - ship.orbit_camera_blend) * camera_response;
                if ship.orbit_plane_normal.is_none() && ship.orbit_camera_blend < 0.001 {
                    ship.orbit_center = None;
                    ship.orbit_camera_blend = 0.0;
                }
                distance += movement;

                if coop_enabled && coop_game_mode == GameMode::FreeWorld {
                    let coop_ship = &mut coop_controller.ship;
                    let coop_previous = coop_ship.position;
                    let coop_movement = coop_ship.speed * delta_time;
                    if window.is_key_down(Key::G)
                        || gamepad_two.pressed(crate::input::Gamepad::SELECT)
                        || gamepad_two.pressed(crate::input::Gamepad::B)
                    {
                        coop_ship.planet_locked = false;
                        coop_ship.spiky_planet_locked = false;
                        coop_ship.race_planet_locked = false;
                        coop_ship.orbit_release_active = true;
                        coop_ship.orbit_plane_normal = None;
                        coop_vesper_orbit_index = None;
                    }

                    // P2 owns this portal transition.  It is intentionally
                    // immediate so P1's cinematic transition remains local
                    // to the left viewport.
                    let coop_portal = wormhole_position(coop_active_galaxy);
                    let coop_offset = coop_ship.position.sub(coop_portal);
                    let coop_near_nova = coop_active_galaxy == Galaxy::Vesper
                        && coop_ship
                            .position
                            .sub(nova_portal_position())
                            .dot(coop_ship.position.sub(nova_portal_position()))
                            <= 42.0 * 42.0;
                    if !coop_ship.planet_locked
                        && (coop_offset.dot(coop_offset) <= 42.0 * 42.0 || coop_near_nova)
                    {
                        coop_active_galaxy = if coop_near_nova {
                            Galaxy::Nova
                        } else {
                            coop_active_galaxy.portal_destination()
                        };
                        let exit = match coop_active_galaxy {
                            Galaxy::Vesper => Vec3::new(-420.0, 0.0, 0.0),
                            Galaxy::Nova => Vec3::new(0.0, 20.0, -150.0),
                            Galaxy::Home => Vec3::new(150.0, 0.0, 80.0),
                        };
                        coop_ship.position = wormhole_position(coop_active_galaxy).add(exit);
                        coop_ship.speed = coop_ship.spec.cruise_speed;
                        coop_vesper_orbit_index = None;
                    }

                    match coop_active_galaxy {
                        Galaxy::Home => {
                            let empty_colliders: [ObjectCollider; 0] = [];
                            let water_center = water_planet_position(world_time);
                            let spiky_center = spiky_planet_position(world_time);
                            let race_center = race_planet_position(world_time);
                            if !coop_ship.planet_locked {
                                coop_ship.race_planet_locked = race_planet_is_closest(
                                    coop_ship.position,
                                    water_center,
                                    spiky_center,
                                    race_center,
                                );
                                coop_ship.spiky_planet_locked = !coop_ship.race_planet_locked
                                    && spiky_planet_is_closer(
                                        coop_ship.position,
                                        water_center,
                                        spiky_center,
                                    );
                            }
                            let (center, radius, rotation, colliders) =
                                if coop_ship.race_planet_locked {
                                    (
                                        race_center,
                                        race_track::PLANET_RADIUS,
                                        world_time * RACE_PLANET_ROTATION_SPEED,
                                        empty_colliders.as_slice(),
                                    )
                                } else if coop_ship.spiky_planet_locked {
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
                                        world_time * WATER_PLANET_ROTATION_SPEED,
                                        object_colliders.as_slice(),
                                    )
                                };
                            move_around_planet(
                                coop_ship,
                                coop_movement,
                                center,
                                radius,
                                rotation,
                                colliders,
                                coop_input.yaw,
                                coop_input.pitch,
                                false,
                            );
                            if ship_touches_sun(
                                coop_previous,
                                coop_ship.position,
                                coop_ship.spec.collision_radius,
                                sun_collision_radius,
                            ) {
                                coop_ship.position = coop_previous;
                                coop_ship.speed = 0.0;
                                coop_ship.planet_locked = false;
                                coop_ship.orbit_center = None;
                                coop_ship.orbit_plane_normal = None;
                            }
                        }
                        Galaxy::Vesper => {
                            coop_ship.spiky_planet_locked = false;
                            coop_ship.race_planet_locked = false;
                            let nearest = coop_vesper_orbit_index.unwrap_or_else(|| {
                                vesper_planets
                                    .iter()
                                    .enumerate()
                                    .min_by(|(a, _), (b, _)| {
                                        let da = coop_ship
                                            .position
                                            .sub(vesper_planet_position(*a, world_time));
                                        let db = coop_ship
                                            .position
                                            .sub(vesper_planet_position(*b, world_time));
                                        da.dot(da).total_cmp(&db.dot(db))
                                    })
                                    .map(|(index, _)| index)
                                    .unwrap_or(0)
                            });
                            let body = &vesper_planets[nearest];
                            move_around_planet(
                                coop_ship,
                                coop_movement,
                                vesper_planet_position(nearest, world_time),
                                VESPER_ORBIT_RADII[nearest],
                                body.rotation_y + world_time * 0.035,
                                &[],
                                coop_input.yaw,
                                coop_input.pitch,
                                true,
                            );
                            coop_vesper_orbit_index = coop_ship.planet_locked.then_some(nearest);
                            if !coop_ship.planet_locked {
                                coop_ship.position =
                                    apply_vesper_black_hole_gravity(coop_ship.position, delta_time);
                                let offset = coop_ship.position.sub(VESPER_SYSTEM_CENTER);
                                if offset.dot(offset).sqrt() <= VESPER_BLACK_HOLE_EVENT_HORIZON {
                                    coop_ship.position = coop_previous;
                                    coop_ship.speed = 0.0;
                                }
                            }
                        }
                        Galaxy::Nova => {
                            coop_ship.spiky_planet_locked = false;
                            coop_ship.race_planet_locked = false;
                            move_around_planet(
                                coop_ship,
                                coop_movement,
                                castle_planet.center,
                                NOVA_CASTLE_ORBIT_RADIUS,
                                castle_planet.rotation_y + world_time * 0.018,
                                &[],
                                coop_input.yaw,
                                coop_input.pitch,
                                true,
                            );
                            coop_vesper_orbit_index = None;
                        }
                    }

                    if !coop_ship.planet_locked {
                        let asteroid_field = match coop_active_galaxy {
                            Galaxy::Home => Some((Vec3::new(SUN_POSITION_X, 20.0, 0.0), 350.0)),
                            Galaxy::Vesper => Some((VESPER_SYSTEM_CENTER, 350.0)),
                            Galaxy::Nova => Some((Vec3::new(0.0, 70.0, 0.0), 300.0)),
                        };
                        if let Some((center, radius)) = asteroid_field {
                            if let Some(safe) = space_decor::asteroid_collision_position(
                                coop_previous,
                                coop_ship.position,
                                coop_ship.spec.collision_radius,
                                coop_active_galaxy,
                                center,
                                radius,
                                world_time,
                            ) {
                                coop_ship.position = safe;
                                coop_ship.speed =
                                    coop_ship.speed.min(coop_ship.spec.cruise_speed * 0.38);
                            }
                        }
                    }
                    let camera_target = if coop_ship.orbit_plane_normal.is_some() {
                        1.0
                    } else {
                        0.0
                    };
                    let response = 1.0 - (-2.4 * delta_time).exp();
                    coop_ship.orbit_camera_blend +=
                        (camera_target - coop_ship.orbit_camera_blend) * response;
                    if coop_ship.orbit_plane_normal.is_none()
                        && coop_ship.orbit_camera_blend < 0.001
                    {
                        coop_ship.orbit_center = None;
                        coop_ship.orbit_camera_blend = 0.0;
                    }

                    // Keep both physical ships from occupying the same space.
                    let separation = coop_ship.position.sub(ship.position);
                    let minimum = coop_ship.spec.collision_radius + ship.spec.collision_radius;
                    let distance_squared = separation.dot(separation);
                    if distance_squared > 0.0001 && distance_squared < minimum * minimum {
                        let correction = separation
                            .normalize()
                            .mul((minimum - distance_squared.sqrt()) * 0.5);
                        coop_ship.position = coop_ship.position.add(correction);
                        ship.position = ship.position.sub(correction);
                        coop_ship.speed = coop_ship.speed.min(coop_ship.spec.cruise_speed);
                        ship.speed = ship.speed.min(ship.spec.cruise_speed);
                    }
                }

                let portal_center = wormhole_position(active_galaxy);
                let near_wormhole =
                    crossed_wormhole(previous_ship_position, ship.position, portal_center, 42.0);
                let near_nova_portal = active_galaxy == Galaxy::Vesper
                    && crossed_wormhole(
                        previous_ship_position,
                        ship.position,
                        nova_portal_position(),
                        42.0,
                    );
                if galaxy_transition.is_none()
                    && (near_wormhole || near_nova_portal)
                    && !ship.planet_locked
                    && black_hole_absorption_timer <= 0.0
                {
                    let (portal_position, destination) = if near_nova_portal {
                        (nova_portal_position(), Galaxy::Nova)
                    } else {
                        (portal_center, active_galaxy.portal_destination())
                    };
                    galaxy_transition = Some(GalaxyTransit {
                        elapsed: 0.0,
                        start_position: ship.position,
                        portal_position,
                        destination,
                        switched: false,
                    });
                    window.set_title("Agujero de gusano | Entrando en hipervelocidad");
                }

                if black_hole_absorption_timer > 0.0 {
                    window.set_title("Agujero negro | La nave esta siendo absorbida");
                } else if galaxy_transition.is_some() {
                    window.set_title("Atravesando el agujero | Hipervelocidad");
                } else if near_wormhole || near_nova_portal {
                    window.set_title("Agujero de gusano | Atravesalo para viajar");
                } else if ship.planet_locked {
                    if ship.race_planet_locked {
                        window.set_title(
                            "Jardines de Oxido | Enter: iniciar carrera | Backspace: salir de orbita",
                        );
                    } else if ship.spiky_planet_locked {
                        window.set_title(
                            "Nova Rosa | Enter: Guardián del Núcleo | Backspace: salir",
                        );
                    } else if active_galaxy == Galaxy::Vesper {
                        let planet_name = match vesper_orbit_index.unwrap_or(0) {
                            0 => "Reloj Arena",
                            _ => "Casa Torre",
                        };
                        if vesper_orbit_index == Some(1) {
                            window.set_title(
                                "Casa Torre | Enter: jugar Resident Evil nivel 1 | Backspace: salir",
                            );
                        } else {
                            window.set_title(&format!(
                                "{planet_name} | En orbita | Backspace: salir"
                            ));
                        }
                    } else if active_galaxy == Galaxy::Nova {
                        window
                            .set_title("Planeta Castillo | En orbita | Backspace: salir de orbita");
                    } else {
                        window.set_title(
                            "Abismo Azul | Enter: iniciar nivel | Backspace: salir",
                        );
                    }
                } else {
                    window.set_title(match active_galaxy {
                        Galaxy::Home => "Sistema solar | Vuelo libre 3D | F11: fullscreen",
                        Galaxy::Vesper => "Galaxia Vesper | Portales: Sistema solar y Nova",
                        Galaxy::Nova => "Galaxia Nova | Planeta Castillo | Portal: Vesper",
                    });
                }
                if matches!(active_galaxy, Galaxy::Home | Galaxy::Vesper) {
                    let battle_center = match active_galaxy {
                        Galaxy::Home => Vec3::new(SUN_POSITION_X, 20.0, 0.0),
                        Galaxy::Vesper => VESPER_SYSTEM_CENTER,
                        Galaxy::Nova => unreachable!(),
                    };
                    ambient_battle.update(
                        delta_time,
                        world_time,
                        battle_center,
                        ship_controller.ship.position,
                        selected_ship,
                        &choices,
                    );
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
                    sun_center,
                    &home_orbit_rings,
                    &wormhole,
                    wormhole_position(active_galaxy),
                    active_galaxy,
                    &vesper_planets,
                    &vesper_black_hole,
                    &vesper_orbit_rings,
                    &castle_planet,
                    &race_track.planet,
                    &choices,
                    &ambient_battle,
                    [&planet_lods[0], &planet_lods[1], &planet_lods[2]],
                    &mut planet_lod_states,
                    &mut vesper_lod_states,
                    &mut castle_lod_state,
                    &mut scene_framebuffer,
                    &mut triangle_batch,
                    &ship_controller.ship,
                    world_time,
                    (distance
                        * if active_galaxy == Galaxy::Home {
                            0.0025
                        } else {
                            0.012
                        })
                    .rem_euclid(1.0),
                    turbo_power,
                    light_speed,
                    ship_controller.charge_ratio(),
                    solar_explosion_timer <= 0.0
                        && !(black_hole_absorption_timer > 0.0
                            && black_hole_absorption_timer
                                < VESPER_BLACK_HOLE_ABSORPTION_DURATION * 0.28),
                    (tower_reward_timer > 0.0).then_some((
                        &two_star_dragon_ball,
                        5.5 - tower_reward_timer,
                        tower_reward_position,
                    )),
                );
                if coop_enabled {
                    coop_triangle_batch.begin_frame();
                    let coop_turbo = coop_controller.turbo_power(coop_controller.ship.speed);
                    if coop_game_mode == GameMode::FreeWorld {
                        render_scene(
                            coop_active_model,
                            coop_ship_center,
                            &turbo_plume,
                            &laser_bolt,
                            &coop_controller.projectiles,
                            &planet,
                            planet_center,
                            planet_scale,
                            &spiky_planet,
                            spiky_planet_center,
                            spiky_planet_scale,
                            &sun,
                            sun_center,
                            &home_orbit_rings,
                            &wormhole,
                            wormhole_position(coop_active_galaxy),
                            coop_active_galaxy,
                            &vesper_planets,
                            &vesper_black_hole,
                            &vesper_orbit_rings,
                            &castle_planet,
                            &race_track.planet,
                            &choices,
                            &ambient_battle,
                            [&planet_lods[0], &planet_lods[1], &planet_lods[2]],
                            &mut coop_planet_lod_states,
                            &mut coop_vesper_lod_states,
                            &mut coop_castle_lod_state,
                            &mut coop_framebuffer,
                            &mut coop_triangle_batch,
                            &coop_controller.ship,
                            world_time,
                            0.0,
                            coop_turbo,
                            false,
                            coop_controller.charge_ratio(),
                            true,
                            None,
                        );
                    } else if coop_game_mode == GameMode::WaterLevel {
                        render_water_level(
                            &water_level_spatial,
                            &enemy_models,
                            &enemy_centers,
                            &coop_water_level_state,
                            &mut coop_enemy_instance_buffers,
                            &bubble_model,
                            &underwater_boost_ring,
                            &four_star_dragon_ball,
                            &reward_pedestal,
                            &reward_chamber,
                            &reward_door,
                            &coop_bubble_emitter.particles,
                            coop_active_model,
                            coop_ship_center,
                            &turbo_plume,
                            &laser_bolt,
                            &enemy_laser_bolt,
                            &coop_controller.projectiles,
                            &mut coop_framebuffer,
                            &mut coop_triangle_batch,
                            &coop_controller.ship,
                            world_time,
                            coop_turbo,
                            coop_controller.charge_ratio(),
                        );
                    } else if coop_game_mode == GameMode::CoreGuardian {
                        coop_guardian_fight.render_scene(
                            &mut coop_framebuffer,
                            coop_active_model,
                            coop_ship_center,
                            coop_controller.ship.spec,
                            &three_star_dragon_ball,
                        );
                    } else if coop_game_mode == GameMode::HourglassRescue {
                        coop_hourglass_rescue.render(
                            &mut coop_framebuffer,
                            coop_active_model,
                            &coop_controller.ship,
                        );
                        coop_hourglass_rescue.render_weapon(
                            &mut coop_framebuffer,
                            &laser_bolt,
                            &coop_controller.projectiles,
                            &coop_controller.ship,
                            coop_controller.charge_ratio(),
                        );
                    } else if coop_game_mode == GameMode::LavaCastle {
                        coop_lava_castle.render(
                            &mut coop_framebuffer,
                            coop_active_model,
                            &laser_bolt,
                            &six_star_dragon_ball,
                            &coop_controller,
                        );
                    } else if coop_game_mode == GameMode::LavaCastleInterior {
                        coop_lava_castle_interior.render(
                            &mut coop_framebuffer,
                            coop_active_model,
                            &laser_bolt,
                            &seven_star_dragon_ball,
                            &coop_controller,
                        );
                    } else if coop_game_mode == GameMode::Racing {
                        coop_race_track.render(
                            &mut coop_framebuffer,
                            &mut coop_triangle_batch,
                            &race_vehicles,
                            &race_sky,
                            &coop_controller,
                            &turbo_plume,
                            &laser_bolt,
                            &one_star_dragon_ball,
                            world_time,
                        );
                    } else if coop_game_mode == GameMode::TowerGuard {
                        coop_tower_game.render(&mut coop_framebuffer, &tower_sprites);
                    }
                }
                if solar_explosion_timer > 0.0 {
                    let camera = chase_camera(&ship_controller.ship, false);
                    draw_solar_ship_explosion(
                        &mut scene_framebuffer,
                        &camera,
                        solar_explosion_position,
                        solar_explosion_timer,
                    );
                }
                if let Some(transit) = &galaxy_transition {
                    draw_hyperspace_transition(&mut scene_framebuffer, transit.elapsed);
                }
            }
            GameMode::Racing => {
                let race_input = race_track::RaceInput {
                    accelerate: window.is_key_down(Key::W)
                        || window.is_key_down(Key::Up)
                        || gamepad.accelerate,
                    reverse: window.is_key_down(Key::S)
                        || window.is_key_down(Key::Down)
                        || gamepad.brake,
                    brake: gamepad.brake,
                    steering: -yaw_input,
                    drift: window.is_key_down(Key::Space) || gamepad.down(crate::input::Gamepad::X),
                    boost: window.is_key_down(Key::LeftShift)
                        || window.is_key_down(Key::RightShift)
                        || gamepad.boost,
                    pause: gamepad.pressed(crate::input::Gamepad::START),
                    confirm: crate::input::pressed(Key::Enter)
                        || gamepad.pressed(crate::input::Gamepad::A),
                };
                if race_track.reward_complete() {
                    collected_dragon_balls |= 0b0000001;
                    game_mode = GameMode::FreeWorld;
                    ship_controller = ShipController::new(ship_spec);
                    race_track.leave(&mut ship_controller, race_planet_position(world_time));
                    window.set_title("Sistema solar | Esfera de 1 estrella obtenida");
                } else {
                    race_track.update(&mut ship_controller, race_input, delta_time);
                    race_track.render(
                        &mut scene_framebuffer,
                        &mut triangle_batch,
                        &race_vehicles,
                        &race_sky,
                        &ship_controller,
                        &turbo_plume,
                        &laser_bolt,
                        &one_star_dragon_ball,
                        world_time,
                    );
                }
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
                    &water_level_state,
                    &mut enemy_instance_buffers,
                    &bubble_model,
                    &underwater_boost_ring,
                    &four_star_dragon_ball,
                    &reward_pedestal,
                    &reward_chamber,
                    &reward_door,
                    &bubble_emitter.particles,
                    level_ship_model,
                    level_ship_center,
                    &turbo_plume,
                    &laser_bolt,
                    &enemy_laser_bolt,
                    &ship_controller.projectiles,
                    &mut scene_framebuffer,
                    &mut triangle_batch,
                    &ship_controller.ship,
                    world_time,
                    turbo_power,
                    ship_controller.charge_ratio(),
                );
                if water_level_state.dragon_ball_collected
                    && water_level_state.mission_complete_timer <= 0.0
                {
                    collected_dragon_balls |= 0b0001000;
                    game_mode = GameMode::FreeWorld;
                    bubble_emitter.clear();
                    ship_controller = ShipController::new(ship_spec);
                    launch_ship_from_planet(
                        &mut ship_controller,
                        water_planet_position(world_time),
                        WATER_PLANET_RADIUS,
                    );
                    window.set_title("Sistema solar | Nivel completado | Vuelo libre");
                    println!("Nivel submarino completado");
                }
            }
            GameMode::CoreGuardian => {
                let input = GuardianInput {
                    horizontal: (window.is_key_down(Key::D) as i8 as f32
                        - window.is_key_down(Key::A) as i8 as f32)
                        + gamepad.move_x,
                    vertical: (window.is_key_down(Key::S) as i8 as f32
                        + window.is_key_down(Key::Down) as i8 as f32
                        - window.is_key_down(Key::W) as i8 as f32
                        - window.is_key_down(Key::Up) as i8 as f32)
                        - gamepad.move_y,
                    fire: window.is_key_down(Key::Space)
                        || window.is_key_down(Key::Enter)
                        || gamepad.fire,
                    confirm: crate::input::pressed(Key::Enter)
                        || gamepad.pressed(crate::input::Gamepad::A),
                    exit: crate::input::pressed(Key::Backspace)
                        || gamepad.pressed(crate::input::Gamepad::B),
                };
                let action = guardian_fight.update(
                    delta_time,
                    input,
                    scene_framebuffer.width,
                    scene_framebuffer.height,
                );
                if action == GuardianAction::Exit {
                    game_mode = GameMode::FreeWorld;
                    launch_ship_after_guardian_exit(&mut ship_controller, world_time);
                    window.set_title("Sistema solar | Vuelo libre 3D | F11: fullscreen");
                }
                guardian_fight.render_scene(
                    &mut scene_framebuffer,
                    active_model,
                    ship_center,
                    ship_controller.ship.spec,
                    &three_star_dragon_ball,
                );
            }
            GameMode::TowerGuard => {
                let (window_width, window_height) = window.get_size();
                let cursor = window
                    .get_mouse_pos(minifb::MouseMode::Clamp)
                    .map(|(x, y)| {
                        (
                            (x / window_width.max(1) as f32).clamp(0.0, 1.0),
                            (y / window_height.max(1) as f32).clamp(0.0, 1.0),
                        )
                    })
                    .unwrap_or(last_tower_cursor);
                let cursor_moved = (cursor.0 - last_tower_cursor.0).abs() > 0.0005
                    || (cursor.1 - last_tower_cursor.1).abs() > 0.0005;
                let mouse_look = (
                    cursor.0 - last_tower_cursor.0,
                    cursor.1 - last_tower_cursor.1,
                );
                last_tower_cursor = cursor;
                let mouse_down = window.get_mouse_down(minifb::MouseButton::Left);
                let mouse_clicked = mouse_down && !tower_mouse_down;
                tower_mouse_down = mouse_down;
                let action = tower_game.update(
                    delta_time,
                    tower_guard::InputFrame {
                        cursor,
                        cursor_moved,
                        fire: mouse_clicked || crate::input::pressed(Key::Space),
                        fire_held: mouse_down || window.is_key_down(Key::Space),
                        mouse_clicked,
                        confirm: crate::input::pressed(Key::Enter),
                        exit: crate::input::pressed(Key::Backspace),
                        reload: crate::input::pressed(Key::R),
                        arrow_x: window.is_key_down(Key::Right) as i8
                            - window.is_key_down(Key::Left) as i8,
                        arrow_y: window.is_key_down(Key::Down) as i8
                            - window.is_key_down(Key::Up) as i8,
                        move_x: window.is_key_down(Key::D) as i8 - window.is_key_down(Key::A) as i8,
                        move_y: window.is_key_down(Key::W) as i8 - window.is_key_down(Key::S) as i8,
                        interact: crate::input::pressed(Key::E),
                        look_x: mouse_look.0,
                        look_y: mouse_look.1,
                        aiming: window.get_mouse_down(minifb::MouseButton::Right),
                        weapon: if crate::input::pressed(Key::Key1) {
                            1
                        } else if crate::input::pressed(Key::Key2) {
                            2
                        } else if crate::input::pressed(Key::Key3) {
                            3
                        } else {
                            0
                        },
                    },
                    scene_framebuffer.width,
                    scene_framebuffer.height,
                    &tower_sprites,
                );
                for cue in tower_game.take_audio_cues().into_iter().flatten() {
                    tower_audio.play(cue);
                }
                match action {
                    tower_guard::TowerAction::Continue => {}
                    tower_guard::TowerAction::Restart => {
                        tower_game = tower_guard::TowerGame::new(&tower_sprites);
                        window.set_cursor_visibility(false);
                    }
                    tower_guard::TowerAction::Exit => {
                        tower_audio.stop_music();
                        game_mode = GameMode::FreeWorld;
                        launch_ship_from_planet(
                            &mut ship_controller,
                            vesper_planet_position(1, world_time),
                            vesper_planets[1].radius,
                        );
                        window.set_cursor_visibility(true);
                        window.set_title("Casa Torre | En orbita | Backspace: salir");
                        continue;
                    }
                }
                tower_game.render(&mut scene_framebuffer, &tower_sprites);
                window.set_cursor_visibility(matches!(
                    tower_game.phase(),
                    tower_guard::Phase::Victory | tower_guard::Phase::Defeat
                ));
            }
            GameMode::HourglassRescue => {
                if hourglass_rescue.is_paused() {
                    if crate::input::pressed(Key::Equal)
                        || crate::input::pressed(Key::NumPadPlus)
                        || crate::input::pressed(Key::RightBracket)
                    {
                        minigame_music.adjust(true);
                    }
                    if crate::input::pressed(Key::Minus)
                        || crate::input::pressed(Key::NumPadMinus)
                        || crate::input::pressed(Key::LeftBracket)
                    {
                        minigame_music.adjust(false);
                    }
                    if crate::input::pressed(Key::M) {
                        minigame_music.toggle_mute();
                    }
                }
                let restart_pressed =
                    crate::input::pressed(Key::R) || gamepad.pressed(crate::input::Gamepad::Y);
                let rescue_input = hourglass_rescue::Input {
                    forward: window.is_key_down(Key::Space) || gamepad.accelerate,
                    brake: window.is_key_down(Key::LeftCtrl)
                        || window.is_key_down(Key::RightCtrl)
                        || gamepad.brake,
                    left: window.is_key_down(Key::A)
                        || window.is_key_down(Key::Left)
                        || gamepad.horizontal() < 0,
                    right: window.is_key_down(Key::D)
                        || window.is_key_down(Key::Right)
                        || gamepad.horizontal() > 0,
                    rise: window.is_key_down(Key::W)
                        || window.is_key_down(Key::Up)
                        || gamepad.vertical() > 0,
                    descend: window.is_key_down(Key::S)
                        || window.is_key_down(Key::Down)
                        || gamepad.vertical() < 0,
                    roll_left: window.is_key_down(Key::Q) || gamepad.roll_left,
                    roll_right: window.is_key_down(Key::E) || gamepad.roll_right,
                    boost: window.is_key_down(Key::LeftShift)
                        || window.is_key_down(Key::RightShift)
                        || gamepad.boost,
                    firing: window.is_key_down(Key::Enter) || gamepad.fire,
                    pause: crate::input::pressed(Key::P)
                        || crate::input::pressed(Key::Escape)
                        || gamepad.pressed(crate::input::Gamepad::START),
                    exit: crate::input::pressed(Key::Backspace)
                        || gamepad.pressed(crate::input::Gamepad::B),
                    restart: restart_pressed,
                    recover: restart_pressed,
                };
                if hourglass_rescue.update(&mut ship_controller.ship, rescue_input, delta_time)
                    == hourglass_rescue::Action::Exit
                {
                    if hourglass_rescue.mission_won() {
                        collected_dragon_balls |= 0b0010000;
                    }
                    game_mode = GameMode::FreeWorld;
                    launch_ship_from_planet(
                        &mut ship_controller,
                        vesper_planet_position(0, world_time),
                        vesper_planets[0].radius,
                    );
                    window.set_title("Galaxia Vesper | Vuelo libre | Reloj de Arena");
                    continue;
                }
                ship_controller.update_weapon_and_projectiles(rescue_input.firing, delta_time);
                hourglass_rescue.update_enemies(
                    &mut ship_controller.ship,
                    &mut ship_controller.projectiles,
                    delta_time,
                );
                hourglass_rescue.render(
                    &mut scene_framebuffer,
                    active_model,
                    &ship_controller.ship,
                );
                hourglass_rescue.render_weapon(
                    &mut scene_framebuffer,
                    &laser_bolt,
                    &ship_controller.projectiles,
                    &ship_controller.ship,
                    ship_controller.charge_ratio(),
                );
            }
            GameMode::LavaCastle => {
                let castle_input = lava_castle::Input {
                    pitch: pitch_input,
                    yaw: active_yaw_input,
                    roll: roll_input,
                    accelerate: window.is_key_down(Key::Space) || gamepad.accelerate,
                    brake: window.is_key_down(Key::LeftCtrl)
                        || window.is_key_down(Key::RightCtrl)
                        || gamepad.brake,
                    boost: window.is_key_down(Key::LeftShift)
                        || window.is_key_down(Key::RightShift)
                        || gamepad.boost,
                    fire: window.is_key_down(Key::Enter) || gamepad.fire,
                    restart: crate::input::pressed(Key::R)
                        || gamepad.pressed(crate::input::Gamepad::Y),
                    exit: crate::input::pressed(Key::Backspace)
                        || gamepad.pressed(crate::input::Gamepad::B),
                };
                let castle_action =
                    lava_castle.update(&mut ship_controller, castle_input, delta_time);
                match castle_action {
                    lava_castle::Action::Continue => {}
                    lava_castle::Action::Exit | lava_castle::Action::InteriorTransition => {
                        if castle_action == lava_castle::Action::InteriorTransition {
                            collected_dragon_balls |= 0b0100000;
                            game_mode = GameMode::LavaCastleInterior;
                            lava_castle_interior.enter(&mut ship_controller);
                            window.set_title("Nivel Final | Castillo de Lava | Parte 2");
                        } else {
                            game_mode = GameMode::FreeWorld;
                            launch_ship_from_planet(
                                &mut ship_controller,
                                castle_planet.center,
                                castle_planet.radius,
                            );
                            window.set_title("Galaxia Nova | Planeta Castillo");
                        }
                        continue;
                    }
                }
                lava_castle.render(
                    &mut scene_framebuffer,
                    active_model,
                    &laser_bolt,
                    &six_star_dragon_ball,
                    &ship_controller,
                );
            }
            GameMode::LavaCastleInterior => {
                let action = lava_castle_interior.update(
                    &mut ship_controller,
                    &input,
                    crate::input::pressed(Key::R) || gamepad.pressed(crate::input::Gamepad::Y),
                    crate::input::pressed(Key::Backspace)
                        || gamepad.pressed(crate::input::Gamepad::B),
                    delta_time,
                );
                match action {
                    lava_castle_interior::Action::Continue => {}
                    lava_castle_interior::Action::Exit | lava_castle_interior::Action::Victory => {
                        if action == lava_castle_interior::Action::Victory {
                            collected_dragon_balls |= 0b1000000;
                        }
                        game_mode = GameMode::FreeWorld;
                        launch_ship_from_planet(
                            &mut ship_controller,
                            castle_planet.center,
                            castle_planet.radius,
                        );
                        window.set_title(if action == lava_castle_interior::Action::Victory {
                            "JUEGO COMPLETADO | Esfera de 7 estrellas obtenida"
                        } else {
                            "Galaxia Nova | Planeta Castillo"
                        });
                        continue;
                    }
                }
                lava_castle_interior.render(
                    &mut scene_framebuffer,
                    active_model,
                    &laser_bolt,
                    &seven_star_dragon_ball,
                    &ship_controller,
                );
            }
        }

        // When P1 is inside a level, P2 keeps its own renderer and session in
        // the right viewport instead of inheriting P1's global mode.
        if coop_enabled && game_mode != GameMode::FreeWorld {
            coop_triangle_batch.begin_frame();
            let coop_turbo = coop_controller.turbo_power(coop_controller.ship.speed);
            match coop_game_mode {
                GameMode::FreeWorld => {
                    coop_controller.update(&coop_input, delta_time);
                    coop_controller.rebase_far_flight();
                    let coop_ship = &mut coop_controller.ship;
                    let previous_position = coop_ship.position;
                    let movement = coop_ship.speed * delta_time;
                    if gamepad_two.pressed(crate::input::Gamepad::SELECT)
                        || gamepad_two.pressed(crate::input::Gamepad::B)
                    {
                        coop_ship.planet_locked = false;
                        coop_ship.spiky_planet_locked = false;
                        coop_ship.race_planet_locked = false;
                        coop_ship.orbit_release_active = true;
                        coop_ship.orbit_plane_normal = None;
                        coop_vesper_orbit_index = None;
                    }
                    let coop_portal = wormhole_position(coop_active_galaxy);
                    let coop_offset = coop_ship.position.sub(coop_portal);
                    let coop_near_nova = coop_active_galaxy == Galaxy::Vesper
                        && coop_ship
                            .position
                            .sub(nova_portal_position())
                            .dot(coop_ship.position.sub(nova_portal_position()))
                            <= 42.0 * 42.0;
                    if !coop_ship.planet_locked
                        && (coop_offset.dot(coop_offset) <= 42.0 * 42.0 || coop_near_nova)
                    {
                        coop_active_galaxy = if coop_near_nova {
                            Galaxy::Nova
                        } else {
                            coop_active_galaxy.portal_destination()
                        };
                        let exit = match coop_active_galaxy {
                            Galaxy::Vesper => Vec3::new(-420.0, 0.0, 0.0),
                            Galaxy::Nova => Vec3::new(0.0, 20.0, -150.0),
                            Galaxy::Home => Vec3::new(150.0, 0.0, 80.0),
                        };
                        coop_ship.position = wormhole_position(coop_active_galaxy).add(exit);
                        coop_ship.speed = coop_ship.spec.cruise_speed;
                        coop_vesper_orbit_index = None;
                    }
                    match coop_active_galaxy {
                        Galaxy::Home => {
                            let water_center = water_planet_position(world_time);
                            let spiky_center = spiky_planet_position(world_time);
                            let race_center = race_planet_position(world_time);
                            if !coop_ship.planet_locked {
                                coop_ship.race_planet_locked = race_planet_is_closest(
                                    coop_ship.position,
                                    water_center,
                                    spiky_center,
                                    race_center,
                                );
                                coop_ship.spiky_planet_locked = !coop_ship.race_planet_locked
                                    && spiky_planet_is_closer(
                                        coop_ship.position,
                                        water_center,
                                        spiky_center,
                                    );
                            }
                            let (center, radius, rotation, colliders) =
                                if coop_ship.race_planet_locked {
                                    (
                                        race_center,
                                        race_track::PLANET_RADIUS,
                                        world_time * RACE_PLANET_ROTATION_SPEED,
                                        &[] as &[ObjectCollider],
                                    )
                                } else if coop_ship.spiky_planet_locked {
                                    (
                                        spiky_center,
                                        SPIKY_PLANET_RADIUS,
                                        world_time * SPIKY_PLANET_ROTATION_SPEED,
                                        &[] as &[ObjectCollider],
                                    )
                                } else {
                                    (
                                        water_center,
                                        planet_surface_radius,
                                        world_time * WATER_PLANET_ROTATION_SPEED,
                                        object_colliders.as_slice(),
                                    )
                                };
                            move_around_planet(
                                coop_ship,
                                movement,
                                center,
                                radius,
                                rotation,
                                colliders,
                                coop_input.yaw,
                                coop_input.pitch,
                                false,
                            );
                            if ship_touches_sun(
                                previous_position,
                                coop_ship.position,
                                coop_ship.spec.collision_radius,
                                sun_collision_radius,
                            ) {
                                coop_ship.position = previous_position;
                                coop_ship.speed = 0.0;
                            }
                        }
                        Galaxy::Vesper => {
                            let nearest = coop_vesper_orbit_index.unwrap_or_else(|| {
                                vesper_planets
                                    .iter()
                                    .enumerate()
                                    .min_by(|(a, _), (b, _)| {
                                        let da = coop_ship
                                            .position
                                            .sub(vesper_planet_position(*a, world_time));
                                        let db = coop_ship
                                            .position
                                            .sub(vesper_planet_position(*b, world_time));
                                        da.dot(da).total_cmp(&db.dot(db))
                                    })
                                    .map(|(index, _)| index)
                                    .unwrap_or(0)
                            });
                            let body = &vesper_planets[nearest];
                            move_around_planet(
                                coop_ship,
                                movement,
                                vesper_planet_position(nearest, world_time),
                                VESPER_ORBIT_RADII[nearest],
                                body.rotation_y + world_time * 0.035,
                                &[],
                                coop_input.yaw,
                                coop_input.pitch,
                                true,
                            );
                            coop_vesper_orbit_index = coop_ship.planet_locked.then_some(nearest);
                            if !coop_ship.planet_locked {
                                coop_ship.position =
                                    apply_vesper_black_hole_gravity(coop_ship.position, delta_time);
                            }
                        }
                        Galaxy::Nova => move_around_planet(
                            coop_ship,
                            movement,
                            castle_planet.center,
                            NOVA_CASTLE_ORBIT_RADIUS,
                            castle_planet.rotation_y + world_time * 0.018,
                            &[],
                            coop_input.yaw,
                            coop_input.pitch,
                            true,
                        ),
                    }
                    if !coop_ship.planet_locked {
                        let (center, radius) = match coop_active_galaxy {
                            Galaxy::Home => (Vec3::new(SUN_POSITION_X, 20.0, 0.0), 350.0),
                            Galaxy::Vesper => (VESPER_SYSTEM_CENTER, 350.0),
                            Galaxy::Nova => (Vec3::new(0.0, 70.0, 0.0), 300.0),
                        };
                        if let Some(safe_position) = space_decor::asteroid_collision_position(
                            previous_position,
                            coop_ship.position,
                            coop_ship.spec.collision_radius,
                            coop_active_galaxy,
                            center,
                            radius,
                            world_time,
                        ) {
                            coop_ship.position = safe_position;
                            coop_ship.speed =
                                coop_ship.speed.min(coop_ship.spec.cruise_speed * 0.38);
                        }
                    }
                    let camera_target = if coop_ship.orbit_plane_normal.is_some() {
                        1.0
                    } else {
                        0.0
                    };
                    let camera_response = 1.0 - (-2.4 * delta_time).exp();
                    coop_ship.orbit_camera_blend +=
                        (camera_target - coop_ship.orbit_camera_blend) * camera_response;
                    if coop_ship.orbit_plane_normal.is_none()
                        && coop_ship.orbit_camera_blend < 0.001
                    {
                        coop_ship.orbit_center = None;
                        coop_ship.orbit_camera_blend = 0.0;
                    }
                    render_scene(
                        active_model,
                        ship_center,
                        &turbo_plume,
                        &laser_bolt,
                        &coop_controller.projectiles,
                        &planet,
                        planet_center,
                        planet_scale,
                        &spiky_planet,
                        spiky_planet_center,
                        spiky_planet_scale,
                        &sun,
                        sun_center,
                        &home_orbit_rings,
                        &wormhole,
                        wormhole_position(coop_active_galaxy),
                        coop_active_galaxy,
                        &vesper_planets,
                        &vesper_black_hole,
                        &vesper_orbit_rings,
                        &castle_planet,
                        &race_track.planet,
                        &choices,
                        &ambient_battle,
                        [&planet_lods[0], &planet_lods[1], &planet_lods[2]],
                        &mut coop_planet_lod_states,
                        &mut coop_vesper_lod_states,
                        &mut coop_castle_lod_state,
                        &mut coop_framebuffer,
                        &mut coop_triangle_batch,
                        &coop_controller.ship,
                        world_time,
                        0.0,
                        coop_turbo,
                        false,
                        coop_controller.charge_ratio(),
                        true,
                        None,
                    );
                }
                GameMode::WaterLevel => render_water_level(
                    &water_level_spatial,
                    &enemy_models,
                    &enemy_centers,
                    &coop_water_level_state,
                    &mut coop_enemy_instance_buffers,
                    &bubble_model,
                    &underwater_boost_ring,
                    &four_star_dragon_ball,
                    &reward_pedestal,
                    &reward_chamber,
                    &reward_door,
                    &coop_bubble_emitter.particles,
                    coop_active_model,
                    coop_ship_center,
                    &turbo_plume,
                    &laser_bolt,
                    &enemy_laser_bolt,
                    &coop_controller.projectiles,
                    &mut coop_framebuffer,
                    &mut coop_triangle_batch,
                    &coop_controller.ship,
                    world_time,
                    coop_turbo,
                    coop_controller.charge_ratio(),
                ),
                GameMode::CoreGuardian => coop_guardian_fight.render_scene(
                    &mut coop_framebuffer,
                    coop_active_model,
                    coop_ship_center,
                    coop_controller.ship.spec,
                    &three_star_dragon_ball,
                ),
                GameMode::HourglassRescue => {
                    coop_hourglass_rescue.render(
                        &mut coop_framebuffer,
                        coop_active_model,
                        &coop_controller.ship,
                    );
                    coop_hourglass_rescue.render_weapon(
                        &mut coop_framebuffer,
                        &laser_bolt,
                        &coop_controller.projectiles,
                        &coop_controller.ship,
                        coop_controller.charge_ratio(),
                    );
                }
                GameMode::LavaCastle => coop_lava_castle.render(
                    &mut coop_framebuffer,
                    coop_active_model,
                    &laser_bolt,
                    &six_star_dragon_ball,
                    &coop_controller,
                ),
                GameMode::LavaCastleInterior => coop_lava_castle_interior.render(
                    &mut coop_framebuffer,
                    coop_active_model,
                    &laser_bolt,
                    &seven_star_dragon_ball,
                    &coop_controller,
                ),
                GameMode::Racing => coop_race_track.render(
                    &mut coop_framebuffer,
                    &mut coop_triangle_batch,
                    &race_vehicles,
                    &race_sky,
                    &coop_controller,
                    &turbo_plume,
                    &laser_bolt,
                    &one_star_dragon_ball,
                    world_time,
                ),
                GameMode::TowerGuard => {
                    coop_tower_game.render(&mut coop_framebuffer, &tower_sprites)
                }
            }
        }

        if galaxy_transition
            .as_ref()
            .is_some_and(|transit| transit.elapsed >= 2.15)
        {
            galaxy_transition = None;
            window.set_title(match active_galaxy {
                Galaxy::Home => "Sistema solar | Vuelo libre 3D | F11: fullscreen",
                Galaxy::Vesper => "Galaxia Vesper | Portales: Sistema solar y Nova",
                Galaxy::Nova => "Galaxia Nova | Portal disponible: Vesper",
            });
        }

        let update_scene_ms = update_scene_start.elapsed().as_secs_f32() * 1_000.0;
        let post_start = Instant::now();
        if advanced_lighting
            && game_mode != GameMode::TowerGuard
            && game_mode != GameMode::HourglassRescue
        {
            hybrid_ray_tracing.apply_high(&mut scene_framebuffer);
        }
        let post_ms = post_start.elapsed().as_secs_f32() * 1_000.0;
        let copy_start = Instant::now();
        if coop_enabled {
            upscale_split_scene(&scene_framebuffer, &coop_framebuffer, &mut framebuffer);
        } else {
            upscale_scene(&scene_framebuffer, &mut framebuffer);
        }
        let copy_ms = copy_start.elapsed().as_secs_f32() * 1_000.0;
        // All gameplay modes use the shared HUD below. Keep the legacy
        // renderers isolated while their mode-specific data is migrated.
        if false {
            if game_mode == GameMode::CoreGuardian {
                guardian_fight.draw_hud(&mut framebuffer);
                draw_selection_label(&mut framebuffer, "GUARDIAN DEL NUCLEO", 600, 8, 2, 0xffe2ed);
                let message = if guardian_fight.is_victory() {
                    if guardian_fight.reward_received() {
                        "ESFERA DE 3 ESTRELLAS OBTENIDA"
                    } else {
                        "JEFE DESTRUIDO - RECIBIENDO RECOMPENSA"
                    }
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
            } else if game_mode == GameMode::HourglassRescue {
                hourglass_rescue.draw_hud_with_audio(
                    &mut framebuffer,
                    &ship_controller.ship,
                    minigame_music.volume_percent(),
                    minigame_music.muted(),
                );
            } else if game_mode != GameMode::TowerGuard {
                let form = if game_mode == GameMode::Racing {
                    race_vehicles[selected_ship]
                        .map(|(_, spec)| spec.name)
                        .unwrap_or("CARRO")
                } else if selected_ship == 4 {
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
                if game_mode != GameMode::Racing {
                    draw_selection_label(
                        &mut framebuffer,
                        &form.to_ascii_uppercase(),
                        600,
                        22,
                        2,
                        0x99e226,
                    );
                }
                if game_mode == GameMode::WaterLevel {
                    let enemy_counter = format!(
                        "ENEMIGOS: {}/{}",
                        water_level_state.defeated_enemies,
                        water_level_state.enemies.len() + 1
                    );
                    draw_selection_label(&mut framebuffer, &enemy_counter, 1035, 24, 2, 0x65dfea);
                    draw_selection_label(
                        &mut framebuffer,
                        water_level_state.stage().label(),
                        600,
                        54,
                        2,
                        0x8cebf4,
                    );
                    let hull_status = format!(
                        "CASCO: {}/{}   ALERTA: FORMACIONES HOSTILES",
                        water_level_state.hull_health, WATER_SUBMARINE_MAX_HEALTH
                    );
                    draw_selection_label(
                        &mut framebuffer,
                        &hull_status,
                        180,
                        54,
                        2,
                        if water_level_state.hit_invulnerability > 0.0 {
                            0xff7a62
                        } else {
                            0x72f3ff
                        },
                    );
                    if water_level_state.stage() == WaterStage::SharkBoss {
                        let boss_status = if water_level_state.boss_defeated {
                            if water_level_state.boss_explosion_timer > 0.0 {
                                "DESTRUCCION DEL TIBURON - MANTEN DISTANCIA".to_string()
                            } else if water_level_state.reward_door_open
                                < WATER_REWARD_DOOR_PASSABLE
                            {
                                "COMPUERTA DEL SANTUARIO ABRIENDOSE".to_string()
                            } else {
                                "TIBURON DESTRUIDO - SALIDA ABIERTA".to_string()
                            }
                        } else {
                            format!(
                                "TIBURON MECANICO  VIDA {}/{}  ENTER: DISPARAR",
                                water_level_state.boss_health, WATER_BOSS_MAX_HEALTH
                            )
                        };
                        draw_selection_label(
                            &mut framebuffer,
                            &boss_status,
                            600,
                            82,
                            2,
                            if water_level_state.boss_hit_flash > 0.0 {
                                0xffffff
                            } else {
                                0xff5252
                            },
                        );
                    }
                    if water_level_state.stage() == WaterStage::RewardChamber {
                        let reward_status = if water_level_state.dragon_ball_collected {
                            "ESFERA DE 4 ESTRELLAS OBTENIDA - SALIDA ABIERTA"
                        } else {
                            "ACERCATE A LA ESFERA DE 4 ESTRELLAS"
                        };
                        draw_selection_label(&mut framebuffer, reward_status, 600, 82, 2, 0xffa62b);
                    }
                }
                if game_mode != GameMode::Racing {
                    draw_selection_label(
                        &mut framebuffer,
                        "ESC: PAUSA   F1: CONTROLES",
                        600,
                        766,
                        2,
                        0xffffff,
                    );
                }
                if active_galaxy == Galaxy::Home && game_mode == GameMode::FreeWorld {
                    draw_selection_label(
                        &mut framebuffer,
                        "SISTEMA SOLAR  3: JARDINES DE OXIDO  ENTER EN ORBITA: GRAN PREMIO",
                        600,
                        738,
                        1,
                        0xa9d8ff,
                    );
                }
                let resolution_label = if dynamic_resolution.enabled {
                    format!("F2: RES AUTO {}%", dynamic_resolution.percentage())
                } else {
                    "F2: RES AUTO OFF".to_string()
                };
                draw_selection_label(&mut framebuffer, &resolution_label, 1040, 766, 2, 0x9fc5df);
            }
        }

        let objective = match game_mode {
            GameMode::FreeWorld
                if collected_dragon_balls & 0b0111_1111 == 0b0111_1111
                    && active_galaxy == Galaxy::Vesper =>
            {
                "NEXO ESTABILIZADO: VUELA LIBRE"
            }
            GameMode::FreeWorld if collected_dragon_balls & 0b0111_1111 == 0b0111_1111 => {
                "NEXO COMPLETO: REGRESA A VESPER"
            }
            GameMode::FreeWorld if active_galaxy == Galaxy::Home => {
                "BUSCA LAS 7 ESFERAS DEL NEXO"
            }
            GameMode::FreeWorld => "RECUPERA LAS ESFERAS DEL NEXO",
            GameMode::WaterLevel if water_level_state.stage() == WaterStage::SharkBoss => {
                "DESTRUYE AL TIBURON MECANICO"
            }
            GameMode::WaterLevel if water_level_state.stage() == WaterStage::RewardChamber => {
                "RECUPERA LA ESFERA DE 4 ESTRELLAS"
            }
            GameMode::WaterLevel => "AVANZA POR EL ABISMO AZUL",
            GameMode::Racing => "COMPLETA EL CIRCUITO JARDINES DE OXIDO",
            GameMode::CoreGuardian if guardian_fight.is_exposed() => "ATACA EL NUCLEO EXPUESTO",
            GameMode::CoreGuardian => "SOBREVIVE Y ABRE EL NUCLEO",
            GameMode::TowerGuard => "DEFIENDE LA TORRE",
            GameMode::HourglassRescue => "REUNE FRAGMENTOS Y CONSIGUE LA ESFERA",
            GameMode::LavaCastle => "DERROTA AL GUARDIAN Y ABRE EL CASTILLO",
            GameMode::LavaCastleInterior => "REUNE 3 FRAGMENTOS Y ABRE EL SELLO FINAL",
        };
        let shield_ratio = match game_mode {
            GameMode::WaterLevel => {
                water_level_state.hull_health as f32 / WATER_SUBMARINE_MAX_HEALTH as f32
            }
            GameMode::CoreGuardian => guardian_fight.shield_ratio(),
            GameMode::HourglassRescue => hourglass_rescue.shield_ratio(),
            GameMode::TowerGuard => tower_game.health_ratio(),
            GameMode::LavaCastle => lava_castle.player_health_ratio(),
            GameMode::LavaCastleInterior => lava_castle_interior.health_ratio(),
            _ => 1.0,
        };
        let score = match game_mode {
            GameMode::WaterLevel => water_level_state.defeated_enemies as u32 * 1_250,
            GameMode::CoreGuardian => ((1.0 - guardian_fight.health_ratio()) * 100_000.0) as u32,
            GameMode::TowerGuard => tower_game.score(),
            GameMode::HourglassRescue => hourglass_rescue.rescued_count() as u32 * 1_000,
            GameMode::LavaCastle => 0,
            GameMode::LavaCastleInterior => 0,
            GameMode::Racing => {
                (ship_controller.ship.position.x.abs() + ship_controller.ship.position.z.abs())
                    as u32
                    * 25
            }
            GameMode::FreeWorld => ship_controller.projectiles.len() as u32 * 100,
        };
        let boss_hud = match game_mode {
            GameMode::WaterLevel
                if water_level_state.stage() == WaterStage::SharkBoss
                    && !water_level_state.boss_defeated =>
            {
                Some((
                    "JEFE: TIBURON MECANICO",
                    water_level_state.boss_health as f32 / WATER_BOSS_MAX_HEALTH as f32,
                ))
            }
            GameMode::CoreGuardian if !guardian_fight.is_victory() => {
                Some(("JEFE: GUARDIAN DEL NUCLEO", guardian_fight.health_ratio()))
            }
            GameMode::LavaCastle => {
                Some(("JEFE: GUARDIAN DE LAVA", lava_castle.boss_health_ratio()))
            }
            _ => None,
        };
        if coop_enabled {
            draw_split_coop_hud(
                &mut framebuffer,
                &ship_controller,
                &coop_controller,
                collected_dragon_balls,
            );
        } else {
            draw_unified_combat_hud(
                &mut framebuffer,
                objective,
                shield_ratio,
                score,
                ship_controller.turbo_power(ship_controller.ship.speed),
                if game_mode == GameMode::CoreGuardian {
                    guardian_fight.weapon_charge_ratio()
                } else {
                    ship_controller.charge_ratio()
                },
                boss_hud,
                if game_mode == GameMode::CoreGuardian {
                    guardian_fight.aim_screen_position()
                } else {
                    (0.5, 0.5)
                },
            );
            if game_mode == GameMode::FreeWorld && ship_controller.ship.planet_locked {
                if let Some(planet) = story_guide::planet_for_orbit(
                    active_galaxy,
                    vesper_orbit_index,
                    ship_controller.ship.race_planet_locked,
                    ship_controller.ship.spiky_planet_locked,
                ) {
                    story_guide.draw(&mut framebuffer, planet, collected_dragon_balls);
                }
            }
        }
        if story_epilogue_timer > 0.0 {
            draw_story_epilogue(&mut framebuffer, story_epilogue_timer);
        }
        if performance_hud {
            let stats = triangle_batch.stats();
            let displayed_lods = if active_galaxy == Galaxy::Vesper {
                [vesper_lod_states[0].level, vesper_lod_states[1].level, 0]
            } else {
                [
                    planet_lod_states[0].level,
                    planet_lod_states[1].level,
                    planet_lod_states[2].level,
                ]
            };
            // F3 diagnostics stay compact in the upper-left corner.
            let panel_left = (framebuffer.width as f32 * 0.018) as usize;
            let panel_top = (framebuffer.height as f32 * 0.125) as usize;
            let panel_width = (framebuffer.width as f32 * 0.21) as usize;
            let panel_height = (framebuffer.height as f32 * 0.105) as usize;
            hud_rect(
                &mut framebuffer,
                panel_left,
                panel_top,
                panel_width,
                panel_height,
            );
            let center = (panel_left + panel_width / 2) as i32;
            let row = (framebuffer.height as f32 * 0.0175) as i32;
            draw_selection_label(
                &mut framebuffer,
                "RENDIMIENTO",
                center,
                panel_top as i32 + 5,
                1,
                0x46e6ff,
            );
            draw_selection_label(
                &mut framebuffer,
                &format!(
                    "FPS {:.0} FRAME {:.0} MS",
                    dynamic_resolution.fps_average, dynamic_resolution.frame_ms_average
                ),
                center,
                panel_top as i32 + 5 + row,
                1,
                0xffffff,
            );
            draw_selection_label(
                &mut framebuffer,
                &format!(
                    "GEO {:.0} SORT {:.0} RAST {:.0} MS",
                    stats.geometry_ms, stats.sort_ms, stats.raster_ms
                ),
                center,
                panel_top as i32 + 5 + row * 2,
                1,
                0xc6d8ff,
            );
            draw_selection_label(
                &mut framebuffer,
                &format!(
                    "MODELOS {} TRI {} RES {}%",
                    stats.models_visible,
                    stats.triangles_submitted,
                    dynamic_resolution.percentage()
                ),
                center,
                panel_top as i32 + 5 + row * 3,
                1,
                0x9fffd2,
            );
            draw_selection_label(
                &mut framebuffer,
                &format!(
                    "DRS {} LOD {}/{}/{}",
                    dynamic_resolution.status,
                    displayed_lods[0],
                    displayed_lods[1],
                    displayed_lods[2],
                ),
                center,
                panel_top as i32 + 5 + row * 4,
                1,
                0xffd78a,
            );
        }
        if crate::input::pressed(Key::P) {
            framebuffer.save_png("captura.png")?;
            println!("Captura guardada: captura.png");
        }
        crate::input::clear();
        let present_start = Instant::now();
        window.update_with_buffer(&framebuffer.pixels, WIDTH, HEIGHT)?;
        let present_ms = present_start.elapsed().as_secs_f32() * 1_000.0;
        if let Some(run) = benchmark.as_mut() {
            let mut stats = triangle_batch.stats();
            let (local_stats, local_calls) = take_local_render_stats();
            let (race_geometry_ms, race_vertices, race_triangles) =
                take_race_benchmark_categories();
            stats.models_visible += local_stats.models_visible;
            stats.models_culled += local_stats.models_culled;
            stats.vertices_transformed += local_stats.vertices_transformed;
            stats.triangles_submitted += local_stats.triangles_submitted;
            stats.geometry_ms += local_stats.geometry_ms;
            stats.sort_ms += local_stats.sort_ms;
            stats.raster_ms += local_stats.raster_ms;
            if run.warmup_left == 0 && run.samples.is_empty() {
                println!("BENCH local_render_calls={local_calls}");
                if let Ok(path) = std::env::var("GRAPH_BENCH_CAPTURE") {
                    framebuffer.save_png(&path)?;
                    println!("BENCH capture={path}");
                    return Ok(false);
                }
            }
            let finished = run.record(BenchmarkSample {
                frame_ms: benchmark_frame_start.elapsed().as_secs_f32() * 1_000.0,
                update_scene_ms,
                geometry_ms: stats.geometry_ms,
                sort_ms: stats.sort_ms,
                raster_ms: stats.raster_ms,
                post_ms,
                copy_ms,
                present_ms,
                models_visible: stats.models_visible,
                models_culled: stats.models_culled,
                vertices: stats.vertices_transformed,
                triangles: stats.triangles_submitted,
                race_geometry_ms,
                race_vertices,
                race_triangles,
            });
            if finished {
                run.print(render_worker_count(), advanced_lighting);
                return Ok(false);
            }
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_workers_scale_with_model_size_without_tiny_jobs() {
        assert_eq!(transform_worker_count(4_095, 32), 1);
        assert_eq!(transform_worker_count(4_096, 32), 2);
        assert_eq!(transform_worker_count(26_056, 32), 13);
        assert_eq!(transform_worker_count(200_000, 32), 32);
        assert_eq!(transform_worker_count(200_000, 1), 1);
    }

    #[test]
    fn free_flight_selects_the_nearest_planet_for_orbit_collision() {
        let ship = Vec3::new(95.0, 0.0, 0.0);
        let water = Vec3::new(0.0, 0.0, 0.0);
        let spiky = Vec3::new(100.0, 0.0, 0.0);
        assert!(spiky_planet_is_closer(ship, water, spiky));
        assert!(!spiky_planet_is_closer(ship, spiky, water));
    }

    #[test]
    fn home_planets_follow_distinct_orbits_around_the_sun() {
        let orbit_center = Vec3::new(SUN_POSITION_X, 0.0, 0.0);
        let positions = [
            (water_planet_position(0.0), PLANET_ORBIT_RADIUS),
            (spiky_planet_position(0.0), SPIKY_PLANET_ORBIT_RADIUS),
            (race_planet_position(0.0), RACE_PLANET_ORBIT_RADIUS),
        ];
        for (position, expected_radius) in positions {
            let offset = position.sub(orbit_center);
            assert!((offset.dot(offset).sqrt() - expected_radius).abs() < 0.01);
        }
        assert!(
            water_planet_position(20.0)
                .sub(water_planet_position(0.0))
                .dot(water_planet_position(20.0).sub(water_planet_position(0.0)))
                > 1.0
        );
        assert!(
            race_planet_position(20.0)
                .sub(race_planet_position(0.0))
                .dot(race_planet_position(20.0).sub(race_planet_position(0.0)))
                > 1.0
        );
    }

    #[test]
    fn home_planets_use_vesper_orbital_speeds_and_have_three_visible_rings() {
        assert_eq!(PLANET_ORBIT_SPEED, VESPER_SYSTEM_ORBIT_SPEEDS[0].abs());
        assert_eq!(
            SPIKY_PLANET_ORBIT_SPEED.abs(),
            VESPER_SYSTEM_ORBIT_SPEEDS[1].abs()
        );
        assert_eq!(RACE_PLANET_ORBIT_SPEED, VESPER_SYSTEM_ORBIT_SPEEDS[0].abs());

        let rings = make_home_orbit_rings();
        for index in 0..3 {
            assert_eq!(
                rings
                    .faces
                    .iter()
                    .filter(|face| face.group == format!("orbit_{index}"))
                    .count(),
                160 * 2
            );
        }
        assert!(rings.faces.iter().any(|face| {
            face.group == "orbit_0" && face.color == MaterialColor::Fixed([58, 174, 255])
        }));
        assert!(rings.faces.iter().any(|face| {
            face.group == "orbit_1" && face.color == MaterialColor::Fixed([226, 75, 255])
        }));
        assert!(rings.faces.iter().any(|face| {
            face.group == "orbit_2" && face.color == MaterialColor::Fixed([255, 205, 68])
        }));
    }

    #[test]
    fn vesper_planets_orbit_the_central_black_hole() {
        assert_eq!(
            VESPER_SYSTEM_ORBIT_SPEEDS[0],
            -VESPER_SYSTEM_ORBIT_SPEEDS[1]
        );
        for index in 0..2 {
            let start = vesper_planet_position(index, 0.0);
            let later = vesper_planet_position(index, 12.0);
            let radial = start.sub(VESPER_SYSTEM_CENTER);
            let later_radial = later.sub(VESPER_SYSTEM_CENTER);
            let horizontal_radius = (radial.x * radial.x + radial.z * radial.z).sqrt();
            let later_radius =
                (later_radial.x * later_radial.x + later_radial.z * later_radial.z).sqrt();
            assert!((horizontal_radius - VESPER_SYSTEM_ORBIT_RADII[index]).abs() < 0.01);
            assert!((later_radius - horizontal_radius).abs() < 0.01);
            assert!(later.sub(start).dot(later.sub(start)) > 100.0);
        }
    }

    #[test]
    fn vesper_planets_have_visible_lines_matching_their_orbits() {
        let rings = make_vesper_orbit_rings();
        for index in 0..2 {
            let group = format!("vesper_orbit_{index}");
            let faces: Vec<_> = rings
                .faces
                .iter()
                .filter(|face| face.group == group)
                .collect();
            assert_eq!(faces.len(), 160 * 2);
            let mut radii = faces.iter().flat_map(|face| face.indices).map(|vertex| {
                let point = rings.vertices[vertex];
                (point.x * point.x + point.z * point.z).sqrt()
            });
            let first = radii.next().expect("orbit ring vertices");
            assert!((first - VESPER_SYSTEM_ORBIT_RADII[index]).abs() < 2.0);
            assert!(faces.iter().all(|face| {
                face.indices.iter().all(|&vertex| {
                    (rings.vertices[vertex].y - VESPER_ORBIT_HEIGHTS[index]).abs() < 0.001
                })
            }));
        }
    }

    #[test]
    fn vesper_black_hole_has_a_dark_core_and_layered_accretion_disk() {
        let model = make_vesper_black_hole();
        assert!(
            model
                .core
                .faces
                .iter()
                .any(|face| face.group == "vesper_black_hole_core")
        );
        let disk_faces = model
            .disk
            .faces
            .iter()
            .filter(|face| face.group == "accretion_disk")
            .count();
        assert_eq!(disk_faces, 48 * 256 * 2);
        assert!(model.disk.faces.iter().any(|face| {
            face.group == "accretion_disk"
                && matches!(face.color, MaterialColor::Fixed([r, g, b]) if r > g && g > b)
        }));
        assert!(model.filaments.faces.iter().any(|face| {
            face.group == "accretion_filament"
                && matches!(face.color, MaterialColor::Fixed([r, g, b]) if r > g && g > b)
        }));
        assert!(
            model
                .lensing_arcs
                .faces
                .iter()
                .any(|face| { face.group == "lensing_arc_upper" })
        );
    }

    #[test]
    fn vesper_black_hole_gravity_pulls_nearby_ships_inward() {
        let start = VESPER_SYSTEM_CENTER.add(Vec3::new(120.0, 14.0, 0.0));
        let pulled = apply_vesper_black_hole_gravity(start, 0.5);
        let start_offset = start.sub(VESPER_SYSTEM_CENTER);
        let pulled_offset = pulled.sub(VESPER_SYSTEM_CENTER);
        assert!(pulled_offset.dot(pulled_offset) < start_offset.dot(start_offset));

        let outside =
            VESPER_SYSTEM_CENTER.add(Vec3::new(VESPER_BLACK_HOLE_GRAVITY_RADIUS + 10.0, 0.0, 0.0));
        let unchanged = apply_vesper_black_hole_gravity(outside, 1.0);
        assert!(unchanged.sub(outside).dot(unchanged.sub(outside)) < 0.0001);
    }

    #[test]
    fn black_hole_absorption_spirals_the_ship_into_the_core() {
        let start = VESPER_SYSTEM_CENTER.add(Vec3::new(24.0, 5.0, 0.0));
        let halfway = vesper_absorption_position(start, 0.5);
        let end = vesper_absorption_position(start, 1.0);
        let start_distance = start
            .sub(VESPER_SYSTEM_CENTER)
            .dot(start.sub(VESPER_SYSTEM_CENTER));
        let halfway_distance = halfway
            .sub(VESPER_SYSTEM_CENTER)
            .dot(halfway.sub(VESPER_SYSTEM_CENTER));
        assert!(halfway_distance < start_distance);
        assert!(
            end.sub(VESPER_SYSTEM_CENTER)
                .dot(end.sub(VESPER_SYSTEM_CENTER))
                < 0.0001
        );
    }

    #[test]
    fn home_portal_is_positioned_directly_above_the_sun() {
        let sun = Vec3::new(SUN_POSITION_X, 20.0, 0.0);
        let portal = wormhole_position(Galaxy::Home);
        assert!((portal.x - sun.x).abs() < 0.001);
        assert!((portal.z - sun.z).abs() < 0.001);
        assert!(portal.y > sun.y + 100.0);

        let model = make_wormhole_sphere(22.0, 18, 28);
        let (center, _) = model.bounds();
        let vertical_radius = model
            .vertices
            .iter()
            .map(|vertex| (vertex.y - center.y).abs())
            .fold(0.0, f32::max);
        assert!(vertical_radius < 3.0, "portal is not lying flat");
    }

    #[test]
    fn touching_or_crossing_the_sun_is_fatal_even_at_high_speed() {
        let sun = Vec3::new(SUN_POSITION_X, 20.0, 0.0);
        assert!(ship_touches_sun(
            sun.add(Vec3::new(-90.0, 0.0, 0.0)),
            sun.add(Vec3::new(90.0, 0.0, 0.0)),
            2.0,
            38.0,
        ));
        assert!(!ship_touches_sun(
            sun.add(Vec3::new(-90.0, 55.0, 0.0)),
            sun.add(Vec3::new(90.0, 55.0, 0.0)),
            2.0,
            38.0,
        ));
    }

    #[test]
    fn approaching_spiky_planet_cannot_cross_its_surface() {
        let center = Vec3::default();
        let mut ship = Ship::new(&ASTRA_SPEC);
        ship.position = Vec3::new(100.0, 0.0, 0.0);
        ship.yaw = 0.0;
        move_around_planet(
            &mut ship,
            80.0,
            center,
            SPIKY_PLANET_RADIUS,
            0.0,
            &[],
            0,
            0,
            false,
        );
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
    fn vesper_orbit_stays_horizontal_around_the_planet_equator() {
        let center = Vec3::new(20.0, 48.0, -10.0);
        let mut ship = Ship::new(&ASTRA_SPEC);
        ship.position = center.add(Vec3::new(50.0, 13.0, 0.0));
        ship.planet_locked = true;

        move_around_planet(&mut ship, 5.0, center, 34.0, 0.0, &[], 0, 0, true);

        assert!((ship.position.y - center.y).abs() < 0.001);
        assert!(ship.pitch.abs() < 0.001);
        let plane = ship.orbit_plane_normal.expect("Vesper orbit plane");
        assert!(plane.x.abs() < 0.001);
        assert!((plane.y - 1.0).abs() < 0.001);
        assert!(plane.z.abs() < 0.001);
    }

    #[test]
    fn nova_castle_captures_the_ship_in_a_horizontal_orbit() {
        let center = Vec3::new(260.0, 90.0, 230.0);
        let mut ship = Ship::new(&ASTRA_SPEC);
        ship.position = center.add(Vec3::new(49.0, 0.0, 0.0));

        move_around_planet(
            &mut ship,
            4.0,
            center,
            NOVA_CASTLE_ORBIT_RADIUS,
            0.0,
            &[],
            0,
            0,
            true,
        );

        assert!(ship.planet_locked);
        assert!((ship.position.y - center.y).abs() < 0.001);
        let distance = ship
            .position
            .sub(center)
            .dot(ship.position.sub(center))
            .sqrt();
        let minimum = NOVA_CASTLE_ORBIT_RADIUS
            + ship.spec.collision_radius
            + COLLISION_MARGIN
            + ORBIT_FLIGHT_ALTITUDE;
        assert!(distance >= minimum - 0.001);
        let plane = ship.orbit_plane_normal.expect("Nova orbit plane");
        assert!(plane.x.abs() < 0.001);
        assert!((plane.y - 1.0).abs() < 0.001);
        assert!(plane.z.abs() < 0.001);
    }

    #[test]
    fn nova_castle_is_directly_ahead_of_the_portal_exit() {
        let exit = wormhole_position(Galaxy::Nova).add(Vec3::new(0.0, 20.0, -150.0));
        let castle = Vec3::new(260.0, 90.0, 230.0);
        let mut ship = Ship::new(&ASTRA_SPEC);
        ship.position = exit;
        ship.yaw = std::f32::consts::PI;
        let forward = ship.movement_basis().0;
        let direction = castle.sub(exit).normalize();
        assert!(forward.dot(direction) > 0.999);
    }

    #[test]
    fn vesper_orbit_camera_is_farther_than_the_home_orbit_camera() {
        let mut ship = Ship::new(&ASTRA_SPEC);
        ship.position = Vec3::new(40.0, 0.0, 0.0);
        ship.orbit_center = Some(Vec3::default());
        ship.orbit_camera_blend = 1.0;

        let home = chase_camera(&ship, false);
        let vesper = chase_camera(&ship, true);
        let home_distance = home.position.dot(home.position).sqrt();
        let vesper_distance = vesper.position.dot(vesper.position).sqrt();

        assert!(vesper_distance > home_distance + 10.0);
        assert!((vesper.position.y - VESPER_ORBIT_CAMERA_HEIGHT).abs() < 0.001);
    }

    #[test]
    fn home_orbit_camera_frames_the_complete_largest_planet() {
        let center = Vec3::new(12.0, -4.0, 30.0);
        let mut ship = Ship::new(&ASTRA_SPEC);
        ship.position = center.add(Vec3::new(52.0, 0.0, 0.0));
        ship.orbit_center = Some(center);
        ship.orbit_camera_blend = 1.0;

        let camera = chase_camera(&ship, false);
        let target = center.add(Vec3::new(0.0, HOME_ORBIT_TARGET_HEIGHT, 0.0));
        let target_offset = target.sub(camera.position);
        assert!(camera.forward.dot(target_offset.normalize()) > 0.9999);

        let center_depth = center.sub(camera.position).dot(camera.forward);
        let focal = HEIGHT as f32 / (VERTICAL_FOV * 0.5).tan() * 0.5;
        let largest_planet_radius = race_track::PLANET_RADIUS;
        let projected_radius = largest_planet_radius * focal / center_depth;
        assert!(
            projected_radius < HEIGHT as f32 * 0.42,
            "planet radius {projected_radius} px does not fit the orbit view"
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
    fn wormhole_energy_is_animated_and_leaves_no_solid_physical_ring() {
        let camera = test_camera();
        let center = Vec3::new(0.0, 0.0, 100.0);
        let background = 0x02030b;
        let mut first = Framebuffer::new(320, 200);
        first.clear(background);
        draw_wormhole_energy(&mut first, &camera, center, 0.0);
        let lit = first
            .pixels
            .iter()
            .filter(|&&pixel| pixel != background)
            .count();
        assert!(lit > 500);
        assert_eq!(first.pixels[100 * 320 + 160], background);

        let mut later = Framebuffer::new(320, 200);
        later.clear(background);
        draw_wormhole_energy(&mut later, &camera, center, 0.7);
        assert!(
            first
                .pixels
                .iter()
                .zip(&later.pixels)
                .filter(|(a, b)| a != b)
                .count()
                > 300
        );
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
        render_model(
            &test_triangle(5.0),
            &mut framebuffer,
            &camera,
            |vertex| vertex.add(Vec3::new(100.0, 0.0, 0.0)),
            None,
            None,
        );
        assert!(framebuffer.pixels.iter().all(|&pixel| pixel == 0));
    }

    #[test]
    fn render_stats_count_visible_and_culled_geometry() {
        let camera = test_camera();
        let mut batch = TriangleBatch::new(WIDTH, HEIGHT);
        let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
        batch.begin_frame();
        queue_model(
            &test_triangle(5.0),
            &mut batch,
            &camera,
            |vertex| vertex,
            None,
            None,
        );
        queue_model(
            &test_triangle(-5.0),
            &mut batch,
            &camera,
            |vertex| vertex,
            None,
            None,
        );
        batch.flush_with_workers(&mut framebuffer, 1);

        let stats = batch.stats();
        assert_eq!(stats.models_visible, 1);
        assert_eq!(stats.models_culled, 1);
        assert_eq!(stats.vertices_transformed, 3);
        assert_eq!(stats.triangles_submitted, 1);
        assert!(stats.geometry_ms.is_finite());
        assert!(stats.raster_ms.is_finite());
    }

    #[test]
    fn offscreen_projectile_effects_do_not_touch_the_framebuffer() {
        let camera = test_camera();
        let mut framebuffer = Framebuffer::new(320, 200);
        framebuffer.clear(0x102030);
        let projectile = Projectile {
            position: Vec3::new(10_000.0, 0.0, 10.0),
            direction: Vec3::new(0.0, 0.0, 1.0),
            lifetime: 1.0,
            scale: 2.0,
        };
        draw_projectile_glow(
            &mut framebuffer,
            &[projectile],
            &camera,
            0.0,
            [255, 255, 255],
            1.65,
        );
        assert!(framebuffer.pixels.iter().all(|&pixel| pixel == 0x102030));
    }

    #[test]
    fn hierarchical_occlusion_rejects_only_fully_hidden_triangles() {
        let width = 32;
        let height = 32;
        let covered = vec![0.5; width * height];
        let grid = CoarseOcclusion::build(&covered, width, 0, height);
        let triangle = |depth| {
            [
                ScreenVertex {
                    x: 2.0,
                    y: 2.0,
                    z: depth,
                },
                ScreenVertex {
                    x: 14.0,
                    y: 2.0,
                    z: depth,
                },
                ScreenVertex {
                    x: 8.0,
                    y: 14.0,
                    z: depth,
                },
            ]
        };
        assert!(grid.hides(triangle(0.25), width));
        assert!(!grid.hides(triangle(0.75), width));

        let mut partly_open = covered;
        partly_open[4 * width + 4] = f32::NEG_INFINITY;
        let grid = CoarseOcclusion::build(&partly_open, width, 0, height);
        assert!(!grid.hides(triangle(0.25), width));
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
    fn batched_rendering_reuses_normal_buffer_and_refreshes_projection_on_resize() {
        let camera = test_camera();
        let mut model = test_triangle(5.0);
        model.faces[0].smooth = true;
        model.recalculate_normals();
        let mut batch = TriangleBatch::new(WIDTH, HEIGHT);

        queue_model(&model, &mut batch, &camera, |vertex| vertex, None, None);
        let normal_allocation = batch.camera_normal_scratch.as_ptr();
        queue_model(
            &model,
            &mut batch,
            &camera,
            |vertex| vertex.add(Vec3::new(0.25, 0.0, 0.0)),
            None,
            None,
        );
        assert_eq!(batch.camera_normal_scratch.as_ptr(), normal_allocation);

        batch.commands.clear();
        let previous_focal = batch.projection.focal;
        batch.resize(WIDTH / 2, HEIGHT / 2);
        assert!(batch.projection.focal < previous_focal);
    }

    #[test]
    fn reused_model_batch_matches_a_fresh_batch() {
        let camera = test_camera();
        let model = test_triangle(5.0);
        let mut reused = Framebuffer::new(64, 64);
        let mut fresh = Framebuffer::new(64, 64);

        render_model(&model, &mut reused, &camera, |vertex| vertex, None, None);

        let mut fresh_batch = TriangleBatch::new(fresh.width, fresh.height);
        queue_model(
            &model,
            &mut fresh_batch,
            &camera,
            |vertex| vertex,
            None,
            None,
        );
        fresh_batch.flush(&mut fresh);

        assert_eq!(reused.pixels, fresh.pixels);
    }

    #[test]
    fn tiled_rasterizer_preserves_depth_across_tile_boundaries() {
        let mut framebuffer = Framebuffer::new(64, 64);
        let mut single_worker = Framebuffer::new(64, 64);
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
        let mut reference_batch = TriangleBatch::new(single_worker.width, single_worker.height);
        reference_batch.commands.push(TriangleCommand {
            vertices: vertices(0.5),
            style: TriangleStyle::Flat(0xff0000),
        });
        reference_batch.commands.push(TriangleCommand {
            vertices: vertices(1.0),
            style: TriangleStyle::Flat(0x00ff00),
        });
        batch.commands.push(TriangleCommand {
            vertices: vertices(0.5),
            style: TriangleStyle::Flat(0xff0000),
        });
        batch.commands.push(TriangleCommand {
            vertices: vertices(1.0),
            style: TriangleStyle::Flat(0x00ff00),
        });
        batch.flush_with_workers(&mut framebuffer, 2);
        reference_batch.flush_with_workers(&mut single_worker, 1);

        assert_eq!(framebuffer.pixels, single_worker.pixels);
        assert_eq!(framebuffer.pixels[31 * 64 + 32], 0x00ff00);
        assert_eq!(framebuffer.pixels[32 * 64 + 32], 0x00ff00);
        assert!(batch.commands.is_empty());
    }

    #[test]
    fn lightweight_profile_keeps_native_resolution_under_load() {
        let mut resolution = DynamicResolution::new();
        for _ in 0..80 {
            resolution.update(1.0 / 40.0, RenderStats::default());
        }
        assert_eq!(resolution.scale_index, 0);
        assert_eq!(resolution.dimensions(), (WIDTH, HEIGHT));

        for _ in 0..600 {
            resolution.update(1.0 / 100.0, RenderStats::default());
        }
        assert_eq!(resolution.scale_index, 0);
        assert_eq!(resolution.dimensions(), (WIDTH, HEIGHT));
    }

    #[test]
    fn racing_resolution_stays_native() {
        let mut resolution = DynamicResolution::new();
        assert!(!resolution.prefer_racing_resolution());
        assert_eq!(resolution.percentage(), 100);
        assert_eq!(resolution.status, "HD NATIVO");
        assert!(!resolution.prefer_racing_resolution());
    }

    #[test]
    fn disabling_dynamic_resolution_restores_full_size() {
        let mut resolution = DynamicResolution::new();
        resolution.scale_index = RESOLUTION_SCALES.len() - 1;
        assert!(!resolution.toggle());
        assert!(!resolution.enabled);
        assert_eq!(resolution.dimensions(), (WIDTH, HEIGHT));
    }

    #[test]
    fn dynamic_resolution_preserves_a_high_quality_floor() {
        let mut resolution = DynamicResolution::new();
        resolution.scale_index = RESOLUTION_SCALES.len() - 1;
        assert_eq!(resolution.percentage(), 100);
        assert_eq!(resolution.dimensions(), (WIDTH, HEIGHT));
    }

    #[test]
    fn vesper_hd_restores_and_holds_native_resolution() {
        let mut resolution = DynamicResolution::new();
        resolution.scale_index = RESOLUTION_SCALES.len() - 1;
        resolution.low_time = 0.7;

        assert!(!resolution.prefer_native_resolution("VESPER HD"));
        assert_eq!(resolution.percentage(), 100);
        assert_eq!(resolution.dimensions(), (WIDTH, HEIGHT));
        assert_eq!(resolution.low_time, 0.0);
        assert_eq!(resolution.status, "VESPER HD");
        assert!(!resolution.prefer_native_resolution("VESPER HD"));
    }

    #[test]
    fn dynamic_resolution_uses_render_load_and_ignores_isolated_spikes() {
        let mut loaded = DynamicResolution::new();
        let expensive_render = RenderStats {
            raster_ms: 22.0,
            ..RenderStats::default()
        };
        for _ in 0..180 {
            loaded.update(1.0 / 60.0, expensive_render);
        }
        assert_eq!(loaded.scale_index, 0);
        assert_eq!(loaded.dimensions(), (WIDTH, HEIGHT));

        let mut stable = DynamicResolution::new();
        stable.update(0.1, RenderStats::default());
        for _ in 0..120 {
            stable.update(1.0 / 60.0, RenderStats::default());
        }
        assert_eq!(stable.scale_index, 0);
    }

    #[test]
    fn dynamic_resolution_hysteresis_avoids_boundary_oscillation() {
        let mut resolution = DynamicResolution::new();
        for frame in 0..600 {
            let frame_time = if frame % 2 == 0 {
                1.0 / 52.0
            } else {
                1.0 / 65.0
            };
            resolution.update(frame_time, RenderStats::default());
        }
        assert_eq!(resolution.scale_index, 0);
        assert_eq!(resolution.status, "ESTABLE");
    }

    #[test]
    fn planet_lod_uses_hysteresis_and_reduces_geometry() {
        let lods = make_planet_lods(WATER_PLANET_RADIUS, PlanetVisual::Water);
        assert!(lods.low.faces.len() < 200);

        let mut state = PlanetLodState::default();
        assert_eq!(state.update(80.0), 0);
        assert_eq!(state.update(25.0), 0);
        assert_eq!(state.update(23.0), 1);
        assert_eq!(state.update(30.0), 1);
        assert_eq!(state.update(35.0), 0);
    }

    #[test]
    fn projected_planet_size_decreases_with_distance() {
        let camera = test_camera();
        let focal = ProjectionCache::new(WIDTH, HEIGHT).focal;
        let near = projected_planet_radius(
            &camera,
            camera.position.add(camera.forward.mul(100.0)),
            WATER_PLANET_RADIUS,
            focal,
        );
        let far = projected_planet_radius(
            &camera,
            camera.position.add(camera.forward.mul(400.0)),
            WATER_PLANET_RADIUS,
            focal,
        );
        assert!(near > far * 3.9);
    }

    #[test]
    fn spiky_tentacles_wave_while_planet_surface_stays_anchored() {
        let surface = Vec3::new(1.47, 0.0, 0.0);
        assert_eq!(animate_spiky_tentacle(surface, 0.0).x, surface.x);
        assert_eq!(animate_spiky_tentacle(surface, 2.0).y, surface.y);

        let tip = Vec3::new(2.30, 0.18, 0.12);
        let first = animate_spiky_tentacle(tip, 0.0);
        let later = animate_spiky_tentacle(tip, 1.0);
        let movement = later.sub(first);
        assert!(movement.dot(movement) > 0.02);
        assert!(first.sub(tip).dot(first.sub(tip)) < 0.12);
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
    fn high_hybrid_ray_tracing_uses_depth_and_preserves_empty_background() {
        let mut frame = Framebuffer::new(12, 12);
        frame.clear(0x102030);
        for y in 2..10 {
            for x in 2..10 {
                let index = y * frame.width + x;
                frame.pixels[index] = 0x406080;
                frame.buffers_mut().1[index] = 0.25;
            }
        }
        let background = frame.pixels[0];
        let surface_before = frame.pixels[6 * frame.width + 6];
        let mut tracer = HybridRayTracing::new();
        tracer.apply_high(&mut frame);
        assert_eq!(frame.pixels[0], background);
        assert_ne!(frame.pixels[6 * frame.width + 6], surface_before);
    }

    #[test]
    fn blue_metal_texture_loads_and_drives_arwing_and_astra_surfaces() {
        let texture = blue_metal_texture().expect("blue metal texture must load");
        assert_eq!((texture.width, texture.height), (512, 512));
        let point = Vec3::new(1.7, 0.9, -2.4);
        let arwing =
            procedural_surface_color(SurfaceShader::ArwingBlue, point, 0, 0, 0.0, 1.0, 0.25);
        let astra =
            procedural_surface_color(SurfaceShader::AstraCobalt, point, 0, 0, 0.0, 1.0, 0.25);
        assert_ne!(arwing, astra);
        assert_ne!(arwing, pack([30, 78, 190], 1.0));
    }

    #[test]
    fn metallic_texture_drives_all_white_ship_shaders() {
        for shader in [
            SurfaceShader::ArwingHull,
            SurfaceShader::BuzzWhiteMetal,
            SurfaceShader::McQueenWhiteMetal,
        ] {
            let first =
                procedural_surface_color(shader, Vec3::new(1.7, 0.9, -2.4), 0, 0, 0.0, 1.0, 0.25);
            let second =
                procedural_surface_color(shader, Vec3::new(4.9, -1.3, 3.8), 0, 0, 0.0, 1.0, 0.25);
            assert_ne!(
                first, second,
                "white shader {shader:?} lacks texture detail"
            );
        }
    }

    #[test]
    fn race_grass_loads_the_complete_optimized_texture_set() {
        let textures = grass_textures().expect("race grass textures must load");
        for texture in [
            &textures.color,
            &textures.normal,
            &textures.roughness,
            &textures.ao,
        ] {
            assert_eq!((texture.width, texture.height), (512, 512));
            assert_eq!(texture.pixels.len(), 512 * 512);
        }
        let textured = procedural_surface_color(
            SurfaceShader::RaceGrassTexture,
            Vec3::new(18.0, -0.8, 27.0),
            0,
            0,
            0.0,
            1.0,
            0.0,
        );
        assert_ne!(textured, pack([48, 126, 42], 1.0));
    }

    #[test]
    fn race_asphalt_variants_load_and_produce_distinct_surfaces() {
        let textures = race_asphalt_textures().expect("race asphalt textures must load");
        let maps = [
            &textures.clean_color,
            &textures.clean_height,
            &textures.asphalt04_color,
            &textures.asphalt04_height,
        ];
        for texture in maps {
            // Optimized color maps are 256 px while the authored height maps
            // retain 512 px of useful detail.
            assert!((256..=512).contains(&texture.width));
            assert_eq!(texture.width, texture.height);
        }
        assert!(maps.iter().any(|texture| texture.width == 512));
        let point = Vec3::new(108.0, 0.0, 207.0);
        let clean =
            procedural_surface_color(SurfaceShader::RaceAsphaltClean, point, 0, 0, 0.0, 1.0, 0.1);
        let asphalt04 =
            procedural_surface_color(SurfaceShader::RaceAsphalt04, point, 0, 0, 0.0, 1.0, 0.1);
        assert_ne!(clean, asphalt04);
    }

    #[test]
    fn ground030_loads_the_complete_optimized_texture_set() {
        let textures = ground_textures().expect("Ground030 textures must load");
        for texture in [
            &textures.color,
            &textures.normal,
            &textures.roughness,
            &textures.ao,
        ] {
            assert_eq!((texture.width, texture.height), (512, 512));
            assert_eq!(texture.pixels.len(), 512 * 512);
        }
        let textured = procedural_surface_color(
            SurfaceShader::RaceGroundTexture,
            Vec3::new(14.0, 8.0, 21.0),
            0,
            0,
            0.0,
            1.0,
            0.15,
        );
        assert_ne!(textured, pack([126, 112, 78], 1.0));
    }

    #[test]
    fn bark014_loads_the_complete_optimized_texture_set() {
        let textures = bark_textures().expect("Bark014 textures must load");
        for texture in [
            &textures.color,
            &textures.normal,
            &textures.roughness,
            &textures.ao,
        ] {
            assert_eq!((texture.width, texture.height), (512, 512));
            assert_eq!(texture.pixels.len(), 512 * 512);
        }
        let textured = procedural_surface_color(
            SurfaceShader::RaceBarkTexture,
            Vec3::new(2.0, 5.5, -1.25),
            0,
            0,
            0.0,
            1.0,
            0.1,
        );
        assert_ne!(textured, pack([91, 68, 45], 1.0));
    }

    #[test]
    fn bricks113a_loads_the_complete_optimized_texture_set() {
        let textures = bricks_textures().expect("Bricks113A textures must load");
        for texture in [
            &textures.color,
            &textures.normal,
            &textures.roughness,
            &textures.ao,
        ] {
            assert_eq!((texture.width, texture.height), (512, 512));
            assert_eq!(texture.pixels.len(), 512 * 512);
        }
        let textured = procedural_surface_color(
            SurfaceShader::RaceBricksTexture,
            Vec3::new(3.5, 7.0, 1.25),
            0,
            0,
            0.0,
            1.0,
            0.1,
        );
        assert_ne!(textured, pack([122, 112, 91], 1.0));
    }

    #[test]
    fn ground054_textures_water_planet_sand_and_grass005_textures_its_grass() {
        let textures = water_sand_textures().expect("Ground054 textures must load");
        for texture in [
            &textures.color,
            &textures.normal,
            &textures.roughness,
            &textures.ao,
        ] {
            assert_eq!((texture.width, texture.height), (512, 512));
            assert_eq!(texture.pixels.len(), 512 * 512);
        }

        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let detailed = load_obj(root.join("assets/planets/water_planet.obj")).unwrap();
        assert!(
            detailed
                .faces
                .iter()
                .any(|face| face.shader == SurfaceShader::WaterPlanetSandTexture)
        );
        assert!(
            detailed
                .faces
                .iter()
                .any(|face| face.shader == SurfaceShader::WaterPlanetGrassTexture)
        );
        assert!(detailed.faces.iter().any(|face| {
            face.material.to_ascii_lowercase().contains("oceano")
                && face.shader == SurfaceShader::Ocean
        }));

        let lod = make_planet_lod(WATER_PLANET_RADIUS, 6, 10, PlanetVisual::Water);
        assert!(
            lod.faces
                .iter()
                .any(|face| face.shader == SurfaceShader::WaterPlanetSandTexture)
        );
        assert!(
            lod.faces
                .iter()
                .any(|face| face.shader == SurfaceShader::WaterPlanetGrassTexture)
        );
    }

    #[test]
    fn asphalt031_textures_only_water_planet_stone_pillars() {
        let textures = asphalt_textures().expect("Asphalt031 textures must load");
        for texture in [
            &textures.color,
            &textures.normal,
            &textures.roughness,
            &textures.ao,
        ] {
            assert_eq!((texture.width, texture.height), (512, 512));
            assert_eq!(texture.pixels.len(), 512 * 512);
        }

        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let planet = load_obj(root.join("assets/planets/water_planet.obj")).unwrap();
        let pillars: Vec<_> = planet
            .faces
            .iter()
            .filter(|face| {
                let group = face.group.to_ascii_lowercase();
                group.starts_with("roca_") && group.contains("_pilar_")
            })
            .collect();
        assert!(!pillars.is_empty());
        assert!(
            pillars
                .iter()
                .all(|face| face.shader == SurfaceShader::WaterPlanetPillarTexture)
        );
        assert!(planet.faces.iter().any(|face| {
            face.group.to_ascii_lowercase().starts_with("roca_")
                && face.group.to_ascii_lowercase().contains("_cesped_")
                && face.shader == SurfaceShader::WaterPlanetGrassTexture
        }));
    }

    #[test]
    fn ground048_textures_the_earth_below_water_planet_grass() {
        let textures = water_earth_textures().expect("Ground048 textures must load");
        for texture in [
            &textures.color,
            &textures.normal,
            &textures.roughness,
            &textures.ao,
        ] {
            assert_eq!((texture.width, texture.height), (512, 512));
            assert_eq!(texture.pixels.len(), 512 * 512);
        }

        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let planet = load_obj(root.join("assets/planets/water_planet.obj")).unwrap();
        let earth_faces: Vec<_> = planet
            .faces
            .iter()
            .filter(|face| face.material.to_ascii_lowercase() == "03_|_tierra.002")
            .collect();
        assert!(!earth_faces.is_empty());
        assert!(
            earth_faces
                .iter()
                .all(|face| face.shader == SurfaceShader::WaterPlanetEarthTexture)
        );
        assert!(
            planet
                .faces
                .iter()
                .filter(|face| face.material.to_ascii_lowercase() == "04_|_cesped.002")
                .all(|face| face.shader == SurfaceShader::WaterPlanetGrassTexture)
        );
    }

    #[test]
    fn underwater_level_uses_asphalt_for_stone_and_ground054_for_sand() {
        assert!(asphalt_textures().is_some());
        assert!(water_sand_textures().is_some());

        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let level = load_obj(root.join("assets/levels/waterlevel_extendido.obj")).unwrap();
        for material in ["underwater_rock", "blue_stone", "deep_cave_rock"] {
            let faces: Vec<_> = level
                .faces
                .iter()
                .filter(|face| face.material.to_ascii_lowercase() == material)
                .collect();
            assert!(!faces.is_empty(), "missing stone material {material}");
            assert!(
                faces
                    .iter()
                    .all(|face| face.shader == SurfaceShader::WaterLevelRockTexture)
            );
        }
        let sand: Vec<_> = level
            .faces
            .iter()
            .filter(|face| face.material.to_ascii_lowercase() == "underwater_sand")
            .collect();
        assert!(!sand.is_empty());
        assert!(
            sand.iter()
                .all(|face| face.shader == SurfaceShader::WaterLevelSandTexture)
        );
    }

    #[test]
    fn guardian_alien_texture_sets_load_optimized() {
        let egg = alien_egg_textures().expect("AlienEgg textures must load");
        for texture in [
            &egg.color,
            &egg.normal,
            &egg.roughness,
            &egg.ao,
            &egg.emissive,
        ] {
            assert_eq!((texture.width, texture.height), (256, 256));
        }
        let tissue = alien_tissue_textures().expect("AlienSkinTissue textures must load");
        for texture in [&tissue.color, &tissue.normal, &tissue.roughness, &tissue.ao] {
            assert_eq!((texture.width, texture.height), (256, 256));
        }
        let veins = alien_vein_textures().expect("AlienVeinSkin textures must load");
        for texture in [
            &veins.color,
            &veins.normal,
            &veins.roughness,
            &veins.ao,
            &veins.emissive,
        ] {
            assert_eq!((texture.width, texture.height), (256, 256));
        }
    }

    #[test]
    fn spiky_planet_uses_vein_skin_on_sphere_and_boss_skin_on_tentacles() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let planet = load_obj(root.join("assets/planets/planeta_espinas.obj")).unwrap();
        let sphere: Vec<_> = planet
            .faces
            .iter()
            .filter(|face| face.material.eq_ignore_ascii_case("Rosa_cuerpo"))
            .collect();
        assert!(!sphere.is_empty());
        assert!(
            sphere
                .iter()
                .all(|face| { face.shader == SurfaceShader::CoreGuardianEnvironmentTexture })
        );

        let tentacle_materials = [
            "Fucsia",
            "Magenta_sombra",
            "Coral_claro",
            "Durazno",
            "Crema",
        ];
        let tentacles: Vec<_> = planet
            .faces
            .iter()
            .filter(|face| {
                tentacle_materials
                    .iter()
                    .any(|material| face.material.eq_ignore_ascii_case(material))
            })
            .collect();
        assert!(!tentacles.is_empty());
        assert!(
            tentacles
                .iter()
                .all(|face| { face.shader == SurfaceShader::WaterMonsterTissueTexture })
        );

        let lod = make_planet_lod(SPIKY_PLANET_RADIUS, 6, 10, PlanetVisual::Spiky);
        assert!(
            lod.faces
                .iter()
                .all(|face| { face.shader == SurfaceShader::CoreGuardianEnvironmentTexture })
        );
    }

    #[test]
    fn exterior_race_planet_textures_only_green_terrain_at_every_lod() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let detailed =
            load_obj(root.join("assets/planets/planeta_carreras_detallado.obj")).unwrap();
        assert!(detailed.faces.iter().any(|face| {
            face.group == "Race_Planet_Terrain"
                && face.shader == SurfaceShader::RacePlanetGrassTexture
        }));
        assert!(
            detailed
                .faces
                .iter()
                .filter(|face| {
                    face.group.contains("Track") || face.group.contains("Boost_Light")
                })
                .all(|face| face.shader != SurfaceShader::RacePlanetGrassTexture)
        );

        let lod = make_planet_lod(race_track::PLANET_RADIUS, 6, 10, PlanetVisual::Racing);
        assert!(
            lod.faces
                .iter()
                .any(|face| { face.shader == SurfaceShader::RacePlanetGrassTexture })
        );
        assert!(lod.faces.iter().any(|face| {
            face.material == "race_rgb_245_208_42"
                && face.shader != SurfaceShader::RacePlanetGrassTexture
        }));
    }

    #[test]
    fn shark_blue_armor_uses_metal_texture_without_touching_lights_or_teeth() {
        let model = load_obj(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("assets/enemies/acuaticenemies.obj"),
        )
        .unwrap();
        assert!(model.faces.iter().any(|face| {
            face.group.contains("ENEMY_SHARK_DRONE") && face.shader == SurfaceShader::SharkBlueMetal
        }));
        for face in model.faces.iter().filter(|face| {
            face.group.contains("ENEMY_SHARK_DRONE_Tooth")
                || face.group.contains("ENEMY_SHARK_DRONE_Eye")
                || face.group.contains("ENEMY_SHARK_DRONE_Engine_Glow")
        }) {
            assert_ne!(face.shader, SurfaceShader::SharkBlueMetal);
        }
    }

    #[test]
    fn unified_hud_draws_panels_bars_reticle_and_boss_health() {
        let mut frame = Framebuffer::new(WIDTH, HEIGHT);
        draw_unified_combat_hud(
            &mut frame,
            "DESTRUYE EL OBJETIVO",
            0.75,
            37_845,
            0.6,
            0.4,
            Some(("JEFE: DESTRUCTOR", 0.5)),
            (0.5, 0.5),
        );
        assert_eq!(frame.pixels[16 * WIDTH + 40], 0x46e6ff);
        assert_ne!(frame.pixels[400 * WIDTH + 600], 0);
        assert_ne!(frame.pixels[780 * WIDTH + 500], 0);
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

        let original_width = level
            .vertices
            .iter()
            .map(|vertex| vertex.y)
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), y| {
                (min.min(y), max.max(y))
            });
        stretch_water_level_width(&mut level, WATER_LEVEL_WIDTH_SCALE);
        let widened_width = level
            .vertices
            .iter()
            .map(|vertex| vertex.y)
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), y| {
                (min.min(y), max.max(y))
            });
        assert!((widened_width.1 - widened_width.0) > (original_width.1 - original_width.0) * 1.44);

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
        assert_eq!(state.enemies.len(), 44);
        assert!(state.enemies.last().unwrap().progress < WATER_LEVEL_LENGTH);
        assert_eq!(state.stage(), WaterStage::Entrance);
        assert_eq!(
            state
                .enemies
                .iter()
                .filter(|enemy| enemy.kind == 0 && (170.0..270.0).contains(&enemy.progress))
                .count(),
            16
        );
    }

    #[test]
    fn aquatic_ruins_progresses_through_six_stages_and_stops_for_boss() {
        let mut state = WaterLevelState::new();
        for (progress, expected) in [
            (20.0, WaterStage::Entrance),
            (100.0, WaterStage::Ruins),
            (200.0, WaterStage::Minefield),
            (300.0, WaterStage::SpeedTunnel),
            (400.0, WaterStage::OpenArena),
            (WATER_BOSS_ARENA_START, WaterStage::SharkBoss),
        ] {
            state.progress = progress;
            assert_eq!(state.stage(), expected);
        }

        let mut ship = Ship::new(&ARWING_SUBMARINE_SPEC);
        state.progress = WATER_BOSS_ARENA_START - 0.1;
        state.update(&mut ship, &ShipInput::default(), 1.0);
        assert_eq!(state.progress, WATER_BOSS_ARENA_START);
    }

    #[test]
    fn charged_shots_defeat_mecha_shark_and_open_exit() {
        let mut state = WaterLevelState::new();
        state.progress = WATER_BOSS_ARENA_START;
        let boss_position = state.boss_position();
        let mut ship = Ship::new(&ARWING_SUBMARINE_SPEC);
        ship.position = Vec3::new(-WATER_BOSS_ARENA_START, 2.5, -9.0);
        let charged_shots_needed = (WATER_BOSS_MAX_HEALTH + 2) / 3;
        let mut projectiles: Vec<Projectile> = (0..charged_shots_needed)
            .map(|_| Projectile {
                position: boss_position,
                direction: Vec3::new(-1.0, 0.0, 0.0),
                lifetime: 1.0,
                scale: 3.0,
            })
            .collect();
        resolve_water_enemy_collisions(&mut state, &mut ship, &mut projectiles);
        assert!(state.boss_defeated);
        assert_eq!(state.boss_health, 0);
        assert_eq!(state.boss_explosion_timer, WATER_BOSS_EXPLOSION_DURATION);

        state.update(&mut ship, &ShipInput::default(), 1.0);
        assert_eq!(state.progress, WATER_BOSS_ARENA_START);
        state.update(
            &mut ship,
            &ShipInput::default(),
            WATER_BOSS_EXPLOSION_DURATION,
        );
        assert!(state.progress <= WATER_REWARD_DOOR_PROGRESS - 4.0);
        state.update(
            &mut ship,
            &ShipInput::default(),
            WATER_REWARD_DOOR_OPEN_TIME,
        );
        state.update(&mut ship, &ShipInput::default(), 0.2);
        assert!(state.progress > WATER_BOSS_ARENA_START);
    }

    #[test]
    fn aquatic_enemies_hold_formations_and_fire_at_the_submarine() {
        let mut state = WaterLevelState::new();
        let first_group: Vec<Vec3> = state.enemies[..4]
            .iter()
            .map(|enemy| enemy.position(0.0))
            .collect();
        assert!(first_group[0].x > first_group[1].x);
        assert!(first_group[1].z < first_group[0].z);
        assert!(first_group[2].z > first_group[0].z);

        state.progress = 20.0;
        state.enemy_fire_timer = 0.0;
        let mut ship = Ship::new(&ARWING_SUBMARINE_SPEC);
        ship.position = Vec3::new(-20.0, 6.5, 0.0);
        state.update_enemy_attacks(&ship, 0.1);
        assert!(!state.enemy_projectiles.is_empty());
        assert!(
            state
                .enemy_projectiles
                .iter()
                .all(|shot| shot.direction.dot(shot.direction) > 0.99)
        );
    }

    #[test]
    fn mecha_shark_cycles_multiple_attacks_and_enrages_at_half_health() {
        let mut state = WaterLevelState::new();
        state.progress = WATER_BOSS_ARENA_START;
        let mut ship = Ship::new(&ARWING_SUBMARINE_SPEC);
        ship.position = Vec3::new(-WATER_BOSS_ARENA_START, 6.0, 0.0);
        let expected_shot_counts = [3, 7, 6, 5];

        for expected in expected_shot_counts {
            for shot in &mut state.enemy_projectiles {
                shot.lifetime = 0.0;
            }
            state.enemy_fire_timer = 0.0;
            state.update_enemy_attacks(&ship, 0.0);
            assert_eq!(
                state
                    .enemy_projectiles
                    .iter()
                    .filter(|shot| shot.lifetime > 0.0)
                    .count(),
                expected
            );
        }

        state.boss_health = WATER_BOSS_MAX_HEALTH / 2;
        state.enemy_fire_timer = 0.0;
        state.update_enemy_attacks(&ship, 0.0);
        assert!(state.enemy_fire_timer < 1.0);
    }

    #[test]
    fn hostile_projectiles_damage_the_submarine_hull() {
        let mut state = WaterLevelState::new();
        let mut ship = Ship::new(&ARWING_SUBMARINE_SPEC);
        ship.position = Vec3::new(-10.0, 6.0, 0.0);
        state.enemy_projectiles.push(Projectile {
            position: ship.position,
            direction: Vec3::new(1.0, 0.0, 0.0),
            lifetime: 1.0,
            scale: 0.6,
        });
        let mut player_shots = Vec::new();
        resolve_water_enemy_collisions(&mut state, &mut ship, &mut player_shots);
        assert_eq!(state.hull_health, WATER_SUBMARINE_MAX_HEALTH - 1);
        assert!(state.hit_invulnerability > 0.0);
        assert_eq!(state.enemy_projectiles[0].lifetime, 0.0);
    }

    #[test]
    fn colliding_with_an_aquatic_enemy_damages_the_ship_and_destroys_enemy() {
        let mut state = WaterLevelState::new();
        let enemy_position = state.enemies[0].position(state.elapsed);
        let mut ship = Ship::new(&ARWING_SUBMARINE_SPEC);
        ship.position = enemy_position;
        let mut player_shots = Vec::new();

        resolve_water_enemy_collisions(&mut state, &mut ship, &mut player_shots);

        assert_eq!(state.hull_health, WATER_SUBMARINE_MAX_HEALTH - 1);
        assert!(!state.enemies[0].alive);
        assert_eq!(state.defeated_enemies, 1);
        assert!(state.hit_invulnerability > 0.0);
    }

    #[test]
    fn colliding_with_the_mecha_shark_causes_heavy_contact_damage() {
        let mut state = WaterLevelState::new();
        state.progress = WATER_BOSS_ARENA_START;
        let mut ship = Ship::new(&ARWING_SUBMARINE_SPEC);
        ship.position = state.boss_position();
        let mut player_shots = Vec::new();

        resolve_water_enemy_collisions(&mut state, &mut ship, &mut player_shots);

        assert_eq!(state.hull_health, WATER_SUBMARINE_MAX_HEALTH - 2);
        assert!(state.hit_invulnerability >= 1.0);
    }

    #[test]
    fn four_star_reward_unlocks_the_final_exit() {
        let reward = make_four_star_dragon_ball();
        for star in 0..4 {
            assert_eq!(
                reward
                    .faces
                    .iter()
                    .filter(|face| face.group == format!("dragon_star_{star}"))
                    .count(),
                10
            );
        }

        let mut state = WaterLevelState::new();
        state.boss_defeated = true;
        state.reward_door_open = 1.0;
        state.progress = WATER_REWARD_PROGRESS;
        state.lateral = WATER_LEVEL_HALF_WIDTH;
        let mut ship = Ship::new(&ARWING_SUBMARINE_SPEC);
        state.update(&mut ship, &ShipInput::default(), 0.1);
        assert_eq!(state.progress, WATER_REWARD_PROGRESS);
        assert!(!state.dragon_ball_collected);
        assert_eq!(state.stage(), WaterStage::RewardChamber);

        state.lateral = 0.0;
        state.height = 7.2;
        state.update(&mut ship, &ShipInput::default(), 0.1);
        assert!(state.dragon_ball_collected);
        assert_eq!(state.mission_complete_timer, WATER_MISSION_COMPLETE_TIME);
        let frozen_position = ship.position;
        state.update(&mut ship, &ShipInput::default(), 1.0);
        assert_eq!(ship.speed, 0.0);
        assert_eq!(ship.position.x, frozen_position.x);
        assert_eq!(ship.position.y, frozen_position.y);
        assert_eq!(ship.position.z, frozen_position.z);
        assert_eq!(state.progress, WATER_REWARD_PROGRESS);
        state.update(
            &mut ship,
            &ShipInput::default(),
            WATER_MISSION_COMPLETE_TIME,
        );
        assert_eq!(state.mission_complete_timer, 0.0);
    }

    #[test]
    fn race_reward_is_a_one_star_dragon_ball() {
        let reward = make_one_star_dragon_ball();
        assert_eq!(
            reward
                .faces
                .iter()
                .filter(|face| face.group.starts_with("dragon_star_"))
                .count(),
            10
        );
        assert!(
            reward
                .faces
                .iter()
                .any(|face| face.group == "dragon_star_0")
        );
        assert!(
            !reward
                .faces
                .iter()
                .any(|face| face.group == "dragon_star_1")
        );
    }

    #[test]
    fn guardian_reward_is_a_three_star_dragon_ball() {
        let reward = make_three_star_dragon_ball();
        assert_eq!(
            reward
                .faces
                .iter()
                .filter(|face| face.group.starts_with("dragon_star_"))
                .count(),
            30
        );
        for star in 0..3 {
            assert!(
                reward
                    .faces
                    .iter()
                    .any(|face| face.group == format!("dragon_star_{star}"))
            );
        }
        assert!(
            !reward
                .faces
                .iter()
                .any(|face| face.group == "dragon_star_3")
        );
    }

    #[test]
    fn tower_reward_is_a_two_star_dragon_ball() {
        let reward = make_two_star_dragon_ball();
        assert_eq!(
            reward
                .faces
                .iter()
                .filter(|face| face.group.starts_with("dragon_star_"))
                .count(),
            20
        );
        assert!(
            reward
                .faces
                .iter()
                .any(|face| face.group == "dragon_star_0")
        );
        assert!(
            reward
                .faces
                .iter()
                .any(|face| face.group == "dragon_star_1")
        );
        assert!(
            !reward
                .faces
                .iter()
                .any(|face| face.group == "dragon_star_2")
        );
    }

    #[test]
    fn every_dragon_ball_uses_refractive_glass_without_texturing_the_stars() {
        for ball in [
            make_one_star_dragon_ball(),
            make_two_star_dragon_ball(),
            make_three_star_dragon_ball(),
            make_four_star_dragon_ball(),
            make_six_star_dragon_ball(),
            make_seven_star_dragon_ball(),
        ] {
            let shell: Vec<_> = ball
                .faces
                .iter()
                .filter(|face| face.group == "dragon_ball")
                .collect();
            assert!(!shell.is_empty());
            assert!(
                shell
                    .iter()
                    .all(|face| face.shader == SurfaceShader::DragonBallGlass && face.smooth)
            );
            assert!(
                ball.faces
                    .iter()
                    .filter(|face| face.group.starts_with("dragon_star_"))
                    .all(|face| face.shader != SurfaceShader::DragonBallGlass)
            );
        }

        let background = vec![0x204060; 16 * 16];
        let surface = 0xf48416;
        let refracted = refract_dragon_ball_color(
            surface,
            &background,
            16,
            16,
            8,
            8,
            Vec3::new(0.7, -0.3, 1.1),
            0.5,
            0.2,
        );
        assert_ne!(refracted, surface);
        assert_ne!(refracted, background[0]);
    }

    #[test]
    fn mission_complete_overlay_covers_the_gameplay_view() {
        let mut frame = Framebuffer::new(WIDTH, HEIGHT);
        frame.clear(0xffffff);
        draw_mission_complete_overlay(&mut frame, 2.0);
        assert_ne!(frame.pixels[100 * WIDTH + 100], 0xffffff);
        assert_eq!(frame.pixels[245 * WIDTH + 300], 0x46e6ff);
        assert_ne!(frame.pixels[400 * WIDTH + 600], 0);
    }

    #[test]
    fn reward_room_is_closed_until_the_boss_door_opens() {
        let chamber = make_reward_chamber();
        for group in [
            "reward_room_floor",
            "reward_room_ceiling",
            "reward_room_side_wall",
            "reward_room_back_wall",
            "reward_room_front_left",
            "reward_room_front_right",
            "reward_room_door_lintel",
        ] {
            assert!(chamber.faces.iter().any(|face| face.group == group));
        }
        let door = make_reward_door();
        assert!(door.faces.iter().any(|face| face.group == "reward_door"));

        let mut state = WaterLevelState::new();
        state.boss_defeated = true;
        state.progress = WATER_REWARD_DOOR_PROGRESS;
        let mut ship = Ship::new(&ARWING_SUBMARINE_SPEC);
        state.update(&mut ship, &ShipInput::default(), 0.1);
        assert_eq!(state.progress, WATER_REWARD_DOOR_PROGRESS - 4.0);
        assert_eq!(state.stage(), WaterStage::SharkBoss);

        state.update(
            &mut ship,
            &ShipInput::default(),
            WATER_REWARD_DOOR_OPEN_TIME,
        );
        assert!(state.reward_door_open >= WATER_REWARD_DOOR_PASSABLE);
        state.update(&mut ship, &ShipInput::default(), 0.2);
        assert!(state.progress > WATER_REWARD_DOOR_PROGRESS - 4.0);
        assert_eq!(state.stage(), WaterStage::RewardChamber);

        let mut misaligned = WaterLevelState::new();
        misaligned.boss_defeated = true;
        misaligned.reward_door_open = 1.0;
        misaligned.progress = WATER_REWARD_DOOR_PROGRESS - 2.0;
        misaligned.lateral = 10.0;
        misaligned.height = 10.5;
        let previous_lateral = misaligned.lateral;
        misaligned.update(&mut ship, &ShipInput::default(), 0.1);
        assert_eq!(misaligned.progress, WATER_REWARD_DOOR_PROGRESS - 4.0);
        assert!(misaligned.lateral.abs() < previous_lateral);
        assert!(misaligned.height < 10.5);
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
    fn detailed_arwing_car_has_four_wheels_smooth_panels_and_aligned_thrusters() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let car =
            load_obj(root.join("assets/vehicles/arwing/arwing2_carro_detallado.obj")).unwrap();
        assert!(car.vertices.len() > 1_900);
        assert!(car.faces.len() > 1_600);
        assert!(car.faces.iter().any(|face| face.smooth));
        let radius = model_radius(&car, model_center(&car)) * ARWING_CAR_SPEC.model_scale;
        assert!(
            (1.45..2.5).contains(&radius),
            "unexpected car radius {radius}"
        );

        for group in [
            "Arwing_Car_FrontWheel_L_Tire",
            "Arwing_Car_FrontWheel_R_Tire",
            "Arwing_Car_RearWheel_L_Tire",
            "Arwing_Car_RearWheel_R_Tire",
            "Arwing_Car_Core_L",
            "Arwing_Car_Core_R",
        ] {
            assert!(
                car.faces.iter().any(|face| face.group == group),
                "missing Arwing car group {group}"
            );
        }
        for material in [
            "ArwingCar_HullWhite",
            "ArwingCar_FinBlue",
            "ArwingCar_CockpitBlue",
            "ArwingCar_Tire",
            "ArwingCar_EngineCyan",
            "ArwingCar_LaserRed",
        ] {
            assert!(
                car.faces.iter().any(|face| face.material == material),
                "missing Arwing car material {material}"
            );
        }

        let mut points = car
            .faces
            .iter()
            .filter(|face| face.group == "Arwing_Car_Core_L")
            .flat_map(|face| face.indices)
            .map(|index| car.vertices[index]);
        let first = points.next().expect("missing Arwing car engine core");
        let (mut min, mut max) = (first, first);
        for point in points {
            min.x = min.x.min(point.x);
            min.y = min.y.min(point.y);
            min.z = min.z.min(point.z);
            max.x = max.x.max(point.x);
            max.y = max.y.max(point.y);
            max.z = max.z.max(point.z);
        }
        let core = min.add(max).mul(0.5).sub(model_center(&car));
        let actual = (
            -core.z * ARWING_CAR_SPEC.model_scale,
            core.x.abs() * ARWING_CAR_SPEC.model_scale,
            -core.y * ARWING_CAR_SPEC.model_scale,
        );
        assert!(
            (actual.0 - ARWING_CAR_SPEC.turbo_back).abs() < 0.02,
            "{actual:?}"
        );
        assert!(
            (actual.1 - ARWING_CAR_SPEC.turbo_side).abs() < 0.02,
            "{actual:?}"
        );
        assert!(
            (actual.2 - ARWING_CAR_SPEC.turbo_down).abs() < 0.02,
            "{actual:?}"
        );
    }

    #[test]
    fn detailed_kirby_car_has_face_four_wheels_star_and_aligned_thrusters() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let car = load_obj(root.join("assets/vehicles/kirby/kirby_carro_detallado.obj")).unwrap();
        assert!(car.vertices.len() > 13_000);
        assert!(car.faces.len() > 13_000);
        assert!(car.faces.iter().any(|face| face.smooth));
        let radius = model_radius(&car, model_center(&car)) * KIRBY_CAR_SPEC.model_scale;
        assert!(
            (1.5..2.5).contains(&radius),
            "unexpected Kirby car radius {radius}"
        );

        for group in [
            "Kirby_Car_Eye_L",
            "Kirby_Car_Eye_R",
            "Kirby_Car_HoodStar",
            "Kirby_Car_FrontWheel_L_Tire",
            "Kirby_Car_FrontWheel_R_Tire",
            "Kirby_Car_RearWheel_L_Tire",
            "Kirby_Car_RearWheel_R_Tire",
            "Kirby_Car_Core_L",
            "Kirby_Car_Core_R",
            "Kirby_Car_ExhaustStar",
            "Kirby_Car_Highlight_Roof",
            "Kirby_Car_InnerCabin",
            "Kirby_Car_Undertray",
            "Kirby_Car_FrontWheelWell_L",
            "Kirby_Car_RearWheelWell_R",
        ] {
            assert!(
                car.faces.iter().any(|face| face.group == group),
                "missing Kirby car group {group}"
            );
        }
        for material in [
            "KirbyCar_Pink",
            "KirbyCar_Red",
            "KirbyCar_EyeBlue",
            "KirbyCar_StarMetal",
            "KirbyCar_Tire",
            "KirbyCar_CyanGlow",
        ] {
            assert!(
                car.faces.iter().any(|face| face.material == material),
                "missing Kirby car material {material}"
            );
        }

        let mut points = car
            .faces
            .iter()
            .filter(|face| face.group == "Kirby_Car_Core_L")
            .flat_map(|face| face.indices)
            .map(|index| car.vertices[index]);
        let first = points.next().expect("missing Kirby car engine core");
        let (mut min, mut max) = (first, first);
        for point in points {
            min.x = min.x.min(point.x);
            min.y = min.y.min(point.y);
            min.z = min.z.min(point.z);
            max.x = max.x.max(point.x);
            max.y = max.y.max(point.y);
            max.z = max.z.max(point.z);
        }
        let core = min.add(max).mul(0.5).sub(model_center(&car));
        let actual = (
            -core.z * KIRBY_CAR_SPEC.model_scale,
            core.x.abs() * KIRBY_CAR_SPEC.model_scale,
            -core.y * KIRBY_CAR_SPEC.model_scale,
        );
        assert!(
            (actual.0 - KIRBY_CAR_SPEC.turbo_back).abs() < 0.02,
            "{actual:?}"
        );
        assert!(
            (actual.1 - KIRBY_CAR_SPEC.turbo_side).abs() < 0.02,
            "{actual:?}"
        );
        assert!(
            (actual.2 - KIRBY_CAR_SPEC.turbo_down).abs() < 0.02,
            "{actual:?}"
        );
    }

    #[test]
    fn race_cars_share_the_same_visual_length() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let cars = [
            (
                "assets/vehicles/arwing/arwing2_carro_detallado.obj",
                &ARWING_CAR_SPEC,
            ),
            (
                "assets/vehicles/kirby/kirby_carro_detallado.obj",
                &KIRBY_CAR_SPEC,
            ),
            (
                "assets/vehicles/astra/astra_carro_detallado.obj",
                &ASTRA_CAR_SPEC,
            ),
            (
                "assets/vehicles/cowboy/cb_carro_detallado.obj",
                &COWBOYB_CAR_SPEC,
            ),
            (
                "assets/vehicles/buzz/buzz_carro_detallado.obj",
                &BUZZ_CAR_SPEC,
            ),
            (
                "assets/vehicles/mcqueen/mcqueen_carro_detallado.obj",
                &MCQUEEN_CAR_SPEC,
            ),
        ];

        for (path, spec) in cars {
            let model = load_obj(root.join(path)).unwrap();
            let min = model
                .vertices
                .iter()
                .map(|vertex| vertex.z)
                .fold(f32::INFINITY, f32::min);
            let max = model
                .vertices
                .iter()
                .map(|vertex| vertex.z)
                .fold(f32::NEG_INFINITY, f32::max);
            let visual_length = (max - min) * spec.model_scale;
            assert!(
                (visual_length - 3.0).abs() < 0.001,
                "{} has visual length {visual_length}",
                spec.name
            );
        }
    }

    #[test]
    fn blender_sun_asset_has_plasma_corona_and_valid_scale() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let sun = load_obj(root.join("assets/planets/sol_detallado.obj")).unwrap();
        assert!(sun.vertices.len() > 3_500);
        assert!(sun.faces.len() > 7_000);
        let radius = model_radius(&sun, model_center(&sun));
        assert!(
            (55.0..75.0).contains(&radius),
            "unexpected sun radius {radius}"
        );
        for material in [
            "sun_hot_plasma",
            "sun_bright_cells",
            "sun_dark_cells",
            "sun_corona_flare",
        ] {
            assert!(
                sun.faces
                    .iter()
                    .any(|face| { face.material == material && face.shader == SurfaceShader::Sun })
            );
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

    #[test]
    fn nova_castle_planet_asset_loads_with_authored_materials() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let model = load_obj(root.join("assets/planets/planeta_castillo_lava.obj")).unwrap();
        assert!(model.vertices.len() > 7_000);
        assert!(model.faces.len() > 7_000);

        for material in [
            "Castle_Rock",
            "Castle_Lava",
            "Castle_Bricks",
            "Castle_Portal_Core",
            "Castle_Flag",
        ] {
            assert!(
                model.faces.iter().any(|face| face.material == material),
                "falta el material {material}"
            );
        }
        assert!(model.faces.iter().any(|face| {
            face.material == "Castle_Bricks"
                && face.shader == SurfaceShader::RaceBricksTexture
                && face.color == MaterialColor::Fixed([117, 110, 117])
        }));
        assert!(model.faces.iter().any(|face| {
            face.material == "Castle_Rock" && face.shader == SurfaceShader::CastleRockTexture
        }));
        assert!(model.faces.iter().any(|face| {
            face.material == "Castle_Lava" && face.shader == SurfaceShader::CastleLavaTexture
        }));
        assert!(model.faces.iter().any(|face| {
            face.material == "Castle_Portal_Core"
                && face.color == MaterialColor::Fixed([255, 46, 255])
                && face.shader == SurfaceShader::CastlePortalGlass
        }));
    }

    #[test]
    fn optimized_castle_planet_uses_reduced_close_and_medium_lods() {
        let original = load_obj("assets/planets/planeta_castillo_lava.obj").unwrap();
        let optimized = load_obj("assets/planets/planeta_castillo_lava_optimizado.obj").unwrap();
        let medium =
            load_obj("assets/planets/planeta_castillo_lava_optimizado_medium.obj").unwrap();
        assert!(optimized.faces.len() < original.faces.len());
        assert!(medium.faces.len() < optimized.faces.len() / 2);
        for material in ["Castle_Lava", "Castle_Bricks", "Castle_Portal_Core"] {
            assert!(optimized.faces.iter().any(|face| face.material == material));
        }
        let planet = VesperPlanet::load(
            "assets/planets/planeta_castillo_lava_optimizado.obj",
            Vec3::default(),
            58.0,
            0.0,
            PlanetVisual::Castle,
        )
        .unwrap();
        assert!(planet.medium.is_some());
    }

    #[test]
    fn castle_lava_loads_and_animates_the_complete_png_texture_set() {
        let textures = lava_textures().expect("Lava005 PNG textures must load");
        for texture in [
            &textures.color,
            &textures.normal,
            &textures.roughness,
            &textures.emission,
            &textures.displacement,
        ] {
            assert_eq!((texture.width, texture.height), (512, 512));
            assert_eq!(texture.pixels.len(), 512 * 512);
        }
        let point = Vec3::new(3.2, -1.1, 4.7);
        let first = procedural_surface_color(
            SurfaceShader::CastleLavaTexture,
            point,
            0,
            0,
            0.0,
            1.0,
            0.15,
        );
        let animated = procedural_surface_color(
            SurfaceShader::CastleLavaTexture,
            point,
            0,
            0,
            1.0,
            1.0,
            0.15,
        );
        assert_ne!(first, animated);
    }
}
