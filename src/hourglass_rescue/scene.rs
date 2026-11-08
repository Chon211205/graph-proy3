use std::ops::Range;

use crate::obj_loader::{Face, MaterialColor, Model, SurfaceShader, Vec3};

pub(super) const CAPSULE_COUNT: usize = 5;
pub(super) const CAPSULE_POSITIONS: [Vec3; CAPSULE_COUNT] = [
    Vec3::new(0.0, 9.0, 2.0),
    Vec3::new(-32.0, 18.0, -35.0),
    Vec3::new(34.0, 38.0, -8.0),
    Vec3::new(-24.0, 103.0, -8.0),
    Vec3::new(25.0, 109.0, -24.0),
];

pub(super) struct Scene {
    pub model: Model,
    pub sand_vertices: Range<usize>,
    pub capsule_faces: [Range<usize>; CAPSULE_COUNT],
    capsule_indices: [Vec<[usize; 3]>; CAPSULE_COUNT],
    portal_faces: Range<usize>,
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
        let [x, y, z] = [half.x, half.y, half.z];
        let [cx, cy, cz] = [center.x, center.y, center.z];
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

    fn ring_wall(
        &mut self,
        center_y: f32,
        height: f32,
        radius: f32,
        segments: usize,
        palette: &[[u8; 3]],
    ) {
        for index in 0..segments {
            let a0 = index as f32 * std::f32::consts::TAU / segments as f32;
            let a1 = (index + 1) as f32 * std::f32::consts::TAU / segments as f32;
            let color = palette[index % palette.len()];
            let low0 = Vec3::new(
                radius * a0.cos(),
                center_y - height * 0.5,
                radius * a0.sin(),
            );
            let low1 = Vec3::new(
                radius * a1.cos(),
                center_y - height * 0.5,
                radius * a1.sin(),
            );
            let high1 = Vec3::new(
                radius * a1.cos(),
                center_y + height * 0.5,
                radius * a1.sin(),
            );
            let high0 = Vec3::new(
                radius * a0.cos(),
                center_y + height * 0.5,
                radius * a0.sin(),
            );
            self.quad(low0, low1, high1, high0, color);
        }
    }

    fn arch(&mut self, center: Vec3, radius: f32, depth: f32, color: [u8; 3]) {
        let segments = 12;
        let thickness = 2.0;
        for index in 0..segments {
            let a0 = std::f32::consts::PI * index as f32 / segments as f32;
            let a1 = std::f32::consts::PI * (index + 1) as f32 / segments as f32;
            let inner = radius - thickness;
            let outer0 = Vec3::new(
                center.x + radius * a0.cos(),
                center.y + radius * a0.sin(),
                center.z,
            );
            let outer1 = Vec3::new(
                center.x + radius * a1.cos(),
                center.y + radius * a1.sin(),
                center.z,
            );
            let inner0 = Vec3::new(
                center.x + inner * a0.cos(),
                center.y + inner * a0.sin(),
                center.z,
            );
            let inner1 = Vec3::new(
                center.x + inner * a1.cos(),
                center.y + inner * a1.sin(),
                center.z,
            );
            let back = Vec3::new(0.0, 0.0, depth);
            self.quad(outer0, outer1, inner1, inner0, color);
            self.quad(
                outer0.add(back),
                inner0.add(back),
                inner1.add(back),
                outer1.add(back),
                color,
            );
            self.quad(
                outer0,
                outer0.add(back),
                outer1.add(back),
                outer1,
                [
                    color[0].saturating_sub(28),
                    color[1].saturating_sub(24),
                    color[2].saturating_sub(20),
                ],
            );
        }
    }

    fn capsule(&mut self, center: Vec3) -> Range<usize> {
        let start = self.faces.len();
        let sides = 10;
        let rings = [(0.0, 1.25), (0.35, 1.6), (2.35, 1.6), (2.75, 1.25)];
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
                center.x + a0.cos() * 1.25,
                center.y + 2.76,
                center.z + a0.sin() * 1.25,
            );
            let top1 = Vec3::new(
                center.x + a1.cos() * 1.25,
                center.y + 2.76,
                center.z + a1.sin() * 1.25,
            );
            self.triangle(
                top0,
                top1,
                Vec3::new(center.x, center.y + 3.0, center.z),
                [240, 199, 112],
            );
            let window = |angle: f32| {
                Vec3::new(
                    center.x + angle.cos() * 1.62,
                    center.y + 1.75,
                    center.z + angle.sin() * 1.62,
                )
            };
            self.quad(
                window(a0),
                window(a1),
                window(a1).add(Vec3::new(0.0, 0.58, 0.0)),
                window(a0).add(Vec3::new(0.0, 0.58, 0.0)),
                [77, 225, 235],
            );
        }
        start..self.faces.len()
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
        let rings = 7;
        let segments = 40;
        let radius = 64.0;
        let mut grid = Vec::with_capacity((rings + 1) * segments);
        for ring in 0..=rings {
            let r = radius * ring as f32 / rings as f32;
            for segment in 0..segments {
                let angle = segment as f32 * std::f32::consts::TAU / segments as f32;
                let x = r * angle.cos();
                let z = r * angle.sin();
                let dune = (x * 0.075 + z * 0.039).sin() * 0.9 + (z * 0.12).cos() * 0.45;
                grid.push(Vec3::new(x, -8.0 + dune, z));
            }
        }
        self.vertices.extend(grid.iter().copied());
        let first_face = self.faces.len();
        for ring in 0..rings {
            for segment in 0..segments {
                let next = (segment + 1) % segments;
                let i0 = ring * segments + segment;
                let i1 = ring * segments + next;
                let i2 = (ring + 1) * segments + next;
                let i3 = (ring + 1) * segments + segment;
                let color = if (ring + segment) % 4 == 0 {
                    [124, 63, 143]
                } else {
                    [104, 49, 124]
                };
                for indices in [[i0, i1, i2], [i0, i2, i3]] {
                    let mut face =
                        Face::new(indices.map(|i| start + i), "violet_sand", "sand_surface");
                    face.shader = SurfaceShader::Flat;
                    face.color = MaterialColor::Fixed(color);
                    self.faces.push(face);
                }
            }
        }
        let _face_range = first_face..self.faces.len();
        start..self.vertices.len()
    }

    fn finish(self) -> Model {
        Model::new(self.vertices, self.faces)
    }
}

impl Scene {
    pub fn new() -> Self {
        let mut builder = Builder::new();
        let sand_vertices = builder.sand_dunes();
        builder.ring_wall(
            23.0,
            62.0,
            76.0,
            40,
            &[[86, 48, 117], [111, 60, 140], [139, 82, 157], [67, 47, 98]],
        );
        builder.ring_wall(
            108.0,
            70.0,
            65.0,
            36,
            &[[103, 61, 129], [134, 77, 153], [83, 58, 119]],
        );

        // The tapered neck makes the two chambers read as a single giant hourglass.
        for segment in 0..28 {
            let a0 = segment as f32 * std::f32::consts::TAU / 28.0;
            let a1 = (segment + 1) as f32 * std::f32::consts::TAU / 28.0;
            let p = |radius: f32, y: f32, angle: f32| {
                Vec3::new(radius * angle.cos(), y, radius * angle.sin())
            };
            builder.quad(
                p(19.0, 51.0, a0),
                p(19.0, 51.0, a1),
                p(12.0, 78.0, a1),
                p(12.0, 78.0, a0),
                if segment % 2 == 0 {
                    [190, 137, 205]
                } else {
                    [131, 83, 161]
                },
            );
        }

        // Broken crystal facets, warm sandstone ledges, and a few readable landmarks.
        for index in 0..18 {
            let angle = index as f32 * std::f32::consts::TAU / 18.0;
            let position = Vec3::new(
                angle.cos() * 72.0,
                15.0 + (index % 4) as f32 * 8.0,
                angle.sin() * 72.0,
            );
            let tip = position.add(Vec3::new(
                angle.cos() * 4.0,
                8.0 + (index % 3) as f32 * 2.0,
                angle.sin() * 4.0,
            ));
            let a = position.add(Vec3::new(-2.2, -3.0, 0.0));
            let b = position.add(Vec3::new(2.2, -3.0, 0.0));
            let c = position.add(Vec3::new(0.0, -3.0, 2.2));
            builder.triangle(a, b, tip, [160, 99, 196]);
            builder.triangle(b, c, tip, [211, 156, 232]);
            builder.triangle(c, a, tip, [126, 77, 170]);
        }
        for (x, z, y, width) in [
            (-46.0, -25.0, 0.0, 17.0),
            (44.0, -32.0, 6.0, 18.0),
            (-39.0, 36.0, 27.0, 14.0),
            (40.0, 29.0, 94.0, 14.0),
        ] {
            builder.box_at(
                Vec3::new(x, y - 1.5, z),
                Vec3::new(width, 1.5, 9.0),
                [143, 99, 72],
            );
            builder.box_at(
                Vec3::new(x, y + 0.4, z),
                Vec3::new(width, 0.6, 9.4),
                [206, 159, 107],
            );
            builder.box_at(
                Vec3::new(x - width * 0.6, y + 4.5, z),
                Vec3::new(1.8, 5.0, 2.0),
                [168, 119, 83],
            );
            builder.box_at(
                Vec3::new(x + width * 0.6, y + 4.5, z),
                Vec3::new(1.8, 5.0, 2.0),
                [168, 119, 83],
            );
        }
        for (x, z, y) in [
            (-50.0, -48.0, -3.0),
            (50.0, -3.0, -2.0),
            (-48.0, 45.0, 20.0),
            (46.0, 43.0, 20.0),
            (-35.0, -44.0, 88.0),
            (35.0, -47.0, 92.0),
        ] {
            builder.box_at(
                Vec3::new(x, y + 7.0, z),
                Vec3::new(2.3, 7.0, 2.7),
                [151, 101, 70],
            );
            builder.box_at(
                Vec3::new(x, y + 14.5, z),
                Vec3::new(3.4, 1.5, 3.6),
                [216, 167, 109],
            );
        }
        builder.arch(Vec3::new(-46.0, 8.0, -29.0), 12.0, 5.0, [190, 132, 86]);
        builder.arch(Vec3::new(42.0, 32.0, -8.0), 10.0, 4.0, [207, 155, 100]);
        builder.arch(Vec3::new(0.0, 99.0, -54.0), 13.0, 5.0, [201, 151, 104]);

        // Small floating ruin fragments frame the neck without blocking its flight path.
        for (x, y, z, size) in [
            (-31.0, 42.0, -31.0, 6.5),
            (30.0, 47.0, -37.0, 5.5),
            (-27.0, 121.0, -34.0, 7.0),
            (29.0, 125.0, -38.0, 6.0),
        ] {
            builder.box_at(
                Vec3::new(x, y, z),
                Vec3::new(size, 2.0, size * 0.7),
                [111, 79, 109],
            );
            builder.box_at(
                Vec3::new(x, y - 3.2, z),
                Vec3::new(size * 0.45, 3.4, size * 0.36),
                [79, 62, 91],
            );
        }

        let capsule_faces = std::array::from_fn(|index| builder.capsule(CAPSULE_POSITIONS[index]));
        let portal_faces = builder.portal_ring(Vec3::new(0.0, 106.0, -48.0), 9.5);
        let model = builder.finish();
        let capsule_indices = std::array::from_fn(|index| {
            capsule_faces[index]
                .clone()
                .map(|face_index| model.faces[face_index].indices)
                .collect()
        });
        Self {
            model,
            sand_vertices,
            capsule_faces,
            capsule_indices,
            portal_faces,
        }
    }

    pub fn update_sand(&mut self, offset: f32) {
        // The visible mesh and the collision sampler share this exact vertical offset.
        for vertex in &mut self.model.vertices[self.sand_vertices.clone()] {
            vertex.y = -8.0 + dune_height(vertex.x, vertex.z) + offset;
        }
        self.model.recalculate_bounds();
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
        let color = if open { [84, 229, 241] } else { [61, 77, 100] };
        for face in &mut self.model.faces[self.portal_faces.clone()] {
            face.color = MaterialColor::Fixed(color);
        }
    }
}

pub(super) fn dune_height(x: f32, z: f32) -> f32 {
    (x * 0.075 + z * 0.039).sin() * 0.9 + (z * 0.12).cos() * 0.45
}
