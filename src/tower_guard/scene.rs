use crate::framebuffer::Framebuffer;

#[derive(Clone, Copy)]
struct ScreenRect {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
    depth: f32,
}

const FOREGROUND_COVERS: [ScreenRect; 4] = [
    ScreenRect {
        left: 0.00,
        top: 0.70,
        right: 0.16,
        bottom: 0.90,
        depth: 0.76,
    },
    ScreenRect {
        left: 0.20,
        top: 0.82,
        right: 0.40,
        bottom: 0.93,
        depth: 0.92,
    },
    ScreenRect {
        left: 0.73,
        top: 0.81,
        right: 0.91,
        bottom: 0.94,
        depth: 0.91,
    },
    ScreenRect {
        left: 0.88,
        top: 0.67,
        right: 1.00,
        bottom: 0.89,
        depth: 0.77,
    },
];

pub fn cover_blocks(x: f32, y: f32, target_depth: f32) -> bool {
    FOREGROUND_COVERS.iter().any(|cover| {
        x >= cover.left
            && x <= cover.right
            && y >= cover.top
            && y <= cover.bottom
            && target_depth < cover.depth
    })
}

pub fn draw_arena(frame: &mut Framebuffer, time: f32) {
    let w = frame.width as f32;
    let h = frame.height as f32;
    for y in 0..frame.height {
        let t = y as f32 / frame.height.max(1) as f32;
        let top = [15.0, 23.0, 34.0];
        let bottom = [49.0, 44.0, 39.0];
        let haze = ((t - 0.42).abs() * 2.0).clamp(0.0, 1.0);
        let rgb = std::array::from_fn::<_, 3, _>(|i| {
            (top[i] * (1.0 - t) + bottom[i] * t + (1.0 - haze) * [8.0, 8.0, 6.0][i]) as u32
        });
        let color = (rgb[0] << 16) | (rgb[1] << 8) | rgb[2];
        frame.pixels[y * frame.width..(y + 1) * frame.width].fill(color);
    }

    // Moonlit haze behind the industrial tower.
    for band in 0..12 {
        let y = (h * (0.26 + band as f32 * 0.025)) as usize;
        let alpha = 0.025 + (11 - band) as f32 * 0.003;
        rect(
            frame,
            0.0,
            y as f32,
            w,
            2.0 + band as f32 * 0.35,
            tint(0x88929b, alpha),
        );
    }
    // Distant roofline and ruined silhouettes.
    polygon(
        frame,
        &[
            (0.0, 0.40),
            (0.12, 0.27),
            (0.19, 0.35),
            (0.31, 0.22),
            (0.40, 0.39),
        ],
        0x1b242b,
    );
    polygon(
        frame,
        &[
            (0.61, 0.39),
            (0.73, 0.24),
            (0.80, 0.33),
            (0.90, 0.20),
            (1.0, 0.36),
        ],
        0x1b242b,
    );

    draw_tower(frame, time);
    draw_courtyard(frame);
    draw_back_cover(frame);
}

pub fn draw_foreground_cover(frame: &mut Framebuffer) {
    for (index, cover) in FOREGROUND_COVERS.iter().enumerate() {
        let width = cover.right - cover.left;
        let height = cover.bottom - cover.top;
        let color = if index % 2 == 0 { 0x373b3c } else { 0x403d38 };
        rect(frame, cover.left, cover.top, width, height, color);
        rect(
            frame,
            cover.left + width * 0.04,
            cover.top + height * 0.08,
            width * 0.92,
            height * 0.16,
            0x565044,
        );
        line(
            frame,
            cover.left,
            cover.bottom,
            cover.right,
            cover.bottom - height * 0.06,
            0x242a2d,
            3.0,
        );
        let bolt_count = (width * frame.width as f32 / 24.0).max(2.0) as usize;
        for bolt in 0..bolt_count {
            let x = cover.left + width * (bolt as f32 + 0.5) / bolt_count as f32;
            circle(frame, x, cover.top + height * 0.14, 2.0, 0xc18b3d);
        }
    }
    // Warm, restrained warning lamps around the firing line.
    for x in [0.28, 0.50, 0.72] {
        circle(frame, x, 0.94, 3.0, 0xdb5140);
    }
}

fn draw_tower(frame: &mut Framebuffer, time: f32) {
    // Outer walls, buttresses and roofline.
    polygon(
        frame,
        &[
            (0.12, 0.46),
            (0.19, 0.22),
            (0.28, 0.18),
            (0.31, 0.11),
            (0.69, 0.11),
            (0.73, 0.20),
            (0.84, 0.25),
            (0.88, 0.51),
        ],
        0x24292b,
    );
    rect(frame, 0.17, 0.31, 0.66, 0.28, 0x343634);
    rect(frame, 0.23, 0.22, 0.54, 0.18, 0x3a3934);
    polygon(
        frame,
        &[(0.30, 0.23), (0.34, 0.12), (0.66, 0.12), (0.71, 0.23)],
        0x464037,
    );
    polygon(
        frame,
        &[(0.35, 0.13), (0.43, 0.07), (0.56, 0.07), (0.65, 0.13)],
        0x272d31,
    );
    rect(frame, 0.15, 0.48, 0.70, 0.04, 0x1c2326);
    rect(frame, 0.18, 0.50, 0.64, 0.018, 0x91805d);

    // Structural columns and cross braces create readable foreground depth.
    for x in [0.205, 0.29, 0.43, 0.57, 0.71, 0.795] {
        rect(frame, x, 0.22, 0.018, 0.31, 0x5a5142);
        rect(frame, x + 0.018, 0.27, 0.009, 0.25, 0x292f31);
    }
    line(frame, 0.18, 0.38, 0.30, 0.27, 0x1b2326, 5.0);
    line(frame, 0.82, 0.38, 0.70, 0.27, 0x1b2326, 5.0);
    line(frame, 0.28, 0.50, 0.43, 0.39, 0x23292b, 4.0);
    line(frame, 0.72, 0.50, 0.57, 0.39, 0x23292b, 4.0);

    // Amber windows, with a few lights flickering softly.
    for row in 0..3 {
        for col in 0..7 {
            let x = 0.245 + col as f32 * 0.075;
            let y = 0.285 + row as f32 * 0.062;
            let pulse = ((time * 0.8 + (row * 7 + col) as f32 * 1.7).sin() * 0.5 + 0.5) * 0.22;
            rect(frame, x, y, 0.028, 0.038, tint(0xf2a846, 0.74 + pulse));
            rect(frame, x + 0.004, y + 0.004, 0.020, 0.029, 0xffcd72);
            line(frame, x + 0.014, y, x + 0.014, y + 0.035, 0x625139, 1.0);
        }
    }

    // Central gate, approach steps and illuminated warning frame.
    rect(frame, 0.453, 0.39, 0.094, 0.18, 0x171c20);
    rect(frame, 0.463, 0.401, 0.074, 0.16, 0x4a3024);
    rect(frame, 0.477, 0.414, 0.046, 0.147, 0x211f1b);
    line(frame, 0.45, 0.39, 0.45, 0.57, 0xd89a43, 3.0);
    line(frame, 0.55, 0.39, 0.55, 0.57, 0xd89a43, 3.0);
    for step in 0..5 {
        let width = 0.08 + step as f32 * 0.035;
        let top = 0.55 + step as f32 * 0.022;
        rect(frame, 0.5 - width * 0.5, top, width, 0.016, 0x54473b);
        line(
            frame,
            0.5 - width * 0.5,
            top,
            0.5 + width * 0.5,
            top,
            0x88724d,
            1.5,
        );
    }
    // Side antennae and broken lamps.
    for side in [0.13, 0.85] {
        line(frame, side, 0.50, side + 0.02, 0.24, 0x434648, 5.0);
        line(frame, side + 0.02, 0.24, side + 0.09, 0.19, 0x594a38, 3.0);
        circle(frame, side + 0.09, 0.19, 4.0, 0xc94b36);
    }
}

fn draw_courtyard(frame: &mut Framebuffer) {
    polygon(
        frame,
        &[(0.0, 0.52), (1.0, 0.52), (1.0, 1.0), (0.0, 1.0)],
        0x353332,
    );
    polygon(
        frame,
        &[(0.34, 0.52), (0.66, 0.52), (0.91, 1.0), (0.09, 1.0)],
        0x47423b,
    );
    polygon(
        frame,
        &[(0.39, 0.53), (0.61, 0.53), (0.75, 1.0), (0.25, 1.0)],
        0x50483d,
    );
    // Perspective joints in the paved floor.
    for i in 1..9 {
        let t = i as f32 / 9.0;
        let y = 0.55 + t * t * 0.43;
        line(
            frame,
            0.0,
            y,
            1.0,
            y,
            if i % 2 == 0 { 0x3c3935 } else { 0x423d38 },
            1.0,
        );
    }
    for x in [0.10, 0.22, 0.36, 0.64, 0.78, 0.90] {
        line(frame, x, 0.55, x * 0.28 + 0.36, 0.99, 0x3c3935, 1.2);
    }
    // Red painted approach lanes and dim warning strips.
    for side in [-1.0_f32, 1.0] {
        let x0 = 0.5 + side * 0.14;
        let x1 = 0.5 + side * 0.42;
        line(frame, x0, 0.60, x1, 0.96, 0x9d4939, 4.0);
        for tick in 0..5 {
            let t = tick as f32 / 5.0;
            let x = x0 + (x1 - x0) * t;
            let y = 0.60 + 0.36 * t;
            line(frame, x - 0.018, y, x + 0.018, y + 0.006, 0xc27c3d, 2.0);
        }
    }
}

fn draw_back_cover(frame: &mut Framebuffer) {
    for (x, y, width, height) in [
        (0.035, 0.63, 0.12, 0.055),
        (0.22, 0.70, 0.12, 0.045),
        (0.70, 0.69, 0.12, 0.048),
        (0.84, 0.62, 0.13, 0.058),
    ] {
        rect(frame, x, y, width, height, 0x514a3e);
        rect(frame, x, y, width, height * 0.16, 0x8a7043);
        line(
            frame,
            x + 0.02,
            y + height,
            x + width * 0.92,
            y + height,
            0x20282d,
            2.0,
        );
    }
}

fn rect(frame: &mut Framebuffer, x: f32, y: f32, width: f32, height: f32, color: u32) {
    let left = (x * frame.width as f32).floor().max(0.0) as usize;
    let top = (y * frame.height as f32).floor().max(0.0) as usize;
    let right = ((x + width) * frame.width as f32)
        .ceil()
        .min(frame.width as f32) as usize;
    let bottom = ((y + height) * frame.height as f32)
        .ceil()
        .min(frame.height as f32) as usize;
    for row in top..bottom {
        frame.pixels[row * frame.width + left..row * frame.width + right].fill(color);
    }
}

fn polygon(frame: &mut Framebuffer, points: &[(f32, f32)], color: u32) {
    if points.len() < 3 {
        return;
    }
    let min_y = points.iter().map(|p| p.1).fold(1.0_f32, f32::min);
    let max_y = points.iter().map(|p| p.1).fold(0.0_f32, f32::max);
    let start = (min_y * frame.height as f32).floor().max(0.0) as usize;
    let end = (max_y * frame.height as f32)
        .ceil()
        .min(frame.height as f32) as usize;
    for y in start..end {
        let scan = (y as f32 + 0.5) / frame.height as f32;
        let mut intersections = Vec::new();
        for i in 0..points.len() {
            let (x0, y0) = points[i];
            let (x1, y1) = points[(i + 1) % points.len()];
            if (y0 <= scan && scan < y1) || (y1 <= scan && scan < y0) {
                intersections.push(x0 + (scan - y0) * (x1 - x0) / (y1 - y0));
            }
        }
        intersections.sort_by(f32::total_cmp);
        for span in intersections.chunks_exact(2) {
            let left = (span[0] * frame.width as f32).floor().max(0.0) as usize;
            let right = (span[1] * frame.width as f32)
                .ceil()
                .min(frame.width as f32) as usize;
            if left < right {
                frame.pixels[y * frame.width + left..y * frame.width + right].fill(color);
            }
        }
    }
}

fn line(frame: &mut Framebuffer, x0: f32, y0: f32, x1: f32, y1: f32, color: u32, thickness: f32) {
    let x0 = x0 * frame.width as f32;
    let y0 = y0 * frame.height as f32;
    let x1 = x1 * frame.width as f32;
    let y1 = y1 * frame.height as f32;
    let steps = ((x1 - x0).abs().max((y1 - y0).abs()) as usize).max(1);
    let radius = thickness * frame.height as f32 / 800.0 * 0.5;
    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        circle_px(frame, x0 + (x1 - x0) * t, y0 + (y1 - y0) * t, radius, color);
    }
}

fn circle(frame: &mut Framebuffer, x: f32, y: f32, radius: f32, color: u32) {
    circle_px(
        frame,
        x * frame.width as f32,
        y * frame.height as f32,
        radius,
        color,
    );
}

fn circle_px(frame: &mut Framebuffer, x: f32, y: f32, radius: f32, color: u32) {
    let rx = radius.max(1.0);
    let ry = rx * frame.height as f32 / frame.width.max(1) as f32;
    let left = (x - rx).floor().max(0.0) as usize;
    let right = (x + rx).ceil().min(frame.width as f32) as usize;
    let top = (y - ry).floor().max(0.0) as usize;
    let bottom = (y + ry).ceil().min(frame.height as f32) as usize;
    for py in top..bottom {
        for px in left..right {
            let dx = (px as f32 + 0.5 - x) / rx;
            let dy = (py as f32 + 0.5 - y) / ry;
            if dx * dx + dy * dy <= 1.0 {
                frame.pixels[py * frame.width + px] = color;
            }
        }
    }
}

fn tint(color: u32, opacity: f32) -> u32 {
    let opacity = opacity.clamp(0.0, 1.0);
    let r = (((color >> 16) & 255) as f32 * opacity) as u32;
    let g = (((color >> 8) & 255) as f32 * opacity) as u32;
    let b = ((color & 255) as f32 * opacity) as u32;
    (r << 16) | (g << 8) | b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn barricades_block_enemies_behind_them_but_not_in_front() {
        assert!(cover_blocks(0.10, 0.78, 0.42));
        assert!(!cover_blocks(0.10, 0.78, 0.80));
        assert!(!cover_blocks(0.50, 0.78, 0.42));
    }
}
