use crate::obj_loader::{Face, MaterialColor, Model, SurfaceShader, Vec3};
use std::f32::consts::{PI, TAU};

pub(crate) const CORE_RADIUS: f32 = 22.0;
pub(crate) const VISUAL_SCALE: f32 = 1.22;
const DISK_INNER_RADIUS: f32 = 22.45;
const DISK_OUTER_RADIUS: f32 = 78.0;
const DISK_ANGULAR_SEGMENTS: usize = 256;
const DISK_RADIAL_ROWS: usize = 48;

pub(crate) struct BlackHole {
    pub(crate) core: Model,
    pub(crate) disk: Model,
    pub(crate) filaments: Model,
    pub(crate) lensing_arcs: Model,
}

/// Immutable meshes; only the transforms and low-amplitude plasma brightness
/// vary per frame. This avoids allocating or rebuilding geometry while flying.
pub(crate) fn build() -> BlackHole {
    let mut core_vertices = Vec::new();
    let mut core_faces = Vec::new();
    append_sphere(&mut core_vertices, &mut core_faces, CORE_RADIUS, 24, 64);

    let mut disk_vertices = Vec::new();
    let mut disk_faces = Vec::new();
    append_continuous_disk(&mut disk_vertices, &mut disk_faces);

    let mut filament_vertices = Vec::new();
    let mut filament_faces = Vec::new();
    append_filaments(&mut filament_vertices, &mut filament_faces);

    let mut arc_vertices = Vec::new();
    let mut arc_faces = Vec::new();
    append_lensing_arcs(&mut arc_vertices, &mut arc_faces);

    BlackHole {
        core: Model::new(core_vertices, core_faces),
        disk: Model::new(disk_vertices, disk_faces),
        filaments: Model::new(filament_vertices, filament_faces),
        lensing_arcs: Model::new(arc_vertices, arc_faces),
    }
}

fn append_sphere(
    vertices: &mut Vec<Vec3>,
    faces: &mut Vec<Face>,
    radius: f32,
    stacks: usize,
    slices: usize,
) {
    let top = vertices.len();
    vertices.push(Vec3::new(0.0, radius, 0.0));
    let first_ring = vertices.len();
    for stack in 1..stacks {
        let latitude = PI * stack as f32 / stacks as f32 - PI * 0.5;
        let ring_radius = latitude.cos() * radius;
        let y = latitude.sin() * radius;
        for slice in 0..slices {
            let longitude = TAU * slice as f32 / slices as f32;
            vertices.push(Vec3::new(
                ring_radius * longitude.cos(),
                y,
                ring_radius * longitude.sin(),
            ));
        }
    }
    let bottom = vertices.len();
    vertices.push(Vec3::new(0.0, -radius, 0.0));
    let ring_count = stacks - 1;

    for slice in 0..slices {
        let next = (slice + 1) % slices;
        push_core_face(faces, [top, first_ring + slice, first_ring + next]);
        let last_ring = first_ring + (ring_count - 1) * slices;
        push_core_face(faces, [bottom, last_ring + next, last_ring + slice]);
    }
    for ring in 0..ring_count - 1 {
        let row = first_ring + ring * slices;
        let next_row = row + slices;
        for slice in 0..slices {
            let next = (slice + 1) % slices;
            push_core_face(faces, [row + slice, next_row + slice, row + next]);
            push_core_face(faces, [row + next, next_row + slice, next_row + next]);
        }
    }
}

fn push_core_face(faces: &mut Vec<Face>, indices: [usize; 3]) {
    let mut face = Face::new(indices, "vesper_black_hole_core", "vesper_black_hole_core");
    face.shader = SurfaceShader::Flat;
    face.color = MaterialColor::Fixed([0, 0, 0]);
    faces.push(face);
}

/// One watertight annular mesh: every radial row is shared, every angular
/// neighbor uses the same vertex index, and the last segment wraps to zero.
fn append_continuous_disk(vertices: &mut Vec<Vec3>, faces: &mut Vec<Face>) {
    for row in 0..=DISK_RADIAL_ROWS {
        let t = row as f32 / DISK_RADIAL_ROWS as f32;
        // Concentrate vertices near the photon rim, where color and curvature
        // change fastest, while preserving fine detail at the outer fade.
        let radius = DISK_INNER_RADIUS + (DISK_OUTER_RADIUS - DISK_INNER_RADIUS) * t.powf(1.32);
        for segment in 0..DISK_ANGULAR_SEGMENTS {
            let angle = TAU * segment as f32 / DISK_ANGULAR_SEGMENTS as f32;
            vertices.push(disk_point(radius, angle));
        }
    }

    for row in 0..DISK_RADIAL_ROWS {
        for segment in 0..DISK_ANGULAR_SEGMENTS {
            let next_segment = (segment + 1) % DISK_ANGULAR_SEGMENTS;
            let a = row * DISK_ANGULAR_SEGMENTS + segment;
            let b = row * DISK_ANGULAR_SEGMENTS + next_segment;
            let c = (row + 1) * DISK_ANGULAR_SEGMENTS + segment;
            let d = (row + 1) * DISK_ANGULAR_SEGMENTS + next_segment;
            let middle_radius =
                (vertices[a].dot(vertices[a]).sqrt() + vertices[d].dot(vertices[d]).sqrt()) * 0.5;
            let color = disk_color(
                middle_radius,
                TAU * segment as f32 / DISK_ANGULAR_SEGMENTS as f32,
            );
            push_color_face(faces, [a, c, b], color, "accretion_disk");
            push_color_face(faces, [b, c, d], color, "accretion_disk");
        }
    }
}

fn disk_point(radius: f32, angle: f32) -> Vec3 {
    // Integer harmonics make every deformation periodic at the angular seam.
    let ripple = (angle * 5.0 + radius * 0.037).sin() * 0.30
        + (angle * 13.0 - radius * 0.021).cos() * 0.14
        + (angle * 29.0 + radius * 0.012).sin() * 0.055;
    let warped_radius = radius + ripple;
    let x = warped_radius * angle.cos();
    let z = warped_radius * angle.sin();
    let y = 0.14 * z
        + (angle * 3.0 + radius * 0.028).sin() * 0.55
        + (angle * 9.0 - radius * 0.016).cos() * 0.20;
    Vec3::new(x, y, z)
}

fn disk_color(radius: f32, angle: f32) -> [u8; 3] {
    let stops = [
        (DISK_INNER_RADIUS, [255.0, 239.0, 185.0]),
        (27.0, [255.0, 203.0, 111.0]),
        (36.0, [255.0, 132.0, 56.0]),
        (51.0, [222.0, 76.0, 35.0]),
        (65.0, [139.0, 37.0, 35.0]),
        (DISK_OUTER_RADIUS, [54.0, 18.0, 30.0]),
    ];
    let upper = stops
        .windows(2)
        .find(|pair| radius <= pair[1].0)
        .unwrap_or(&stops[stops.len() - 2..]);
    let t = ((radius - upper[0].0) / (upper[1].0 - upper[0].0)).clamp(0.0, 1.0);
    let doppler = 0.56 + 0.44 * ((angle - 0.72).cos() * 0.5 + 0.5);
    let turbulence = 0.91
        + 0.10 * (angle * 7.0 + radius * 0.31).sin()
        + 0.07 * (angle * 17.0 - radius * 0.19).cos()
        + 0.035 * (angle * 31.0 + radius * 0.11).sin();
    std::array::from_fn(|channel| {
        let color = upper[0].1[channel] * (1.0 - t) + upper[1].1[channel] * t;
        (color * doppler * turbulence).clamp(0.0, 255.0) as u8
    })
}

fn append_filaments(vertices: &mut Vec<Vec3>, faces: &mut Vec<Face>) {
    const STREAMS: usize = 8;
    const STEPS: usize = 96;
    let colors = [
        [255, 228, 169],
        [255, 177, 92],
        [255, 122, 59],
        [215, 61, 39],
    ];
    for stream in 0..STREAMS {
        let start = vertices.len();
        let phase = TAU * stream as f32 / STREAMS as f32;
        for step in 0..=STEPS {
            let t = step as f32 / STEPS as f32;
            let angle = phase + t * (2.0 + (stream % 3) as f32 * 0.38);
            let radius = 25.0 + t * 53.0 + (angle * 5.0).sin() * 0.34;
            let width = 0.13 + (1.0 - t) * 0.58;
            // Stay a small fixed distance above the same continuous disk
            // surface; this avoids intersections and z-fighting.
            for side in [-1.0, 1.0] {
                let offset = side * width;
                let filament = disk_point(radius + offset, angle);
                vertices.push(Vec3::new(filament.x, filament.y + 0.42, filament.z));
            }
        }
        for step in 0..STEPS {
            let a = start + step * 2;
            let b = a + 2;
            let color = colors[(stream + step / 24) % colors.len()];
            push_color_face(faces, [a, b, a + 1], color, "accretion_filament");
            push_color_face(faces, [a + 1, b, b + 1], color, "accretion_filament");
        }
    }
}

/// Screen-facing lens arcs, placed each frame in the camera/disc basis. The
/// top arc sits behind the event horizon and the lower, weaker arc in front;
/// depth testing therefore produces the correct occultation naturally.
fn append_lensing_arcs(vertices: &mut Vec<Vec3>, faces: &mut Vec<Face>) {
    append_arc(
        vertices,
        faces,
        0.035,
        PI - 0.035,
        26.2,
        -2.1,
        0.29,
        0.64,
        [255, 220, 153],
        "lensing_arc_upper",
    );
    append_arc(
        vertices,
        faces,
        PI + 0.08,
        TAU - 0.08,
        25.0,
        2.6,
        0.24,
        0.25,
        [255, 155, 89],
        "lensing_arc_lower",
    );
    append_arc(
        vertices,
        faces,
        0.12,
        PI - 0.12,
        27.0,
        -1.6,
        0.78,
        0.12,
        [255, 123, 57],
        "lensing_arc_upper_glow",
    );
    append_arc(
        vertices,
        faces,
        PI + 0.18,
        TAU - 0.18,
        26.1,
        2.2,
        0.64,
        0.055,
        [255, 102, 48],
        "lensing_arc_lower_glow",
    );
}

fn append_arc(
    vertices: &mut Vec<Vec3>,
    faces: &mut Vec<Face>,
    start_angle: f32,
    end_angle: f32,
    radius: f32,
    depth: f32,
    half_width: f32,
    alpha_strength: f32,
    color: [u8; 3],
    group: &str,
) {
    const STEPS: usize = 128;
    let base = vertices.len();
    for step in 0..=STEPS {
        let t = step as f32 / STEPS as f32;
        let angle = start_angle + (end_angle - start_angle) * t;
        let r = radius + (angle * 5.0).sin() * 0.12;
        let center = Vec3::new(r * angle.cos(), r * angle.sin(), depth);
        let width = half_width * (0.06 + (t * PI).sin().max(0.0) * 0.94);
        vertices.push(Vec3::new(center.x, center.y - width, center.z));
        vertices.push(Vec3::new(center.x, center.y + width, center.z));
    }
    for step in 0..STEPS {
        let t = (step as f32 + 0.5) / STEPS as f32;
        let alpha = (alpha_strength * (t * PI).sin().max(0.0) * 255.0) as u8;
        let a = base + step * 2;
        let b = a + 2;
        for indices in [[a, b, a + 1], [a + 1, b, b + 1]] {
            let mut face = Face::new(indices, "vesper_lensing_arc", group);
            face.shader = SurfaceShader::Flat;
            face.color = MaterialColor::Fixed(color);
            // Encode the local fade in the material id for the one-off arc
            // style; the regular renderer ignores this metadata.
            face.material = format!("{group}:{alpha}");
            faces.push(face);
        }
    }
}

fn push_color_face(faces: &mut Vec<Face>, indices: [usize; 3], color: [u8; 3], group: &str) {
    let mut face = Face::new(indices, "vesper_accretion_plasma", group);
    face.shader = SurfaceShader::Flat;
    face.color = MaterialColor::Fixed(color);
    faces.push(face);
}

pub(crate) fn disk_alpha(radius: f32) -> u8 {
    let t = ((DISK_OUTER_RADIUS - radius) / (DISK_OUTER_RADIUS - 48.0)).clamp(0.0, 1.0);
    (t * t * 224.0) as u8
}

pub(crate) fn filament_alpha(radius: f32) -> u8 {
    let inner = ((radius - 23.0) / 7.0).clamp(0.0, 1.0);
    let outer = ((DISK_OUTER_RADIUS - radius) / 9.0).clamp(0.0, 1.0);
    (inner.min(outer) * 235.0) as u8
}

pub(crate) fn lensing_arc_alpha(material: &str) -> u8 {
    material
        .rsplit_once(':')
        .and_then(|(_, alpha)| alpha.parse().ok())
        .unwrap_or(0)
}

pub(crate) fn differential_disk_transform(vertex: Vec3, time: f32) -> Vec3 {
    let radius = (vertex.x * vertex.x + vertex.z * vertex.z)
        .sqrt()
        .max(CORE_RADIUS);
    let angular_speed = 0.20 * (CORE_RADIUS / radius).sqrt();
    let (sin, cos) = (time * angular_speed).sin_cos();
    Vec3::new(
        vertex.x * cos - vertex.z * sin,
        vertex.y,
        vertex.x * sin + vertex.z * cos,
    )
}

pub(crate) fn scale_about_center(position: Vec3, center: Vec3) -> Vec3 {
    center.add(position.sub(center).mul(VISUAL_SCALE))
}

pub(crate) fn camera_arc_transform(
    center: Vec3,
    camera_position: Vec3,
    camera_right: Vec3,
    disk_axis: Vec3,
) -> impl Fn(Vec3) -> Vec3 {
    let toward_camera = camera_position.sub(center).normalize();
    let mut right = toward_camera.cross(disk_axis).normalize();
    if right.dot(right) < 1.0e-4 {
        right = camera_right.normalize();
    }
    let up = right.cross(toward_camera).normalize();
    move |point| {
        center
            .add(right.mul(point.x))
            .add(up.mul(point.y))
            .add(toward_camera.mul(point.z))
    }
}
