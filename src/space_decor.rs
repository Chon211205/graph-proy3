use std::sync::OnceLock;

use crate::framebuffer::Framebuffer;
use crate::galaxy::Galaxy;
use crate::obj_loader::{Face, MaterialColor, Model, SurfaceShader, Vec3};

const ASTEROID_COUNT: usize = 34;
const ASTEROID_RING_SEGMENTS: usize = 6;
const DUST_COUNT: u32 = 420;

pub(crate) fn asteroid_belt(galaxy: Galaxy) -> &'static Model {
    static BELTS: OnceLock<[Model; 3]> = OnceLock::new();
    let belts = BELTS.get_or_init(|| {
        [
            make_asteroid_belt([[91, 111, 142], [129, 116, 104], [66, 82, 105]]),
            make_asteroid_belt([[122, 126, 169], [90, 103, 145], [157, 139, 166]]),
            make_asteroid_belt([[137, 112, 91], [93, 112, 108], [171, 153, 121]]),
        ]
    });
    &belts[match galaxy {
        Galaxy::Home => 0,
        Galaxy::Vesper => 1,
        Galaxy::Nova => 2,
    }]
}

fn make_asteroid_belt(palette: [[u8; 3]; 3]) -> Model {
    let mut vertices = Vec::with_capacity(ASTEROID_COUNT * (ASTEROID_RING_SEGMENTS + 2));
    let mut faces = Vec::with_capacity(ASTEROID_COUNT * ASTEROID_RING_SEGMENTS * 2);

    for asteroid in 0..ASTEROID_COUNT {
        let seed = asteroid as u32 * 7919 + 53;
        let angle = unit(seed) * std::f32::consts::TAU;
        let belt_radius = 0.91 + unit(seed ^ 0xa511_e9b3) * 0.18;
        let center = Vec3::new(
            angle.cos() * belt_radius,
            (unit(seed ^ 0x63d8_3595) - 0.5) * 0.035,
            angle.sin() * belt_radius,
        );
        let size = 0.014 + unit(seed ^ 0x9e37_79b9) * 0.021;
        let rotation = unit(seed ^ 0x7f4a_7c15) * std::f32::consts::TAU;
        let base = vertices.len();

        for segment in 0..ASTEROID_RING_SEGMENTS {
            let facet_seed = seed.wrapping_add(segment as u32 * 313);
            let a = rotation + segment as f32 * std::f32::consts::TAU
                / ASTEROID_RING_SEGMENTS as f32;
            let wobble = 0.70 + unit(facet_seed) * 0.55;
            let y = (unit(facet_seed ^ 0xd1b5_4a35) - 0.5) * 0.8;
            vertices.push(center.add(Vec3::new(
                a.cos() * size * wobble,
                y * size,
                a.sin() * size * wobble,
            )));
        }
        let top = vertices.len();
        vertices.push(center.add(Vec3::new(
            (unit(seed ^ 0x94d0_49bb) - 0.5) * size * 0.5,
            size * (0.55 + unit(seed ^ 0x369d_ea0f) * 0.35),
            (unit(seed ^ 0xdb4f_0b91) - 0.5) * size * 0.5,
        )));
        let bottom = vertices.len();
        vertices.push(center.add(Vec3::new(
            (unit(seed ^ 0x3c6e_f372) - 0.5) * size * 0.5,
            -size * (0.48 + unit(seed ^ 0xa54f_f53a) * 0.35),
            (unit(seed ^ 0x510e_527f) - 0.5) * size * 0.5,
        )));

        for segment in 0..ASTEROID_RING_SEGMENTS {
            let next = (segment + 1) % ASTEROID_RING_SEGMENTS;
            let color = palette[(asteroid + segment / 2) % palette.len()];
            for indices in [[top, base + segment, base + next], [bottom, base + next, base + segment]] {
                let mut face = Face::new(indices, "space_asteroid", "asteroid_belt");
                face.shader = SurfaceShader::Flat;
                face.color = MaterialColor::Fixed(color);
                faces.push(face);
            }
        }
    }

    Model::new(vertices, faces)
}

pub(crate) fn draw_dust(framebuffer: &mut Framebuffer, travel: f32, galaxy: Galaxy) {
    let tint = match galaxy {
        Galaxy::Home => [94_u32, 136_u32, 204_u32],
        Galaxy::Vesper => [160, 125, 222],
        Galaxy::Nova => [88, 200, 170],
    };
    let center_x = framebuffer.width as f32 * 0.5;
    let center_y = framebuffer.height as f32 * 0.48;

    for index in 0..DUST_COUNT {
        let seed = index.wrapping_mul(104_729).wrapping_add(17);
        let horizontal = unit(seed) * 2.0 - 1.0;
        let vertical = unit(seed ^ 0x68bc_21eb) * 2.0 - 1.0;
        let initial_depth = unit(seed ^ 0x02e5_be93);
        let depth = (initial_depth - travel * 0.34).rem_euclid(1.0).max(0.07);
        let perspective = 0.33 / depth;
        let x = (center_x + horizontal * framebuffer.width as f32 * perspective) as i32;
        let y = (center_y + vertical * framebuffer.height as f32 * perspective) as i32;
        if x < 0 || y < 0 || x >= framebuffer.width as i32 || y >= framebuffer.height as i32 {
            continue;
        }

        let shimmer = (hash(seed ^ 0xb529_7a4d) % 24) as u32;
        let brightness = (12.0 + (1.0 - depth) * 44.0) as u32 + shimmer;
        let color = ((tint[0] * brightness / 255) << 16)
            | ((tint[1] * brightness / 255) << 8)
            | (tint[2] * brightness / 255);
        set_pixel(framebuffer, x, y, color);
        if depth < 0.20 && index % 7 == 0 {
            set_pixel(framebuffer, x + 1, y, color);
        }
    }
}

fn set_pixel(framebuffer: &mut Framebuffer, x: i32, y: i32, color: u32) {
    if x >= 0 && y >= 0 && x < framebuffer.width as i32 && y < framebuffer.height as i32 {
        framebuffer.pixels[y as usize * framebuffer.width + x as usize] = color;
    }
}

fn unit(seed: u32) -> f32 {
    hash(seed) as f32 / u32::MAX as f32
}

fn hash(mut value: u32) -> u32 {
    value ^= value >> 16;
    value = value.wrapping_mul(0x7feb_352d);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846c_a68b);
    value ^ (value >> 16)
}
