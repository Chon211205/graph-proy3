use std::ops::Range;

use crate::obj_loader::{Face, MaterialColor, Model, SurfaceShader, Vec3};

pub(super) const CAPSULE_COUNT: usize = 5;
pub(super) const CAPSULE_POSITIONS: [Vec3; CAPSULE_COUNT] = [
    Vec3::new(-70.0, 3.5, -130.0),
    Vec3::new(130.0, 29.0, -240.0),
    Vec3::new(-170.0, 31.0, -340.0),
    Vec3::new(175.0, 137.0, -390.0),
    Vec3::new(-140.0, 157.0, -465.0),
];

pub(super) struct Scene {
    pub model: Model,
    pub sand_vertices: Range<usize>,
    pub capsule_faces: [Range<usize>; CAPSULE_COUNT],
    capsule_indices: [Vec<[usize; 3]>; CAPSULE_COUNT],
    clock_faces: [Range<usize>; super::CLOCK_COUNT],
    clock_indices: [Vec<[usize; 3]>; super::CLOCK_COUNT],
    portal_faces: Range<usize>,
    portal_open: bool,
    obstacles: Vec<(Vec3, Vec3)>,
}

struct Builder {
    vertices: Vec<Vec3>,
    faces: Vec<Face>,
}

impl Builder {
    fn new() -> Self {
        Self {
            vertices: Vec::new(),
            faces: Vec::new(),
        }
    }

    fn triangle(&mut self, a: Vec3, b: Vec3, c: Vec3, color: [u8; 3]) {
        let start = self.vertices.len();
        self.vertices.extend([a, b, c]);
        let mut face = Face::new(
            [start, start + 1, start + 2],
            "rescue_geometry",
            "hourglass",
        );
        face.shader = SurfaceShader::Flat;
        face.color = MaterialColor::Fixed(color);
        self.faces.push(face);
    }

    fn quad(&mut self, a: Vec3, b: Vec3, c: Vec3, d: Vec3, color: [u8; 3]) {
        self.triangle(a, b, c, color);
        self.triangle(a, c, d, color);
    }

    fn box_at(&mut self, center: Vec3, half: Vec3, color: [u8; 3]) {
        let (x, y, z) = (half.x, half.y, half.z);
        let (cx, cy, cz) = (center.x, center.y, center.z);
        let p = [
            Vec3::new(cx - x, cy - y, cz - z),
            Vec3::new(cx + x, cy - y, cz - z),
            Vec3::new(cx + x, cy + y, cz - z),
            Vec3::new(cx - x, cy + y, cz - z),
            Vec3::new(cx - x, cy - y, cz + z),
            Vec3::new(cx + x, cy - y, cz + z),
            Vec3::new(cx + x, cy + y, cz + z),
            Vec3::new(cx - x, cy + y, cz + z),
        ];
        for (a, b, c, d) in [
            (0, 3, 2, 1),
            (4, 5, 6, 7),
            (0, 4, 7, 3),
            (1, 2, 6, 5),
            (0, 1, 5, 4),
            (3, 7, 6, 2),
        ] {
            self.quad(p[a], p[b], p[c], p[d], color);
        }
    }

    fn mountain_ridge(&mut self, z: f32, base_y: f32, peaks: &[(f32, f32)], color: [u8; 3]) {
        for (index, pair) in peaks.windows(2).enumerate() {
            let (x0, y0) = pair[0];
            let (x1, y1) = pair[1];
            let top0 = Vec3::new(x0, y0, z);
            let top1 = Vec3::new(x1, y1, z);
            let foot0 = Vec3::new(x0, base_y, z);
            let foot1 = Vec3::new(x1, base_y, z);
            let shade = if index % 3 == 0 {
                color
            } else {
                [
                    color[0].saturating_sub(11),
                    color[1].saturating_sub(8),
                    color[2].saturating_sub(5),
                ]
            };
            self.triangle(foot0, top0, top1, shade);
            self.triangle(
                foot0,
                top1,
                foot1,
                [
                    shade[0].saturating_sub(17),
                    shade[1].saturating_sub(11),
                    shade[2].saturating_sub(4),
                ],
            );
            if index % 2 == 0 {
                let shoulder = Vec3::new((x0 + x1) * 0.5, (y0 + y1) * 0.5 + 3.0, z - 0.8);
                self.triangle(
                    top0,
                    shoulder,
                    top1,
                    [
                        color[0].saturating_add(13),
                        color[1].saturating_add(7),
                        color[2].saturating_add(4),
                    ],
                );
            }
        }
    }

    fn mountain_mass(&mut self, center: Vec3, width: f32, height: f32, depth: f32) {
        // Layered cross sections make a broad, rounded low-poly escarpment
        // instead of a single pyramid silhouette.
        let slices = [
            (-1.0, 0.72, 0.75),
            (-0.45, 1.0, 0.94),
            (0.28, 0.86, 1.0),
            (1.0, 0.62, 0.78),
        ];
        let across = 12;
        let mut rows: Vec<Vec<Vec3>> = Vec::with_capacity(slices.len());
        for (depth_t, width_scale, height_scale) in slices {
            let mut row = Vec::with_capacity(across + 1);
            for i in 0..=across {
                let nx = i as f32 / across as f32 * 2.0 - 1.0;
                let profile = (1.0 - nx * nx).max(0.0).powf(0.52);
                let irregular = (nx * 7.0 + depth_t * 3.0).sin() * height * 0.045 * profile;
                row.push(Vec3::new(
                    center.x + nx * width * width_scale,
                    center.y + profile * height * height_scale + irregular,
                    center.z + depth_t * depth,
                ));
            }
            rows.push(row);
        }
        for layer in 0..rows.len() - 1 {
            for i in 0..across {
                let light = (i + layer * 3) % 5 == 0;
                let base = if light {
                    [225, 146, 151]
                } else if layer == 0 {
                    [91, 58, 111]
                } else {
                    [154, 79, 139]
                };
                self.quad(
                    rows[layer][i],
                    rows[layer + 1][i],
                    rows[layer + 1][i + 1],
                    rows[layer][i + 1],
                    base,
                );
            }
        }
    }

    fn floating_island(&mut self, x: f32, y: f32, z: f32, width: f32, color: [u8; 3]) {
        let half = width * 0.5;
        self.box_at(
            Vec3::new(x, y, z),
            Vec3::new(half, 1.8, width * 0.28),
            color,
        );
        self.box_at(
            Vec3::new(x, y - 4.0, z),
            Vec3::new(width * 0.22, 3.0, width * 0.16),
            [
                color[0].saturating_sub(30),
                color[1].saturating_sub(24),
                color[2].saturating_sub(18),
            ],
        );
        for (dx, height) in [(-0.30, 0.5), (0.0, 1.0), (0.28, 0.65)] {
            let tower_w = width * if height > 0.9 { 0.12 } else { 0.095 };
            let tower_h = width * 0.28 * height;
            self.box_at(
                Vec3::new(x + dx * width, y + 1.8 + tower_h * 0.5, z),
                Vec3::new(tower_w, tower_h * 0.5, tower_w),
                [
                    color[0].saturating_add(12),
                    color[1].saturating_add(7),
                    color[2].saturating_add(3),
                ],
            );
            self.box_at(
                Vec3::new(x + dx * width, y + 1.8 + tower_h, z),
                Vec3::new(tower_w * 1.3, 0.6, tower_w * 1.3),
                [236, 177, 117],
            );
        }
    }

    fn arch(&mut self, center: Vec3, radius: f32, depth: f32, color: [u8; 3]) {
        let segments = 14;
        let thickness = radius * 0.18;
        for i in 0..segments {
            let a0 = std::f32::consts::PI * i as f32 / segments as f32;
            let a1 = std::f32::consts::PI * (i + 1) as f32 / segments as f32;
            let point = |r: f32, a: f32, z: f32| {
                Vec3::new(center.x + r * a.cos(), center.y + r * a.sin(), center.z + z)
            };
            let outer0 = point(radius, a0, 0.0);
            let outer1 = point(radius, a1, 0.0);
            let inner0 = point(radius - thickness, a0, 0.0);
            let inner1 = point(radius - thickness, a1, 0.0);
            let shade = if i % 2 == 0 {
                color
            } else {
                [
                    color[0].saturating_sub(18),
                    color[1].saturating_sub(12),
                    color[2].saturating_sub(8),
                ]
            };
            self.quad(outer0, outer1, inner1, inner0, shade);
            self.quad(
                point(radius, a0, depth),
                point(radius - thickness, a0, depth),
                point(radius - thickness, a1, depth),
                point(radius, a1, depth),
                shade,
            );
            self.quad(
                outer0,
                point(radius, a0, depth),
                point(radius, a1, depth),
                outer1,
                [
                    shade[0].saturating_sub(28),
                    shade[1].saturating_sub(19),
                    shade[2].saturating_sub(11),
                ],
            );
        }
        for side in [-1.0, 1.0] {
            let leg = Vec3::new(
                center.x + side * (radius - thickness * 0.52),
                center.y - radius * 0.52,
                center.z + depth * 0.5,
            );
            self.box_at(
                leg,
                Vec3::new(thickness * 0.52, radius * 0.48, depth * 0.55),
                color,
            );
            self.box_at(
                Vec3::new(leg.x, center.y - radius, leg.z),
                Vec3::new(thickness * 0.85, 1.1, depth * 0.7),
                [226, 169, 111],
            );
        }
    }

    fn crystal(
        &mut self,
        center: Vec3,
        half_width: f32,
        height: f32,
        half_depth: f32,
        palette: [[u8; 3]; 4],
    ) {
        let base = [
            Vec3::new(center.x - half_width, center.y, center.z - half_depth),
            Vec3::new(center.x + half_width, center.y, center.z - half_depth),
            Vec3::new(center.x + half_width, center.y, center.z + half_depth),
            Vec3::new(center.x - half_width, center.y, center.z + half_depth),
        ];
        let tip = Vec3::new(
            center.x + half_width * 0.18,
            center.y + height,
            center.z - half_depth * 0.12,
        );
        for side in 0..4 {
            self.triangle(base[side], base[(side + 1) % 4], tip, palette[side]);
        }
        self.quad(base[0], base[3], base[2], base[1], palette[0]);
    }

    fn capsule(&mut self, center: Vec3) -> Range<usize> {
        let start = self.faces.len();
        let sides = 10;
        let rings = [(0.0, 1.65), (0.45, 2.0), (2.95, 2.0), (3.35, 1.65)];
        for side in 0..sides {
            let a0 = side as f32 * std::f32::consts::TAU / sides as f32;
            let a1 = (side + 1) as f32 * std::f32::consts::TAU / sides as f32;
            for band in 0..rings.len() - 1 {
                let (y0, r0) = rings[band];
                let (y1, r1) = rings[band + 1];
                let p = |angle: f32, y: f32, radius: f32| {
                    Vec3::new(
                        center.x + angle.cos() * radius,
                        center.y + y,
                        center.z + angle.sin() * radius,
                    )
                };
                let color = match band {
                    0 => [204, 147, 70],
                    2 => [239, 201, 124],
                    _ => [232, 220, 188],
                };
                self.quad(
                    p(a0, y0, r0),
                    p(a1, y0, r0),
                    p(a1, y1, r1),
                    p(a0, y1, r1),
                    color,
                );
            }
            let top0 = Vec3::new(
                center.x + a0.cos() * 1.65,
                center.y + 3.36,
                center.z + a0.sin() * 1.65,
            );
            let top1 = Vec3::new(
                center.x + a1.cos() * 1.65,
                center.y + 3.36,
                center.z + a1.sin() * 1.65,
            );
            self.triangle(
                top0,
                top1,
                Vec3::new(center.x, center.y + 3.6, center.z),
                [240, 199, 112],
            );
            let window = |angle: f32| {
                Vec3::new(
                    center.x + angle.cos() * 2.02,
                    center.y + 2.1,
                    center.z + angle.sin() * 2.02,
                )
            };
            self.quad(
                window(a0),
                window(a1),
                window(a1).add(Vec3::new(0.0, 0.72, 0.0)),
                window(a0).add(Vec3::new(0.0, 0.72, 0.0)),
                [77, 225, 235],
            );
        }
        start..self.faces.len()
    }

    fn clock(&mut self, center: Vec3) -> Range<usize> {
        let start = self.faces.len();
        // A bright, chunky low-poly clock that remains readable while flying.
        self.box_at(center, Vec3::new(3.4, 3.4, 0.65), [235, 177, 55]);
        self.box_at(
            center.add(Vec3::new(0.0, 0.0, -0.72)),
            Vec3::new(2.65, 2.65, 0.18),
            [247, 235, 181],
        );
        self.box_at(
            center.add(Vec3::new(0.0, 0.75, -1.02)),
            Vec3::new(0.20, 1.25, 0.12),
            [62, 50, 75],
        );
        self.box_at(
            center.add(Vec3::new(0.75, 0.0, -1.02)),
            Vec3::new(0.95, 0.20, 0.12),
            [62, 50, 75],
        );
        start..self.faces.len()
    }

    fn dragon_sphere(&mut self, center: Vec3) {
        let rings = 8;
        let sides = 14;
        let radius = 4.2;
        for ring in 0..rings {
            let v0 = -std::f32::consts::FRAC_PI_2
                + std::f32::consts::PI * ring as f32 / rings as f32;
            let v1 = -std::f32::consts::FRAC_PI_2
                + std::f32::consts::PI * (ring + 1) as f32 / rings as f32;
            for side in 0..sides {
                let a0 = std::f32::consts::TAU * side as f32 / sides as f32;
                let a1 = std::f32::consts::TAU * (side + 1) as f32 / sides as f32;
                let p = |v: f32, a: f32| {
                    center.add(Vec3::new(
                        radius * v.cos() * a.cos(),
                        radius * v.sin(),
                        radius * v.cos() * a.sin(),
                    ))
                };
                let color = if (ring + side) % 3 == 0 {
                    [255, 207, 61]
                } else {
                    [238, 112, 25]
                };
                self.quad(p(v0, a0), p(v0, a1), p(v1, a1), p(v1, a0), color);
            }
        }
    }

    fn portal_ring(&mut self, center: Vec3, radius: f32) -> Range<usize> {
        let start = self.faces.len();
        let segments = 24;
        let tube = 1.4;
        for index in 0..segments {
            let a0 = index as f32 * std::f32::consts::TAU / segments as f32;
            let a1 = (index + 1) as f32 * std::f32::consts::TAU / segments as f32;
            let p = |angle: f32, r: f32, z: f32| {
                Vec3::new(
                    center.x + r * angle.cos(),
                    center.y + r * angle.sin(),
                    center.z + z,
                )
            };
            let color = if index % 3 == 0 {
                [79, 106, 135]
            } else {
                [55, 76, 103]
            };
            self.quad(
                p(a0, radius - tube, -0.8),
                p(a1, radius - tube, -0.8),
                p(a1, radius + tube, -0.8),
                p(a0, radius + tube, -0.8),
                color,
            );
            self.quad(
                p(a0, radius - tube, 0.8),
                p(a0, radius + tube, 0.8),
                p(a1, radius + tube, 0.8),
                p(a1, radius - tube, 0.8),
                color,
            );
            self.quad(
                p(a0, radius + tube, -0.8),
                p(a1, radius + tube, -0.8),
                p(a1, radius + tube, 0.8),
                p(a0, radius + tube, 0.8),
                color,
            );
        }
        start..self.faces.len()
    }

    fn sand_dunes(&mut self) -> Range<usize> {
        let start = self.vertices.len();
        // A broad rectangular height field keeps the horizon open and avoids
        // the concentric rings produced by a polar floor mesh.
        let (columns, rows) = (56usize, 64usize);
        let (min_x, max_x, min_z, max_z) = (-600.0f32, 600.0f32, -720.0f32, 220.0f32);
        for row in 0..=rows {
            let z = min_z + (max_z - min_z) * row as f32 / rows as f32;
            for column in 0..=columns {
                let x = min_x + (max_x - min_x) * column as f32 / columns as f32;
                self.vertices
                    .push(Vec3::new(x, -8.0 + dune_height(x, z), z));
            }
        }
        for row in 0..rows {
            for column in 0..columns {
                let i0 = start + row * (columns + 1) + column;
                let i1 = i0 + 1;
                let i3 = i0 + columns + 1;
                let i2 = i3 + 1;
                let cx = (self.vertices[i0].x + self.vertices[i2].x) * 0.5;
                let cz = (self.vertices[i0].z + self.vertices[i2].z) * 0.5;
                let dune_band = (cz * 0.018 + (cx * 0.012).sin() * 0.7).sin() * 6.0;
                let color = dune_palette(dune_height(cx, cz) * 0.24 + dune_band);
                for indices in [[i0, i1, i2], [i0, i2, i3]] {
                    let mut face = Face::new(indices, "violet_sand", "sand_surface");
                    face.shader = SurfaceShader::Flat;
                    face.color = MaterialColor::Fixed(color);
                    self.faces.push(face);
                }
            }
        }
        start..self.vertices.len()
    }

    fn finish(self) -> Model {
        Model::new(self.vertices, self.faces)
    }
}

impl Scene {
    pub fn new() -> Self {
        let mut builder = Builder::new();
        let mut obstacles = Vec::new();
        let sand_vertices = builder.sand_dunes();
        builder.mountain_ridge(
            -205.0,
            -10.0,
            &[
                (-280.0, -4.0),
                (-230.0, 18.0),
                (-174.0, 37.0),
                (-118.0, 47.0),
                (-62.0, 30.0),
                (-18.0, 15.0),
                (30.0, 19.0),
                (84.0, 41.0),
                (142.0, 54.0),
                (198.0, 42.0),
                (246.0, 15.0),
                (280.0, -4.0),
            ],
            [152, 82, 142],
        );
        builder.mountain_ridge(
            -222.0,
            -14.0,
            &[
                (-280.0, -14.0),
                (-226.0, 1.0),
                (-170.0, 14.0),
                (-112.0, 21.0),
                (-54.0, 7.0),
                (0.0, -4.0),
                (64.0, 4.0),
                (126.0, 24.0),
                (184.0, 28.0),
                (240.0, 11.0),
                (280.0, -14.0),
            ],
            [80, 55, 111],
        );
        for (x, z, width, height, depth) in [
            (-132.0, -102.0, 35.0, 49.0, 30.0),
            (137.0, -132.0, 38.0, 55.0, 34.0),
        ] {
            let base = -10.0 + dune_height(x, z);
            builder.mountain_mass(Vec3::new(x, base, z), width, height, depth);
            obstacles.push((
                Vec3::new(x, base + height * 0.48, z),
                Vec3::new(width * 0.76, height * 0.48, depth),
            ));
        }
        // Distant landmarks make the expanded valley readable as several
        // places to explore, while leaving the capsule flight lanes open.
        builder.mountain_ridge(
            -510.0,
            -18.0,
            &[
                (-570.0, -10.0),
                (-470.0, 28.0),
                (-360.0, 60.0),
                (-250.0, 34.0),
                (-130.0, 72.0),
                (0.0, 45.0),
                (135.0, 88.0),
                (260.0, 50.0),
                (390.0, 69.0),
                (500.0, 31.0),
                (570.0, -10.0),
            ],
            [154, 78, 145],
        );
        builder.mountain_ridge(
            -625.0,
            -22.0,
            &[
                (-570.0, -15.0),
                (-430.0, 22.0),
                (-280.0, 40.0),
                (-110.0, 16.0),
                (60.0, 53.0),
                (220.0, 26.0),
                (390.0, 57.0),
                (520.0, 24.0),
                (570.0, -15.0),
            ],
            [91, 58, 122],
        );
        for (x, y, z, width, color) in [
            (-48.0, 75.0, -112.0, 42.0, [148, 91, 136]),
            (51.0, 89.0, -146.0, 49.0, [164, 103, 140]),
            (2.0, 111.0, -188.0, 32.0, [137, 82, 131]),
            (-315.0, 93.0, -390.0, 58.0, [151, 91, 143]),
            (320.0, 112.0, -510.0, 66.0, [169, 105, 151]),
            (45.0, 182.0, -640.0, 46.0, [144, 82, 139]),
        ] {
            builder.floating_island(x, y, z, width, color);
            obstacles.push((
                Vec3::new(x, y, z),
                Vec3::new(width * 0.52, 4.5, width * 0.31),
            ));
        }

        // Side ruins frame the open route. Their supports do not cross the flight lane.
        let left_arch = Vec3::new(-58.0, -8.0 + dune_height(-58.0, -70.0) + 17.0, -70.0);
        let right_arch = Vec3::new(68.0, -8.0 + dune_height(68.0, -112.0) + 21.0, -112.0);
        builder.arch(left_arch, 17.0, 6.0, [205, 139, 96]);
        builder.arch(right_arch, 21.0, 6.8, [222, 158, 105]);
        for (center, radius, depth) in [(left_arch, 17.0, 6.0), (right_arch, 21.0, 6.8)] {
            let thickness = radius * 0.18;
            for side in [-1.0f32, 1.0] {
                obstacles.push((
                    Vec3::new(
                        center.x + side * (radius - thickness * 0.52),
                        center.y - radius * 0.52,
                        center.z,
                    ),
                    Vec3::new(thickness * 0.52, radius * 0.48, depth * 0.55),
                ));
            }
        }

        // Close stones add scale and provide visual reference without blocking capsules.
        for (x, z, size) in [
            (-72.0, -14.0, 3.6),
            (74.0, -34.0, 3.1),
            (-72.0, -59.0, 4.2),
            (75.0, -82.0, 3.8),
            (-73.0, -105.0, 3.0),
            (74.0, -129.0, 3.4),
        ] {
            let y = -8.0 + dune_height(x, z);
            builder.crystal(
                Vec3::new(x, y, z),
                size,
                size * 0.72,
                size * 0.62,
                [
                    [112, 68, 117],
                    [171, 113, 139],
                    [88, 55, 104],
                    [205, 134, 157],
                ],
            );
        }

        // Clockwork sentinels patrol the visual route. Their collision volumes
        // use the same time-damage rule as the ruins and other hazards.
        for (x, y, z) in [
            (-35.0, 30.0, -155.0),
            (105.0, 72.0, -325.0),
            (-95.0, 132.0, -445.0),
        ] {
            builder.box_at(Vec3::new(x, y, z), Vec3::new(4.0, 2.0, 5.0), [137, 42, 78]);
            builder.crystal(
                Vec3::new(x, y + 2.0, z),
                5.0,
                3.0,
                3.0,
                [[231, 70, 73], [177, 39, 81], [255, 122, 64], [112, 30, 72]],
            );
            obstacles.push((Vec3::new(x, y, z), Vec3::new(5.0, 5.0, 6.0)));
        }

        let capsule_faces = std::array::from_fn(|index| builder.capsule(CAPSULE_POSITIONS[index]));
        let clock_faces =
            std::array::from_fn(|index| builder.clock(super::CLOCK_POSITIONS[index]));
        let portal_faces = builder.portal_ring(Vec3::new(0.0, 155.0, -505.0), 12.0);
        builder.dragon_sphere(Vec3::new(0.0, 155.0, -510.0));
        let model = builder.finish();
        let capsule_indices = std::array::from_fn(|index| {
            capsule_faces[index]
                .clone()
                .map(|face_index| model.faces[face_index].indices)
                .collect()
        });
        let clock_indices = std::array::from_fn(|index| {
            clock_faces[index]
                .clone()
                .map(|face_index| model.faces[face_index].indices)
                .collect()
        });
        Self {
            model,
            sand_vertices,
            capsule_faces,
            capsule_indices,
            clock_faces,
            clock_indices,
            portal_faces,
            portal_open: false,
            obstacles,
        }
    }

    pub fn update_sand(&mut self, offset: f32) {
        // The visible mesh and the collision sampler share this exact vertical offset.
        for vertex in &mut self.model.vertices[self.sand_vertices.clone()] {
            vertex.y = -8.0 + dune_height(vertex.x, vertex.z) + offset;
        }
        self.model.recalculate_bounds();
    }

    pub fn resolve_obstacle(&self, position: Vec3, clearance: f32) -> Option<Vec3> {
        if !self.portal_open {
            let center = Vec3::new(0.0, 155.0, -505.0);
            let delta = position.sub(center);
            if delta.x.abs() < 11.0 + clearance
                && delta.y.abs() < 11.0 + clearance
                && delta.z.abs() < 2.0 + clearance
            {
                let mut corrected = position;
                corrected.z = center.z + if delta.z >= 0.0 { 2.0 + clearance } else { -2.0 - clearance };
                return Some(corrected);
            }
        }
        for &(center, half) in &self.obstacles {
            let delta = position.sub(center);
            let expanded = Vec3::new(half.x + clearance, half.y + clearance, half.z + clearance);
            if delta.x.abs() >= expanded.x
                || delta.y.abs() >= expanded.y
                || delta.z.abs() >= expanded.z
            {
                continue;
            }
            let penetration = [
                expanded.x - delta.x.abs(),
                expanded.y - delta.y.abs(),
                expanded.z - delta.z.abs(),
            ];
            let mut corrected = position;
            if penetration[0] <= penetration[1] && penetration[0] <= penetration[2] {
                corrected.x = center.x
                    + if delta.x >= 0.0 {
                        expanded.x
                    } else {
                        -expanded.x
                    };
            } else if penetration[1] <= penetration[2] {
                corrected.y = center.y
                    + if delta.y >= 0.0 {
                        expanded.y
                    } else {
                        -expanded.y
                    };
            } else {
                corrected.z = center.z
                    + if delta.z >= 0.0 {
                        expanded.z
                    } else {
                        -expanded.z
                    };
            }
            return Some(corrected);
        }
        None
    }

    pub fn hide_capsule(&mut self, index: usize) {
        self.set_capsule_visible(index, false);
    }

    pub fn set_capsule_visible(&mut self, index: usize, visible: bool) {
        let (Some(range), Some(original)) = (
            self.capsule_faces.get(index),
            self.capsule_indices.get(index),
        ) else {
            return;
        };
        for (face, &indices) in self.model.faces[range.clone()].iter_mut().zip(original) {
            face.indices = if visible { indices } else { [0, 0, 0] };
        }
    }

    pub fn set_portal_open(&mut self, open: bool) {
        self.portal_open = open;
        let color = if open { [84, 229, 241] } else { [61, 77, 100] };
        for face in &mut self.model.faces[self.portal_faces.clone()] {
            face.color = MaterialColor::Fixed(color);
        }
    }

    pub fn hide_clock(&mut self, index: usize) {
        let (Some(range), Some(original)) =
            (self.clock_faces.get(index), self.clock_indices.get(index))
        else {
            return;
        };
        for (face, _) in self.model.faces[range.clone()].iter_mut().zip(original) {
            face.indices = [0, 0, 0];
        }
    }
}

pub(super) fn dune_height(x: f32, z: f32) -> f32 {
    (x * 0.018 + z * 0.012).sin() * 12.0
        + (z * 0.024 - x * 0.013).cos() * 9.0
        + (x * 0.041 + z * 0.062).sin() * 3.5
}

fn dune_palette(height: f32) -> [u8; 3] {
    let t = ((height + 25.0) / 50.0).clamp(0.0, 1.0);
    [
        (61.0 + 153.0 * t) as u8,
        (35.0 + 97.0 * t) as u8,
        (101.0 + 87.0 * t) as u8,
    ]
}
