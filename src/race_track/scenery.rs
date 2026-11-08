use std::collections::BTreeMap;

use crate::obj_loader::{Face, MaterialColor, Model, SurfaceShader, Vec3};

use super::course::{CURB_WIDTH, HALF_ROAD, RaceCourse};

pub(super) struct RaceScenery {
    pub forests: Vec<Model>,
    pub forest_lods: Vec<Model>,
    pub arches: Vec<Model>,
    pub terrain: Vec<Model>,
    pub hills: Vec<Model>,
}

impl RaceScenery {
    pub fn new(course: &RaceCourse) -> Self {
        let mut forests = Vec::new();
        let mut forest_lods = Vec::new();
        let mut arches = Vec::new();
        // Small spatial chunks let the renderer reject most vegetation before
        // transforming its detailed smooth geometry. This matters especially
        // in fullscreen where rasterization has less CPU headroom.
        const SCENERY_CHUNKS: usize = 18;
        let chunk_size = course.samples.len().div_ceil(SCENERY_CHUNKS);
        for chunk in 0..SCENERY_CHUNKS {
            let start = chunk * chunk_size;
            let end = ((chunk + 1) * chunk_size).min(course.samples.len());
            let mut forest = MeshBuilder::default();
            let mut forest_lod = MeshBuilder::default();
            for index in (start..end).step_by(5) {
                let sample = course.samples[index];
                let side = Vec3::new(-sample.tangent.z, 0.0, sample.tangent.x);
                for direction in [-1.0, 1.0] {
                    let seed = (index * 13 + chunk * 31) as f32;
                    let tree_height = 5.5 + seed.sin().abs() * 3.8;
                    let crown_radius = tree_height * 0.58;
                    let clearance = HALF_ROAD + CURB_WIDTH + crown_radius + 4.0;
                    let spacing = 12.0 + ((index * 7 + chunk * 13) % 11) as f32;
                    let Some(position) = safe_offset_from_course(
                        course,
                        sample.position,
                        side,
                        direction,
                        HALF_ROAD + CURB_WIDTH + spacing,
                        clearance,
                    ) else {
                        continue;
                    };
                    forest.tree(position, tree_height, seed);
                    forest_lod.tree_lod(position, tree_height, seed);
                    forest.fern(
                        position.add(side.mul(direction * 7.0)),
                        sample.tangent,
                        seed * 0.7,
                    );
                }
            }
            forests.push(forest.finish());
            forest_lods.push(forest_lod.finish());

            let mut arch = MeshBuilder::default();
            let center_index = (start + end) / 2;
            if chunk % 6 == 3 {
                arch.stone_arch(course.samples[center_index]);
            }
            arches.push(arch.finish());
        }

        let mut hills = MeshBuilder::default();
        hills.hills(course);
        let mut terrain = MeshBuilder::default();
        terrain.terrain();
        Self {
            forests,
            forest_lods,
            arches,
            terrain: spatial_chunks(terrain.finish(), 100.0),
            hills: spatial_chunks(hills.finish(), 100.0),
        }
    }
}

fn spatial_chunks(model: Model, cell_size: f32) -> Vec<Model> {
    let mut cells: BTreeMap<(i32, i32), Vec<Face>> = BTreeMap::new();
    for source in model.faces {
        let center = source
            .indices
            .iter()
            .map(|&index| model.vertices[index])
            .fold(Vec3::default(), |sum, vertex| sum.add(vertex))
            .mul(1.0 / 3.0);
        let key = (
            (center.x / cell_size).floor() as i32,
            (center.z / cell_size).floor() as i32,
        );
        cells.entry(key).or_default().push(source);
    }
    cells
        .into_values()
        .map(|faces| {
            let mut remap = vec![usize::MAX; model.vertices.len()];
            let mut vertices = Vec::new();
            let faces = faces
                .into_iter()
                .map(|source| {
                    let indices = source.indices.map(|old| {
                        if remap[old] == usize::MAX {
                            remap[old] = vertices.len();
                            vertices.push(model.vertices[old]);
                        }
                        remap[old]
                    });
                    let mut face = Face::new(indices, source.material, source.group);
                    face.shader = source.shader;
                    face.color = source.color;
                    face.smooth = source.smooth;
                    face
                })
                .collect();
            Model::new(vertices, faces)
        })
        .collect()
}

fn safe_offset_from_course(
    course: &RaceCourse,
    origin: Vec3,
    side: Vec3,
    direction: f32,
    initial_offset: f32,
    clearance: f32,
) -> Option<Vec3> {
    let mut offset = initial_offset;
    while offset <= 160.0 {
        let candidate = origin.add(side.mul(direction * offset));
        let (_, distance, _) = course.nearest(candidate);
        if distance >= clearance {
            return Some(candidate);
        }
        offset += 4.0;
    }
    None
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
        face.shader = match group {
            "alien_tree_roots" => SurfaceShader::RaceBarkTexture,
            "mossy_arch_stone" | "arch_carved_bases" | "arch_carved_capitals" => {
                SurfaceShader::RaceBricksTexture
            }
            _ => SurfaceShader::Flat,
        };
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
            12,
            7,
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
            10,
            6,
            [82, 166, 108],
        );
        let glow_leaf = crown.add(Vec3::new(-height * 0.2, height * 0.2, height * 0.14));
        self.sphere(
            glow_leaf,
            Vec3::new(height * 0.16, height * 0.25, height * 0.16),
            9,
            5,
            if (seed as i32).rem_euclid(4) == 0 {
                [189, 171, 96]
            } else {
                [135, 190, 115]
            },
        );
        for cluster in 0..3 {
            let angle = seed + cluster as f32 * std::f32::consts::TAU / 3.0;
            self.sphere(
                crown.add(Vec3::new(
                    angle.cos() * height * 0.34,
                    height * (0.04 + cluster as f32 * 0.035),
                    angle.sin() * height * 0.34,
                )),
                Vec3::new(height * 0.25, height * 0.20, height * 0.25),
                9,
                5,
                [55 + cluster as u8 * 9, 142 + cluster as u8 * 8, 92],
            );
        }
        // Exposed roots visually anchor the tree instead of leaving a trunk
        // balanced on a single point.
        for root in 0..4 {
            let angle = seed + root as f32 * std::f32::consts::TAU / 4.0;
            self.box_aligned(
                base.add(Vec3::new(angle.cos() * 0.8, 0.12, angle.sin() * 0.8)),
                Vec3::new(angle.cos(), 0.0, angle.sin()),
                Vec3::new(0.24, 0.18, 1.15),
                [83, 70, 50],
                "alien_tree_roots",
            );
        }
    }

    /// A closed, deliberately authored distant tree.  The old LOD kept every
    /// third triangle from the detailed mesh, leaving visible holes in trunks
    /// and crowns.  Fewer complete volumes are both cheaper and read as a tree
    /// much more clearly at small screen sizes.
    fn tree_lod(&mut self, base: Vec3, height: f32, seed: f32) {
        let trunk_height = height * 0.55;
        self.tapered_trunk_lod(base, trunk_height, 1.15, 0.47, seed);
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
            6,
            3,
            [82, 166, 108],
        );
        let glow_leaf = crown.add(Vec3::new(-height * 0.2, height * 0.2, height * 0.14));
        self.sphere(
            glow_leaf,
            Vec3::new(height * 0.16, height * 0.25, height * 0.16),
            5,
            3,
            if (seed as i32).rem_euclid(4) == 0 {
                [189, 171, 96]
            } else {
                [135, 190, 115]
            },
        );
        for root in 0..3 {
            let angle = seed + root as f32 * std::f32::consts::TAU / 3.0;
            self.box_aligned(
                base.add(Vec3::new(angle.cos() * 0.75, 0.12, angle.sin() * 0.75)),
                Vec3::new(angle.cos(), 0.0, angle.sin()),
                Vec3::new(0.24, 0.18, 1.05),
                [83, 70, 50],
                "alien_tree_roots",
            );
        }
    }

    fn tapered_trunk_lod(
        &mut self,
        base: Vec3,
        height: f32,
        bottom: f32,
        top: f32,
        seed: f32,
    ) {
        let sides = 7;
        let start = self.vertices.len();
        for side in 0..sides {
            let angle = side as f32 * std::f32::consts::TAU / sides as f32 + seed;
            self.vertices.push(
                base.add(Vec3::new(angle.cos() * bottom, 0.0, angle.sin() * bottom)),
            );
            self.vertices.push(
                base.add(Vec3::new(angle.cos() * top, height, angle.sin() * top)),
            );
        }
        for side in 0..sides {
            let next = (side + 1) % sides;
            for indices in [
                [start + side * 2, start + next * 2, start + next * 2 + 1],
                [start + side * 2, start + next * 2 + 1, start + side * 2 + 1],
            ] {
                let mut face = Face::new(indices, "procedural", "alien_tree_trunks");
                face.color = MaterialColor::Fixed([91, 75, 54]);
                face.shader = SurfaceShader::RaceBarkTexture;
                face.smooth = true;
                self.faces.push(face);
            }
        }
    }

    fn tapered_trunk(&mut self, base: Vec3, height: f32, bottom: f32, top: f32, seed: f32) {
        let sides = 12;
        let start = self.vertices.len();
        for side in 0..sides {
            let a = side as f32 * std::f32::consts::TAU / sides as f32 + seed;
            self.vertices
                .push(base.add(Vec3::new(a.cos() * bottom, 0.0, a.sin() * bottom)));
            self.vertices
                .push(base.add(Vec3::new(a.cos() * top, height, a.sin() * top)));
        }
        for side in 0..sides {
            let next = (side + 1) % sides;
            for indices in [
                [start + side * 2, start + next * 2, start + next * 2 + 1],
                [start + side * 2, start + next * 2 + 1, start + side * 2 + 1],
            ] {
                let mut face = Face::new(indices, "procedural", "alien_tree_trunks");
                face.color = MaterialColor::Fixed([91, 75, 54]);
                face.shader = SurfaceShader::RaceBarkTexture;
                face.smooth = true;
                self.faces.push(face);
            }
            if side % 3 == 0 {
                let a = side as f32 * std::f32::consts::TAU / sides as f32 + seed;
                let b = (side + 1) as f32 * std::f32::consts::TAU / sides as f32 + seed;
                let low_a = base.add(Vec3::new(a.cos() * bottom, 0.0, a.sin() * bottom));
                let low_b = base.add(Vec3::new(b.cos() * bottom, 0.0, b.sin() * bottom));
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
        for leaf in 0..9 {
            let t = 0.18 + leaf as f32 * 0.085;
            let spine = base.add(Vec3::new(0.0, 2.1 * t, 0.0));
            let direction = side.mul(if leaf % 2 == 0 { 1.0 } else { -1.0 });
            let length = 1.0 + ((seed + leaf as f32).sin() * 0.5).abs();
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
        // Keep the arch pillars outside both the rumble strip and the
        // continuous barrier line; only the crossbar passes over the track.
        let outer = HALF_ROAD + CURB_WIDTH + 4.5;
        for direction in [-1.0, 1.0] {
            let pillar_center = sample.position.add(side.mul(outer * direction));
            self.box_aligned(
                pillar_center,
                sample.tangent,
                Vec3::new(2.8, 6.7, 2.6),
                [107, 107, 83],
                "mossy_arch_stone",
            );
            self.box_aligned(
                pillar_center.sub(Vec3::new(0.0, 3.0, 0.0)),
                sample.tangent,
                Vec3::new(3.35, 0.55, 3.1),
                [91, 91, 73],
                "arch_carved_bases",
            );
            self.box_aligned(
                pillar_center.add(Vec3::new(0.0, 3.2, 0.0)),
                sample.tangent,
                Vec3::new(3.25, 0.42, 2.95),
                [139, 130, 98],
                "arch_carved_capitals",
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
        for segment in 0..15 {
            let amount = segment as f32 / 14.0;
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
        let start = self.vertices.len();
        for stack in 0..=stacks {
            let v = stack as f32 / stacks as f32;
            let latitude = (v - 0.5) * std::f32::consts::PI;
            for sector in 0..sectors {
                let longitude = sector as f32 * std::f32::consts::TAU / sectors as f32;
                self.vertices.push(center.add(Vec3::new(
                    radii.x * latitude.cos() * longitude.cos(),
                    radii.y * latitude.sin(),
                    radii.z * latitude.cos() * longitude.sin(),
                )));
            }
        }
        for stack in 0..stacks {
            for sector in 0..sectors {
                let next = (sector + 1) % sectors;
                let a = start + stack * sectors + sector;
                let b = start + stack * sectors + next;
                let c = start + (stack + 1) * sectors + next;
                let d = start + (stack + 1) * sectors + sector;
                for indices in [[a, b, c], [a, c, d]] {
                    let mut face = Face::new(indices, "procedural", "smooth_canopies");
                    face.color = MaterialColor::Fixed(color);
                    face.shader = SurfaceShader::RaceGrassTexture;
                    face.smooth = true;
                    self.faces.push(face);
                }
            }
        }
    }

    fn hills(&mut self, course: &RaceCourse) {
        // Build layered ridges beside the road, deriving every placement
        // from the actual course and checking the full road loop for nearby
        // sections. This prevents a later hairpin from running through a hill.
        for (index, sample) in course.samples.iter().step_by(10).enumerate() {
            let side = Vec3::new(-sample.tangent.z, 0.0, sample.tangent.x);
            for direction in [-1.0, 1.0] {
                let Some(center) = safe_offset_from_course(
                    course,
                    sample.position,
                    side,
                    direction,
                    86.0,
                    68.0,
                ) else {
                    continue;
                };
                let radius = 34.0 + (index % 4) as f32 * 4.0;
                let height = 14.0 + (index % 5) as f32 * 2.5;
                let segments = 16;
                let peak = center.add(Vec3::new(
                    (index as f32 * 1.7).sin() * 4.0,
                    height,
                    (index as f32 * 1.3).cos() * 4.0,
                ));
                for segment in 0..segments {
                    let a0 = segment as f32 * std::f32::consts::TAU / segments as f32;
                    let a1 = (segment + 1) as f32 * std::f32::consts::TAU / segments as f32;
                    let base0 = center.add(Vec3::new(a0.cos() * radius, 0.0, a0.sin() * radius));
                    let base1 = center.add(Vec3::new(a1.cos() * radius, 0.0, a1.sin() * radius));
                    let mid0 = center.add(Vec3::new(
                        a0.cos() * radius * 0.52,
                        height * (0.48 + (segment % 3) as f32 * 0.045),
                        a0.sin() * radius * 0.52,
                    ));
                    let mid1 = center.add(Vec3::new(
                        a1.cos() * radius * 0.52,
                        height * (0.48 + ((segment + 1) % 3) as f32 * 0.045),
                        a1.sin() * radius * 0.52,
                    ));
                    self.quad(
                        base0,
                        base1,
                        mid1,
                        mid0,
                        if segment % 2 == 0 {
                            [58, 111, 76]
                        } else {
                            [70, 124, 80]
                        },
                        "misty_green_hills",
                    );
                    for face in self.faces.iter_mut().rev().take(2) {
                        face.shader = SurfaceShader::RaceGroundTexture;
                    }
                    self.tri(
                        mid0,
                        mid1,
                        peak,
                        if segment % 2 == 0 {
                            [79, 132, 83]
                        } else {
                            [96, 143, 91]
                        },
                        "detailed_hill_peaks",
                    );
                    if let Some(face) = self.faces.last_mut() {
                        face.shader = SurfaceShader::RaceGroundTexture;
                    }
                }
            }
        }
    }

    fn terrain(&mut self) {
        // Extend well beyond the playable route so the chase camera never
        // exposes the sky background through the edges of the ground plane.
        // Tile count stays moderate; the extra coverage does not add objects.
        let width = 1_100.0;
        let depth = 900.0;
        let origin_x = -350.0;
        let origin_z = -300.0;
        let columns = 44;
        let rows = 36;
        for row in 0..rows {
            for column in 0..columns {
                let x0 = column as f32 * width / columns as f32 + origin_x;
                let x1 = (column + 1) as f32 * width / columns as f32 + origin_x;
                let z0 = row as f32 * depth / rows as f32 + origin_z;
                let z1 = (row + 1) as f32 * depth / rows as f32 + origin_z;
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
                for face in self.faces.iter_mut().rev().take(2) {
                    face.shader = SurfaceShader::RaceGrassTexture;
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forest_and_hill_geometry_stays_clear_of_the_drivable_surface() {
        let course = RaceCourse::new();
        let scenery = RaceScenery::new(&course);
        assert_eq!(scenery.forests.len(), 18);
        assert_eq!(scenery.forest_lods.len(), scenery.forests.len());
        assert_eq!(scenery.arches.len(), 18);
        assert!(scenery.terrain.len() > 1);
        assert!(scenery.hills.len() > 1);
        assert_eq!(
            scenery.terrain.iter().map(|model| model.faces.len()).sum::<usize>(),
            44 * 36 * 2 + 30 * 2
        );
        assert!(scenery
            .terrain
            .iter()
            .chain(&scenery.hills)
            .all(|model| !model.faces.is_empty() && !model.vertices.is_empty()));
        assert!(scenery
            .terrain
            .iter()
            .flat_map(|model| &model.faces)
            .filter(|face| face.group == "lush_track_ground")
            .all(|face| face.shader == SurfaceShader::RaceGrassTexture));
        for (detailed, lod) in scenery.forests.iter().zip(&scenery.forest_lods) {
            assert!(lod.faces.len() <= detailed.faces.len().div_ceil(3));
            for group in ["alien_tree_trunks", "alien_tree_roots", "smooth_canopies"] {
                assert!(lod.faces.iter().any(|face| face.group == group));
            }
            assert!(lod.faces.iter().all(|face| {
                face.group != "smooth_canopies"
                    || face.shader == SurfaceShader::RaceGrassTexture
            }));
        }
        for model in scenery.forests.iter().chain(&scenery.hills) {
            for vertex in &model.vertices {
                let (_, distance, _) = course.nearest(*vertex);
                assert!(
                    distance > HALF_ROAD + 0.8,
                    "scenery intrudes into road clearance at {vertex:?} (distance {distance})"
                );
            }
        }
        let terrain_min_x = scenery
            .terrain
            .iter()
            .flat_map(|model| &model.vertices)
            .map(|vertex| vertex.x)
            .fold(f32::INFINITY, f32::min);
        let terrain_max_x = scenery
            .terrain
            .iter()
            .flat_map(|model| &model.vertices)
            .map(|vertex| vertex.x)
            .fold(f32::NEG_INFINITY, f32::max);
        let terrain_min_z = scenery
            .terrain
            .iter()
            .flat_map(|model| &model.vertices)
            .map(|vertex| vertex.z)
            .fold(f32::INFINITY, f32::min);
        let terrain_max_z = scenery
            .terrain
            .iter()
            .flat_map(|model| &model.vertices)
            .map(|vertex| vertex.z)
            .fold(f32::NEG_INFINITY, f32::max);
        for sample in &course.samples {
            assert!(sample.position.x - terrain_min_x > 240.0);
            assert!(terrain_max_x - sample.position.x > 240.0);
            assert!(sample.position.z - terrain_min_z > 240.0);
            assert!(terrain_max_z - sample.position.z > 240.0);
        }
        assert!(scenery
            .forests
            .iter()
            .any(|model| model.faces.iter().any(|face| face.group == "alien_tree_roots")));
        for group in ["alien_tree_trunks", "smooth_canopies"] {
            assert!(scenery.forests.iter().any(|model| model
                .faces
                .iter()
                .any(|face| face.group == group && face.smooth)));
        }
        assert!(scenery.forests.iter().all(|model| model
            .faces
            .iter()
            .filter(|face| matches!(face.group.as_str(), "alien_tree_trunks" | "alien_tree_roots"))
            .all(|face| face.shader == SurfaceShader::RaceBarkTexture)));
        assert!(scenery
            .arches
            .iter()
            .any(|model| model.faces.iter().any(|face| face.group == "arch_carved_capitals")));
        assert!(scenery.arches.iter().all(|model| model
            .faces
            .iter()
            .filter(|face| matches!(
                face.group.as_str(),
                "mossy_arch_stone" | "arch_carved_bases" | "arch_carved_capitals"
            ))
            .all(|face| face.shader == SurfaceShader::RaceBricksTexture)));
        assert!(scenery
            .hills
            .iter()
            .flat_map(|model| &model.faces)
            .any(|face| face.group == "detailed_hill_peaks"));
        assert!(scenery.hills.iter().flat_map(|model| &model.faces).all(|face| {
            !matches!(face.group.as_str(), "misty_green_hills" | "detailed_hill_peaks")
                || face.shader == SurfaceShader::RaceGroundTexture
        }));
        assert!(scenery.forests.iter().all(|model| model
            .faces
            .iter()
            .filter(|face| face.group == "smooth_canopies")
            .all(|face| face.shader == SurfaceShader::RaceGrassTexture)));
    }
}
