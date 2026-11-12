use crate::framebuffer::Framebuffer;
use crate::obj_loader::{Model, Vec3};
use crate::ship::{Projectile, Ship};
use crate::{Camera, model_center, render_model};

use super::{CAPSULE_COUNT, CAPSULE_POSITIONS, HourglassRescue};

impl HourglassRescue {
    pub fn render(&self, frame: &mut Framebuffer, ship_model: &Model, ship: &Ship) {
        let sky = [17, 19, 48];
        for y in 0..frame.height {
            let t = y as f32 / frame.height.max(1) as f32;
            let glow = (1.0 - ((t - 0.40) / 0.72).abs()).clamp(0.0, 1.0);
            let haze = glow.powf(1.45);
            let color = [
                (sky[0] as f32 + 183.0 * haze) as u32,
                (sky[1] as f32 + 78.0 * haze) as u32,
                (sky[2] as f32 + 114.0 * haze) as u32,
            ];
            frame.pixels[y * frame.width..(y + 1) * frame.width]
                .fill((color[0] << 16) | (color[1] << 8) | color[2]);
        }
        self.draw_stars(frame);
        frame.buffers_mut().1.fill(f32::NEG_INFINITY);
        let camera = self.camera(ship);
        self.draw_sand_stream(frame, &camera);
        render_model(&self.scene.model, frame, &camera, |v| v, None, None);
        let ship_center = model_center(ship_model);
        // Render with the exact same basis used by movement. The previous
        // level-specific formula used a different yaw convention and drew the
        // ship roughly 90 degrees away from its real travel direction.
        let (forward, right, up) = ship.basis();
        render_model(
            ship_model,
            frame,
            &camera,
            |vertex| {
                let local = vertex.sub(ship_center).mul(ship.spec.model_scale * 2.5);
                ship.position
                    .add(ship.spec.model_to_world(local, forward, right, up))
            },
            None,
            Some(ship.spec),
        );
        self.draw_beacons(frame, &camera);
    }

    pub fn render_weapon(
        &self,
        frame: &mut Framebuffer,
        laser_model: &Model,
        projectiles: &[Projectile],
        ship: &Ship,
        charge: f32,
    ) {
        let camera = self.camera(ship);
        for projectile in projectiles.iter().filter(|shot| shot.lifetime > 0.0) {
            let mut right = projectile
                .direction
                .cross(Vec3::new(0.0, 1.0, 0.0))
                .normalize();
            if right.dot(right) < 0.001 {
                right = Vec3::new(1.0, 0.0, 0.0);
            }
            let up = right.cross(projectile.direction).normalize();
            render_model(
                laser_model,
                frame,
                &camera,
                |vertex| {
                    projectile
                        .position
                        .add(projectile.direction.mul(vertex.x * projectile.scale))
                        .add(up.mul(vertex.y * projectile.scale))
                        .add(right.mul(vertex.z * projectile.scale))
                },
                None,
                Some(ship.spec),
            );
        }
        crate::draw_projectile_glow(
            frame,
            projectiles,
            &camera,
            self.visual_time,
            ship.spec.laser_color,
            1.65,
        );
        for projectile in self
            .enemy_projectiles
            .iter()
            .filter(|shot| shot.lifetime > 0.0)
        {
            let mut right = projectile
                .direction
                .cross(Vec3::new(0.0, 1.0, 0.0))
                .normalize();
            if right.dot(right) < 0.001 {
                right = Vec3::new(1.0, 0.0, 0.0);
            }
            let up = right.cross(projectile.direction).normalize();
            render_model(
                laser_model,
                frame,
                &camera,
                |vertex| {
                    projectile
                        .position
                        .add(projectile.direction.mul(vertex.x * projectile.scale))
                        .add(up.mul(vertex.y * projectile.scale))
                        .add(right.mul(vertex.z * projectile.scale))
                },
                None,
                Some(ship.spec),
            );
        }
        crate::draw_projectile_glow(
            frame,
            &self.enemy_projectiles,
            &camera,
            self.visual_time,
            [255, 92, 38],
            1.45,
        );

        if charge > 0.0 {
            let (forward, right, up) = ship.basis();
            let center = ship
                .position
                .add(forward.mul(ship.spec.muzzle_forward + 0.4));
            let scale = 0.35 + charge * 1.15;
            render_model(
                laser_model,
                frame,
                &camera,
                |vertex| {
                    center
                        .add(forward.mul(vertex.x * scale))
                        .add(up.mul(vertex.y * scale))
                        .add(right.mul(vertex.z * scale))
                },
                None,
                Some(ship.spec),
            );
        }
    }

    fn draw_stars(&self, frame: &mut Framebuffer) {
        for i in 0..44i32 {
            let x = (i * 347 + 97).rem_euclid(frame.width as i32);
            let y = (i * 173 + 31).rem_euclid((frame.height as i32 * 44 / 100).max(1));
            let pulse = (0.72 + (self.visual_time * 1.2 + i as f32).sin().abs() * 0.28) * 190.0;
            set_pixel(
                frame,
                x,
                y,
                ((pulse as u32) << 16) | ((pulse as u32) << 8) | 0xff,
            );
        }
    }

    fn draw_sand_stream(&self, frame: &mut Framebuffer, camera: &Camera) {
        // Layered 3D particle ribbons taper into the hourglass waist. Their
        // curve moves with time and reverses when gravity flips.
        let direction = 1.0;
        for strand in 0..9 {
            let mut previous = None;
            let phase = strand as f32 * std::f32::consts::TAU / 9.0;
            for step in 0..42 {
                let linear = step as f32 / 41.0;
                let t = if direction > 0.0 {
                    linear
                } else {
                    1.0 - linear
                };
                let spread = (1.0 - t).powf(1.15) * 42.0 + 0.5;
                let sway = (self.visual_time * direction * 0.55 + t * 3.0 + phase).sin();
                let world = Vec3::new(
                    sway * spread * 0.44 + phase.cos() * spread * 0.23,
                    96.0 - t * 70.0,
                    -178.0 + t * 87.0 + phase.sin() * spread * 0.16,
                );
                let point = project(camera, world, frame.width, frame.height);
                if let (Some(a), Some(b)) = (previous, point) {
                    let color = if strand % 3 == 0 { 0xffc6e6 } else { 0xed98dd };
                    draw_line(frame, a, b, color, if strand % 3 == 0 { 2 } else { 1 });
                }
                if let Some((x, y)) = point {
                    if step % 4 == 0 {
                        set_pixel(frame, x, y, 0xffe2d1);
                    }
                }
                previous = point;
            }
        }
    }

    pub(super) fn camera(&self, ship: &Ship) -> Camera {
        // Chase camera: stay behind the real travel direction while keeping
        // the horizon stable instead of inheriting the ship's barrel roll.
        let view_forward = ship.movement_basis().0;
        let world_up = Vec3::new(0.0, 1.0, 0.0);
        let position = ship
            .position
            .sub(view_forward.mul(ship.spec.chase_distance * 1.45))
            .add(world_up.mul(ship.spec.chase_height));
        let target = ship
            .position
            .add(view_forward.mul(ship.spec.chase_look_ahead * 0.55));
        let camera_forward = target.sub(position).normalize();
        let mut right = camera_forward.cross(world_up).normalize();
        if right.dot(right) < 0.001 {
            right = Vec3::new(1.0, 0.0, 0.0);
        }
        Camera {
            position,
            forward: camera_forward,
            right,
            up: right.cross(camera_forward).normalize(),
        }
    }

    fn draw_beacons(&self, frame: &mut Framebuffer, camera: &Camera) {
        let mut points = [None; CAPSULE_COUNT];
        if let Some(index) = self.rescue_target {
            let position = CAPSULE_POSITIONS[index];
            let offset = position.sub(camera.position);
            if offset.dot(offset).sqrt() < 95.0 {
                points[index] = Some((position, true));
            }
        }
        let pulse = 0.8 + (self.visual_time * 3.6).sin().abs() * 0.2;
        for (point, selected) in points.into_iter().flatten() {
            if let Some((x, y)) = project(camera, point, frame.width, frame.height) {
                if selected {
                    // A slim beacon remains legible against the crystal walls
                    // without covering the capsule sprite or the scenery.
                    for py in (y - 28).max(0)..(y - 7).max(0) {
                        set_pixel(frame, x, py, 0x3d91c6);
                    }
                    for dy in -12i32..=12 {
                        for dx in -12i32..=12 {
                            let d2 = dx * dx + dy * dy;
                            if (d2 - 100).abs() <= 12 {
                                set_pixel(frame, x + dx, y + dy, 0x8b62bd);
                            }
                        }
                    }
                }
                let radius = if selected { 4 } else { 3 };
                for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        if dx * dx + dy * dy <= radius * radius {
                            let px = x + dx;
                            let py = y + dy;
                            let color = if self.rescued as usize == CAPSULE_COUNT {
                                0xffd36c
                            } else {
                                (80.0 * pulse) as u32 * 65536 + 0x55dce8
                            };
                            set_pixel(frame, px, py, color);
                        }
                    }
                }
            }
        }
    }
}

fn set_pixel(frame: &mut Framebuffer, x: i32, y: i32, color: u32) {
    if x >= 0 && y >= 0 && x < frame.width as i32 && y < frame.height as i32 {
        frame.pixels[y as usize * frame.width + x as usize] = color;
    }
}

fn draw_line(frame: &mut Framebuffer, start: (i32, i32), end: (i32, i32), color: u32, width: i32) {
    let dx = end.0 as i64 - start.0 as i64;
    let dy = end.1 as i64 - start.1 as i64;
    let steps = dx.abs().max(dy.abs()).max(1);
    // This is a decorative sand ribbon, so discard pathological segments
    // before iteration. Projection can still produce extreme coordinates
    // when the ship/camera is in an invalid transient state; bounding work
    // here keeps a bad segment from overflowing or stalling the game loop.
    let max_steps = frame.width.saturating_add(frame.height).saturating_mul(4) as i64;
    if steps > max_steps {
        return;
    }
    for step in 0..=steps {
        let t = step as f64 / steps as f64;
        let x = (start.0 as f64 + dx as f64 * t) as i32;
        let y = (start.1 as f64 + dy as f64 * t) as i32;
        for offset in -width..=width {
            set_pixel(frame, x + offset, y, color);
            set_pixel(frame, x, y + offset, color);
        }
    }
}

pub(super) fn project(
    camera: &Camera,
    point: Vec3,
    width: usize,
    height: usize,
) -> Option<(i32, i32)> {
    let offset = point.sub(camera.position);
    let x = offset.dot(camera.right);
    let y = offset.dot(camera.up);
    let z = offset.dot(camera.forward);
    if z <= 0.2 {
        return None;
    }
    let focal = height as f32 / (60.0_f32.to_radians() * 0.5).tan() * 0.5;
    let screen_x = width as f32 * 0.5 + x * focal / z;
    let screen_y = height as f32 * 0.5 - y * focal / z;
    // Sand-stream vertices can pass arbitrarily close to the camera. Reject
    // those extreme projections before converting to integers or rasterizing
    // a multi-million-pixel line (which used to panic and close the window).
    if !screen_x.is_finite()
        || !screen_y.is_finite()
        || screen_x < -(width as f32) * 2.0
        || screen_x > width as f32 * 3.0
        || screen_y < -(height as f32) * 2.0
        || screen_y > height as f32 * 3.0
    {
        return None;
    }
    Some((screen_x as i32, screen_y as i32))
}
