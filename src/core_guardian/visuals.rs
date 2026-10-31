use super::{CoreGuardian, Phase, STARTING_SHIELDS};
use crate::framebuffer::Framebuffer;

impl CoreGuardian {
    pub fn render_scene(&self, framebuffer: &mut Framebuffer) {
        let width = framebuffer.width as f32;
        let height = framebuffer.height as f32;
        draw_cavern(framebuffer, self.elapsed);
        let boss_x = width * (0.5 + (self.elapsed * 0.48).sin() * 0.035);
        let boss_y = height * 0.34;
        circle(framebuffer, boss_x, boss_y, height * 0.14, 0xb84967);
        circle(
            framebuffer,
            boss_x,
            boss_y,
            height * 0.055,
            if self.phase == Phase::Exposed {
                0xffc43c
            } else {
                0x8c4b3a
            },
        );
        let plates = [(0.0, -0.075), (-0.072, 0.045), (0.072, 0.045)];
        for (index, (offset_x, offset_y)) in plates.into_iter().enumerate() {
            let opening = self.opening_amount(index);
            circle(
                framebuffer,
                boss_x + offset_x * width * (1.0 + opening),
                boss_y + offset_y * height - opening * height * 0.07,
                height * 0.052,
                if self.phase == Phase::Warning && index == self.open_plate {
                    0xf19a75
                } else {
                    0xce6a75
                },
            );
        }
        for shot in self.player_shots.iter().filter(|shot| shot.alive) {
            circle(framebuffer, shot.x * width, shot.y * height, 4.0, 0xfff0a8);
        }
        for seed in self.seeds.iter().filter(|seed| seed.alive) {
            circle(framebuffer, seed.x * width, seed.y * height, 10.0, 0xffbb2e);
        }
        let player_x = self.player_x * width;
        let player_y = (0.82 + self.player_y * 0.10) * height;
        triangle(
            framebuffer,
            (player_x, player_y - height * 0.09),
            (player_x - height * 0.10, player_y + height * 0.04),
            (player_x + height * 0.10, player_y + height * 0.04),
            0xe8443b,
        );
    }

    pub fn draw_hud(&self, framebuffer: &mut Framebuffer) {
        let width = framebuffer.width as f32;
        let left = width * 0.33;
        let bar_width = width * 0.34;
        rect(framebuffer, left, 34.0, bar_width, 12.0, 0x350c20);
        rect(
            framebuffer,
            left + 2.0,
            36.0,
            (bar_width - 4.0) * self.health_ratio(),
            8.0,
            0xf62e58,
        );
        for index in 0..STARTING_SHIELDS {
            circle(
                framebuffer,
                28.0 + index as f32 * 34.0,
                31.0,
                12.0,
                if index < self.shields {
                    0xffd8e5
                } else {
                    0x67404f
                },
            );
        }
    }
}

fn draw_cavern(framebuffer: &mut Framebuffer, elapsed: f32) {
    let width = framebuffer.width as f32;
    let height = framebuffer.height as f32;
    if framebuffer.width == 0 || framebuffer.height == 0 {
        return;
    }
    for y in 0..framebuffer.height {
        let depth = y as f32 / framebuffer.height.max(1) as f32;
        let pulse = (elapsed * 0.8 + depth * 4.0).sin() * 3.0;
        let red = (35.0 + depth * 55.0 + pulse) as u32;
        let green = (9.0 + depth * 15.0 + pulse * 0.3) as u32;
        let blue = (31.0 + depth * 22.0) as u32;
        let color = (red << 16) | (green << 8) | blue;
        let row = y * framebuffer.width;
        framebuffer.pixels[row..row + framebuffer.width].fill(color);
    }

    // Deep rock silhouettes frame a clear central lane for the battle.
    let horizon = height * 0.38;
    polygon(
        framebuffer,
        &[
            (0.0, 0.0),
            (width * 0.28, 0.0),
            (width * 0.20, horizon * 0.72),
            (width * 0.34, horizon),
            (width * 0.18, horizon * 1.18),
            (width * 0.25, height * 0.68),
            (width * 0.14, height),
            (0.0, height),
        ],
        0x51213f,
    );
    polygon(
        framebuffer,
        &[
            (width, 0.0),
            (width * 0.72, 0.0),
            (width * 0.80, horizon * 0.72),
            (width * 0.66, horizon),
            (width * 0.82, horizon * 1.18),
            (width * 0.75, height * 0.68),
            (width * 0.86, height),
            (width, height),
        ],
        0x49203d,
    );

    // Repeating ledges imply forward movement without a sky or a star field.
    let travel = (elapsed * 34.0) % (height * 0.22);
    for index in 0..4 {
        let y = horizon + index as f32 * height * 0.16 + travel;
        if y > height * 0.95 {
            continue;
        }
        line(
            framebuffer,
            (width * 0.08, y),
            (width * 0.37, y - height * 0.035),
            5.0,
            0x8b365a,
        );
        line(
            framebuffer,
            (width * 0.63, y - height * 0.035),
            (width * 0.92, y),
            5.0,
            0x783052,
        );
    }
}

fn polygon(framebuffer: &mut Framebuffer, points: &[(f32, f32)], color: u32) {
    if points.len() < 3 {
        return;
    }
    for index in 1..points.len() - 1 {
        triangle(
            framebuffer,
            points[0],
            points[index],
            points[index + 1],
            color,
        );
    }
}

fn line(
    framebuffer: &mut Framebuffer,
    from: (f32, f32),
    to: (f32, f32),
    thickness: f32,
    color: u32,
) {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let length = (dx * dx + dy * dy).sqrt().max(1.0);
    let steps = (length / 3.0).ceil() as usize;
    for step in 0..=steps {
        let t = step as f32 / steps.max(1) as f32;
        circle(
            framebuffer,
            from.0 + dx * t,
            from.1 + dy * t,
            thickness * 0.5,
            color,
        );
    }
}

fn rect(framebuffer: &mut Framebuffer, x: f32, y: f32, width: f32, height: f32, color: u32) {
    for py in (y as i32).max(0)..((y + height) as i32).min(framebuffer.height as i32) {
        for px in (x as i32).max(0)..((x + width) as i32).min(framebuffer.width as i32) {
            framebuffer.pixels[py as usize * framebuffer.width + px as usize] = color;
        }
    }
}

fn circle(framebuffer: &mut Framebuffer, cx: f32, cy: f32, radius: f32, color: u32) {
    let r2 = radius * radius;
    for y in ((cy - radius).floor() as i32).max(0)
        ..=((cy + radius).ceil() as i32).min(framebuffer.height as i32 - 1)
    {
        for x in ((cx - radius).floor() as i32).max(0)
            ..=((cx + radius).ceil() as i32).min(framebuffer.width as i32 - 1)
        {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            if dx * dx + dy * dy <= r2 {
                framebuffer.pixels[y as usize * framebuffer.width + x as usize] = color;
            }
        }
    }
}

fn triangle(
    framebuffer: &mut Framebuffer,
    a: (f32, f32),
    b: (f32, f32),
    c: (f32, f32),
    color: u32,
) {
    let area = (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
    if area.abs() < 0.01 {
        return;
    }
    let left = a.0.min(b.0).min(c.0).floor().max(0.0) as i32;
    let right =
        a.0.max(b.0)
            .max(c.0)
            .ceil()
            .min(framebuffer.width as f32 - 1.0) as i32;
    let top = a.1.min(b.1).min(c.1).floor().max(0.0) as i32;
    let bottom =
        a.1.max(b.1)
            .max(c.1)
            .ceil()
            .min(framebuffer.height as f32 - 1.0) as i32;
    for y in top..=bottom {
        for x in left..=right {
            let p = (x as f32 + 0.5, y as f32 + 0.5);
            let e0 = (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0);
            let e1 = (c.0 - b.0) * (p.1 - b.1) - (c.1 - b.1) * (p.0 - b.0);
            let e2 = (a.0 - c.0) * (p.1 - c.1) - (a.1 - c.1) * (p.0 - c.0);
            if (e0 >= 0.0 && e1 >= 0.0 && e2 >= 0.0) || (e0 <= 0.0 && e1 <= 0.0 && e2 <= 0.0) {
                framebuffer.pixels[y as usize * framebuffer.width + x as usize] = color;
            }
        }
    }
}
