mod framebuffer;
mod input;
mod menu;
mod obj_loader;
mod ship;
mod triangle;

use std::collections::HashMap;
use std::f32::consts::{PI, TAU};
use std::time::Instant;

use framebuffer::Framebuffer;
use minifb::{Key, ScaleMode, Window, WindowOptions};
use obj_loader::{load_obj, Face, Model, Vec3};
use ship::{
    Projectile, Ship, ShipController, ShipInput, ShipSpec, ARWING_SPEC, ARWING_SUBMARINE_SPEC,
    ASTRA_SPEC, ASTRA_SUBMARINE_SPEC, BUZZ_SPEC, COWBOYB_SPEC, COWBOYB_SUBMARINE_SPEC, KIRBY_SPEC,
    KIRBY_SUBMARINE_SPEC,
};
use triangle::{draw_triangle, draw_triangle_shaded, ScreenVertex};

const WIDTH: usize = 1200;
const HEIGHT: usize = 800;
const NEAR_PLANE: f32 = 0.2;
const LIGHT_SPEED: f32 = 600.0;
const PLANET_ORBIT_SPEED: f32 = 0.02;
const SUN_POSITION_X: f32 = -520.0;
const PLANET_ORBIT_RADIUS: f32 = 280.0;
const WATER_PLANET_RADIUS: f32 = 24.0;
const COLLISION_MARGIN: f32 = 4.0;
const AVOIDANCE_DISTANCE: f32 = 52.0;
const ORBIT_LIMIT_RADIUS: f32 = 60.0;
const ORBIT_FLIGHT_ALTITUDE: f32 = 8.0;
const WATER_LEVEL_LENGTH: f32 = 175.0;
const WATER_LEVEL_HALF_WIDTH: f32 = 7.6;
const WATER_LEVEL_SOFT_BORDER: f32 = 6.2;
const WATER_LEVEL_BORDER_STRENGTH: f32 = 26.0;
const WATER_LEVEL_MIN_HEIGHT: f32 = 2.4;
const WATER_LEVEL_MAX_HEIGHT: f32 = 11.5;
const WATER_LEVEL_VERTICAL_OFFSET: f32 = 5.2;
const WATER_LEVEL_SOURCE_START: f32 = 90.0;
const WATER_ENEMY_SCALE: f32 = 1.05;

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

struct LevelCollider {
    min: Vec3,
    max: Vec3,
}

#[derive(Clone, Copy, PartialEq)]
enum GameMode {
    FreeWorld,
    WaterLevel,
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
        self.particles.clear();
        self.timer = 0.0;
    }

    fn update(&mut self, ship: &Ship, delta_time: f32) {
        self.timer -= delta_time;
        if self.timer <= 0.0 {
            let (forward, right, up) = ship.basis();
            for side in ship.spec.turbo_sides {
                let random = hash(self.serial.wrapping_mul(977).wrapping_add(17));
                let jitter = random as f32 / u32::MAX as f32 - 0.5;
                self.particles.push(BubbleParticle {
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

        for bubble in &mut self.particles {
            bubble.position = bubble.position.add(bubble.velocity.mul(delta_time));
            bubble.velocity = bubble.velocity.add(Vec3::new(0.0, 0.45 * delta_time, 0.0));
            bubble.lifetime -= delta_time;
            bubble.scale += delta_time * 0.035;
        }
        self.particles.retain(|bubble| bubble.lifetime > 0.0);
    }
}

impl WaterLevelState {
    fn new() -> Self {
        let enemies = (0..12)
            .map(|index| WaterEnemy {
                kind: index % 3,
                progress: 18.0 + index as f32 * 13.0,
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
        self.progress += ship.speed * delta_time;
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
        ship.orbit_axis = None;
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

    for face in model.faces.iter().filter(|face| face.group == group_name) {
        let mut indices = [0_usize; 3];
        for (target, source) in indices.iter_mut().zip(face.indices) {
            *target = *index_map.entry(source).or_insert_with(|| {
                let index = vertices.len();
                vertices.push(model.vertices[source]);
                index
            });
        }
        faces.push(Face {
            indices,
            material: face.material.clone(),
            group: face.group.clone(),
        });
    }

    if faces.is_empty() {
        return Err(format!("No se encontro el grupo {group_name}"));
    }
    Ok(Model { vertices, faces })
}

fn stretch_model_group_height(model: &mut Model, group_name: &str, base: f32, factor: f32) {
    let mut affected = vec![false; model.vertices.len()];
    for face in model.faces.iter().filter(|face| face.group == group_name) {
        for index in face.indices {
            affected[index] = true;
        }
    }
    for (vertex, should_stretch) in model.vertices.iter_mut().zip(affected) {
        if should_stretch {
            vertex.z = base + (vertex.z - base) * factor;
        }
    }
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
    colliders: &[LevelCollider],
) {
    for collider in colliders {
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
    projectiles: &mut Vec<Projectile>,
) {
    for projectile in projectiles.iter_mut().filter(|shot| shot.lifetime > 0.0) {
        for enemy in state.enemies.iter_mut().filter(|enemy| enemy.alive) {
            let offset = projectile.position.sub(enemy.position(state.elapsed));
            let hit_radius = enemy.collision_radius() + projectile.scale * 0.35;
            if offset.dot(offset) <= hit_radius * hit_radius {
                enemy.alive = false;
                projectile.lifetime = 0.0;
                state.defeated_enemies += 1;
                break;
            }
        }
    }

    for enemy in state.enemies.iter().filter(|enemy| enemy.alive) {
        let enemy_position = enemy.position(state.elapsed);
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
    projectiles.retain(|projectile| projectile.lifetime > 0.0);
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

#[derive(Clone, Copy, PartialEq)]
enum SurfaceShader {
    Ocean,
    Waterfall,
    Earth,
    Grass,
    Rock,
    DarkRock,
    Sand,
    Wood,
    Leaves,
    YellowBlock,
    WhiteMark,
    Flat,
}

fn surface_shader(material: &str) -> SurfaceShader {
    let name = material.to_ascii_lowercase();
    if name.contains("oceano") {
        SurfaceShader::Ocean
    } else if name.contains("cascada") {
        SurfaceShader::Waterfall
    } else if name.contains("roca_oscura") || name.contains("deep_cave_rock") {
        SurfaceShader::DarkRock
    } else if name.contains("roca")
        || name.contains("underwater_rock")
        || name.contains("blue_stone")
    {
        SurfaceShader::Rock
    } else if name.contains("tierra") {
        SurfaceShader::Earth
    } else if name.contains("cesped") || name.contains("seaweed") {
        SurfaceShader::Grass
    } else if name.contains("arena") || name.contains("underwater_sand") {
        SurfaceShader::Sand
    } else if name.contains("tronco") {
        SurfaceShader::Wood
    } else if name.contains("hojas") {
        SurfaceShader::Leaves
    } else if name.contains("bloque_amarillo") {
        SurfaceShader::YellowBlock
    } else if name.contains("interrogacion_blanca") {
        SurfaceShader::WhiteMark
    } else {
        SurfaceShader::Flat
    }
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

fn color_for_material(name: &str, effect_spec: Option<&ShipSpec>) -> [u8; 3] {
    let name = name.to_ascii_lowercase();
    if name.starts_with("buzz_") {
        return match name.as_str() {
            "buzz_white" => [238, 242, 248],
            "buzz_green" => [153, 226, 38],
            "buzz_purple" => [116, 61, 177],
            "buzz_red" => [235, 46, 60],
            "buzz_glass" => [80, 177, 200],
            "buzz_dark" => [29, 36, 54],
            "buzz_blue" => [45, 132, 240],
            "buzz_gold" => [245, 190, 42],
            _ => [238, 242, 248],
        };
    }
    if name.contains("cyan_coral") || name.contains("crystal_cyan") {
        [38, 220, 226]
    } else if name.contains("orange_coral") {
        [244, 105, 42]
    } else if name.contains("pink_coral") {
        [242, 89, 164]
    } else if name.contains("purple_coral") {
        [144, 73, 205]
    } else if name.contains("bright_seaweed") {
        [50, 184, 103]
    } else if name.contains("seaweed_green") {
        [20, 108, 72]
    } else if name.contains("spike_orange") {
        [239, 113, 38]
    } else if name.contains("spike_yellow") || name.contains("yellow_glow") {
        [250, 214, 54]
    } else if name.contains("shark_blue") {
        [42, 104, 150]
    } else if name.contains("shark_light") {
        [138, 191, 203]
    } else if name.contains("jelly_purple") {
        [125, 58, 188]
    } else if name.contains("jelly_pink") {
        [241, 91, 178]
    } else if name.contains("cyan_glow") {
        [40, 225, 242]
    } else if name.contains("red_glow") {
        [255, 42, 46]
    } else if name.contains("laser_core") {
        effect_spec.map_or([255, 245, 190], |spec| spec.laser_core_color)
    } else if name.contains("bubble_submarine") {
        [155, 232, 255]
    } else if name.contains("laser") {
        effect_spec.map_or([255, 48, 38], |spec| spec.laser_color)
    } else if name.contains("turbo_core") {
        effect_spec.map_or([220, 252, 255], |spec| spec.turbo_core_color)
    } else if name.contains("turbo") {
        effect_spec.map_or([36, 142, 255], |spec| spec.turbo_color)
    } else if name.contains("luz_cian") {
        [40, 205, 255]
    } else if name.contains("azul_profundo") {
        [18, 44, 98]
    } else if name.contains("azul_cobalto") {
        [34, 106, 190]
    } else if name.contains("soft_pink") {
        [255, 158, 198]
    } else if name.contains("light_pink") || name.contains("rosy_cheeks") {
        [255, 105, 158]
    } else if name.contains("burgundy") {
        [92, 22, 48]
    } else if name.contains("golden") || name.contains("gold") {
        [236, 176, 42]
    } else if name.contains("coral_rojo") {
        [238, 42, 58]
    } else if name.contains("coral_morado") {
        [151, 46, 205]
    } else if name.contains("coral_amarillo") {
        [245, 174, 42]
    } else if name.contains("planta_marina") {
        [18, 116, 76]
    } else if name.contains("ruina") {
        [84, 112, 108]
    } else if name.contains("energy_submarina") {
        [30, 220, 240]
    } else if name.contains("sun") {
        [255, 174, 48]
    } else if name.contains("tierra") {
        [112, 72, 42]
    } else if name.contains("arena") {
        [218, 188, 112]
    } else if name.contains("cesped") {
        [55, 142, 72]
    } else if name.contains("planet_land") {
        [62, 153, 91]
    } else if name.contains("planet_ice") {
        [180, 225, 238]
    } else if name.contains("planet") || name.contains("blue") {
        [47, 87, 210]
    } else if name.contains("glass") || name.contains("vidrio_oscuro") {
        [24, 60, 78]
    } else if name.contains("dark") || name.contains("gris_oscuro") {
        [43, 51, 64]
    } else if name.contains("yellow") || name.contains("amarillo") {
        [242, 197, 57]
    } else if name.contains("red") || name.contains("rojo") {
        [199, 49, 57]
    } else if name.contains("metal") {
        [103, 121, 143]
    } else {
        [220, 230, 238]
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
            let wave = (wave_a + wave_b) * 0.5;
            let sparkle = (x as f32 * 0.07 + y as f32 * 0.04 + time * 3.2)
                .sin()
                .max(0.0)
                .powf(18.0);
            (
                5.0 + fresnel * 32.0 + sparkle * 115.0,
                72.0 + wave * 18.0 + fresnel * 80.0 + sparkle * 95.0,
                142.0 + wave * 22.0 + fresnel * 105.0 + sparkle * 70.0,
                0.88 + wave * 0.12,
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
        SurfaceShader::Flat => (220.0, 230.0, 238.0, 1.0),
    };

    let scale = brightness * light;
    ((red * scale).clamp(0.0, 255.0) as u32) << 16
        | ((green * scale).clamp(0.0, 255.0) as u32) << 8
        | (blue * scale).clamp(0.0, 255.0) as u32
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
            faces.push(Face {
                indices: [first, second, first + 1],
                material: material.to_string(),
                group: if sun { "sun" } else { "planet" }.to_string(),
            });
            faces.push(Face {
                indices: [first + 1, second, second + 1],
                material: material.to_string(),
                group: if sun { "sun" } else { "planet" }.to_string(),
            });
        }
    }
    Model { vertices, faces }
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
        faces.push(Face {
            indices: [0, segment + 1, next + 1],
            material: if segment % 2 == 0 {
                "turbo_core"
            } else {
                "turbo_glow"
            }
            .to_string(),
            group: "turbo".to_string(),
        });
    }
    Model { vertices, faces }
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
        faces.push(Face {
            indices: [0, current, next],
            material: material.to_string(),
            group: "laser".to_string(),
        });
        faces.push(Face {
            indices: [1, next, current],
            material: material.to_string(),
            group: "laser".to_string(),
        });
    }
    Model { vertices, faces }
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
        .map(|indices| Face {
            indices,
            material: "bubble_submarine".to_string(),
            group: "bubble_submarine".to_string(),
        })
        .collect();
    Model { vertices, faces }
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

fn place_ship_near_water_planet(controller: &mut ShipController, world_time: f32) {
    controller.reset();
    let ship = &mut controller.ship;
    ship.position = water_planet_position(world_time).add(Vec3::new(45.0, 8.0, 0.0));
    ship.pitch = -0.08;
    ship.speed = ship.spec.cruise_speed;
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
            ship.orbit_axis = Some(normal);
            ship.orbit_center = Some(planet_center);
        } else if distance > AVOIDANCE_DISTANCE + 8.0 || ship.orbit_release_active {
            ship.orbit_axis = None;
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

        let (sin_rotation, cos_rotation) = planet_rotation.sin_cos();
        for collider in object_colliders {
            let local = collider.center;
            let collider_center = Vec3::new(
                local.x * cos_rotation - local.z * sin_rotation,
                local.y,
                local.x * sin_rotation + local.z * cos_rotation,
            )
            .add(planet_center);
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

fn render_model<F>(
    model: &Model,
    framebuffer: &mut Framebuffer,
    camera: &Camera,
    transform: F,
    water_time: Option<f32>,
    effect_spec: Option<&ShipSpec>,
) where
    F: Fn(Vec3) -> Vec3,
{
    let camera_vertices: Vec<Vec3> = model
        .vertices
        .iter()
        .map(|vertex| {
            let relative = transform(*vertex).sub(camera.position);
            Vec3::new(
                relative.dot(camera.right),
                relative.dot(camera.up),
                relative.dot(camera.forward),
            )
        })
        .collect();
    let focal = HEIGHT as f32 / (60.0_f32.to_radians() * 0.5).tan() * 0.5;
    let light = Vec3::new(-0.3, 0.8, -0.5).normalize();

    for face in &model.faces {
        let [i0, i1, i2] = face.indices;
        let v0 = camera_vertices[i0];
        let v1 = camera_vertices[i1];
        let v2 = camera_vertices[i2];
        if v0.z <= NEAR_PLANE || v1.z <= NEAR_PLANE || v2.z <= NEAR_PLANE {
            continue;
        }
        let project = |vertex: Vec3| ScreenVertex {
            x: WIDTH as f32 * 0.5 + vertex.x * focal / vertex.z,
            y: HEIGHT as f32 * 0.5 - vertex.y * focal / vertex.z,
            z: 1.0 / vertex.z,
        };
        let normal = v1.sub(v0).cross(v2.sub(v0)).normalize();
        let brightness = 0.34 + normal.dot(light).abs() * 0.66;
        let a = project(v0);
        let b = project(v1);
        let c = project(v2);
        let shader = surface_shader(&face.material);

        if shader != SurfaceShader::Flat {
            let p0 = model.vertices[i0];
            let p1 = model.vertices[i1];
            let p2 = model.vertices[i2];
            let center = v0.add(v1).add(v2).mul(1.0 / 3.0);
            let view_direction = center.mul(-1.0).normalize();
            let fresnel = (1.0 - normal.dot(view_direction).abs()).powf(3.0);
            let time = water_time.unwrap_or_default();

            draw_triangle_shaded(framebuffer, a, b, c, |w0, w1, w2, x, y| {
                let point = p0.mul(w0).add(p1.mul(w1)).add(p2.mul(w2));
                procedural_surface_color(shader, point, x, y, time, brightness, fresnel)
            });
        } else {
            draw_triangle(
                framebuffer,
                a,
                b,
                c,
                pack(color_for_material(&face.material, effect_spec), brightness),
            );
        }
    }
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
    sun: &Model,
    framebuffer: &mut Framebuffer,
    ship: &Ship,
    world_time: f32,
    travel: f32,
    turbo_power: f32,
    fire_charge: f32,
) {
    framebuffer.clear(0);
    draw_skybox(framebuffer, travel);
    let camera = chase_camera(ship);
    let sun_position = Vec3::new(SUN_POSITION_X, 20.0, 0.0);
    let planet_position = water_planet_position(world_time);
    let planet_rotation = world_time * 0.25;

    render_model(
        sun,
        framebuffer,
        &camera,
        |vertex| vertex.add(sun_position),
        None,
        None,
    );
    render_model(
        planet,
        framebuffer,
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

    render_ship_components(
        ship_model,
        ship_center,
        turbo_plume,
        laser_bolt,
        projectiles,
        framebuffer,
        &camera,
        ship,
        world_time,
        turbo_power,
        fire_charge,
        true,
    );
}

fn render_ship_components(
    ship_model: &Model,
    ship_center: Vec3,
    turbo_plume: &Model,
    laser_bolt: &Model,
    projectiles: &[Projectile],
    framebuffer: &mut Framebuffer,
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
            render_model(
                turbo_plume,
                framebuffer,
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
    for projectile in projectiles {
        let mut laser_right = projectile
            .direction
            .cross(Vec3::new(0.0, 1.0, 0.0))
            .normalize();
        if laser_right.dot(laser_right) < 0.001 {
            laser_right = Vec3::new(1.0, 0.0, 0.0);
        }
        let laser_up = laser_right.cross(projectile.direction).normalize();
        render_model(
            laser_bolt,
            framebuffer,
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
        render_model(
            laser_bolt,
            framebuffer,
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
    render_model(
        ship_model,
        framebuffer,
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

    for y in 0..framebuffer.height {
        let depth = y as f32 / height;
        let surface_light = (1.0 - depth).powf(1.65);
        let vertical_darkness = 0.50 + surface_light * 0.34;

        for x in 0..framebuffer.width {
            let normalized_x = x as f32 / width * 2.0 - 1.0;
            let edge_darkness = (1.0 - normalized_x * normalized_x * 0.30).clamp(0.62, 1.0);

            // Un haz ancho que nace en la superficie y se abre al descender.
            let swaying_center = (time * 0.22).sin() * 0.16;
            let cone_width = 0.12 + depth * 0.62;
            let cone = (1.0 - (normalized_x - swaying_center).abs() / cone_width).clamp(0.0, 1.0)
                * (1.0 - depth * 0.72);

            // Causticas suaves: se notan arriba y desaparecen en la profundidad.
            let caustic = (normalized_x * 18.0 + depth * 11.0 + time * 1.7)
                .sin()
                .max(0.0)
                .powf(5.0)
                * surface_light
                * 0.13;
            let light =
                (vertical_darkness * edge_darkness + cone * 0.22 + caustic).clamp(0.30, 1.0);

            let pixel = framebuffer.pixels[y * framebuffer.width + x];
            let red = ((pixel >> 16) & 255) as f32;
            let green = ((pixel >> 8) & 255) as f32;
            let blue = (pixel & 255) as f32;

            // El agua absorbe primero los rojos y conserva mas azul/cian.
            let red = (red * light * 0.67).clamp(0.0, 255.0) as u32;
            let green = (green * light * 0.91 + cone * 5.0).clamp(0.0, 255.0) as u32;
            let blue = (blue * light + surface_light * 4.0 + cone * 8.0).clamp(0.0, 255.0) as u32;
            framebuffer.pixels[y * framebuffer.width + x] = (red << 16) | (green << 8) | blue;
        }
    }
}

fn render_water_enemies(
    enemy_models: &[Model; 3],
    enemy_centers: &[Vec3; 3],
    enemies: &[WaterEnemy],
    elapsed: f32,
    current_progress: f32,
    framebuffer: &mut Framebuffer,
    camera: &Camera,
) {
    for enemy in enemies
        .iter()
        .filter(|enemy| enemy.alive && (enemy.progress - current_progress).abs() < 150.0)
    {
        let position = enemy.position(elapsed);
        let angle = match enemy.kind {
            0 => elapsed * 0.55 + enemy.phase,
            1 => (elapsed * 1.15 + enemy.phase).cos() * 0.18,
            _ => (elapsed * 0.45 + enemy.phase).sin() * 0.12,
        };
        let forward = Vec3::new(angle.cos(), 0.0, angle.sin());
        let right = Vec3::new(-angle.sin(), 0.0, angle.cos());
        let up = Vec3::new(0.0, 1.0, 0.0);
        render_model(
            &enemy_models[enemy.kind],
            framebuffer,
            camera,
            |vertex| {
                let local = vertex.sub(enemy_centers[enemy.kind]).mul(WATER_ENEMY_SCALE);
                position
                    .add(forward.mul(local.x))
                    .add(right.mul(local.y))
                    .add(up.mul(local.z))
            },
            None,
            None,
        );
    }
}

fn render_water_level(
    level: &Model,
    enemy_models: &[Model; 3],
    enemy_centers: &[Vec3; 3],
    enemies: &[WaterEnemy],
    enemy_elapsed: f32,
    bubble_model: &Model,
    bubbles: &[BubbleParticle],
    ship_model: &Model,
    ship_center: Vec3,
    turbo_plume: &Model,
    laser_bolt: &Model,
    projectiles: &[Projectile],
    framebuffer: &mut Framebuffer,
    ship: &Ship,
    world_time: f32,
    progress: f32,
    turbo_power: f32,
    fire_charge: f32,
) {
    framebuffer.clear(0);
    draw_underwater_background(framebuffer, progress * 0.002);
    let camera = chase_camera(ship);
    render_model(
        level,
        framebuffer,
        &camera,
        level_to_world,
        Some(world_time),
        None,
    );
    render_water_enemies(
        enemy_models,
        enemy_centers,
        enemies,
        enemy_elapsed,
        progress,
        framebuffer,
        &camera,
    );
    for bubble in bubbles {
        render_model(
            bubble_model,
            framebuffer,
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
        framebuffer,
        &camera,
        ship,
        world_time,
        turbo_power,
        fire_charge,
        false,
    );
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

fn select_ship(
    window: &mut Window,
    framebuffer: &mut Framebuffer,
    choices: &[(&Model, &ShipSpec)],
    buzz_variants: &[Model; 3],
) -> Result<Option<usize>, minifb::Error> {
    crate::input::clear();
    window.update();
    let mut selected = 0_usize;
    let mut variant = 0;
    let start = Instant::now();
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
            variant = (variant + 1) % 3;
        }
        if crate::input::pressed(Key::Enter) {
            return Ok(Some(selected));
        }
        window.set_title("Hangar | A/D: nave | V: formas de Buzz | Enter: jugar | Esc: menu");
        framebuffer.clear(0);
        draw_skybox(framebuffer, 0.0);
        draw_selection_label(framebuffer, "ELIGE TU NAVE", 600, 35, 4, 0x99e226);
        for (i, (_, spec)) in choices.iter().enumerate() {
            let y = 120 + i * 94;
            draw_selection_panel(framebuffer, 40, y, 420, y + 76, i == selected);
            draw_selection_label(
                framebuffer,
                &spec.name.to_ascii_uppercase(),
                230,
                y as i32 + 27,
                3,
                if i == selected { 0x99e226 } else { 0xa9bbd2 },
            );
        }
        draw_selection_panel(framebuffer, 450, 120, 1160, 650, true);
        let (base_model, spec) = choices[selected];
        let model = if selected == 4 {
            &buzz_variants[variant]
        } else {
            base_model
        };
        let center = model_center(model);
        let forward = Vec3::new(0.0, -0.13, 1.0).normalize();
        let right = forward.cross(Vec3::new(0.0, 1.0, 0.0)).normalize();
        let camera = Camera {
            position: Vec3::new(0.0, 2.0, -15.0),
            forward,
            right,
            up: right.cross(forward).normalize(),
        };
        let angle = start.elapsed().as_secs_f32() * 0.50;
        let heading = Vec3::new(angle.sin(), 0.0, -angle.cos());
        let side = Vec3::new(angle.cos(), 0.0, angle.sin());
        let scale = 3.4 / model_radius(model, center).max(0.001);
        render_model(
            model,
            framebuffer,
            &camera,
            |v| {
                Vec3::new(-3.8, 0.0, 0.0).add(spec.model_to_world(
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
            805,
            155,
            3,
            0xf0f3fa,
        );
        if selected == 4 {
            let names = ["NAVE", "CARRO", "SUBMARINO"];
            draw_selection_label(framebuffer, names[variant], 805, 560, 3, 0x99e226);
            draw_selection_label(
                framebuffer,
                "V: VER OTRA TRANSFORMACION",
                805,
                610,
                2,
                0xb7a1df,
            );
        }
        draw_selection_label(
            framebuffer,
            "A / D: ELEGIR    ENTER: JUGAR    ESC: MENU",
            600,
            695,
            2,
            0xd5deec,
        );
        draw_selection_label(
            framebuffer,
            "BUZZ: T CAMBIA NAVE / CARRO - SUBMARINO AUTOMATICO",
            600,
            745,
            2,
            0xb7a1df,
        );
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
    let arwing_path = "assets/vehicles/nave.obj";
    let kirby_path = "assets/vehicles/kirby.obj";
    let astra_path = "assets/vehicles/nave_astra.obj";
    let cowboyb_path = "assets/vehicles/nave_cowboyb.obj";
    let astra_submarine_path = "assets/vehicles/astra_submarino.obj";
    let cowboyb_submarine_path = "assets/vehicles/cb_submarino.obj";
    let arwing_submarine_path = "assets/vehicles/arwingsub.obj";
    let kirby_submarine_path = "assets/vehicles/kirbysub.obj";
    let arwing_model =
        load_obj(arwing_path).map_err(|error| format!("Error al cargar {arwing_path}: {error}"))?;
    let kirby_model =
        load_obj(kirby_path).map_err(|error| format!("Error al cargar {kirby_path}: {error}"))?;
    let astra_model =
        load_obj(astra_path).map_err(|error| format!("Error al cargar {astra_path}: {error}"))?;
    let cowboyb_model = load_obj(cowboyb_path)
        .map_err(|error| format!("Error al cargar {cowboyb_path}: {error}"))?;
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
        load_obj("assets/vehicles/buzz_nave.obj")?,
        load_obj("assets/vehicles/buzz_carro.obj")?,
        load_obj("assets/vehicles/buzz_submarino.obj")?,
    ];
    let choices = [
        (&arwing_model, &ARWING_SPEC),
        (&kirby_model, &KIRBY_SPEC),
        (&astra_model, &ASTRA_SPEC),
        (&cowboyb_model, &COWBOYB_SPEC),
        (&buzz_variants[0], &BUZZ_SPEC),
    ];
    let selected_ship = loop {
        if menu::show(&mut window, &mut framebuffer, false)? == menu::Action::Quit {
            return Ok(false);
        }
        if let Some(index) = select_ship(&mut window, &mut framebuffer, &choices, &buzz_variants)? {
            break index;
        }
        if !window.is_open() {
            return Ok(false);
        }
    };
    let (ship_model, ship_spec) = choices[selected_ship];
    let base_center = model_center(ship_model);
    let buzz_centers = [
        model_center(&buzz_variants[0]),
        model_center(&buzz_variants[1]),
        model_center(&buzz_variants[2]),
    ];
    let mut car_mode = false;
    let (level_ship_model, level_ship_spec) = match selected_ship {
        0 => (&arwing_submarine_model, &ARWING_SUBMARINE_SPEC),
        1 => (&kirby_submarine_model, &KIRBY_SUBMARINE_SPEC),
        2 => (&astra_submarine_model, &ASTRA_SUBMARINE_SPEC),
        3 => (&cowboyb_submarine_model, &COWBOYB_SUBMARINE_SPEC),
        4 => (&buzz_variants[2], &BUZZ_SPEC),
        _ => unreachable!(),
    };
    let level_ship_center = model_center(level_ship_model);
    window.set_title("Sistema solar | Vuelo libre 3D | F11: fullscreen");

    let planet_path = "assets/planets/water_planet.obj";
    let planet =
        load_obj(planet_path).map_err(|error| format!("Error al cargar {planet_path}: {error}"))?;
    let water_level_path = "assets/levels/waterlevel.obj";
    let mut water_level = load_obj(water_level_path)
        .map_err(|error| format!("Error al cargar {water_level_path}: {error}"))?;
    water_level
        .faces
        .retain(|face| face.group != "CAVES_AND_ARCHES");
    stretch_model_group_height(&mut water_level, "TERRAIN_WALLS", -5.0, 2.4);
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
    let level_colliders = build_level_colliders(&water_level);
    let planet_center = model_center(&planet);
    let planet_scale = WATER_PLANET_RADIUS / model_radius(&planet, planet_center).max(0.001);
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

    let mut ship_controller = ShipController::new(ship_spec);
    let mut game_mode = GameMode::FreeWorld;
    let mut water_level_state = WaterLevelState::new();
    let mut bubble_emitter = BubbleEmitter::new();
    let mut previous = Instant::now();
    let mut world_time = 0.0_f32;
    let mut distance = 0.0_f32;

    while window.is_open() {
        if crate::input::pressed(Key::Escape) {
            match menu::show(&mut window, &mut framebuffer, true)? {
                menu::Action::Quit => return Ok(false),
                menu::Action::Hangar => return Ok(true),
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
        if selected_ship == 4 && game_mode == GameMode::FreeWorld && crate::input::pressed(Key::T) {
            car_mode = !car_mode;
        }
        let now = Instant::now();
        let delta_time = now.duration_since(previous).as_secs_f32().min(0.05);
        previous = now;
        world_time += delta_time;

        if crate::input::pressed(Key::F11) {
            fullscreen = !fullscreen;
            window = create_window(fullscreen)?;
        }
        if crate::input::pressed(Key::R) {
            ship_controller.reset();
            if game_mode == GameMode::WaterLevel {
                water_level_state = WaterLevelState::new();
                bubble_emitter.clear();
            }
            distance = 0.0;
        }
        if game_mode == GameMode::FreeWorld && crate::input::pressed(Key::Key1) {
            place_ship_near_water_planet(&mut ship_controller, world_time);
            println!("Warp completado: planeta acuatico");
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
            ship_controller = ShipController::new(level_ship_spec);
            water_level_state = WaterLevelState::new();
            bubble_emitter.clear();
            game_mode = GameMode::WaterLevel;
            entered_level = true;
            window.set_title("Nivel submarino | Backspace: volver al sistema solar");
            println!("Nivel submarino iniciado");
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
        } else {
            ship_model
        };
        let ship_center = if selected_ship == 4 {
            buzz_centers[if game_mode == GameMode::WaterLevel {
                2
            } else {
                usize::from(car_mode)
            }]
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
                    ship.orbit_release_active = true;
                    ship.orbit_axis = None;
                    println!("Orbita liberada");
                }

                move_around_planet(
                    ship,
                    movement,
                    water_planet_position(world_time),
                    planet_surface_radius,
                    world_time * 0.25,
                    &object_colliders,
                    yaw_input,
                    pitch_input,
                );
                let camera_target = if ship.orbit_axis.is_some() { 1.0 } else { 0.0 };
                let camera_response = 1.0 - (-2.4 * delta_time).exp();
                ship.orbit_camera_blend +=
                    (camera_target - ship.orbit_camera_blend) * camera_response;
                if ship.orbit_axis.is_none() && ship.orbit_camera_blend < 0.001 {
                    ship.orbit_center = None;
                    ship.orbit_camera_blend = 0.0;
                }
                distance += movement;

                if ship.planet_locked {
                    window.set_title("Planeta acuatico | Enter: iniciar nivel | Backspace: salir");
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
                    &sun,
                    &mut framebuffer,
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
                    &level_colliders,
                );
                resolve_water_enemy_collisions(
                    &mut water_level_state,
                    &mut ship_controller.ship,
                    &mut ship_controller.projectiles,
                );
                bubble_emitter.update(&ship_controller.ship, delta_time);
                let turbo_power = ship_controller.turbo_power(ship_controller.ship.speed);
                render_water_level(
                    &water_level,
                    &enemy_models,
                    &enemy_centers,
                    &water_level_state.enemies,
                    water_level_state.elapsed,
                    &bubble_model,
                    &bubble_emitter.particles,
                    level_ship_model,
                    level_ship_center,
                    &turbo_plume,
                    &laser_bolt,
                    &ship_controller.projectiles,
                    &mut framebuffer,
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
        }

        let form = if selected_ship == 4 {
            if game_mode == GameMode::WaterLevel {
                "BUZZ - SUBMARINO"
            } else if car_mode {
                "BUZZ - CARRO"
            } else {
                "BUZZ - NAVE"
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
    fn buzz_assets_load_with_valid_geometry_and_recognized_materials() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut sizes = Vec::new();
        for form in ["nave", "carro", "submarino"] {
            let model = load_obj(root.join(format!("assets/vehicles/buzz_{form}.obj"))).unwrap();
            assert!(model
                .vertices
                .iter()
                .all(|v| v.x.is_finite() && v.y.is_finite() && v.z.is_finite()));
            assert!(model
                .faces
                .iter()
                .all(|face| face.indices.iter().all(|&i| i < model.vertices.len())));
            for material in [
                "buzz_white",
                "buzz_green",
                "buzz_purple",
                "buzz_red",
                "buzz_glass",
            ] {
                assert!(
                    model.faces.iter().any(|f| f.material == material),
                    "{form}: {material}"
                );
            }
            let radius = model_radius(&model, model_center(&model)) * BUZZ_SPEC.model_scale;
            assert!(
                (1.0..3.0).contains(&radius),
                "{form}: unexpected size {radius}"
            );
            sizes.push((model.vertices.len(), model.faces.len()));
        }
        assert_ne!(sizes[0], sizes[1]);
        assert_ne!(sizes[0], sizes[2]);
        assert_ne!(sizes[1], sizes[2]);
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
