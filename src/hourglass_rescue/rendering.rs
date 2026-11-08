use crate::framebuffer::Framebuffer;
use crate::obj_loader::{Model, Vec3};
use crate::ship::Ship;
use crate::{Camera, model_center, render_model};

use super::{CAPSULE_COUNT, CAPSULE_POSITIONS, EXIT_POSITION, HourglassRescue};

impl HourglassRescue {
    pub fn render(&self, frame: &mut Framebuffer, ship_model: &Model, ship: &Ship) {
        let horizon = if self.inversion_complete {
            [22, 12, 37]
        } else {
            [20, 13, 35]
        };
        for y in 0..frame.height {
            let t = y as f32 / frame.height.max(1) as f32;
            let glow = (1.0 - (t - 0.48).abs() * 1.8).clamp(0.0, 1.0);
            let row = ((horizon[0] as f32 + glow * 23.0) as u32) << 16
                | ((horizon[1] as f32 + glow * 13.0) as u32) << 8
                | (horizon[2] as f32 + glow * 29.0) as u32;
            frame.pixels[y * frame.width..(y + 1) * frame.width].fill(row);
        }
        frame.buffers_mut().1.fill(f32::NEG_INFINITY);
        let camera = self.camera(ship);
        render_model(&self.scene.model, frame, &camera, |v| v, None, None);
        let ship_center = model_center(ship_model);
        let (forward, right, up) = ship.basis();
        render_model(
            ship_model,
            frame,
            &camera,
            |vertex| {
                let local = vertex.sub(ship_center).mul(ship.spec.model_scale * 1.75);
                ship.position
                    .add(ship.spec.model_to_world(local, forward, right, up))
            },
            None,
            Some(ship.spec),
        );
        self.draw_beacons(frame, &camera);
    }

    fn camera(&self, ship: &Ship) -> Camera {
        let (forward, _, up) = ship.camera_basis();
        let position = ship
            .position
            .sub(forward.mul(19.0))
            .add(Vec3::new(0.0, 7.0, 0.0));
        let target = ship
            .position
            .add(forward.mul(10.0))
            .add(Vec3::new(0.0, 2.5, 0.0));
        let camera_forward = target.sub(position).normalize();
        let mut right = camera_forward.cross(Vec3::new(0.0, 1.0, 0.0)).normalize();
        if right.dot(right) < 0.001 {
            right = Vec3::new(1.0, 0.0, 0.0);
        }
        Camera {
            position,
            forward: camera_forward,
            right,
            up: right.cross(camera_forward).normalize().add(up.mul(0.0)),
        }
    }

    fn draw_beacons(&self, frame: &mut Framebuffer, camera: &Camera) {
        let mut points = [None; CAPSULE_COUNT];
        if self.rescued as usize == CAPSULE_COUNT {
            points[0] = Some((EXIT_POSITION, true));
        } else {
            for (index, point) in points.iter_mut().enumerate() {
                if self.capsules[index] == super::CapsuleState::Pending
                    && (index < 3 || self.inversion_complete)
                    && !self.capsule_is_buried(index)
                {
                    *point = Some((CAPSULE_POSITIONS[index], self.rescue_target == Some(index)));
                }
            }
        }
        let pulse = 0.8 + (self.visual_time * 3.6).sin().abs() * 0.2;
        for (point, selected) in points.into_iter().flatten() {
            if let Some((x, y)) = project(camera, point, frame.width, frame.height) {
                let radius = if selected { 5 } else { 3 };
                for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        if dx * dx + dy * dy <= radius * radius {
                            let px = x + dx;
                            let py = y + dy;
                            if px >= 0
                                && py >= 0
                                && px < frame.width as i32
                                && py < frame.height as i32
                            {
                                let color = if self.rescued as usize == CAPSULE_COUNT {
                                    0xffd36c
                                } else {
                                    (80.0 * pulse) as u32 * 65536 + 0x55dce8
                                };
                                frame.pixels[py as usize * frame.width + px as usize] = color;
                            }
                        }
                    }
                }
            }
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
    Some((
        (width as f32 * 0.5 + x * focal / z) as i32,
        (height as f32 * 0.5 - y * focal / z) as i32,
    ))
}
