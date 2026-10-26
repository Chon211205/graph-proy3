mod framebuffer;
mod obj_loader;
mod triangle;

use framebuffer::Framebuffer;
use minifb::{Key, KeyRepeat, ScaleMode, Window, WindowOptions};
use obj_loader::{load_obj, Model, Vec3};
use triangle::{draw_triangle, ScreenVertex};

const WIDTH: usize = 1200;
const HEIGHT: usize = 800;
const MAX_SPEED: f32 = 0.028;
const ACCELERATION: f32 = 0.0035;
const DRAG: f32 = 0.86;

fn screen_size() -> (usize, usize) {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN,
        };

        // GetSystemMetrics is safe for these read-only screen metrics.
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
        "LAB 4 - Nave Espacial OBJ | Rust | F11: pantalla completa",
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
    let n = name.to_ascii_lowercase();

    if n.contains("blue") {
        [47, 87, 235]
    } else if n.contains("glass") {
        [24, 60, 78]
    } else if n.contains("dark") {
        [43, 51, 64]
    } else if n.contains("yellow") {
        [242, 197, 57]
    } else if n.contains("red") {
        [199, 49, 57]
    } else if n.contains("metal") {
        [103, 121, 143]
    } else {
        [220, 230, 238]
    }
}

fn pack(rgb: [u8; 3], brightness: f32) -> u32 {
    let channel = |v: u8| (v as f32 * brightness).clamp(0.0, 255.0) as u32;

    (channel(rgb[0]) << 16) | (channel(rgb[1]) << 8) | channel(rgb[2])
}

fn render(
    model: &Model,
    framebuffer: &mut Framebuffer,
    ship_x: f32,
    ship_y: f32,
    yaw: f32,
    pitch: f32,
    roll: f32,
) {
    framebuffer.clear(0x172234);

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

    let center = Vec3::new(
        (min.x + max.x) * 0.5,
        (min.y + max.y) * 0.5,
        (min.z + max.z) * 0.5,
    );

    let initial = Vec3::new(1.05, 0.85, -1.55).normalize();

    let camera = initial;

    let right = Vec3::new(0.0, 1.0, 0.0).cross(camera).normalize();

    let up = camera.cross(right).normalize();

    let light = Vec3::new(0.2, 1.0, -0.7).normalize();

    let camera_coords: Vec<Vec3> = model
        .vertices
        .iter()
        .map(|vertex| {
            let v = vertex.sub(center);

            let v = Vec3::new(v.dot(right), v.dot(up), v.dot(camera));

            let (sin_yaw, cos_yaw) = yaw.sin_cos();
            let v = Vec3::new(
                v.x * cos_yaw + v.z * sin_yaw,
                v.y,
                -v.x * sin_yaw + v.z * cos_yaw,
            );

            let (sin_pitch, cos_pitch) = pitch.sin_cos();
            let v = Vec3::new(
                v.x,
                v.y * cos_pitch - v.z * sin_pitch,
                v.y * sin_pitch + v.z * cos_pitch,
            );

            let (sin_roll, cos_roll) = roll.sin_cos();
            Vec3::new(
                v.x * cos_roll - v.y * sin_roll,
                v.x * sin_roll + v.y * cos_roll,
                v.z,
            )
        })
        .collect();

    let mut x0 = f32::INFINITY;
    let mut x1 = f32::NEG_INFINITY;
    let mut y0 = f32::INFINITY;
    let mut y1 = f32::NEG_INFINITY;

    for vertex in &camera_coords {
        x0 = x0.min(vertex.x);
        x1 = x1.max(vertex.x);

        y0 = y0.min(vertex.y);
        y1 = y1.max(vertex.y);
    }

    let scale_x = (WIDTH as f32 * 0.48) / (x1 - x0).max(0.001);

    let scale_y = (HEIGHT as f32 * 0.48) / (y1 - y0).max(0.001);

    let scale = scale_x.min(scale_y);

    let mx = (x0 + x1) * 0.5;
    let my = (y0 + y1) * 0.5;

    let screen: Vec<ScreenVertex> = camera_coords
        .iter()
        .map(|vertex| ScreenVertex {
            x: WIDTH as f32 * (0.5 + ship_x * 0.28) + (vertex.x - mx) * scale,

            y: HEIGHT as f32 * (0.5 + ship_y * 0.25) - (vertex.y - my) * scale,

            z: vertex.z,
        })
        .collect();

    for face in &model.faces {
        let [i0, i1, i2] = face.indices;

        let v0 = camera_coords[i0];
        let v1 = camera_coords[i1];
        let v2 = camera_coords[i2];

        let normal = v1.sub(v0).cross(v2.sub(v0)).normalize();

        let brightness = 0.53 + 0.47 * normal.dot(light).abs();

        let color = pack(color_for_material(&face.material), brightness);

        draw_triangle(framebuffer, screen[i0], screen[i1], screen[i2], color);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "assets/nave.obj".to_string());

    let model = load_obj(&path).map_err(|e| format!("Error al cargar {}: {}", path, e))?;

    println!("Modelo: {}", path);
    println!("Vertices: {}", model.vertices.len());
    println!("Triangulos: {}", model.faces.len());
    println!("Controles: WASD/flechas = mover, Q/E = giro, P = captura, F11 = fullscreen");

    let mut fullscreen = false;
    let mut window = create_window(fullscreen)?;

    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);

    let mut ship_x: f32 = 0.0;
    let mut ship_y: f32 = 0.0;
    let mut velocity_x: f32 = 0.0;
    let mut velocity_y: f32 = 0.0;
    let mut yaw: f32 = 0.0;
    let mut pitch: f32 = 0.0;
    let mut roll: f32 = 0.0;
    let mut barrel_roll: f32 = 0.0;

    render(&model, &mut framebuffer, ship_x, ship_y, yaw, pitch, roll);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        if window.is_key_pressed(Key::F11, KeyRepeat::No) {
            fullscreen = !fullscreen;
            window = create_window(fullscreen)?;
            println!(
                "Pantalla completa: {}",
                if fullscreen {
                    "activada"
                } else {
                    "desactivada"
                }
            );
        }

        let horizontal = (window.is_key_down(Key::D) || window.is_key_down(Key::Right)) as i8
            - (window.is_key_down(Key::A) || window.is_key_down(Key::Left)) as i8;
        let vertical = (window.is_key_down(Key::S) || window.is_key_down(Key::Down)) as i8
            - (window.is_key_down(Key::W) || window.is_key_down(Key::Up)) as i8;

        velocity_x = (velocity_x + horizontal as f32 * ACCELERATION).clamp(-MAX_SPEED, MAX_SPEED);
        velocity_y = (velocity_y + vertical as f32 * ACCELERATION).clamp(-MAX_SPEED, MAX_SPEED);

        if horizontal == 0 {
            velocity_x *= DRAG;
        }
        if vertical == 0 {
            velocity_y *= DRAG;
        }

        ship_x = (ship_x + velocity_x).clamp(-1.0, 1.0);
        ship_y = (ship_y + velocity_y).clamp(-1.0, 1.0);

        if ship_x.abs() >= 1.0 {
            velocity_x = 0.0;
        }
        if ship_y.abs() >= 1.0 {
            velocity_y = 0.0;
        }

        if window.is_key_down(Key::Q) {
            barrel_roll -= 0.10;
        }
        if window.is_key_down(Key::E) {
            barrel_roll += 0.10;
        }

        let target_yaw = velocity_x * 6.0;
        let target_pitch = -velocity_y * 12.0;
        let target_roll = barrel_roll - velocity_x * 18.0;

        yaw += (target_yaw - yaw) * 0.14;
        pitch += (target_pitch - pitch) * 0.14;
        roll += (target_roll - roll) * 0.18;

        render(&model, &mut framebuffer, ship_x, ship_y, yaw, pitch, roll);

        if window.is_key_pressed(Key::P, KeyRepeat::No) {
            framebuffer.save_png("captura.png")?;
            println!("Captura guardada: captura.png");
        }

        window.update_with_buffer(&framebuffer.pixels, WIDTH, HEIGHT)?;
    }

    Ok(())
}
