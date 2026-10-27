mod framebuffer;
mod obj_loader;
mod triangle;

use std::f32::consts::{PI, TAU};
use std::time::Instant;

use framebuffer::Framebuffer;
use minifb::{Key, KeyRepeat, ScaleMode, Window, WindowOptions};
use obj_loader::{load_obj, Face, Model, Vec3};
use triangle::{draw_triangle, draw_triangle_shaded, ScreenVertex};

const WIDTH: usize = 1200;
const HEIGHT: usize = 800;
const NEAR_PLANE: f32 = 0.2;
const SHIP_SCALE: f32 = 0.055;
const MIN_SPEED: f32 = 4.0;
const MAX_SPEED: f32 = 55.0;
const LIGHT_SPEED: f32 = 600.0;
const PLANET_ORBIT_SPEED: f32 = 0.02;
const ROLL_INPUT_SPEED: f32 = PI * 0.5;
const ROLL_SPEED: f32 = PI;
const TURN_RESPONSE: f32 = 7.0;
const MAX_YAW_RATE: f32 = 1.15;
const MAX_PITCH_RATE: f32 = 1.1;
const STEERING_BANK: f32 = 0.48;

struct Ship {
    position: Vec3,
    yaw: f32,
    pitch: f32,
    roll: f32,
    roll_target: f32,
    yaw_rate: f32,
    pitch_rate: f32,
    steering_bank: f32,
    speed: f32,
}

impl Ship {
    fn new() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            roll: 0.0,
            roll_target: 0.0,
            yaw_rate: 0.0,
            pitch_rate: 0.0,
            steering_bank: 0.0,
            speed: 12.0,
        }
    }

    fn basis(&self) -> (Vec3, Vec3, Vec3) {
        self.basis_with_roll(self.roll + self.steering_bank)
    }

    fn camera_basis(&self) -> (Vec3, Vec3, Vec3) {
        self.basis_with_roll(self.roll)
    }

    fn basis_with_roll(&self, roll: f32) -> (Vec3, Vec3, Vec3) {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let forward = Vec3::new(-cos_pitch * cos_yaw, sin_pitch, -cos_pitch * sin_yaw).normalize();
        let level_right = forward.cross(Vec3::new(0.0, 1.0, 0.0)).normalize();
        let level_up = level_right.cross(forward).normalize();
        let (sin_roll, cos_roll) = roll.sin_cos();
        let right = level_right.mul(cos_roll).add(level_up.mul(sin_roll));
        let up = level_up.mul(cos_roll).sub(level_right.mul(sin_roll));
        (forward, right, up)
    }
}

struct Camera {
    position: Vec3,
    forward: Vec3,
    right: Vec3,
    up: Vec3,
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
    } else if name.contains("roca_oscura") {
        SurfaceShader::DarkRock
    } else if name.contains("roca") {
        SurfaceShader::Rock
    } else if name.contains("tierra") {
        SurfaceShader::Earth
    } else if name.contains("cesped") {
        SurfaceShader::Grass
    } else if name.contains("arena") {
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
    window.set_target_fps(60);
    Ok(window)
}

fn color_for_material(name: &str) -> [u8; 3] {
    let name = name.to_ascii_lowercase();
    if name.contains("sun") {
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
    } else if name.contains("glass") {
        [24, 60, 78]
    } else if name.contains("dark") {
        [43, 51, 64]
    } else if name.contains("yellow") {
        [242, 197, 57]
    } else if name.contains("red") {
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
            });
            faces.push(Face {
                indices: [first + 1, second, second + 1],
                material: material.to_string(),
            });
        }
    }
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
    let position = ship.position.sub(forward.mul(12.0)).add(ship_up.mul(4.0));
    let target = ship.position.add(forward.mul(8.0));
    let camera_forward = target.sub(position).normalize();
    let right = ship_right;
    let up = right.cross(camera_forward).normalize();
    Camera {
        position,
        forward: camera_forward,
        right,
        up,
    }
}

fn water_planet_position(world_time: f32) -> Vec3 {
    let sun_position = Vec3::new(-400.0, 20.0, 0.0);
    let orbit = world_time * PLANET_ORBIT_SPEED;
    sun_position.add(Vec3::new(orbit.cos() * 220.0, -20.0, orbit.sin() * 220.0))
}

fn render_model<F>(
    model: &Model,
    framebuffer: &mut Framebuffer,
    camera: &Camera,
    transform: F,
    water_time: Option<f32>,
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
                pack(color_for_material(&face.material), brightness),
            );
        }
    }
}

fn render_scene(
    ship_model: &Model,
    ship_center: Vec3,
    planet: &Model,
    planet_center: Vec3,
    planet_scale: f32,
    sun: &Model,
    framebuffer: &mut Framebuffer,
    ship: &Ship,
    world_time: f32,
    travel: f32,
) {
    framebuffer.clear(0);
    draw_skybox(framebuffer, travel);
    let camera = chase_camera(ship);
    let sun_position = Vec3::new(-400.0, 20.0, 0.0);
    let planet_position = water_planet_position(world_time);
    let planet_rotation = world_time * 0.25;

    render_model(
        sun,
        framebuffer,
        &camera,
        |vertex| vertex.add(sun_position),
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
    );

    let (forward, right, up) = ship.basis();
    render_model(
        ship_model,
        framebuffer,
        &camera,
        |vertex| {
            let local = vertex.sub(ship_center).mul(SHIP_SCALE);
            ship.position
                .add(forward.mul(-local.x))
                .add(up.mul(local.y))
                .add(right.mul(local.z))
        },
        None,
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "assets/nave.obj".to_string());
    let ship_model = load_obj(&path).map_err(|error| format!("Error al cargar {path}: {error}"))?;
    let ship_center = model_center(&ship_model);
    let planet_path = "assets/water_planet.obj";
    let planet =
        load_obj(planet_path).map_err(|error| format!("Error al cargar {planet_path}: {error}"))?;
    let planet_center = model_center(&planet);
    let planet_scale = 24.0 / model_radius(&planet, planet_center).max(0.001);
    let sun = make_sphere(55.0, 18, 28, true);

    println!("Vuelo libre 3D iniciado");
    println!("W/S o flechas: subir/bajar | Espacio: acelerar | Ctrl: frenar");
    println!("A/D: direccion | Mantener Q/E: alabeo | R: reiniciar | P: captura");
    println!("1: warp al planeta acuatico | Shift: velocidad de luz");

    let mut fullscreen = false;
    let mut window = create_window(fullscreen)?;
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
    let mut ship = Ship::new();
    let start = Instant::now();
    let mut previous = Instant::now();
    let mut distance = 0.0_f32;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let now = Instant::now();
        let delta_time = now.duration_since(previous).as_secs_f32().min(0.05);
        previous = now;
        let world_time = start.elapsed().as_secs_f32();

        if window.is_key_pressed(Key::F11, KeyRepeat::No) {
            fullscreen = !fullscreen;
            window = create_window(fullscreen)?;
        }
        if window.is_key_pressed(Key::R, KeyRepeat::No) {
            ship = Ship::new();
            distance = 0.0;
        }
        if window.is_key_pressed(Key::Key1, KeyRepeat::No) {
            let destination = water_planet_position(world_time);
            ship.position = destination.add(Vec3::new(55.0, 8.0, 0.0));
            ship.yaw = 0.0;
            ship.pitch = -0.08;
            ship.roll = 0.0;
            ship.roll_target = 0.0;
            ship.yaw_rate = 0.0;
            ship.pitch_rate = 0.0;
            ship.steering_bank = 0.0;
            ship.speed = 12.0;
            println!("Warp completado: planeta acuatico");
        }

        let pitch_input = (window.is_key_down(Key::W) || window.is_key_down(Key::Up)) as i8
            - (window.is_key_down(Key::S) || window.is_key_down(Key::Down)) as i8;
        let yaw_input = window.is_key_down(Key::D) as i8 - window.is_key_down(Key::A) as i8;

        let response = 1.0 - (-TURN_RESPONSE * delta_time).exp();
        let target_pitch_rate = pitch_input as f32 * MAX_PITCH_RATE;
        let target_yaw_rate = yaw_input as f32 * MAX_YAW_RATE;
        ship.pitch_rate += (target_pitch_rate - ship.pitch_rate) * response;
        ship.yaw_rate += (target_yaw_rate - ship.yaw_rate) * response;
        ship.pitch = (ship.pitch + ship.pitch_rate * delta_time).clamp(-1.25, 1.25);
        ship.yaw = (ship.yaw + ship.yaw_rate * delta_time).rem_euclid(TAU);

        let target_bank = yaw_input as f32 * STEERING_BANK;
        ship.steering_bank += (target_bank - ship.steering_bank) * response;

        let roll_input = window.is_key_down(Key::E) as i8 - window.is_key_down(Key::Q) as i8;
        ship.roll_target += roll_input as f32 * ROLL_INPUT_SPEED * delta_time;

        let roll_difference = ship.roll_target - ship.roll;
        let roll_step = (ROLL_SPEED * delta_time).min(roll_difference.abs());
        ship.roll += roll_difference.signum() * roll_step;

        if window.is_key_down(Key::Space) {
            ship.speed = (ship.speed + 20.0 * delta_time).min(MAX_SPEED);
        }
        if window.is_key_down(Key::LeftCtrl) || window.is_key_down(Key::RightCtrl) {
            ship.speed = (ship.speed - 28.0 * delta_time).max(MIN_SPEED);
        }

        let (forward, _, _) = ship.basis();
        let light_speed = window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift);
        let active_speed = if light_speed { LIGHT_SPEED } else { ship.speed };
        let movement = active_speed * delta_time;
        ship.position = ship.position.add(forward.mul(movement));
        distance += movement;

        render_scene(
            &ship_model,
            ship_center,
            &planet,
            planet_center,
            planet_scale,
            &sun,
            &mut framebuffer,
            &ship,
            world_time,
            (distance * 0.0025).rem_euclid(1.0),
        );

        if window.is_key_pressed(Key::P, KeyRepeat::No) {
            framebuffer.save_png("captura.png")?;
            println!("Captura guardada: captura.png");
        }
        window.update_with_buffer(&framebuffer.pixels, WIDTH, HEIGHT)?;
    }
    Ok(())
}
