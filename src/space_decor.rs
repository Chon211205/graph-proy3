use std::sync::OnceLock;

use crate::framebuffer::Framebuffer;
use crate::galaxy::Galaxy;
use crate::obj_loader::{Face, MaterialColor, Model, SurfaceShader, Vec3};

// A belt is mostly fine debris; keep large rocks sparse and add many tiny chips.
const ASTEROID_COUNT: usize = 40;
const ASTEROID_RING_SEGMENTS: usize = 7;
const ASTEROID_SHAPE_RINGS: usize = 3;
const BELT_DUST_COUNT: usize = 144;
const DUST_COUNT: u32 = 420;

pub(crate) struct Asteroid {
    pub model: Model,
    pub orbit_fraction: f32,
    pub orbit_phase: f32,
    pub orbit_speed: f32,
    pub spin_phase: f32,
    pub spin_speed: f32,
    pub vertical_offset: f32,
}

pub(crate) fn asteroid_belt(galaxy: Galaxy) -> &'static Model {
    static BELTS: OnceLock<[Model; 3]> = OnceLock::new();
    let belts = BELTS.get_or_init(|| {
        [
            make_dust_belt([[91, 111, 142], [129, 116, 104], [66, 82, 105]]),
            make_dust_belt([[122, 126, 169], [90, 103, 145], [157, 139, 166]]),
            make_dust_belt([[137, 112, 91], [93, 112, 108], [171, 153, 121]]),
        ]
    });
    &belts[match galaxy {
        Galaxy::Home => 0,
        Galaxy::Vesper => 1,
        Galaxy::Nova => 2,
    }]
}

pub(crate) fn asteroids(galaxy: Galaxy) -> &'static [Asteroid] {
    static FIELDS: OnceLock<[Vec<Asteroid>; 3]> = OnceLock::new();
    let fields = FIELDS.get_or_init(|| {
        [
            make_asteroids([[91, 111, 142], [129, 116, 104], [66, 82, 105]], 0x13a5_8f21),
            make_asteroids([[122, 126, 169], [90, 103, 145], [157, 139, 166]], 0x7e31_4c99),
            make_asteroids([[137, 112, 91], [93, 112, 108], [171, 153, 121]], 0xc049_827b),
        ]
    });
    &fields[match galaxy {
        Galaxy::Home => 0,
        Galaxy::Vesper => 1,
        Galaxy::Nova => 2,
    }]
}

fn make_asteroids(palette: [[u8; 3]; 3], galaxy_seed: u32) -> Vec<Asteroid> {
    (0..ASTEROID_COUNT)
        .map(|index| {
            let seed = (index as u32).wrapping_mul(7919).wrapping_add(galaxy_seed);
            Asteroid {
                model: make_asteroid(seed, palette, index),
                orbit_fraction: 0.84 + unit(seed ^ 0xa511_e9b3) * 0.32,
                orbit_phase: index as f32 * 2.399_963_1 + (unit(seed) - 0.5) * 0.28,
                // Much slower than the planets; each rock has a slightly
                // different period so the belt does not rotate as one object.
                orbit_speed: 0.010 + unit(seed ^ 0x63d8_3595) * 0.014,
                spin_phase: unit(seed ^ 0x7f4a_7c15) * std::f32::consts::TAU,
                spin_speed: 0.055 + unit(seed ^ 0x243f_6a88) * 0.105,
                vertical_offset: (unit(seed ^ 0x369d_ea0f) - 0.5) * 6.0,
            }
        })
        .collect()
}

fn make_asteroid(seed: u32, palette: [[u8; 3]; 3], asteroid: usize) -> Model {
    let size = 2.8 + unit(seed ^ 0x9e37_79b9) * 4.7;
    let rotation = unit(seed ^ 0x7f4a_7c15) * std::f32::consts::TAU;
    let mut vertices = Vec::with_capacity(ASTEROID_RING_SEGMENTS * ASTEROID_SHAPE_RINGS + 2);
    let mut faces = Vec::with_capacity(ASTEROID_RING_SEGMENTS * (ASTEROID_SHAPE_RINGS + 1));
    let mut ring_bases = [0; ASTEROID_SHAPE_RINGS];
    let ring_heights = [-0.30, 0.04, 0.34];
    let ring_widths = [0.72, 1.0, 0.78];
    let body_stretch = 0.78 + unit(seed ^ 0x243f_6a88) * 0.46;

    for ring in 0..ASTEROID_SHAPE_RINGS {
        ring_bases[ring] = vertices.len();
        for segment in 0..ASTEROID_RING_SEGMENTS {
            let facet_seed = seed
                .wrapping_add(segment as u32 * 313)
                .wrapping_add(ring as u32 * 0x9e37);
            let angle = rotation
                + segment as f32 * std::f32::consts::TAU / ASTEROID_RING_SEGMENTS as f32
                + (unit(facet_seed ^ 0xd1b5_4a35) - 0.5) * 0.20;
            let wobble = 0.72 + unit(facet_seed) * 0.52;
            let vertical_jag = (unit(facet_seed ^ 0x85eb_ca6b) - 0.5) * 0.15;
            vertices.push(Vec3::new(
                angle.cos() * size * ring_widths[ring] * wobble * body_stretch,
                size * (ring_heights[ring] + vertical_jag),
                angle.sin() * size * ring_widths[ring] * wobble / body_stretch,
            ));
        }
    }
    let top = vertices.len();
    vertices.push(Vec3::new(
        (unit(seed ^ 0x94d0_49bb) - 0.5) * size * 0.62,
        size * (0.48 + unit(seed ^ 0x369d_ea0f) * 0.24),
        (unit(seed ^ 0xdb4f_0b91) - 0.5) * size * 0.62,
    ));
    let bottom = vertices.len();
    vertices.push(Vec3::new(
        (unit(seed ^ 0x3c6e_f372) - 0.5) * size * 0.62,
        -size * (0.42 + unit(seed ^ 0xa54f_f53a) * 0.25),
        (unit(seed ^ 0x510e_527f) - 0.5) * size * 0.62,
    ));

    for segment in 0..ASTEROID_RING_SEGMENTS {
        let next = (segment + 1) % ASTEROID_RING_SEGMENTS;
        let shade_seed = seed.wrapping_add(segment as u32 * 911);
        let color = palette[(asteroid + segment / 2) % palette.len()];
        let mut push_face = |indices: [usize; 3], layer: usize, face_index: usize| {
            let mut face = Face::new(indices, "space_asteroid", "asteroid_belt");
            face.shader = SurfaceShader::Flat;
            let shade = 0.68
                + unit(shade_seed ^ (layer as u32 * 0x45d9_f3b + face_index as u32)) * 0.46;
            face.color = MaterialColor::Fixed([
                (color[0] as f32 * shade).min(255.0) as u8,
                (color[1] as f32 * shade).min(255.0) as u8,
                (color[2] as f32 * shade).min(255.0) as u8,
            ]);
            faces.push(face);
        };

        for layer in 0..ASTEROID_SHAPE_RINGS - 1 {
            let lower = ring_bases[layer];
            let upper = ring_bases[layer + 1];
            let a = lower + segment;
            let b = lower + next;
            let c = upper + segment;
            let d = upper + next;
            if segment % 2 == 0 {
                push_face([a, c, b], layer, 0);
                push_face([b, c, d], layer, 1);
            } else {
                push_face([a, c, d], layer, 0);
                push_face([a, d, b], layer, 1);
            }
        }
        let upper = ring_bases[ASTEROID_SHAPE_RINGS - 1];
        let lower = ring_bases[0];
        push_face([top, upper + segment, upper + next], ASTEROID_SHAPE_RINGS, 0);
        push_face([bottom, lower + next, lower + segment], ASTEROID_SHAPE_RINGS + 1, 0);
    }

    Model::new(vertices, faces)
}

fn make_dust_belt(palette: [[u8; 3]; 3]) -> Model {
    let mut vertices = Vec::with_capacity(BELT_DUST_COUNT * 3);
    let mut faces = Vec::with_capacity(BELT_DUST_COUNT);
    // Fine fragments trace the same narrow orbital lane as the rocks.
    for chip in 0..BELT_DUST_COUNT {
        let seed = chip as u32 * 65_537 + 0x51ed_270b;
        let angle = unit(seed) * std::f32::consts::TAU;
        let radius = 0.82 + unit(seed ^ 0x9e37_79b9) * 0.36;
        let center = Vec3::new(
            angle.cos() * radius,
            angle.sin() * radius * 0.055
                + (unit(seed ^ 0x63d8_3595) - 0.5) * 0.035,
            angle.sin() * radius,
        );
        let size = 0.0015 + unit(seed ^ 0xa511_e9b3) * 0.003;
        let spin = unit(seed ^ 0x7f4a_7c15) * std::f32::consts::TAU;
        let base = vertices.len();
        for corner in 0..3 {
            let a = spin + corner as f32 * std::f32::consts::TAU / 3.0;
            vertices.push(center.add(Vec3::new(a.cos() * size, 0.0, a.sin() * size)));
        }
        let color = palette[(chip / 3 + (hash(seed) as usize % palette.len())) % palette.len()];
        let mut face = Face::new([base, base + 1, base + 2], "space_dust", "asteroid_belt");
        face.shader = SurfaceShader::Flat;
        face.color = MaterialColor::Fixed([
            (color[0] as f32 * 0.58) as u8,
            (color[1] as f32 * 0.58) as u8,
            (color[2] as f32 * 0.58) as u8,
        ]);
        faces.push(face);
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
