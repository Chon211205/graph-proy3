use crate::obj_loader::{Face, MaterialColor, Model, SurfaceShader, Vec3};
use std::f32::consts::TAU;

const CORE_RADIUS: f32 = 22.0;
const DISK_OUTER_RADIUS: f32 = 78.0;
const DISK_SEGMENTS: usize = 256;

/// Builds the Vesper black hole from a pitch-black event horizon, a hot inner
/// photon ring and a turbulent, warm accretion disk. Geometry stays immutable;
/// the caller rotates the complete model at runtime.
pub(crate) fn build_model() -> Model {
    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    append_sphere(&mut vertices, &mut faces, CORE_RADIUS, 24, 48);
    append_accretion_disk(&mut vertices, &mut faces);
    append_lensed_photon_ring(&mut vertices, &mut faces);
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
            face.color = MaterialColor::Fixed([0, 0, 0]);
            faces.push(face);
            let mut face = Face::new(
                [a + 1, b, b + 1],
                "vesper_black_hole_core",
                "vesper_black_hole_core",
            );
            face.shader = SurfaceShader::Flat;
            face.color = MaterialColor::Fixed([0, 0, 0]);
            faces.push(face);
        }
    }
}

fn append_accretion_disk(vertices: &mut Vec<Vec3>, faces: &mut Vec<Face>) {
    // Broad, turbulent thermal zones keep the disk from reading as a stack of
    // perfect rings. The innermost zone is a narrow, almost white photon rim.
    let bands = [
        (22.15, 24.0, [255, 244, 197]),
        (24.0, 29.5, [255, 190, 91]),
        (29.5, 41.0, [255, 112, 44]),
        (41.0, 57.0, [191, 50, 31]),
        (57.0, DISK_OUTER_RADIUS, [82, 20, 32]),
    ];

    for (band_index, (inner, outer, base_color)) in bands.into_iter().enumerate() {
        let start = vertices.len();
        for segment in 0..DISK_SEGMENTS {
            let angle = TAU * segment as f32 / DISK_SEGMENTS as f32;
            let turbulence = (angle * 4.7 + band_index as f32 * 1.9).sin() * 0.48
                + (angle * 13.0 - band_index as f32 * 0.7).cos() * 0.26
                + (angle * 27.0 + band_index as f32 * 2.1).sin() * 0.10;
            let radial_warp = 1.0 - inner / DISK_OUTER_RADIUS;
            // Tilt and twist the disk so its far side appears to rise around
            // the event horizon instead of forming a flat plate.
            let warp = (angle * 1.7 + band_index as f32 * 0.55).sin()
                * radial_warp
                * 2.8
                + (angle * 4.0 - band_index as f32).cos() * radial_warp * 0.7;
            for (radius, side) in [(inner + turbulence, 0.0), (outer + turbulence, 1.0)] {
                let flare = ((angle * 3.0 + band_index as f32).sin().max(0.0)) * 0.52;
                let y = warp + side * 0.22 + flare * (1.0 - band_index as f32 * 0.12);
                vertices.push(Vec3::new(angle.cos() * radius, y, angle.sin() * radius));
            }
        }

        for segment in 0..DISK_SEGMENTS {
            let next = (segment + 1) % DISK_SEGMENTS;
            let a = start + segment * 2;
            let b = start + next * 2;
            // Broken outer filaments and a bright approaching side add motion
            // and asymmetry without adding per-frame simulation cost.
            let breakup = (angle_hash(segment, band_index) * 12.0).sin();
            if band_index >= 3 && breakup > 0.82 {
                continue;
            }
            let color = turbulent_color(base_color, segment, band_index);
            push_flat_face(faces, [a, b, a + 1], color, "accretion_disk");
            push_flat_face(faces, [a + 1, b, b + 1], color, "accretion_disk");
        }
    }
}

fn append_lensed_photon_ring(vertices: &mut Vec<Vec3>, faces: &mut Vec<Face>) {
    // A thin, hot crescent sits just outside the horizon. The vertical wave
    // makes it peek above the far-side edge like a gravitationally lensed rim.
    const SEGMENTS: usize = 256;
    let start = vertices.len();
    for segment in 0..SEGMENTS {
        let angle = TAU * segment as f32 / SEGMENTS as f32;
        let (sin, cos) = angle.sin_cos();
        let radius = 23.25 + (angle * 5.0).sin() * 0.10;
        let lift = (angle.sin().max(0.0)).powf(1.7) * 3.1;
        for r in [radius - 0.42, radius + 0.42] {
            vertices.push(Vec3::new(cos * r, lift + (angle * 3.0).sin() * 0.22, sin * r));
        }
    }
    for segment in 0..SEGMENTS {
        let next = (segment + 1) % SEGMENTS;
        let a = start + segment * 2;
        let b = start + next * 2;
        let c = if angle_hash(segment, 0).sin() > -0.25 {
            [255, 237, 174]
        } else {
            [255, 145, 63]
        };
        push_flat_face(faces, [a, b, a + 1], c, "photon_ring");
        push_flat_face(faces, [a + 1, b, b + 1], c, "photon_ring");
    }
}

fn append_plasma_streams(vertices: &mut Vec<Vec3>, faces: &mut Vec<Face>) {
    // Short, tapered spiral filaments break up the continuous annulus and
    // suggest hot matter being dragged around the horizon.
    const STREAMS: usize = 16;
    const STEPS: usize = 36;
    let palette = [
        [255, 218, 151],
        [255, 155, 76],
        [239, 81, 43],
        [255, 235, 196],
        [151, 40, 31],
    ];

    for stream in 0..STREAMS {
        let start = vertices.len();
        let phase = TAU * stream as f32 / STREAMS as f32;
        let arc = 2.6 + (stream % 4) as f32 * 0.48;
        for step in 0..=STEPS {
            let t = step as f32 / STEPS as f32;
            let angle = phase + t * arc;
            let center_radius = 25.0 + t * (DISK_OUTER_RADIUS - 29.0)
                + (t * TAU * 2.0 + phase).sin() * 1.25;
            let width = (0.95 * (1.0 - t).powf(0.7) + 0.09).max(0.09);
            let y = 0.5
                + (angle * 2.4 + phase).sin() * (0.9 + t * 2.5)
                + (angle * 5.1).cos() * 0.35;
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
    let turbulence = (angle * 8.0 + band as f32 * 1.3).sin() * 0.13
        + (angle * 19.0 - band as f32 * 0.9).cos() * 0.09
        + (angle * 37.0 + band as f32 * 2.7).sin() * 0.055;
    let doppler = 0.68 + 0.50 * ((angle - 0.85).cos() * 0.5 + 0.5);
    let variation = (0.91 + turbulence).clamp(0.58, 1.15) * doppler;
    base.map(|channel| (channel as f32 * variation).clamp(0.0, 255.0) as u8)
}

fn angle_hash(segment: usize, band: usize) -> f32 {
    TAU * ((segment * 73 + band * 151) % DISK_SEGMENTS) as f32 / DISK_SEGMENTS as f32
}

fn push_flat_face(faces: &mut Vec<Face>, indices: [usize; 3], color: [u8; 3], group: &str) {
    let mut face = Face::new(indices, "vesper_accretion_plasma", group);
    face.shader = SurfaceShader::Flat;
    face.color = MaterialColor::Fixed(color);
    faces.push(face);
}
