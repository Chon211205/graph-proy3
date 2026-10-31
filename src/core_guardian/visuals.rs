use super::{CoreGuardian, Phase, STARTING_SHIELDS};
use crate::framebuffer::Framebuffer;
use crate::obj_loader::{Face, MaterialColor, Model, Vec3};
use crate::ship::ShipSpec;
use crate::{Camera, render_model};
use std::sync::OnceLock;

static GUARDIAN_BODY: OnceLock<Model> = OnceLock::new();
static GUARDIAN_CORE_CLOSED: OnceLock<Model> = OnceLock::new();
static GUARDIAN_CORE_EXPOSED: OnceLock<Model> = OnceLock::new();
static GUARDIAN_CORE_HIT: OnceLock<Model> = OnceLock::new();
static GUARDIAN_SHELL: OnceLock<Model> = OnceLock::new();
static GUARDIAN_SHELL_WARNING: OnceLock<Model> = OnceLock::new();
static GUARDIAN_TENTACLES: OnceLock<Model> = OnceLock::new();
static GUARDIAN_TUNNEL: OnceLock<Model> = OnceLock::new();
static GUARDIAN_CRYSTALS: OnceLock<Model> = OnceLock::new();
static GUARDIAN_SPINES: OnceLock<Model> = OnceLock::new();
static GUARDIAN_OBSTACLE_MODELS: [OnceLock<Model>; 3] =
    [OnceLock::new(), OnceLock::new(), OnceLock::new()];
static GUARDIAN_PLAYER_BOLT: OnceLock<Model> = OnceLock::new();
static GUARDIAN_ENEMY_SEED: OnceLock<Model> = OnceLock::new();

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
        let (boss_x, boss_y) = self.boss_center();
        draw_boss_tentacles(framebuffer, (boss_x, boss_y), self.elapsed);
        draw_boss_spheres(
            framebuffer,
            (boss_x, boss_y),
            self.elapsed,
            self.phase == Phase::Exposed,
            self.hit_flash > 0.0,
        );
        for index in 0..3 {
            let opening = self.opening_amount(index);
            let warning = self.phase == Phase::Warning && index == self.open_plate;
            draw_shell_plate_3d(framebuffer, (boss_x, boss_y), index, opening, warning);
        }
        draw_obstacles(framebuffer, &self.obstacles, self.elapsed);
        draw_projectiles_3d(framebuffer, &self.player_shots, &self.seeds, self.elapsed);
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

fn draw_shell_plate_3d(
    framebuffer: &mut Framebuffer,
    screen_center: (f32, f32),
    index: usize,
    opening: f32,
    warning: bool,
) {
    let model = if warning {
        GUARDIAN_SHELL_WARNING.get_or_init(|| shell_model([237, 130, 143], [255, 184, 167]))
    } else {
        GUARDIAN_SHELL.get_or_init(|| shell_model([190, 79, 108], [213, 103, 125]))
    };
    let sector_angle = index as f32 * std::f32::consts::TAU / 3.0;
    let hinge_azimuth = std::f32::consts::FRAC_PI_3;
    let hinge_theta: f32 = 1.25;
    let shell_radius = 2.34;
    let pivot_local = Vec3::new(
        shell_radius * hinge_theta.sin() * hinge_azimuth.cos(),
        shell_radius * hinge_theta.sin() * hinge_azimuth.sin(),
        -shell_radius * hinge_theta.cos(),
    );
    let pivot = rotate_z(pivot_local, sector_angle);
    let axis = rotate_z(
        Vec3::new(-hinge_azimuth.sin(), hinge_azimuth.cos(), 0.0),
        sector_angle,
    );
    let direction = if index == 1 { -1.0 } else { 1.0 };
    let angle = opening * direction * 0.95;
    let boss_center = scene_position(framebuffer, screen_center, 14.0);
    let camera = scene_camera();
    render_model(
        model,
        framebuffer,
        &camera,
        |vertex| {
            let rotated = rotate_z(vertex, sector_angle);
            boss_center.add(rotate_around_axis(rotated, pivot, axis, angle))
        },
        None,
        None,
    );
}

fn shell_model(inner_color: [u8; 3], facet_color: [u8; 3]) -> Model {
    let rings = 8;
    let sectors = 10;
    let radius = 2.34;
    let theta_max = 1.32;
    let half_azimuth = std::f32::consts::FRAC_PI_3;
    let mut vertices = Vec::with_capacity((rings + 1) * (sectors + 1));
    for ring in 0..=rings {
        let theta = theta_max * ring as f32 / rings as f32;
        for sector in 0..=sectors {
            let phi = -half_azimuth + 2.0 * half_azimuth * sector as f32 / sectors as f32;
            vertices.push(Vec3::new(
                radius * theta.sin() * phi.cos(),
                radius * theta.sin() * phi.sin(),
                -radius * theta.cos(),
            ));
        }
    }
    let mut faces = Vec::with_capacity(rings * sectors * 2);
    for ring in 0..rings {
        for sector in 0..sectors {
            let a = ring * (sectors + 1) + sector;
            let b = a + 1;
            let c = a + sectors + 1;
            let d = c + 1;
            for indices in [[a, b, c], [b, d, c]] {
                let mut face = Face::new(indices, "guardian_shell", "guardian_shell_plate");
                face.color = MaterialColor::Fixed(if (ring + sector) % 3 == 0 {
                    facet_color
                } else {
                    inner_color
                });
                face.smooth = true;
                faces.push(face);
            }
        }
    }
    let mut model = Model::new(vertices, faces);
    model.recalculate_normals();
    model
}

fn rotate_z(point: Vec3, angle: f32) -> Vec3 {
    let (sin, cos) = angle.sin_cos();
    Vec3::new(
        point.x * cos - point.y * sin,
        point.x * sin + point.y * cos,
        point.z,
    )
}

fn rotate_around_axis(point: Vec3, pivot: Vec3, axis: Vec3, angle: f32) -> Vec3 {
    let offset = point.sub(pivot);
    let (sin, cos) = angle.sin_cos();
    pivot
        .add(offset.mul(cos))
        .add(axis.cross(offset).mul(sin))
        .add(axis.mul(axis.dot(offset) * (1.0 - cos)))
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

fn draw_boss_spheres(
    framebuffer: &mut Framebuffer,
    screen_center: (f32, f32),
    elapsed: f32,
    exposed: bool,
    hit_flash: bool,
) {
    let body = GUARDIAN_BODY.get_or_init(|| sphere_model(2.25, [190, 79, 108]));
    let core = if hit_flash {
        GUARDIAN_CORE_HIT.get_or_init(|| sphere_model(0.82, [255, 247, 189]))
    } else if exposed {
        GUARDIAN_CORE_EXPOSED.get_or_init(|| sphere_model(0.82, [255, 190, 44]))
    } else {
        GUARDIAN_CORE_CLOSED.get_or_init(|| sphere_model(0.82, [156, 74, 46]))
    };
    let body_center = scene_position(framebuffer, screen_center, 14.0);
    let core_center = body_center.add(Vec3::new(0.0, 0.0, -1.45));
    let camera = scene_camera();
    framebuffer.buffers_mut().1.fill(f32::NEG_INFINITY);
    render_model(
        body,
        framebuffer,
        &camera,
        |vertex| vertex.mul(1.0 + (elapsed * 1.7).sin() * 0.035).add(body_center),
        None,
        None,
    );
    let core_pulse = 1.0 + (elapsed * 4.2).sin() * 0.12;
    render_model(
        core,
        framebuffer,
        &camera,
        |vertex| vertex.mul(core_pulse).add(core_center),
        None,
        None,
    );
}

fn draw_boss_tentacles(framebuffer: &mut Framebuffer, screen_center: (f32, f32), elapsed: f32) {
    let model = GUARDIAN_TENTACLES.get_or_init(boss_tentacle_model);
    let center = scene_position(framebuffer, screen_center, 14.0);
    let camera = scene_camera();
    render_model(
        model,
        framebuffer,
        &camera,
        |vertex| {
            let radial = (vertex.x * vertex.x + vertex.y * vertex.y).sqrt();
            let azimuth = vertex.y.atan2(vertex.x);
            let reach = ((radial - 1.5) / 3.5).clamp(0.0, 1.0);
            let wave_phase = elapsed * 2.1 + azimuth * 2.4 + radial * 0.85;
            let bend = wave_phase.sin() * 0.22 * reach;
            let mut animated = rotate_z(vertex, bend);
            animated.z += (wave_phase * 0.82).sin() * 0.28 * reach;
            center.add(animated)
        },
        None,
        None,
    );
}

fn boss_tentacle_model() -> Model {
    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    let angles = [
        0.0,
        std::f32::consts::FRAC_PI_3,
        -std::f32::consts::FRAC_PI_3,
        std::f32::consts::PI,
        2.0 * std::f32::consts::FRAC_PI_3,
        4.0 * std::f32::consts::FRAC_PI_3,
    ];
    let path = [
        Vec3::new(1.55, 0.05, -0.38),
        Vec3::new(2.45, 0.32, -0.52),
        Vec3::new(3.55, 0.22, -0.42),
        Vec3::new(4.45, -0.22, -0.12),
        Vec3::new(5.0, -0.78, 0.08),
    ];
    let radii = [0.50, 0.42, 0.31, 0.18, 0.015];
    for angle in angles {
        for index in 0..path.len() - 1 {
            append_spine_segment(
                &mut vertices,
                &mut faces,
                rotate_z(path[index], angle),
                rotate_z(path[index + 1], angle),
                radii[index],
                radii[index + 1],
                [145, 25, 76],
            );
        }
    }
    Model::new(vertices, faces)
}

fn sphere_model(radius: f32, color: [u8; 3]) -> Model {
    let rings = 12;
    let sectors = 24;
    let mut vertices = Vec::with_capacity((rings + 1) * (sectors + 1));
    for ring in 0..=rings {
        let theta = std::f32::consts::PI * ring as f32 / rings as f32;
        let (sin_theta, cos_theta) = theta.sin_cos();
        for sector in 0..=sectors {
            let phi = std::f32::consts::TAU * sector as f32 / sectors as f32;
            let (sin_phi, cos_phi) = phi.sin_cos();
            vertices.push(Vec3::new(
                radius * sin_theta * cos_phi,
                radius * sin_theta * sin_phi,
                radius * cos_theta,
            ));
        }
    }
    let mut faces = Vec::with_capacity(rings * sectors * 2);
    for ring in 0..rings {
        for sector in 0..sectors {
            let a = ring * (sectors + 1) + sector;
            let b = a + 1;
            let c = a + sectors + 1;
            let d = c + 1;
            for indices in [[a, c, b], [b, c, d]] {
                let mut face = Face::new(indices, "guardian", "guardian_sphere");
                face.color = MaterialColor::Fixed(color);
                face.smooth = true;
                faces.push(face);
            }
        }
    }
    let mut model = Model::new(vertices, faces);
    model.recalculate_normals();
    model
}

fn scene_position(framebuffer: &Framebuffer, screen: (f32, f32), depth: f32) -> Vec3 {
    let tan_half_fov = (60.0_f32.to_radians() * 0.5).tan();
    let aspect = framebuffer.width as f32 / framebuffer.height.max(1) as f32;
    Vec3::new(
        (screen.0 - 0.5) * depth * 2.0 * tan_half_fov * aspect,
        (0.5 - screen.1) * depth * 2.0 * tan_half_fov,
        depth,
    )
}

fn scene_camera() -> Camera {
    Camera {
        position: Vec3::default(),
        forward: Vec3::new(0.0, 0.0, 1.0),
        right: Vec3::new(1.0, 0.0, 0.0),
        up: Vec3::new(0.0, 1.0, 0.0),
    }
}

fn draw_obstacles(framebuffer: &mut Framebuffer, obstacles: &[super::Obstacle], elapsed: f32) {
    let camera = scene_camera();
    for obstacle in obstacles.iter().filter(|obstacle| obstacle.alive) {
        let variant = obstacle.variant % GUARDIAN_OBSTACLE_MODELS.len();
        let model = GUARDIAN_OBSTACLE_MODELS[variant].get_or_init(|| obstacle_model(variant));
        let depth = (17.6 - 11.7 * obstacle.y).clamp(4.0, 24.0);
        let position = scene_position(framebuffer, (obstacle.x, obstacle.y), depth);
        let spin = elapsed * (0.38 + variant as f32 * 0.09);
        render_model(
            model,
            framebuffer,
            &camera,
            |vertex| {
                position.add(rotate_y(rotate_x(vertex, spin * 0.73), spin).mul(obstacle.radius))
            },
            None,
            None,
        );
    }
}

fn draw_projectiles_3d(
    framebuffer: &mut Framebuffer,
    shots: &[super::PlayerShot],
    seeds: &[super::Seed],
    elapsed: f32,
) {
    let camera = scene_camera();
    let bolt = GUARDIAN_PLAYER_BOLT.get_or_init(player_bolt_model);
    for shot in shots.iter().filter(|shot| shot.alive) {
        let depth = (17.6 - 11.7 * shot.y).clamp(4.0, 24.0);
        let position = scene_position(framebuffer, (shot.x, shot.y), depth);
        render_model(
            bolt,
            framebuffer,
            &camera,
            |vertex| position.add(vertex),
            None,
            None,
        );
    }

    let seed_model = GUARDIAN_ENEMY_SEED.get_or_init(enemy_seed_model);
    for (index, seed) in seeds.iter().filter(|seed| seed.alive).enumerate() {
        let depth = (17.6 - 11.7 * seed.y).clamp(4.0, 24.0);
        let position = scene_position(framebuffer, (seed.x, seed.y), depth);
        let spin = elapsed * 1.4 + index as f32;
        render_model(
            seed_model,
            framebuffer,
            &camera,
            |vertex| position.add(rotate_y(rotate_x(vertex, spin * 0.7), spin)),
            None,
            None,
        );
    }
}

fn player_bolt_model() -> Model {
    let vertices = vec![
        Vec3::new(0.0, 0.0, -0.24),
        Vec3::new(0.0, 0.0, 0.25),
        Vec3::new(-0.065, 0.0, 0.0),
        Vec3::new(0.0, -0.065, 0.0),
        Vec3::new(0.065, 0.0, 0.0),
        Vec3::new(0.0, 0.065, 0.0),
    ];
    let indices = [
        [0, 2, 3],
        [0, 3, 4],
        [0, 4, 5],
        [0, 5, 2],
        [1, 3, 2],
        [1, 4, 3],
        [1, 5, 4],
        [1, 2, 5],
    ];
    let faces = indices
        .into_iter()
        .enumerate()
        .map(|(index, indices)| {
            let mut face = Face::new(indices, "guardian_laser", "player_bolt");
            face.color = MaterialColor::Fixed(if index % 2 == 0 {
                [255, 238, 154]
            } else {
                [255, 190, 54]
            });
            face
        })
        .collect();
    Model::new(vertices, faces)
}

fn enemy_seed_model() -> Model {
    let body = sphere_model(0.18, [255, 193, 50]);
    let mut vertices = body.vertices;
    let mut faces: Vec<Face> = body
        .faces
        .into_iter()
        .map(|source| {
            let mut face = Face::new(source.indices, source.material, source.group);
            face.color = source.color;
            face.shader = source.shader;
            face.smooth = source.smooth;
            face
        })
        .collect();
    for direction in [
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(-1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, -1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 0.0, -1.0),
    ] {
        append_spine_segment(
            &mut vertices,
            &mut faces,
            direction.mul(0.12),
            direction.mul(0.40),
            0.09,
            0.008,
            [229, 132, 31],
        );
    }
    Model::new(vertices, faces)
}

fn obstacle_model(variant: usize) -> Model {
    let rings = 7;
    let sectors = 12;
    let mut vertices = Vec::with_capacity((rings + 1) * (sectors + 1));
    for ring in 0..=rings {
        let theta = std::f32::consts::PI * ring as f32 / rings as f32;
        let (sin_theta, cos_theta) = theta.sin_cos();
        for sector in 0..=sectors {
            let phi = std::f32::consts::TAU * sector as f32 / sectors as f32;
            let roughness = 1.0
                + 0.13 * (phi * (3.0 + variant as f32) + theta * 4.0).sin()
                + 0.07 * (theta * 7.0 - phi * 2.0).cos();
            vertices.push(Vec3::new(
                sin_theta * phi.cos() * roughness,
                cos_theta * roughness,
                sin_theta * phi.sin() * roughness,
            ));
        }
    }
    let palettes = [
        [[91, 34, 65], [135, 47, 79], [173, 70, 93]],
        [[105, 41, 60], [164, 66, 78], [204, 105, 105]],
        [[101, 54, 43], [169, 91, 52], [214, 132, 67]],
    ];
    let palette = palettes[variant % palettes.len()];
    let mut faces = Vec::with_capacity(rings * sectors * 2);
    for ring in 0..rings {
        for sector in 0..sectors {
            let a = ring * (sectors + 1) + sector;
            let b = a + 1;
            let c = a + sectors + 1;
            let d = c + 1;
            let color = palette[(ring + sector) % palette.len()];
            for indices in [[a, c, b], [b, c, d]] {
                let mut face = Face::new(indices, "guardian_asteroid", "guardian_obstacle");
                face.color = MaterialColor::Fixed(color);
                faces.push(face);
            }
        }
    }
    Model::new(vertices, faces)
}

fn rotate_x(point: Vec3, angle: f32) -> Vec3 {
    let (sin, cos) = angle.sin_cos();
    Vec3::new(
        point.x,
        point.y * cos - point.z * sin,
        point.y * sin + point.z * cos,
    )
}

fn rotate_y(point: Vec3, angle: f32) -> Vec3 {
    let (sin, cos) = angle.sin_cos();
    Vec3::new(
        point.x * cos + point.z * sin,
        point.y,
        -point.x * sin + point.z * cos,
    )
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
    let position = scene_position(framebuffer, (screen_x, screen_y), 8.0);
    let (sin, cos) = roll.clamp(-0.28, 0.28).sin_cos();
    let forward = Vec3::new(0.0, 0.0, 1.0);
    let right = Vec3::new(cos, sin, 0.0);
    let up = Vec3::new(-sin, cos, 0.0);
    let camera = scene_camera();
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
    if framebuffer.width == 0 || framebuffer.height == 0 {
        return;
    }
    framebuffer.pixels.fill(0x170917);
    let tunnel = GUARDIAN_TUNNEL.get_or_init(cavern_tunnel_model);
    let camera = scene_camera();
    let scroll = (elapsed * 3.2) % 80.0;
    for offset in [-scroll, 80.0 - scroll] {
        render_model(
            tunnel,
            framebuffer,
            &camera,
            |vertex| vertex.add(Vec3::new(0.0, 0.0, offset)),
            None,
            None,
        );
        let crystals = GUARDIAN_CRYSTALS.get_or_init(crystal_cluster_model);
        render_model(
            crystals,
            framebuffer,
            &camera,
            |vertex| vertex.add(Vec3::new(0.0, 0.0, offset)),
            None,
            None,
        );
        let spines = GUARDIAN_SPINES.get_or_init(cavern_spines_model);
        render_model(
            spines,
            framebuffer,
            &camera,
            |vertex| vertex.add(Vec3::new(0.0, 0.0, offset)),
            None,
            None,
        );
    }
}

fn cavern_tunnel_model() -> Model {
    let rings = 16;
    let sectors = 32;
    let length = 80.0;
    let mut vertices = Vec::with_capacity((rings + 1) * (sectors + 1));
    for ring in 0..=rings {
        let t = ring as f32 / rings as f32;
        let wave = (t * std::f32::consts::TAU).sin();
        let radius_x = 17.0 + wave * 1.2;
        let radius_y = 13.0 + wave * 0.8;
        let center_x = wave * 0.45;
        let center_y = (t * std::f32::consts::TAU).cos() * 0.35;
        for sector in 0..=sectors {
            let angle = std::f32::consts::TAU * sector as f32 / sectors as f32;
            let roughness = 1.0 + (angle * 5.0 + t * 8.0).sin() * 0.06;
            vertices.push(Vec3::new(
                center_x + radius_x * roughness * angle.cos(),
                center_y + radius_y * roughness * angle.sin(),
                t * length,
            ));
        }
    }
    let palette = [[53, 20, 48], [72, 25, 53], [91, 30, 59], [112, 39, 67]];
    let mut faces = Vec::with_capacity(rings * sectors * 2);
    for ring in 0..rings {
        for sector in 0..sectors {
            let a = ring * (sectors + 1) + sector;
            let b = a + sectors + 1;
            let c = a + 1;
            let d = b + 1;
            let color_index = ((sector / 3 + ring / 2) % palette.len()) as usize;
            for indices in [[a, b, c], [b, d, c]] {
                let mut face = Face::new(indices, "guardian_rock", "cavern_wall");
                face.color = MaterialColor::Fixed(palette[color_index]);
                faces.push(face);
            }
        }
    }
    Model::new(vertices, faces)
}

fn crystal_cluster_model() -> Model {
    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    for (center, height, width, color) in [
        (Vec3::new(14.2, -1.8, 18.0), 2.2, 0.7, [255, 185, 64]),
        (Vec3::new(-14.1, 2.2, 29.0), 2.8, 0.85, [255, 224, 139]),
        (Vec3::new(13.8, 1.4, 43.0), 2.0, 0.68, [242, 155, 54]),
        (Vec3::new(-14.0, -2.0, 57.0), 2.6, 0.82, [255, 197, 74]),
        (Vec3::new(14.0, 0.0, 70.0), 2.3, 0.72, [255, 231, 162]),
    ] {
        append_crystal(&mut vertices, &mut faces, center, height, width, color);
    }
    Model::new(vertices, faces)
}

fn append_crystal(
    vertices: &mut Vec<Vec3>,
    faces: &mut Vec<Face>,
    center: Vec3,
    height: f32,
    width: f32,
    color: [u8; 3],
) {
    let start = vertices.len();
    for (x, z) in [(-1.0, -1.0), (-1.0, 1.0), (1.0, 1.0), (1.0, -1.0)] {
        vertices.push(center.add(Vec3::new(x * width, 0.0, z * width)));
    }
    vertices.push(center.add(Vec3::new(0.0, height, 0.0)));
    let apex = start + 4;
    let tones = [color, [255, 235, 160], [224, 130, 44], [255, 205, 90]];
    for side in 0..4 {
        let mut face = Face::new(
            [start + side, start + (side + 1) % 4, apex],
            "crystal_gold",
            "cave_crystal",
        );
        face.color = MaterialColor::Fixed(tones[side]);
        faces.push(face);
    }
    let mut bottom = Face::new(
        [start, start + 2, start + 1],
        "crystal_gold",
        "cave_crystal_base",
    );
    bottom.color = MaterialColor::Fixed([152, 76, 42]);
    faces.push(bottom);
    let mut bottom = Face::new(
        [start, start + 3, start + 2],
        "crystal_gold",
        "cave_crystal_base",
    );
    bottom.color = MaterialColor::Fixed([152, 76, 42]);
    faces.push(bottom);
}

fn cavern_spines_model() -> Model {
    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    for (side, z, vertical) in [
        (-1.0, 15.0, -2.0),
        (1.0, 27.0, 1.5),
        (-1.0, 44.0, 2.0),
        (1.0, 61.0, -1.5),
    ] {
        let root = Vec3::new(side * 15.0, vertical, z);
        let bend = Vec3::new(side * 12.5, vertical - 0.5, z + 1.4);
        let curl = Vec3::new(side * 9.3, vertical + 1.2, z + 3.1);
        let tip = Vec3::new(side * 10.4, vertical + 3.1, z + 5.2);
        append_spine_segment(
            &mut vertices,
            &mut faces,
            root,
            bend,
            1.0,
            0.82,
            [117, 24, 64],
        );
        append_spine_segment(
            &mut vertices,
            &mut faces,
            bend,
            curl,
            0.82,
            0.60,
            [166, 34, 86],
        );
        append_spine_segment(
            &mut vertices,
            &mut faces,
            curl,
            tip,
            0.60,
            0.05,
            [214, 61, 117],
        );
    }
    Model::new(vertices, faces)
}

fn append_spine_segment(
    vertices: &mut Vec<Vec3>,
    faces: &mut Vec<Face>,
    from: Vec3,
    to: Vec3,
    from_radius: f32,
    to_radius: f32,
    color: [u8; 3],
) {
    let direction = to.sub(from).normalize();
    let reference = if direction.y.abs() > 0.92 {
        Vec3::new(1.0, 0.0, 0.0)
    } else {
        Vec3::new(0.0, 1.0, 0.0)
    };
    let tangent = direction.cross(reference).normalize();
    let bitangent = direction.cross(tangent).normalize();
    let start = vertices.len();
    let sides = 8;
    for (center, radius) in [(from, from_radius), (to, to_radius)] {
        for side in 0..sides {
            let angle = std::f32::consts::TAU * side as f32 / sides as f32;
            vertices.push(
                center
                    .add(tangent.mul(angle.cos() * radius))
                    .add(bitangent.mul(angle.sin() * radius)),
            );
        }
    }
    for side in 0..sides {
        let next = (side + 1) % sides;
        let shade = if side % 3 == 0 {
            color.map(|channel| channel.saturating_add(30))
        } else {
            color
        };
        let mut face = Face::new(
            [start + side, start + next, start + sides + side],
            "fuchsia_spine",
            "cavern_spine",
        );
        face.color = MaterialColor::Fixed(shade);
        faces.push(face);
        let mut face = Face::new(
            [start + next, start + sides + next, start + sides + side],
            "fuchsia_spine",
            "cavern_spine",
        );
        face.color = MaterialColor::Fixed(shade);
        faces.push(face);
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
