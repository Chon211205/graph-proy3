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

#[allow(dead_code)]
pub fn draw_arena(frame: &mut Framebuffer, time: f32) {
    for y in 0..frame.height {
        let t = y as f32 / frame.height.max(1) as f32;
        let upper = [35.0, 30.0, 27.0];
        let lower = [73.0, 57.0, 42.0];
        let color =
            std::array::from_fn::<_, 3, _>(|i| (upper[i] * (1.0 - t) + lower[i] * t) as u32);
        frame.pixels[y * frame.width..(y + 1) * frame.width]
            .fill((color[0] << 16) | (color[1] << 8) | color[2]);
    }

    // The firing post is inside the old house. A single open doorway gives
    // enemies a readable entrance while the room's beams frame the action.
    polygon(
        frame,
        &[(0.0, 0.0), (1.0, 0.0), (0.94, 0.20), (0.06, 0.20)],
        0x171a1c,
    );
    polygon(
        frame,
        &[(0.0, 0.18), (1.0, 0.18), (0.90, 0.68), (0.10, 0.68)],
        0x39302a,
    );
    polygon(
        frame,
        &[(0.0, 0.65), (1.0, 0.65), (1.0, 1.0), (0.0, 1.0)],
        0x282522,
    );

    draw_wall_panels(frame);
    draw_doorway(frame, time);
    draw_wall_lamps(frame, time);
    draw_floor(frame);
}

#[allow(dead_code)]
fn draw_wall_panels(frame: &mut Framebuffer) {
    // Dark timber posts and cracked plaster make the camera feel sheltered.
    for (x, width) in [
        (0.035, 0.055),
        (0.22, 0.025),
        (0.48, 0.022),
        (0.76, 0.025),
        (0.91, 0.055),
    ] {
        polygon(
            frame,
            &[
                (x, 0.18),
                (x + width, 0.19),
                (x + width * 0.8, 0.68),
                (x - 0.01, 0.68),
            ],
            0x211f1c,
        );
        line(
            frame,
            x + width * 0.72,
            0.22,
            x + width * 0.66,
            0.65,
            0x574638,
            2.0,
        );
    }
    for y in [0.22, 0.38, 0.55, 0.66] {
        line(frame, 0.06, y, 0.94, y + 0.012, 0x282523, 3.0);
    }
    // Broken boarded windows on either side of the entry show the cold night
    // outside without turning the whole arena back into an exterior scene.
    for side in [0.10, 0.78] {
        rect(frame, side, 0.27, 0.12, 0.19, 0x101d28);
        rect(frame, side - 0.012, 0.255, 0.144, 0.018, 0x72583a);
        rect(frame, side - 0.012, 0.452, 0.144, 0.018, 0x72583a);
        rect(frame, side - 0.006, 0.27, 0.012, 0.19, 0x5b4935);
        rect(frame, side + 0.106, 0.27, 0.012, 0.19, 0x5b4935);
        line(frame, side + 0.01, 0.32, side + 0.10, 0.40, 0x41372d, 8.0);
        line(frame, side + 0.10, 0.31, side + 0.01, 0.40, 0x41372d, 7.0);
        rect(frame, side + 0.008, 0.365, 0.102, 0.022, 0x463729);
    }
    // A small framed warning sign adds a house detail, not another HUD panel.
    rect(frame, 0.835, 0.235, 0.055, 0.065, 0x211d19);
    rect(frame, 0.842, 0.242, 0.041, 0.051, 0x9b6935);
    line(frame, 0.849, 0.252, 0.876, 0.280, 0x30241b, 2.0);
    line(frame, 0.876, 0.252, 0.849, 0.280, 0x30241b, 2.0);
}

#[allow(dead_code)]
fn draw_doorway(frame: &mut Framebuffer, time: f32) {
    // Recess and broken door leaves create a clear entrance lane.
    polygon(
        frame,
        &[(0.34, 0.20), (0.66, 0.20), (0.70, 0.67), (0.30, 0.67)],
        0x171615,
    );
    polygon(
        frame,
        &[(0.37, 0.23), (0.63, 0.23), (0.65, 0.64), (0.35, 0.64)],
        0x0d151b,
    );
    polygon(
        frame,
        &[(0.32, 0.20), (0.37, 0.23), (0.35, 0.64), (0.30, 0.67)],
        0x514332,
    );
    polygon(
        frame,
        &[(0.63, 0.23), (0.68, 0.20), (0.70, 0.67), (0.65, 0.64)],
        0x514332,
    );
    line(frame, 0.34, 0.21, 0.66, 0.21, 0xb0844b, 4.0);
    // A dim amber lamp and floor spill keep sprites legible in the dark room.
    let pulse = ((time * 1.7).sin() * 0.5 + 0.5) * 0.10;
    rect(
        frame,
        0.477,
        0.245,
        0.046,
        0.018,
        if pulse > 0.05 { 0xf3b65e } else { 0xc78a3c },
    );
    polygon(
        frame,
        &[(0.38, 0.62), (0.62, 0.62), (0.73, 0.72), (0.27, 0.72)],
        0x57402d,
    );
    for y in [0.56, 0.60, 0.64] {
        line(frame, 0.30, y, 0.70, y, 0x735536, 1.5);
    }
}

#[allow(dead_code)]
fn draw_wall_lamps(frame: &mut Framebuffer, time: f32) {
    for x in [0.27, 0.73] {
        line(frame, x, 0.23, x, 0.27, 0x1c1b18, 2.0);
        rect(frame, x - 0.015, 0.265, 0.03, 0.052, 0x49321f);
        rect(frame, x - 0.009, 0.273, 0.018, 0.035, 0xe79a3d);
        let shimmer = ((time * 2.2 + x).sin() * 0.5 + 0.5) * 0.16;
        rect(
            frame,
            x - 0.005,
            0.278,
            0.010,
            0.023,
            if shimmer > 0.08 { 0xffd17a } else { 0xf0a84c },
        );
    }
}

#[allow(dead_code)]
fn draw_floor(frame: &mut Framebuffer) {
    polygon(
        frame,
        &[(0.10, 0.65), (0.90, 0.65), (1.0, 1.0), (0.0, 1.0)],
        0x40382e,
    );
    polygon(
        frame,
        &[(0.34, 0.65), (0.66, 0.65), (0.84, 1.0), (0.16, 1.0)],
        0x4d4032,
    );
    for i in 1..9 {
        let t = i as f32 / 9.0;
        let y = 0.67 + t * t * 0.32;
        line(
            frame,
            0.0,
            y,
            1.0,
            y,
            if i % 2 == 0 { 0x302c27 } else { 0x5b4935 },
            1.4,
        );
    }
    for x in [0.12, 0.27, 0.40, 0.60, 0.73, 0.88] {
        line(frame, x, 0.66, 0.5 + (x - 0.5) * 2.1, 0.99, 0x302b26, 1.2);
    }
    // Scuffed runner rug leads toward the doorway.
    polygon(
        frame,
        &[(0.41, 0.67), (0.59, 0.67), (0.70, 1.0), (0.30, 1.0)],
        0x382d28,
    );
    line(frame, 0.42, 0.68, 0.31, 0.99, 0x786044, 2.0);
    line(frame, 0.58, 0.68, 0.69, 0.99, 0x786044, 2.0);
}

pub fn draw_foreground_cover(frame: &mut Framebuffer) {
    for (index, cover) in FOREGROUND_COVERS.iter().enumerate() {
        let width = cover.right - cover.left;
        let height = cover.bottom - cover.top;
        let color = if index % 2 == 0 { 0x37332d } else { 0x45392e };
        rect(frame, cover.left, cover.top, width, height, color);
        rect(
            frame,
            cover.left + width * 0.04,
            cover.top + height * 0.08,
            width * 0.92,
            height * 0.16,
            0x665039,
        );
        line(
            frame,
            cover.left,
            cover.bottom,
            cover.right,
            cover.bottom - height * 0.06,
            0x242321,
            3.0,
        );
        let bolt_count = (width * frame.width as f32 / 24.0).max(2.0) as usize;
        for bolt in 0..bolt_count {
            let x = cover.left + width * (bolt as f32 + 0.5) / bolt_count as f32;
            circle(frame, x, cover.top + height * 0.14, 2.0, 0xb78143);
        }
    }
    for x in [0.28, 0.50, 0.72] {
        circle(frame, x, 0.94, 3.0, 0xdb5140);
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

#[allow(dead_code)]
fn polygon(frame: &mut Framebuffer, points: &[(f32, f32)], color: u32) {
    if points.len() < 3 {
        return;
    }
    let start = (points.iter().map(|p| p.1).fold(1.0_f32, f32::min) * frame.height as f32)
        .floor()
        .max(0.0) as usize;
    let end = (points.iter().map(|p| p.1).fold(0.0_f32, f32::max) * frame.height as f32)
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
