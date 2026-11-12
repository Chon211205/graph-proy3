use crate::obj_loader::{Face, MaterialColor, Model, SurfaceShader, Vec3};
use std::f32::consts::TAU;

const CORE_RADIUS: f32 = 22.0;
const DISK_OUTER_RADIUS: f32 = 78.0;
const DISK_SEGMENTS: usize = 192;

/// Builds the Vesper black hole from a pitch-black event horizon, a hot inner
/// photon ring and a turbulent, warm accretion disk. Geometry stays immutable;
/// the caller rotates the complete model at runtime.
pub(crate) fn build_model() -> Model {
    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    append_sphere(&mut vertices, &mut faces, CORE_RADIUS, 24, 48);
    append_accretion_disk(&mut vertices, &mut faces);
    append_plasma_streams(&mut vertices, &mut faces);
    Model::new(vertices, faces)
}

fn append_sphere(
    vertices: &mut Vec<Vec3>,
    faces: &mut Vec<Face>,
    radius: f32,
    stacks: usize,
    slices: usize,
) {
    let start = vertices.len();
    for stack in 0..=stacks {
        let latitude = std::f32::consts::PI * stack as f32 / stacks as f32
            - std::f32::consts::FRAC_PI_2;
        let ring = latitude.cos() * radius;
        let y = latitude.sin() * radius;
        for slice in 0..=slices {
            let longitude = TAU * slice as f32 / slices as f32;
            vertices.push(Vec3::new(
                ring * longitude.cos(),
                y,
                ring * longitude.sin(),
            ));
        }
    }
    for stack in 0..stacks {
        for slice in 0..slices {
            let a = start + stack * (slices + 1) + slice;
            let b = a + slices + 1;
            let mut face = Face::new(
                [a, b, a + 1],
                "vesper_black_hole_core",
                "vesper_black_hole_core",
            );
            face.shader = SurfaceShader::Flat;
            face.color = MaterialColor::Fixed([0, 0, 1]);
            faces.push(face);
            let mut face = Face::new(
                [a + 1, b, b + 1],
                "vesper_black_hole_core",
                "vesper_black_hole_core",
            );
            face.shader = SurfaceShader::Flat;
            face.color = MaterialColor::Fixed([0, 0, 1]);
            faces.push(face);
        }
    }
}

fn append_accretion_disk(vertices: &mut Vec<Vec3>, faces: &mut Vec<Face>) {
    // Narrow thermal layers retain the recognizable disk silhouette without
    // the broad purple spokes that made the previous ring look like a platform.
    let bands = [
        (22.2, 24.2, [255, 245, 207]),
        (24.2, 27.0, [255, 213, 132]),
        (27.0, 31.0, [255, 164, 76]),
        (31.0, 39.0, [247, 103, 43]),
        (39.0, 50.0, [201, 55, 30]),
        (50.0, 63.0, [132, 31, 28]),
        (63.0, DISK_OUTER_RADIUS, [65, 17, 24]),
    ];

    for (band_index, (inner, outer, base_color)) in bands.into_iter().enumerate() {
        let start = vertices.len();
        for segment in 0..DISK_SEGMENTS {
            let angle = TAU * segment as f32 / DISK_SEGMENTS as f32;
            let turbulence = (angle * 5.0 + band_index as f32 * 1.9).sin() * 0.23
                + (angle * 11.0 - band_index as f32 * 0.7).cos() * 0.11;
            let warp = (angle * 2.0 + band_index as f32 * 0.8).sin()
                * (1.0 - inner / DISK_OUTER_RADIUS)
                * 0.8;
            for (radius, side) in [(inner + turbulence, 0.0), (outer + turbulence, 1.0)] {
                let flare = ((angle * 3.0 + band_index as f32).sin().max(0.0)) * 0.32;
                let y = warp + side * 0.15 + flare * (1.0 - band_index as f32 * 0.09);
                vertices.push(Vec3::new(angle.cos() * radius, y, angle.sin() * radius));
            }
        }

        for segment in 0..DISK_SEGMENTS {
            let next = (segment + 1) % DISK_SEGMENTS;
            let a = start + segment * 2;
            let b = start + next * 2;
            let color = turbulent_color(base_color, segment, band_index);
            push_flat_face(faces, [a, b, a + 1], color, "accretion_disk");
            push_flat_face(faces, [a + 1, b, b + 1], color, "accretion_disk");
        }
    }
}

fn append_plasma_streams(vertices: &mut Vec<Vec3>, faces: &mut Vec<Face>) {
    // Fine curved streaks travel with the disk. They add motion and texture
    // when the model rotates, while keeping the black center unobstructed.
    const STREAMS: usize = 10;
    const STEPS: usize = 42;
    let palette = [
        [255, 200, 116],
        [255, 137, 62],
        [213, 66, 34],
        [255, 224, 171],
        [177, 41, 31],
    ];

    for stream in 0..STREAMS {
        let start = vertices.len();
        let phase = TAU * stream as f32 / STREAMS as f32;
        for step in 0..=STEPS {
            let t = step as f32 / STEPS as f32;
            let angle = phase + t * TAU * (1.05 + (stream % 3) as f32 * 0.14);
            let center_radius = 28.0 + t * (DISK_OUTER_RADIUS - 30.0);
            let width = (0.9 * (1.0 - t) + 0.16).max(0.16);
            let y = 0.65 + (angle * 2.4 + phase).sin() * (1.0 + t * 2.2);
            for offset in [-width, width] {
                let radius = center_radius + offset;
                vertices.push(Vec3::new(angle.cos() * radius, y, angle.sin() * radius));
            }
        }

        for step in 0..STEPS {
            let a = start + step * 2;
            let b = a + 2;
            let color = palette[(stream + step / 8) % palette.len()];
            push_flat_face(faces, [a, b, a + 1], color, "accretion_plasma_stream");
            push_flat_face(faces, [a + 1, b, b + 1], color, "accretion_plasma_stream");
        }
    }
}

fn turbulent_color(base: [u8; 3], segment: usize, band: usize) -> [u8; 3] {
    let angle = TAU * segment as f32 / DISK_SEGMENTS as f32;
    let variation = 0.70
        + (angle * 7.0 + band as f32 * 1.4).sin().max(0.0) * 0.24
        + (angle * 17.0 - band as f32 * 0.9).cos().abs() * 0.10;
    base.map(|channel| (channel as f32 * variation).clamp(0.0, 255.0) as u8)
}

fn push_flat_face(faces: &mut Vec<Face>, indices: [usize; 3], color: [u8; 3], group: &str) {
    let mut face = Face::new(indices, "vesper_accretion_plasma", group);
    face.shader = SurfaceShader::Flat;
    face.color = MaterialColor::Fixed(color);
    faces.push(face);
}
