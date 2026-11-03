use crate::obj_loader::{Face, MaterialColor, Model, SurfaceShader, Vec3};

use super::course::{CURB_WIDTH, HALF_ROAD, RaceCourse};

pub(super) struct RaceScenery {
    pub forests: Vec<Model>,
    pub arches: Vec<Model>,
    pub terrain: Model,
    pub hills: Model,
    pub ringed_planet: Model,
}

impl RaceScenery {
    pub fn new(course: &RaceCourse) -> Self {
        let mut forests = Vec::new();
        let mut arches = Vec::new();
        let chunk_size = (course.samples.len() / 6).max(1);
        for chunk in 0..6 {
            let start = chunk * chunk_size;
            let end = ((chunk + 1) * chunk_size).min(course.samples.len());
            let mut forest = MeshBuilder::default();
            for index in (start..end).step_by(5) {
                let sample = course.samples[index];
                let side = Vec3::new(-sample.tangent.z, 0.0, sample.tangent.x);
                for direction in [-1.0, 1.0] {
                    // Keep the forest close enough to frame the road while
                    // leaving a clear sightline through upcoming corners.
                    let spacing = 8.0 + ((index * 7 + chunk * 13) % 11) as f32;
                    let offset = direction * (HALF_ROAD + CURB_WIDTH + spacing);
                    let position = sample.position.add(side.mul(offset));
                    let seed = (index * 13 + chunk * 31) as f32;
                    forest.tree(position, 5.5 + seed.sin().abs() * 3.8, seed);
                    forest.fern(
                        position.add(side.mul(direction * 7.0)),
                        sample.tangent,
                        seed * 0.7,
                    );
                }
            }
            forests.push(forest.finish());

            let mut arch = MeshBuilder::default();
            let center_index = (start + end) / 2;
            if chunk % 2 == 1 {
                arch.stone_arch(course.samples[center_index]);
            }
            arches.push(arch.finish());
        }

        let mut hills = MeshBuilder::default();
        hills.hills();
        let mut terrain = MeshBuilder::default();
        terrain.terrain();
        let mut planet = MeshBuilder::default();
        planet.sphere(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(52.0, 42.0, 42.0),
            16,
            8,
            [89, 157, 179],
        );
        planet.ring(66.0, 83.0, 2.2, 48, [189, 181, 131]);

        Self {
            forests,
            arches,
            terrain: terrain.finish(),
            hills: hills.finish(),
            ringed_planet: planet.finish(),
        }
    }
}

#[derive(Default)]
struct MeshBuilder {
    vertices: Vec<Vec3>,
    faces: Vec<Face>,
}

impl MeshBuilder {
    fn tri(&mut self, a: Vec3, b: Vec3, c: Vec3, color: [u8; 3], group: &str) {
        let base = self.vertices.len();
        self.vertices.extend([a, b, c]);
        let mut face = Face::new([base, base + 1, base + 2], "procedural", group);
        face.color = MaterialColor::Fixed(color);
        face.shader = SurfaceShader::Flat;
        self.faces.push(face);
    }

    fn quad(&mut self, a: Vec3, b: Vec3, c: Vec3, d: Vec3, color: [u8; 3], group: &str) {
        self.tri(a, b, c, color, group);
        self.tri(a, c, d, color, group);
    }

    fn tree(&mut self, base: Vec3, height: f32, seed: f32) {
        let trunk_height = height * 0.55;
        self.tapered_trunk(base, trunk_height, 1.15, 0.47, seed);
        let crown = base.add(Vec3::new(0.0, trunk_height + height * 0.11, 0.0));
        self.sphere(
            crown,
            Vec3::new(height * 0.58, height * 0.34, height * 0.58),
            7,
            4,
            if seed as i32 % 2 == 0 {
                [39, 128, 86]
            } else {
                [48, 145, 103]
            },
        );
        let branch = crown.add(Vec3::new(
            seed.sin() * height * 0.3,
            height * 0.12,
            seed.cos() * height * 0.25,
        ));
        self.sphere(
            branch,
            Vec3::new(height * 0.35, height * 0.24, height * 0.35),
            7,
            4,
            [82, 166, 108],
        );
        let glow_leaf = crown.add(Vec3::new(-height * 0.2, height * 0.2, height * 0.14));
        self.sphere(
            glow_leaf,
            Vec3::new(height * 0.16, height * 0.25, height * 0.16),
            6,
            3,
            if (seed as i32).rem_euclid(4) == 0 {
                [189, 171, 96]
            } else {
                [135, 190, 115]
            },
        );
    }

    fn tapered_trunk(&mut self, base: Vec3, height: f32, bottom: f32, top: f32, seed: f32) {
        let sides = 7;
        for side in 0..sides {
            let a = side as f32 * std::f32::consts::TAU / sides as f32 + seed;
            let b = (side + 1) as f32 * std::f32::consts::TAU / sides as f32 + seed;
            let low_a = base.add(Vec3::new(a.cos() * bottom, 0.0, a.sin() * bottom));
            let low_b = base.add(Vec3::new(b.cos() * bottom, 0.0, b.sin() * bottom));
            let high_a = base.add(Vec3::new(a.cos() * top, height, a.sin() * top));
            let high_b = base.add(Vec3::new(b.cos() * top, height, b.sin() * top));
            self.quad(
                low_a,
                low_b,
                high_b,
                high_a,
                [91, 75, 54],
                "alien_tree_trunks",
            );
            if side % 3 == 0 {
                self.quad(
                    low_a,
                    low_b,
                    low_b.add(Vec3::new(0.0, 0.25, 0.0)),
                    low_a.add(Vec3::new(0.0, 0.25, 0.0)),
                    [78, 132, 79],
                    "moss_on_trunks",
                );
            }
        }
    }

    fn fern(&mut self, base: Vec3, tangent: Vec3, seed: f32) {
        let side = Vec3::new(-tangent.z, 0.0, tangent.x);
        let stem_tip = base.add(Vec3::new(0.0, 2.1, 0.0));
        self.tri(
            base,
            stem_tip,
            base.add(tangent.mul(0.2)),
            [93, 147, 82],
            "fern_stems",
        );
        for leaf in 0..5 {
            let t = 0.25 + leaf as f32 * 0.14;
            let spine = base.add(Vec3::new(0.0, 2.1 * t, 0.0));
            let direction = side.mul(if leaf % 2 == 0 { 1.0 } else { -1.0 });
            let length = 1.3 + ((seed + leaf as f32).sin() * 0.4).abs();
            let tip = spine
                .add(direction.mul(length))
                .add(Vec3::new(0.0, 0.28, 0.0));
            self.tri(
                spine,
                tip,
                spine.add(Vec3::new(0.0, 0.16, 0.0)),
                [77, 165, 100],
                "fern_fronds",
            );
        }
    }

    fn stone_arch(&mut self, sample: super::course::TrackSample) {
        let side = Vec3::new(-sample.tangent.z, 0.0, sample.tangent.x);
        let outer = HALF_ROAD + CURB_WIDTH + 1.5;
        for direction in [-1.0, 1.0] {
            self.box_aligned(
                sample.position.add(side.mul(outer * direction)),
                sample.tangent,
                Vec3::new(2.8, 6.7, 2.6),
                [107, 107, 83],
                "mossy_arch_stone",
            );
            self.box_aligned(
                sample
                    .position
                    .add(side.mul(outer * direction))
                    .add(Vec3::new(0.0, 5.1, 0.0)),
                sample.tangent,
                Vec3::new(3.4, 1.5, 1.8),
                [103, 119, 78],
                "arch_moss",
            );
        }
        for segment in 0..9 {
            let amount = segment as f32 / 8.0;
            let across = -outer + amount * outer * 2.0;
            let arch_height = 6.8 + (1.0 - (amount * 2.0 - 1.0).abs()) * 2.0;
            let center =
                sample
                    .position
                    .add(side.mul(across))
                    .add(Vec3::new(0.0, arch_height, 0.0));
            self.box_aligned(
                center,
                side,
                Vec3::new(2.1, 1.2, 1.7),
                if segment % 3 == 0 {
                    [103, 119, 78]
                } else {
                    [128, 120, 91]
                },
                "mossy_arch_stone",
            );
        }
        for direction in [-1.0, 1.0] {
            let vine_start = sample
                .position
                .add(side.mul(outer * direction))
                .add(Vec3::new(0.0, 7.0, 0.0));
            self.vine(vine_start, Vec3::new(0.0, -4.2, 1.4), [57, 128, 76]);
        }
    }

    fn box_aligned(
        &mut self,
        center: Vec3,
        forward: Vec3,
        half_size: Vec3,
        color: [u8; 3],
        group: &str,
    ) {
        let right = Vec3::new(-forward.z, 0.0, forward.x);
        let corners = [
            center
                .sub(forward.mul(half_size.z))
                .sub(right.mul(half_size.x)),
            center
                .add(forward.mul(half_size.z))
                .sub(right.mul(half_size.x)),
            center
                .add(forward.mul(half_size.z))
                .add(right.mul(half_size.x)),
            center
                .sub(forward.mul(half_size.z))
                .add(right.mul(half_size.x)),
        ];
        let up = Vec3::new(0.0, half_size.y, 0.0);
        self.quad(corners[0], corners[1], corners[2], corners[3], color, group);
        for index in 0..4 {
            let next = (index + 1) % 4;
            self.quad(
                corners[index].sub(up),
                corners[next].sub(up),
                corners[next].add(up),
                corners[index].add(up),
                color,
                group,
            );
        }
        self.quad(
            corners[0].add(up),
            corners[1].add(up),
            corners[2].add(up),
            corners[3].add(up),
            color,
            group,
        );
    }

    fn vine(&mut self, start: Vec3, drift: Vec3, color: [u8; 3]) {
        let steps = 6;
        let mut previous = start;
        for index in 1..=steps {
            let t = index as f32 / steps as f32;
            let point =
                start
                    .add(drift.mul(t))
                    .add(Vec3::new((t * 7.0).sin() * 0.35, -t * t * 1.2, 0.0));
            let side = Vec3::new(0.12, 0.0, 0.0);
            self.quad(
                previous.sub(side),
                point.sub(side),
                point.add(side),
                previous.add(side),
                color,
                "hanging_vines",
            );
            previous = point;
        }
    }

    fn sphere(&mut self, center: Vec3, radii: Vec3, sectors: usize, stacks: usize, color: [u8; 3]) {
        for stack in 0..stacks {
            let v0 = stack as f32 / stacks as f32;
            let v1 = (stack + 1) as f32 / stacks as f32;
            let lat0 = (v0 - 0.5) * std::f32::consts::PI;
            let lat1 = (v1 - 0.5) * std::f32::consts::PI;
            for sector in 0..sectors {
                let a0 = sector as f32 * std::f32::consts::TAU / sectors as f32;
                let a1 = (sector + 1) as f32 * std::f32::consts::TAU / sectors as f32;
                let point = |latitude: f32, longitude: f32| {
                    center.add(Vec3::new(
                        radii.x * latitude.cos() * longitude.cos(),
                        radii.y * latitude.sin(),
                        radii.z * latitude.cos() * longitude.sin(),
                    ))
                };
                let a = point(lat0, a0);
                let b = point(lat0, a1);
                let c = point(lat1, a1);
                let d = point(lat1, a0);
                self.quad(a, b, c, d, color, "faceted_canopies");
            }
        }
    }

    fn ring(&mut self, inner: f32, outer: f32, thickness: f32, sectors: usize, color: [u8; 3]) {
        for sector in 0..sectors {
            let a0 = sector as f32 * std::f32::consts::TAU / sectors as f32;
            let a1 = (sector + 1) as f32 * std::f32::consts::TAU / sectors as f32;
            let point = |radius: f32, angle: f32, y: f32| {
                Vec3::new(radius * angle.cos(), y, radius * angle.sin())
            };
            self.quad(
                point(inner, a0, 0.0),
                point(outer, a0, thickness),
                point(outer, a1, thickness),
                point(inner, a1, 0.0),
                if sector % 3 == 0 {
                    [211, 203, 157]
                } else {
                    color
                },
                "ringed_planet_rings",
            );
        }
    }

    fn hills(&mut self) {
        // Distant low hills use a shallow profile. The old tall, flat
        // triangles sat beside the road and filled the camera with green
        // polygons, hiding the course and the rivals.
        let ridges = [
            (Vec3::new(190.0, 0.0, -72.0), 210.0, 18.0),
            (Vec3::new(190.0, 0.0, 315.0), 210.0, 20.0),
            (Vec3::new(-82.0, 0.0, 120.0), 90.0, 16.0),
            (Vec3::new(462.0, 0.0, 120.0), 90.0, 17.0),
        ];
        for (center, half_width, height) in ridges {
            for segment in 0..24 {
                let t0 = segment as f32 / 24.0;
                let t1 = (segment + 1) as f32 / 24.0;
                let x0 = center.x - half_width + t0 * half_width * 2.0;
                let x1 = center.x - half_width + t1 * half_width * 2.0;
                let hill_y0 = (t0 * std::f32::consts::PI).sin() * height;
                let hill_y1 = (t1 * std::f32::consts::PI).sin() * height;
                let near0 = Vec3::new(x0, 0.0, center.z - 18.0);
                let near1 = Vec3::new(x1, 0.0, center.z - 18.0);
                let crest0 = Vec3::new(x0, hill_y0, center.z + 6.0);
                let crest1 = Vec3::new(x1, hill_y1, center.z + 6.0);
                let far0 = Vec3::new(x0, 0.0, center.z + 34.0);
                let far1 = Vec3::new(x1, 0.0, center.z + 34.0);
                self.quad(near0, near1, crest1, crest0, [58, 111, 76], "misty_green_hills");
                self.quad(crest0, crest1, far1, far0, [79, 132, 83], "hill_slopes");
            }
        }
    }

    fn terrain(&mut self) {
        let width = 620.0;
        let depth = 380.0;
        let columns = 36;
        let rows = 24;
        for row in 0..rows {
            for column in 0..columns {
                let x0 = column as f32 * width / columns as f32 - 110.0;
                let x1 = (column + 1) as f32 * width / columns as f32 - 110.0;
                let z0 = row as f32 * depth / rows as f32 - 60.0;
                let z1 = (row + 1) as f32 * depth / rows as f32 - 60.0;
                let variation = ((column * 37 + row * 19) % 5) as u8;
                let green = 78 + variation * 5;
                let color = [38 + variation * 2, green, 62 + variation * 3];
                self.quad(
                    Vec3::new(x0, -0.8, z0),
                    Vec3::new(x1, -0.8, z0),
                    Vec3::new(x1, -0.8, z1),
                    Vec3::new(x0, -0.8, z1),
                    color,
                    "lush_track_ground",
                );
            }
        }
        for patch in 0..30 {
            let x = ((patch * 73 + 29) % 600) as f32 - 100.0;
            let z = ((patch * 47 + 13) % 360) as f32 - 50.0;
            let radius = 3.5 + (patch % 4) as f32 * 1.4;
            self.quad(
                Vec3::new(x - radius, -0.72, z),
                Vec3::new(x, -0.72, z - radius * 0.55),
                Vec3::new(x + radius, -0.72, z),
                Vec3::new(x, -0.72, z + radius * 0.55),
                if patch % 2 == 0 {
                    [117, 142, 76]
                } else {
                    [143, 133, 72]
                },
                "warm_filtered_sunlight",
            );
        }
    }

    fn finish(self) -> Model {
        Model::new(self.vertices, self.faces)
    }
}
