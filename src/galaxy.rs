use crate::{framebuffer::Framebuffer, obj_loader::Vec3};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Galaxy {
    Home,
    Vesper,
}

pub(crate) struct GalaxyTransit {
    pub elapsed: f32,
    pub start_position: Vec3,
    pub destination: Galaxy,
    pub switched: bool,
}

pub(crate) fn wormhole_position(galaxy: Galaxy) -> Vec3 {
    match galaxy {
        // With the ship's default heading (-X), negative Z places the portal
        // on the right side of the view, opposite the pink planet.
        Galaxy::Home => Vec3::new(-260.0, 48.0, -130.0),
        // Keep the return portal well clear of the new planet cluster.
        Galaxy::Vesper => Vec3::new(350.0, 48.0, 600.0),
    }
}

pub(crate) fn crossed_wormhole(previous: Vec3, current: Vec3, center: Vec3, radius: f32) -> bool {
    let movement = current.sub(previous);
    let length_squared = movement.dot(movement);
    let closest_t = if length_squared > 0.0001 {
        center.sub(previous).dot(movement) / length_squared
    } else {
        0.0
    }
    .clamp(0.0, 1.0);
    let closest = previous.add(movement.mul(closest_t));
    let offset = closest.sub(center);
    offset.dot(offset) < radius * radius
}

fn hash(mut value: u32) -> u32 {
    value ^= value >> 16;
    value = value.wrapping_mul(0x7feb_352d);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846c_a68b);
    value ^ (value >> 16)
}

pub(crate) fn draw_hyperspace_transition(framebuffer: &mut Framebuffer, elapsed: f32) {
    let fade = if elapsed < 0.22 {
        elapsed / 0.22
    } else if elapsed > 1.72 {
        ((2.15 - elapsed) / 0.43).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let center_x = framebuffer.width as f32 * 0.5;
    let center_y = framebuffer.height as f32 * 0.48;
    let intensity = (fade * 255.0) as u32;
    for index in 0..260_u32 {
        let seed = hash(index.wrapping_mul(7919).wrapping_add(53));
        let angle = seed as f32 / u32::MAX as f32 * std::f32::consts::TAU;
        let (sin, cos) = angle.sin_cos();
        let inner = 0.04 + (hash(seed ^ 0xa511e9b3) as f32 / u32::MAX as f32) * 0.48;
        let outer = (1.18 + elapsed * 1.8).min(2.7);
        let x0 = center_x + cos * inner * framebuffer.width as f32 * 0.5;
        let y0 = center_y + sin * inner * framebuffer.height as f32 * 0.5;
        let x1 = center_x + cos * outer * framebuffer.width as f32 * 0.5;
        let y1 = center_y + sin * outer * framebuffer.height as f32 * 0.5;
        let steps = ((x1 - x0).abs().max((y1 - y0).abs()) as usize).max(1);
        let brightness = (intensity * (170 + (seed & 0x55)) / 255).min(255);
        let color = (brightness << 16) | ((brightness * 228 / 255) << 8) | brightness;
        for step in 0..=steps {
            let t = step as f32 / steps as f32;
            let x = (x0 + (x1 - x0) * t) as i32;
            let y = (y0 + (y1 - y0) * t) as i32;
            if x >= 0 && y >= 0 && x < framebuffer.width as i32 && y < framebuffer.height as i32 {
                framebuffer.pixels[y as usize * framebuffer.width + x as usize] = color;
            }
        }
    }
    if elapsed > 0.70 && elapsed < 0.88 {
        let blend = ((elapsed - 0.70) / 0.18)
            .min((0.88 - elapsed) / 0.18)
            .clamp(0.0, 1.0);
        if blend > 0.72 {
            framebuffer.pixels.fill(0x02030b);
        }
    }
}
