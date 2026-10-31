use super::{CoreGuardian, Phase, STARTING_SHIELDS};
use crate::framebuffer::Framebuffer;
use crate::obj_loader::{Model, Vec3};
use crate::ship::ShipSpec;
use crate::{Camera, render_model};

impl CoreGuardian {
    pub fn render_scene(
        &self,
        framebuffer: &mut Framebuffer,
        ship_model: &Model,
        ship_center: Vec3,
        ship_spec: &ShipSpec,
    ) {
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
        let plate_angles = [
            -std::f32::consts::FRAC_PI_2,
            std::f32::consts::FRAC_PI_6,
            5.0 * std::f32::consts::FRAC_PI_6,
        ];
        for (index, angle) in plate_angles.into_iter().enumerate() {
            let opening = self.opening_amount(index);
            let warning = self.phase == Phase::Warning && index == self.open_plate;
            draw_shell_plate(
                framebuffer,
                (boss_x, boss_y),
                height * 0.125,
                angle,
                opening * if index == 0 { -1.0 } else { 1.0 },
                if warning { 0xf19a75 } else { 0xce6a75 },
            );
        }
        for shot in self.player_shots.iter().filter(|shot| shot.alive) {
            circle(framebuffer, shot.x * width, shot.y * height, 4.0, 0xfff0a8);
        }
        for seed in self.seeds.iter().filter(|seed| seed.alive) {
            circle(framebuffer, seed.x * width, seed.y * height, 10.0, 0xffbb2e);
        }
        let (_, boss_y) = self.boss_center();
        draw_reticle(framebuffer, (self.player_x * width, boss_y * height));
        draw_selected_ship(
            framebuffer,
            ship_model,
            ship_center,
            ship_spec,
            self.player_x,
            0.82 + self.player_y * 0.10,
            self.roll,
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

fn draw_shell_plate(
    framebuffer: &mut Framebuffer,
    center: (f32, f32),
    radius: f32,
    angle: f32,
    opening: f32,
    color: u32,
) {
    let half_angle = std::f32::consts::FRAC_PI_3;
    let (sin, cos) = angle.sin_cos();
    let pivot = (center.0 + cos * radius, center.1 + sin * radius);
    let rotation = opening * 0.92;
    let (rotation_sin, rotation_cos) = rotation.sin_cos();
    let mut panel = [center; 8];
    for index in 0..=6 {
        let spoke = angle - half_angle + (2.0 * half_angle * index as f32 / 6.0);
        panel[index + 1] = (
            center.0 + spoke.cos() * radius,
            center.1 + spoke.sin() * radius,
        );
    }
    let transformed = panel.map(|point| {
        let x = point.0 - pivot.0;
        let y = point.1 - pivot.1;
        (
            pivot.0 + x * rotation_cos - y * rotation_sin,
            pivot.1 + x * rotation_sin + y * rotation_cos,
        )
    });
    polygon(framebuffer, &transformed, color);

    let highlight = transformed[2];
    let edge = transformed[3];
    line(framebuffer, highlight, edge, 3.0, 0xf4a2a0);
    circle(framebuffer, pivot.0, pivot.1, radius * 0.07, 0x833953);
}

fn draw_reticle(framebuffer: &mut Framebuffer, center: (f32, f32)) {
    let arm = 18.0;
    let gap = 7.0;
    let outer = 0xfff4ec;
    let shadow = 0x54213f;
    let segments = [
        ((center.0 - arm, center.1), (center.0 - gap, center.1)),
        ((center.0 + gap, center.1), (center.0 + arm, center.1)),
        ((center.0, center.1 - arm), (center.0, center.1 - gap)),
        ((center.0, center.1 + gap), (center.0, center.1 + arm)),
    ];
    for (from, to) in segments {
        line(framebuffer, from, to, 4.0, shadow);
        line(framebuffer, from, to, 2.0, outer);
    }
    circle(framebuffer, center.0, center.1, 3.0, outer);
}

fn draw_selected_ship(
    framebuffer: &mut Framebuffer,
    model: &Model,
    model_center: Vec3,
    spec: &ShipSpec,
    screen_x: f32,
    screen_y: f32,
    roll: f32,
) {
    let depth = 8.0;
    let tan_half_fov = (60.0_f32.to_radians() * 0.5).tan();
    let aspect = framebuffer.width as f32 / framebuffer.height.max(1) as f32;
    let position = Vec3::new(
        (screen_x - 0.5) * depth * 2.0 * tan_half_fov * aspect,
        (0.5 - screen_y) * depth * 2.0 * tan_half_fov,
        depth,
    );
    let (sin, cos) = roll.clamp(-0.28, 0.28).sin_cos();
    let forward = Vec3::new(0.0, 0.0, 1.0);
    let right = Vec3::new(cos, sin, 0.0);
    let up = Vec3::new(-sin, cos, 0.0);
    let camera = Camera {
        position: Vec3::default(),
        forward,
        right: Vec3::new(1.0, 0.0, 0.0),
        up: Vec3::new(0.0, 1.0, 0.0),
    };
    framebuffer.buffers_mut().1.fill(f32::NEG_INFINITY);
    render_model(
        model,
        framebuffer,
        &camera,
        |vertex| {
            let local = vertex.sub(model_center).mul(spec.model_scale);
            position.add(spec.model_to_world(local, forward, right, up))
        },
        None,
        None,
    );
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

    draw_crystal_cluster(
        framebuffer,
        width * 0.075,
        height * 0.79,
        height * 0.13,
        elapsed,
    );
    draw_crystal_cluster(
        framebuffer,
        width * 0.925,
        height * 0.55,
        height * 0.095,
        elapsed + 1.7,
    );
    draw_crystal_cluster(
        framebuffer,
        width * 0.17,
        height * 0.43,
        height * 0.07,
        elapsed + 3.1,
    );
    draw_cave_spines(framebuffer, elapsed);
}

fn draw_crystal_cluster(framebuffer: &mut Framebuffer, x: f32, y: f32, size: f32, time: f32) {
    let shimmer = ((time * 1.6).sin() * 8.0) as i32;
    let gold = (255u32 << 16) | (((176 + shimmer).clamp(0, 255) as u32) << 8) | 57;
    let pale = 0xffe8a0;
    let dark = 0xb66737;
    for (offset_x, scale, color) in [
        (-0.48, 0.64, dark),
        (-0.18, 0.92, gold),
        (0.18, 0.72, pale),
        (0.46, 0.55, gold),
    ] {
        let base_x = x + offset_x * size;
        let crystal_h = size * scale;
        polygon(
            framebuffer,
            &[
                (base_x - size * 0.13, y),
                (base_x - size * 0.08, y - crystal_h * 0.76),
                (base_x + size * 0.02, y - crystal_h),
                (base_x + size * 0.11, y - crystal_h * 0.38),
                (base_x + size * 0.13, y),
            ],
            color,
        );
        line(
            framebuffer,
            (base_x - size * 0.04, y - crystal_h * 0.18),
            (base_x + size * 0.02, y - crystal_h * 0.88),
            2.0,
            0xfff2c2,
        );
    }
}

fn draw_cave_spines(framebuffer: &mut Framebuffer, elapsed: f32) {
    let width = framebuffer.width as f32;
    let height = framebuffer.height as f32;
    let sway = (elapsed * 0.7).sin() * height * 0.008;
    for (x, y, scale, direction) in [
        (0.02, 0.30, 1.0, 1.0),
        (0.09, 0.66, 0.78, 1.0),
        (0.98, 0.34, 1.08, -1.0),
        (0.91, 0.72, 0.82, -1.0),
    ] {
        let root = (width * x, height * y + sway * direction);
        let bend = (root.0 + width * 0.10 * direction, root.1 - height * 0.015);
        let tip = (
            root.0 + width * 0.15 * direction,
            root.1 - height * 0.13 * scale,
        );
        let joint = (
            root.0 + width * 0.17 * direction,
            root.1 - height * 0.25 * scale,
        );
        let outer = 12.0 * scale;
        let inner = 5.0 * scale;
        line(framebuffer, root, bend, outer, 0x7b1948);
        line(framebuffer, bend, tip, outer, 0x9d2055);
        line(framebuffer, tip, joint, outer * 0.72, 0xb42a63);
        line(
            framebuffer,
            (root.0, root.1 - 3.0),
            (tip.0, tip.1 - 3.0),
            inner,
            0xd94379,
        );
        circle(framebuffer, joint.0, joint.1, outer * 0.38, 0x781a43);
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
