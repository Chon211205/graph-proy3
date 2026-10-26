mod framebuffer;
mod obj_loader;
mod triangle;

use framebuffer::Framebuffer;
use minifb::{Key, KeyRepeat, Window, WindowOptions};
use obj_loader::{load_obj, Model, Vec3};
use triangle::{draw_triangle, ScreenVertex};

const WIDTH: usize = 1200;
const HEIGHT: usize = 800;

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
    let channel = |v: u8| {
        (v as f32 * brightness).clamp(0.0, 255.0) as u32
    };

    (channel(rgb[0]) << 16)
        | (channel(rgb[1]) << 8)
        | channel(rgb[2])
}

fn render(
    model: &Model,
    framebuffer: &mut Framebuffer,
    yaw: f32,
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

    let (sin, cos) = yaw.sin_cos();

    let camera = Vec3::new(
        initial.x * cos - initial.z * sin,
        initial.y,
        initial.x * sin + initial.z * cos,
    );

    let right = Vec3::new(0.0, 1.0, 0.0)
        .cross(camera)
        .normalize();

    let up = camera.cross(right).normalize();

    let light = Vec3::new(0.2, 1.0, -0.7).normalize();

    let camera_coords: Vec<Vec3> = model
        .vertices
        .iter()
        .map(|vertex| {
            let v = vertex.sub(center);

            Vec3::new(
                v.dot(right),
                v.dot(up),
                v.dot(camera),
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

    let scale_x =
        (WIDTH as f32 * 0.84) / (x1 - x0).max(0.001);

    let scale_y =
        (HEIGHT as f32 * 0.82) / (y1 - y0).max(0.001);

    let scale = scale_x.min(scale_y);

    let mx = (x0 + x1) * 0.5;
    let my = (y0 + y1) * 0.5;

    let screen: Vec<ScreenVertex> = camera_coords
        .iter()
        .map(|vertex| ScreenVertex {
            x: WIDTH as f32 * 0.5
                + (vertex.x - mx) * scale,

            y: HEIGHT as f32 * 0.5
                - (vertex.y - my) * scale,

            z: vertex.z,
        })
        .collect();

    for face in &model.faces {
        let [i0, i1, i2] = face.indices;

        let v0 = model.vertices[i0];
        let v1 = model.vertices[i1];
        let v2 = model.vertices[i2];

        let normal = v1
            .sub(v0)
            .cross(v2.sub(v0))
            .normalize();

        let brightness =
            0.53 + 0.47 * normal.dot(light).abs();

        let color = pack(
            color_for_material(&face.material),
            brightness,
        );

        draw_triangle(
            framebuffer,
            screen[i0],
            screen[i1],
            screen[i2],
            color,
        );
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "assets/nave.obj".to_string());

    let model = load_obj(&path)
        .map_err(|e| format!("Error al cargar {}: {}", path, e))?;

    println!("Modelo: {}", path);
    println!("Vertices: {}", model.vertices.len());
    println!("Triangulos: {}", model.faces.len());

    let mut window = Window::new(
        "LAB 4 - Nave Espacial OBJ | Rust",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )?;

    window.set_target_fps(60);

    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);

    let mut yaw = 0.0;

    render(&model, &mut framebuffer, yaw);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let mut changed = false;

        if window.is_key_down(Key::Left) {
            yaw -= 0.025;
            changed = true;
        }

        if window.is_key_down(Key::Right) {
            yaw += 0.025;
            changed = true;
        }

        if changed {
            render(&model, &mut framebuffer, yaw);
        }

        if window.is_key_pressed(Key::S, KeyRepeat::No) {
            framebuffer.save_png("captura.png")?;
            println!("Captura guardada: captura.png");
        }

        window.update_with_buffer(
            &framebuffer.pixels,
            WIDTH,
            HEIGHT,
        )?;
    }

    Ok(())
}