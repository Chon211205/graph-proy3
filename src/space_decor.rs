use std::fs::File;
use std::sync::OnceLock;

use crate::framebuffer::Framebuffer;
use crate::galaxy::Galaxy;
use crate::obj_loader::{Face, MaterialColor, Model, SurfaceShader, Vec3};

// A belt is mostly fine debris; keep large rocks sparse and add many tiny chips.
const INNER_ASTEROID_COUNT: usize = 40;
const OUTER_ASTEROID_COUNT: usize = 28;
const ASTEROID_RING_SEGMENTS: usize = 7;
const ASTEROID_SHAPE_RINGS: usize = 3;
const BELT_DUST_COUNT: usize = 144;
const DUST_COUNT: u32 = 420;
const NEBULA_WIDTH: usize = 640;
const NEBULA_HEIGHT: usize = 360;
const STAR_BIT_COUNT: usize = 144;

pub(crate) struct Asteroid {
    pub model: Model,
    pub collision_radius: f32,
    pub orbit_fraction: f32,
    pub orbit_phase: f32,
    pub orbit_speed: f32,
    pub spin_phase: f32,
    pub spin_speed: f32,
    pub vertical_offset: f32,
}

struct NebulaTexture {
    pixels: Vec<[u8; 4]>,
}

impl NebulaTexture {
    fn load(path: &str) -> Option<Self> {
        let file = File::open(path).ok()?;
        let mut decoder = png::Decoder::new(file);
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info().ok()?;
        let mut source = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut source).ok()?;
        let channels = match info.color_type {
            png::ColorType::Rgba => 4,
            png::ColorType::Rgb => 3,
            png::ColorType::GrayscaleAlpha => 2,
            png::ColorType::Grayscale => 1,
            png::ColorType::Indexed => return None,
        };
        let source_width = info.width as usize;
        let source_height = info.height as usize;
        let mut pixels = Vec::with_capacity(NEBULA_WIDTH * NEBULA_HEIGHT);
        for y in 0..NEBULA_HEIGHT {
            let source_y = y * source_height / NEBULA_HEIGHT;
            for x in 0..NEBULA_WIDTH {
                let source_x = x * source_width / NEBULA_WIDTH;
                let offset = (source_y * source_width + source_x) * channels;
                let pixel = match info.color_type {
                    png::ColorType::Rgba => [
                        source[offset],
                        source[offset + 1],
                        source[offset + 2],
                        source[offset + 3],
                    ],
                    png::ColorType::Rgb => {
                        [source[offset], source[offset + 1], source[offset + 2], 255]
                    }
                    png::ColorType::GrayscaleAlpha => [
                        source[offset],
                        source[offset],
                        source[offset],
                        source[offset + 1],
                    ],
                    png::ColorType::Grayscale => {
                        [source[offset], source[offset], source[offset], 255]
                    }
                    png::ColorType::Indexed => unreachable!(),
                };
                pixels.push(pixel);
            }
        }
        Some(Self { pixels })
    }
}

pub(crate) fn asteroid_belt(galaxy: Galaxy) -> &'static Model {
    static BELTS: OnceLock<[Model; 3]> = OnceLock::new();
    let belts = BELTS.get_or_init(|| {
        [
            make_dust_belt(
                [[91, 111, 142], [129, 116, 104], [66, 82, 105]],
                &[0.95, 1.98],
                BELT_DUST_COUNT,
            ),
            make_dust_belt(
                [[122, 126, 169], [90, 103, 145], [157, 139, 166]],
                &[0.95, 1.98],
                BELT_DUST_COUNT,
            ),
            make_dust_belt(
                [[137, 112, 91], [93, 112, 108], [171, 153, 121]],
                &[0.52, 1.45, 2.12],
                288,
            ),
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
            make_asteroids(
                [[91, 111, 142], [129, 116, 104], [66, 82, 105]],
                0x13a5_8f21,
                false,
            ),
            make_asteroids(
                [[122, 126, 169], [90, 103, 145], [157, 139, 166]],
                0x7e31_4c99,
                false,
            ),
            make_asteroids(
                [[137, 112, 91], [93, 112, 108], [171, 153, 121]],
                0xc049_827b,
                true,
            ),
        ]
    });
    &fields[match galaxy {
        Galaxy::Home => 0,
        Galaxy::Vesper => 1,
        Galaxy::Nova => 2,
    }]
}

pub(crate) fn draw_nebula(
    framebuffer: &mut Framebuffer,
    galaxy: Galaxy,
    camera_forward: Vec3,
    camera_right: Vec3,
    camera_up: Vec3,
) {
    let (nebula_index, center_longitude) = match galaxy {
        // These are fixed directions in each system's distant sky.
        Galaxy::Home => (0, std::f32::consts::PI),
        Galaxy::Vesper => (1, -std::f32::consts::FRAC_PI_2),
        Galaxy::Nova => (2, std::f32::consts::FRAC_PI_4),
    };
    static NEBULAS: OnceLock<[Option<NebulaTexture>; 3]> = OnceLock::new();
    let nebulae = NEBULAS.get_or_init(|| {
        [
            NebulaTexture::load("assets/space/nebula_home.png"),
            NebulaTexture::load("assets/space/nebula_vesper.png"),
            NebulaTexture::load("assets/space/nebula_nova.png"),
        ]
    });
    let Some(nebula) = nebulae[nebula_index].as_ref() else {
        return;
    };

    // Project camera rays into a fixed celestial direction. Rotation pans the
    // cloud across the view; translation causes no visible near-field parallax.
    let block = (framebuffer.width.div_ceil(NEBULA_WIDTH))
        .max(framebuffer.height.div_ceil(NEBULA_HEIGHT))
        .max(1);
    let focal = framebuffer.height as f32 * 0.5 / (std::f32::consts::PI / 6.0).tan();
    let center_x = framebuffer.width as f32 * 0.5;
    let center_y = framebuffer.height as f32 * 0.5;
    let angular_width = 1.95;
    let angular_height = 1.05;
    let opacity = match galaxy {
        Galaxy::Home => 0.50,
        Galaxy::Vesper => 0.47,
        Galaxy::Nova => 0.50,
    };
    for y in (0..framebuffer.height).step_by(block) {
        for x in (0..framebuffer.width).step_by(block) {
            let ray = camera_forward
                .add(camera_right.mul((x as f32 - center_x) / focal))
                .add(camera_up.mul((center_y - y as f32) / focal))
                .normalize();
            let longitude = ray.z.atan2(ray.x);
            let longitude_delta = (longitude - center_longitude + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            let latitude = ray.y.clamp(-1.0, 1.0).asin();
            let u = 0.5 + longitude_delta / angular_width;
            let v = 0.5 - latitude / angular_height;
            if !(0.0..1.0).contains(&u) || !(0.0..1.0).contains(&v) {
                continue;
            }
            let source_x = (u * NEBULA_WIDTH as f32) as usize;
            let source_y = (v * NEBULA_HEIGHT as f32) as usize;
            let [red, green, blue, alpha] = nebula.pixels[source_y * NEBULA_WIDTH + source_x];
            let alpha = alpha as f32 / 255.0 * opacity;
            if alpha < 0.008 {
                continue;
            }
            let old = framebuffer.pixels[y * framebuffer.width + x];
            let blend = |shift: u32, source: u8| {
                let background = ((old >> shift) & 255) as f32;
                (background + (source as f32 - background) * alpha).clamp(0.0, 255.0) as u32
            };
            let color = (blend(16, red) << 16) | (blend(8, green) << 8) | blend(0, blue);
            for block_y in y..(y + block).min(framebuffer.height) {
                let start = block_y * framebuffer.width + x;
                let end = (x + block).min(framebuffer.width) + block_y * framebuffer.width;
                framebuffer.pixels[start..end].fill(color);
            }
        }
    }
}

/// Draw distant, four-point star bits as a lightweight 2D sky decoration.
/// Their directions are fixed in the sky, so only camera rotation moves them;
/// flying forward does not make the stars drift like nearby particles.
pub(crate) fn draw_star_bits(
    framebuffer: &mut Framebuffer,
    galaxy: Galaxy,
    camera_forward: Vec3,
    camera_right: Vec3,
    camera_up: Vec3,
    time: f32,
) {
    let (sky_rotation, palette) = match galaxy {
        Galaxy::Home => (
            0.38,
            [[255_u32, 231, 163], [151, 229, 255], [255, 177, 222]],
        ),
        Galaxy::Vesper => (
            2.14,
            [[206_u32, 174, 255], [123, 218, 255], [255, 188, 236]],
        ),
        Galaxy::Nova => (
            4.02,
            [[144_u32, 255, 211], [145, 216, 255], [255, 221, 153]],
        ),
    };

    let width = framebuffer.width as f32;
    let height = framebuffer.height as f32;
    let focal = height * 0.5 / (std::f32::consts::PI / 6.0).tan();
    let center_x = width * 0.5;
    let center_y = height * 0.48;
    let golden_angle = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt());

    for index in 0..STAR_BIT_COUNT {
        // Fibonacci sphere gives a stable, evenly distributed star field.
        let vertical = 1.0 - 2.0 * (index as f32 + 0.5) / STAR_BIT_COUNT as f32;
        let horizontal_radius = (1.0 - vertical * vertical).sqrt();
        let angle = index as f32 * golden_angle + sky_rotation;
        let direction = Vec3::new(
            angle.cos() * horizontal_radius,
            vertical,
            angle.sin() * horizontal_radius,
        );
        let depth = direction.dot(camera_forward);
        if depth <= 0.08 {
            continue;
        }

        let screen_x = center_x + direction.dot(camera_right) / depth * focal;
        let screen_y = center_y - direction.dot(camera_up) / depth * focal;
        if screen_x < -12.0
            || screen_y < -12.0
            || screen_x >= width + 12.0
            || screen_y >= height + 12.0
        {
            continue;
        }

        let seed = (index as u32).wrapping_mul(0x9e37_79b9)
            ^ match galaxy {
                Galaxy::Home => 0x13a5_8f21,
                Galaxy::Vesper => 0x7e31_4c99,
                Galaxy::Nova => 0xc049_827b,
            };
        let twinkle_phase = unit(seed ^ 0x85eb_ca6b) * std::f32::consts::TAU;
        let twinkle_speed = 1.0 + unit(seed ^ 0xa511_e9b3) * 1.5;
        let twinkle = 0.62 + ((time * twinkle_speed + twinkle_phase).sin() * 0.5 + 0.5) * 0.38;
        let radius = 2 + (unit(seed ^ 0x68bc_21eb) * 3.0 * twinkle) as i32;
        let color = palette[(index + (hash(seed) as usize % palette.len())) % palette.len()];
        draw_glowing_star_bit(
            framebuffer,
            screen_x.round() as i32,
            screen_y.round() as i32,
            radius,
            color,
            twinkle,
        );
    }
}

fn draw_glowing_star_bit(
    framebuffer: &mut Framebuffer,
    center_x: i32,
    center_y: i32,
    radius: i32,
    color: [u32; 3],
    intensity: f32,
) {
    let glow_radius = radius * 2 + 2;
    for y in -glow_radius..=glow_radius {
        for x in -glow_radius..=glow_radius {
            let distance_squared = (x * x + y * y) as f32;
            if distance_squared > (glow_radius * glow_radius) as f32 {
                continue;
            }
            let falloff = 1.0 - distance_squared.sqrt() / (glow_radius as f32 + 1.0);
            let strength = falloff * falloff * intensity * 0.36;
            add_pixel(framebuffer, center_x + x, center_y + y, color, strength);
        }
    }

    // Long cardinal rays and short diagonal rays create the iconic glint.
    for offset in -radius..=radius {
        let falloff = 1.0 - offset.abs() as f32 / (radius as f32 + 1.0);
        let strength = (0.58 + falloff * 0.34) * intensity;
        add_pixel(framebuffer, center_x + offset, center_y, color, strength);
        add_pixel(framebuffer, center_x, center_y + offset, color, strength);
    }
    for offset in 1..=(radius / 2).max(1) {
        for (dx, dy) in [
            (offset, offset),
            (-offset, offset),
            (offset, -offset),
            (-offset, -offset),
        ] {
            add_pixel(
                framebuffer,
                center_x + dx,
                center_y + dy,
                color,
                0.48 * intensity,
            );
        }
    }
    add_pixel(framebuffer, center_x, center_y, [255, 255, 255], intensity);
}

fn add_pixel(framebuffer: &mut Framebuffer, x: i32, y: i32, color: [u32; 3], strength: f32) {
    if x < 0 || y < 0 || x >= framebuffer.width as i32 || y >= framebuffer.height as i32 {
        return;
    }
    let pixel = &mut framebuffer.pixels[y as usize * framebuffer.width + x as usize];
    let shifts = [16, 8, 0];
    let mut channels = [0_u32; 3];
    for (channel, shift) in shifts.into_iter().enumerate() {
        let background = (*pixel >> shift) & 255;
        channels[channel] =
            (background as f32 + color[channel] as f32 * strength).clamp(0.0, 255.0) as u32;
    }
    *pixel = (channels[0] << 16) | (channels[1] << 8) | channels[2];
}

fn make_asteroids(palette: [[u8; 3]; 3], galaxy_seed: u32, three_bands: bool) -> Vec<Asteroid> {
    let count = if three_bands {
        120
    } else {
        INNER_ASTEROID_COUNT + OUTER_ASTEROID_COUNT
    };
    (0..count)
        .map(|index| {
            let seed = (index as u32).wrapping_mul(7919).wrapping_add(galaxy_seed);
            let band = index % 3;
            let outer = if three_bands {
                band == 2
            } else {
                index >= INNER_ASTEROID_COUNT
            };
            let orbit_fraction = if three_bands {
                [0.52, 1.45, 2.12][band] + (unit(seed ^ 0xa511_e9b3) - 0.5) * 0.075
            } else {
                let outer = index >= INNER_ASTEROID_COUNT;
                if outer {
                    1.98 + unit(seed ^ 0xa511_e9b3) * 0.10
                } else {
                    0.93 + unit(seed ^ 0xa511_e9b3) * 0.14
                }
            };
            let size_scale = if three_bands {
                if band == 2 { 0.88 } else { 1.0 }
            } else if index >= INNER_ASTEROID_COUNT {
                1.30
            } else {
                1.0
            };
            let (model, collision_radius) = make_asteroid(seed, palette, index, size_scale);
            Asteroid {
                model,
                collision_radius,
                orbit_fraction,
                orbit_phase: index as f32 * 2.399_963_1 + (unit(seed) - 0.5) * 0.28,
                // Much slower than the planets; each rock has a slightly
                // different period so the belt does not rotate as one object.
                orbit_speed: if outer {
                    0.0038 + unit(seed ^ 0x63d8_3595) * 0.006
                } else {
                    0.010 + unit(seed ^ 0x63d8_3595) * 0.014
                },
                spin_phase: unit(seed ^ 0x7f4a_7c15) * std::f32::consts::TAU,
                spin_speed: if outer {
                    0.035 + unit(seed ^ 0x243f_6a88) * 0.075
                } else {
                    0.055 + unit(seed ^ 0x243f_6a88) * 0.105
                },
                vertical_offset: (unit(seed ^ 0x369d_ea0f) - 0.5) * 6.0,
            }
        })
        .collect()
}

fn make_asteroid(
    seed: u32,
    palette: [[u8; 3]; 3],
    asteroid: usize,
    size_scale: f32,
) -> (Model, f32) {
    let size = (2.8 + unit(seed ^ 0x9e37_79b9) * 4.7) * size_scale;
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
            let shade =
                0.68 + unit(shade_seed ^ (layer as u32 * 0x45d9_f3b + face_index as u32)) * 0.46;
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
        push_face(
            [top, upper + segment, upper + next],
            ASTEROID_SHAPE_RINGS,
            0,
        );
        push_face(
            [bottom, lower + next, lower + segment],
            ASTEROID_SHAPE_RINGS + 1,
            0,
        );
    }

    (Model::new(vertices, faces), size * 1.15)
}

fn make_dust_belt(palette: [[u8; 3]; 3], bands: &[f32], count: usize) -> Model {
    let mut vertices = Vec::with_capacity(count * 3);
    let mut faces = Vec::with_capacity(count);
    // Fine fragments trace the same narrow orbital lane as the rocks.
    for chip in 0..count {
        let seed = chip as u32 * 65_537 + 0x51ed_270b;
        let angle = unit(seed) * std::f32::consts::TAU;
        let band = bands[chip % bands.len()];
        let radius = band + (unit(seed ^ 0x9e37_79b9) - 0.5) * 0.10;
        let center = Vec3::new(
            angle.cos() * radius,
            angle.sin() * radius * 0.055 + (unit(seed ^ 0x63d8_3595) - 0.5) * 0.035,
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

pub(crate) fn asteroid_position(
    asteroid: &Asteroid,
    center: Vec3,
    base_radius: f32,
    time: f32,
) -> Vec3 {
    let angle = asteroid.orbit_phase + time * asteroid.orbit_speed;
    let (sin, cos) = angle.sin_cos();
    let radius = asteroid.orbit_fraction * base_radius;
    center.add(Vec3::new(
        cos * radius,
        sin * radius * 0.055 + asteroid.vertical_offset,
        sin * radius,
    ))
}

/// Sweep the ship's movement segment against the visible asteroid field so
/// high speed cannot tunnel through rocks between rendered frames.
pub(crate) fn asteroid_collision_position(
    previous: Vec3,
    current: Vec3,
    ship_radius: f32,
    galaxy: Galaxy,
    center: Vec3,
    base_radius: f32,
    time: f32,
) -> Option<Vec3> {
    let movement = current.sub(previous);
    let movement_length_squared = movement.dot(movement);
    let mut earliest: Option<(f32, Vec3)> = None;

    for asteroid in asteroids(galaxy) {
        let rock = asteroid_position(asteroid, center, base_radius, time);
        let t = if movement_length_squared > 0.0001 {
            rock.sub(previous).dot(movement) / movement_length_squared
        } else {
            0.0
        }
        .clamp(0.0, 1.0);
        let closest = previous.add(movement.mul(t));
        let delta = closest.sub(rock);
        let distance_squared = delta.dot(delta);
        let combined_radius = ship_radius.max(1.0) + asteroid.collision_radius;
        if distance_squared >= combined_radius * combined_radius {
            continue;
        }
        if earliest.is_some_and(|(best_t, _)| t >= best_t) {
            continue;
        }
        let distance = distance_squared.sqrt();
        let normal = if distance > 0.001 {
            delta.mul(1.0 / distance)
        } else {
            let heading = current.sub(rock).normalize();
            if heading.dot(heading) > 0.5 {
                heading
            } else {
                let prior = previous.sub(rock).normalize();
                if prior.dot(prior) > 0.5 {
                    prior
                } else {
                    Vec3::new(1.0, 0.0, 0.0)
                }
            }
        };
        earliest = Some((t, closest.add(normal.mul(combined_radius + 0.4))));
    }
    earliest.map(|(_, resolved)| resolved)
}

pub(crate) fn draw_dust(framebuffer: &mut Framebuffer, travel: f32, galaxy: Galaxy) {
    let tint = match galaxy {
        Galaxy::Home => [94_u32, 136_u32, 204_u32],
        Galaxy::Vesper => [160, 125, 222],
        Galaxy::Nova => [220, 116, 130],
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
