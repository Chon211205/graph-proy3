use crate::obj_loader::{Face, MaterialColor, Model, SurfaceShader, Vec3};

// Wider surface for the 13-car grid while preserving the authored route.
pub(super) const HALF_ROAD: f32 = 12.5;
pub(super) const CURB_WIDTH: f32 = 1.8;
// Fourteen samples preserve the authored final curve while cutting redundant
// nearest-point searches and static geometry for all thirteen racers.
const SAMPLES_PER_SPAN: usize = 14;

const CONTROL_POINTS: [Vec3; 25] = [
    Vec3::new(72.0, 0.0, 207.0),
    Vec3::new(120.0, 0.0, 207.0),
    Vec3::new(182.0, 0.0, 207.0),
    Vec3::new(246.0, 0.0, 207.0),
    Vec3::new(292.0, 0.0, 200.0),
    Vec3::new(321.0, 0.0, 179.0),
    Vec3::new(329.0, 0.0, 151.0),
    Vec3::new(318.0, 0.0, 129.0),
    Vec3::new(290.0, 0.0, 121.0),
    Vec3::new(267.0, 0.0, 137.0),
    Vec3::new(241.0, 0.0, 139.0),
    Vec3::new(223.0, 0.0, 120.0),
    Vec3::new(224.0, 0.0, 94.0),
    Vec3::new(207.0, 0.0, 69.0),
    Vec3::new(179.0, 0.0, 57.0),
    Vec3::new(132.0, 0.0, 55.0),
    Vec3::new(92.0, 0.0, 62.0),
    Vec3::new(70.0, 0.0, 81.0),
    Vec3::new(73.0, 0.0, 105.0),
    Vec3::new(97.0, 0.0, 125.0),
    Vec3::new(105.0, 0.0, 148.0),
    // Broad final bend: enough radius for the widened track and a tangent
    // that merges cleanly into the start/finish straight.
    Vec3::new(55.0, 0.0, 172.0),
    Vec3::new(38.0, 0.0, 188.0),
    Vec3::new(38.0, 0.0, 201.0),
    Vec3::new(55.0, 0.0, 207.0),
];

#[derive(Clone, Copy, Debug)]
pub(super) struct TrackSample {
    pub position: Vec3,
    pub tangent: Vec3,
}

pub(super) struct RaceCourse {
    pub samples: Vec<TrackSample>,
    pub cumulative: Vec<f32>,
    pub length: f32,
    pub road: Vec<Model>,
    pub curbs: Vec<Model>,
    pub barriers: Vec<Model>,
    pub markings: Vec<Model>,
    pub boost_pads: Vec<Vec3>,
}

impl RaceCourse {
    pub fn new() -> Self {
        let mut samples = Vec::with_capacity(CONTROL_POINTS.len() * SAMPLES_PER_SPAN);
        for index in 0..CONTROL_POINTS.len() {
            let p0 = CONTROL_POINTS[(index + CONTROL_POINTS.len() - 1) % CONTROL_POINTS.len()];
            let p1 = CONTROL_POINTS[index];
            let p2 = CONTROL_POINTS[(index + 1) % CONTROL_POINTS.len()];
            let p3 = CONTROL_POINTS[(index + 2) % CONTROL_POINTS.len()];
            for step in 0..SAMPLES_PER_SPAN {
                let t = step as f32 / SAMPLES_PER_SPAN as f32;
                let position = expand_course(catmull_rom(p0, p1, p2, p3, t));
                let ahead = expand_course(catmull_rom(p0, p1, p2, p3, (t + 0.01).min(1.0)));
                let tangent = ahead.sub(position).normalize();
                samples.push(TrackSample { position, tangent });
            }
        }

        let mut cumulative = vec![0.0; samples.len() + 1];
        for index in 0..samples.len() {
            cumulative[index + 1] = cumulative[index]
                + samples[(index + 1) % samples.len()]
                    .position
                    .sub(samples[index].position)
                    .dot(
                        samples[(index + 1) % samples.len()]
                            .position
                            .sub(samples[index].position),
                    )
                    .sqrt();
        }
        let length = *cumulative.last().unwrap_or(&0.0);

        let mut road = MeshBuilder::default();
        let mut curbs = MeshBuilder::default();
        let mut barriers = MeshBuilder::default();
        let mut markings = MeshBuilder::default();
        let mut boost_pads = Vec::new();

        for index in 0..samples.len() {
            let current = samples[index];
            let next = samples[(index + 1) % samples.len()];
            let side = Vec3::new(-current.tangent.z, 0.0, current.tangent.x);
            let next_side = Vec3::new(-next.tangent.z, 0.0, next.tangent.x);
            let stripe_color = if index.is_multiple_of(2) {
                [220, 48, 54]
            } else {
                [246, 240, 211]
            };

            road.quad(
                current.position.add(side.mul(-HALF_ROAD)),
                next.position.add(next_side.mul(-HALF_ROAD)),
                next.position.add(next_side.mul(HALF_ROAD)),
                current.position.add(side.mul(HALF_ROAD)),
                if index % 3 == 0 {
                    [43, 47, 55]
                } else {
                    [49, 52, 59]
                },
                "asphalt",
            );
            for direction in [-1.0, 1.0] {
                let inner = HALF_ROAD * direction;
                let outer = (HALF_ROAD + CURB_WIDTH) * direction;
                curbs.quad(
                    current.position.add(side.mul(inner)),
                    next.position.add(next_side.mul(inner)),
                    next.position.add(next_side.mul(outer)),
                    current.position.add(side.mul(outer)),
                    stripe_color,
                    "red_white_curbs",
                );
                if index.is_multiple_of(2) {
                    let barrier_next = samples[(index + 2) % samples.len()];
                    let delta = barrier_next.position.sub(current.position);
                    let tangent = delta.normalize();
                    let barrier_side = Vec3::new(-tangent.z, 0.0, tangent.x);
                    barriers.box_along(
                        current
                            .position
                            .add(barrier_next.position)
                            .mul(0.5)
                            .add(barrier_side.mul(
                                (HALF_ROAD + CURB_WIDTH + 1.0) * direction,
                            )),
                        tangent,
                        delta.dot(delta).sqrt() * 0.5 + 0.08,
                        0.8,
                        0.65,
                        if (index / 6).is_multiple_of(2) {
                            [103, 115, 121]
                        } else {
                            [79, 94, 102]
                        },
                        "track_barriers",
                    );
                }
            }
            let boost_cycle = SAMPLES_PER_SPAN * 4;
            if index % boost_cycle == SAMPLES_PER_SPAN
                || index % boost_cycle == SAMPLES_PER_SPAN * 3
            {
                boost_pads.push(current.position.add(next.position).mul(0.5));
                for lane in [-1.0, 0.0, 1.0] {
                    let center = current
                        .position
                        .add(next.position)
                        .mul(0.5)
                        .add(side.mul(lane * 5.0));
                    markings.quad(
                        center
                            .sub(current.tangent.mul(2.5))
                            .add(Vec3::new(0.0, 0.12, 0.0))
                            .sub(side.mul(1.2)),
                        center
                            .add(current.tangent.mul(2.5))
                            .add(Vec3::new(0.0, 0.12, 0.0))
                            .sub(side.mul(1.2)),
                        center
                            .add(current.tangent.mul(2.5))
                            .add(Vec3::new(0.0, 0.12, 0.0))
                            .add(side.mul(1.2)),
                        center
                            .sub(current.tangent.mul(2.5))
                            .add(Vec3::new(0.0, 0.12, 0.0))
                            .add(side.mul(1.2)),
                        [255, 186, 62],
                        "turbo_bands",
                    );
                }
            }
        }

        for lane in -4..=4 {
            let offset = lane as f32 * 2.6;
            let station = samples[0].position;
            let side = Vec3::new(-samples[0].tangent.z, 0.0, samples[0].tangent.x);
            markings.quad(
                station
                    .add(side.mul(offset - 1.1))
                    .add(Vec3::new(0.0, 0.14, 0.0)),
                station
                    .add(side.mul(offset + 1.1))
                    .add(Vec3::new(0.0, 0.14, 0.0)),
                station
                    .add(samples[0].tangent.mul(2.5))
                    .add(side.mul(offset + 1.1))
                    .add(Vec3::new(0.0, 0.14, 0.0)),
                station
                    .add(samples[0].tangent.mul(2.5))
                    .add(side.mul(offset - 1.1))
                    .add(Vec3::new(0.0, 0.14, 0.0)),
                if lane % 2 == 0 {
                    [234, 239, 224]
                } else {
                    [36, 42, 48]
                },
                "start_finish",
            );
        }

        Self {
            samples,
            cumulative,
            length,
            road: split_model(road.finish(), 18),
            curbs: split_model(curbs.finish(), 18),
            barriers: split_model(barriers.finish(), 18),
            markings: split_model(markings.finish(), 18),
            boost_pads,
        }
    }

    pub fn sample_at(&self, distance: f32) -> TrackSample {
        let distance = distance.rem_euclid(self.length.max(1.0));
        let index = self
            .cumulative
            .partition_point(|&value| value <= distance)
            .saturating_sub(1)
            .min(self.samples.len().saturating_sub(1));
        let segment_length = (self.cumulative[index + 1] - self.cumulative[index]).max(0.001);
        let amount = (distance - self.cumulative[index]) / segment_length;
        let a = self.samples[index];
        let b = self.samples[(index + 1) % self.samples.len()];
        TrackSample {
            position: a.position.add(b.position.sub(a.position).mul(amount)),
            tangent: a
                .tangent
                .add(b.tangent.sub(a.tangent).mul(amount))
                .normalize(),
        }
    }

    pub fn nearest(&self, position: Vec3) -> (f32, f32, TrackSample) {
        let mut best_distance_sq = f32::INFINITY;
        let mut best_distance = 0.0;
        let mut best_sample = self.samples[0];
        for index in 0..self.samples.len() {
            let a = self.samples[index].position;
            let b = self.samples[(index + 1) % self.samples.len()].position;
            let delta = b.sub(a);
            let length_sq = delta.dot(delta).max(0.001);
            let offset = Vec3::new(position.x - a.x, 0.0, position.z - a.z);
            let t = (offset.dot(delta) / length_sq).clamp(0.0, 1.0);
            let point = a.add(delta.mul(t));
            let diff = Vec3::new(position.x - point.x, 0.0, position.z - point.z);
            let distance_sq = diff.dot(diff);
            if distance_sq < best_distance_sq {
                best_distance_sq = distance_sq;
                best_distance = self.cumulative[index] + length_sq.sqrt() * t;
                best_sample = TrackSample {
                    position: point,
                    tangent: self.samples[index].tangent,
                };
            }
        }
        (best_distance, best_distance_sq.sqrt(), best_sample)
    }
}

fn split_model(model: Model, chunk_count: usize) -> Vec<Model> {
    let faces_per_chunk = model.faces.len().div_ceil(chunk_count.max(1));
    model
        .faces
        .chunks(faces_per_chunk.max(1))
        .map(|source_faces| {
            let mut remap = vec![usize::MAX; model.vertices.len()];
            let mut vertices = Vec::new();
            let mut faces = Vec::with_capacity(source_faces.len());
            for source in source_faces {
                let indices = source.indices.map(|old| {
                    if remap[old] == usize::MAX {
                        remap[old] = vertices.len();
                        vertices.push(model.vertices[old]);
                    }
                    remap[old]
                });
                let mut face = Face::new(indices, source.material.clone(), source.group.clone());
                face.shader = source.shader;
                face.color = source.color;
                face.smooth = source.smooth;
                faces.push(face);
            }
            Model::new(vertices, faces)
        })
        .collect()
}

#[derive(Default)]
struct MeshBuilder {
    vertices: Vec<Vec3>,
    faces: Vec<Face>,
}

impl MeshBuilder {
    fn triangle(&mut self, a: Vec3, b: Vec3, c: Vec3, color: [u8; 3], group: &str) {
        let base = self.vertices.len();
        self.vertices.extend([a, b, c]);
        let mut face = Face::new([base, base + 1, base + 2], "procedural", group);
        face.color = MaterialColor::Fixed(color);
        face.shader = SurfaceShader::Flat;
        self.faces.push(face);
    }

    fn quad(&mut self, a: Vec3, b: Vec3, c: Vec3, d: Vec3, color: [u8; 3], group: &str) {
        self.triangle(a, b, c, color, group);
        self.triangle(a, c, d, color, group);
    }

    fn box_along(
        &mut self,
        center: Vec3,
        tangent: Vec3,
        half_length: f32,
        width: f32,
        height: f32,
        color: [u8; 3],
        group: &str,
    ) {
        let side = Vec3::new(-tangent.z, 0.0, tangent.x);
        let a = center.sub(tangent.mul(half_length)).sub(side.mul(width));
        let b = center.add(tangent.mul(half_length)).sub(side.mul(width));
        let c = center.add(tangent.mul(half_length)).add(side.mul(width));
        let d = center.sub(tangent.mul(half_length)).add(side.mul(width));
        let up = Vec3::new(0.0, height, 0.0);
        self.quad(a, b, c, d, color, group);
        self.quad(a, a.add(up), b.add(up), b, color, group);
        self.quad(b, b.add(up), c.add(up), c, color, group);
        self.quad(c, c.add(up), d.add(up), d, color, group);
        self.quad(d, d.add(up), a.add(up), a, color, group);
        self.quad(a.add(up), d.add(up), c.add(up), b.add(up), color, group);
    }

    fn finish(self) -> Model {
        Model::new(self.vertices, self.faces)
    }
}

fn catmull_rom(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let t2 = t * t;
    let t3 = t2 * t;
    Vec3::new(
        0.5 * ((2.0 * p1.x)
            + (-p0.x + p2.x) * t
            + (2.0 * p0.x - 5.0 * p1.x + 4.0 * p2.x - p3.x) * t2
            + (-p0.x + 3.0 * p1.x - 3.0 * p2.x + p3.x) * t3),
        0.0,
        0.5 * ((2.0 * p1.z)
            + (-p0.z + p2.z) * t
            + (2.0 * p0.z - 5.0 * p1.z + 4.0 * p2.z - p3.z) * t2
            + (-p0.z + 3.0 * p1.z - 3.0 * p2.z + p3.z) * t3),
    )
}

fn expand_course(point: Vec3) -> Vec3 {
    const CENTER_X: f32 = 200.0;
    const CENTER_Z: f32 = 130.0;
    const SCALE: f32 = 1.8;
    Vec3::new(
        CENTER_X + (point.x - CENTER_X) * SCALE,
        point.y,
        CENTER_Z + (point.z - CENTER_Z) * SCALE,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_course_is_closed_drivable_and_has_boost_lanes() {
        let course = RaceCourse::new();
        assert!(course.samples.len() >= 160);
        assert!(
            course.length > 1_200.0,
            "course length was {}",
            course.length
        );
        assert!(course.boost_pads.len() >= 8);
        let start = course.samples[0];
        let end = course.sample_at(course.length);
        let closure_error = start.position.sub(end.position);
        assert!(closure_error.dot(closure_error) < 1.0);
        assert!(
            start.tangent.x > 0.8,
            "start must point down the start straight"
        );
        let (distance, lateral_error, _) = course.nearest(start.position);
        assert!(distance < 1.0);
        assert!(lateral_error < 0.1);
        assert_eq!(course.road.len(), 18);
        assert_eq!(course.curbs.len(), 18);
        assert_eq!(course.barriers.len(), 18);
        assert!(!course.road.iter().any(|chunk| chunk.faces.is_empty()));
        assert!(!course.curbs.iter().any(|chunk| chunk.faces.is_empty()));
        assert_eq!(
            course.barriers.iter().map(|chunk| chunk.faces.len()).sum::<usize>(),
            course.samples.len() * 12
        );
    }

    #[test]
    fn final_turn_merges_smoothly_into_the_start_straight() {
        let course = RaceCourse::new();
        let final_turn_start = 20 * SAMPLES_PER_SPAN;
        let mut minimum_alignment = (1.0_f32, final_turn_start);
        for index in final_turn_start..course.samples.len() {
            let next = (index + 1) % course.samples.len();
            let alignment = course.samples[index]
                .tangent
                .dot(course.samples[next].tangent);
            if alignment < minimum_alignment.0 {
                minimum_alignment = (alignment, index);
            }
        }
        assert!(
            minimum_alignment.0 > 0.985,
            "final turn contained a sharp tangent break: {minimum_alignment:?}"
        );
    }
}
