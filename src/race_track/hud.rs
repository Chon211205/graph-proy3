use crate::{
    HEIGHT, WIDTH, draw_selection_label as label, draw_selection_panel as panel,
    framebuffer::Framebuffer, obj_loader::Vec3,
};

use super::{
    RacePhase,
    course::RaceCourse,
    drive::DriveState,
    progress::{ProgressEvent, RaceProgress},
    racer::Rival,
};

pub(super) fn draw(
    frame: &mut Framebuffer,
    course: &RaceCourse,
    progress: &RaceProgress,
    drive: &DriveState,
    position: Vec3,
    yaw: f32,
    rivals: Option<&[Rival; 3]>,
    place: usize,
    phase: RacePhase,
    countdown: f32,
) {
    draw_space_panel(frame, 24, 14, 248, 176, 0x4ddfff);
    draw_space_panel(frame, 438, 8, 324, 62, 0xffd15c);
    draw_space_panel(frame, 970, 14, 216, 42, 0xffd15c);
    label(frame, "TELEMETRIA ORBITAL", 148, 22, 1, 0x76e9ff);
    label(frame, "GRAN PREMIO ORBITAL", 600, 16, 2, 0xffe778);
    label(
        frame,
        &format!(
            "VUELTA {}/{}",
            progress.current_lap(),
            progress.total_laps()
        ),
        148,
        43,
        2,
        0xffffff,
    );
    label(frame, &format!("PUESTO {place}/4"), 1078, 23, 1, 0xffdf6b);
    label(
        frame,
        &format!(
            "SECTOR {:02}/{:02}",
            progress.current_checkpoint() + 1,
            progress.checkpoint_count()
        ),
        600,
        46,
        1,
        0xaad6ff,
    );
    label(
        frame,
        &format!("{:03.0} KM/H", drive.speed.abs() * 3.6),
        148,
        77,
        2,
        0x85f1ff,
    );
    label(
        frame,
        &format!("TIEMPO {}", format_time(progress.race_time())),
        145,
        112,
        1,
        0xffffff,
    );
    label(
        frame,
        &format!("VUELTA {}", format_time(progress.lap_time())),
        145,
        132,
        1,
        0xc7d8ed,
    );
    if let Some(last) = progress.last_lap_time() {
        label(
            frame,
            &format!("ULTIMA {}", format_time(last)),
            145,
            151,
            1,
            0xb8d7c8,
        );
    }
    if let Some(best) = progress.best_lap_time() {
        label(
            frame,
            &format!("MEJOR {}", format_time(best)),
            145,
            170,
            1,
            0xffdf6b,
        );
    }

    draw_minimap(frame, course, progress, position, yaw, rivals);
    label(frame, "RESERVA TURBO", 112, 675, 1, 0xffd6a0);
    draw_bar(frame, 34, 690, 156, 9, drive.turbo / 3.2, 0xffbc42);
    label(frame, "DERRAPE", 112, 713, 1, 0x9cecff);
    draw_bar(frame, 34, 728, 156, 8, drive.drift_charge, 0x57d9ed);
    if let Some(event) = progress.event() {
        let (message, color) = event_label(event);
        label(frame, message, 600, 95, 2, color);
    }

    match phase {
        RacePhase::Countdown => {
            let number = if countdown > 2.0 {
                "3"
            } else if countdown > 1.0 {
                "2"
            } else if countdown > 0.0 {
                "1"
            } else {
                "YA!"
            };
            draw_space_panel(frame, 438, 258, 324, 286, 0x4ddfff);
            label(frame, "SECUENCIA DE LANZAMIENTO", 600, 282, 1, 0x8deaff);
            label(frame, "MOTORES ORBITALES", 600, 309, 1, 0xffd15c);
            draw_ring(frame, 600, 414, 70, 0x284a67);
            draw_ring(frame, 600, 414, 61, 0x4ddfff);
            draw_ring_ticks(frame, 600, 414, 70);
            let countdown_color = if number == "YA!" { 0x8dffcf } else { 0xffe35c };
            label(frame, number, 600, 367, 5, countdown_color);
            label(frame, "ENLACE DE CARRERA ESTABLE", 600, 496, 1, 0xa7cde1);
            draw_bar(
                frame,
                500,
                521,
                200,
                7,
                (3.25 - countdown).clamp(0.0, 3.25) / 3.25,
                0x4ddfff,
            );
        }
        RacePhase::Paused => {
            panel(frame, 310, 290, 890, 500, true);
            label(frame, "CARRERA EN PAUSA", 600, 330, 3, 0xffffff);
            label(frame, "P / ENTER: CONTINUAR", 600, 395, 2, 0x8feaff);
            label(frame, "BACKSPACE: SALIR AL ESPACIO", 600, 442, 2, 0xffdf6b);
        }
        RacePhase::Finished => {
            panel(frame, 300, 250, 900, 555, true);
            label(
                frame,
                if place == 1 {
                    "VICTORIA"
                } else {
                    "CARRERA FINALIZADA"
                },
                600,
                294,
                3,
                if place == 1 { 0xffdf6b } else { 0xffffff },
            );
            label(
                frame,
                &format!("POSICION FINAL {place}/4"),
                600,
                350,
                2,
                0xffdf6b,
            );
            label(
                frame,
                &format!("TIEMPO {}", format_time(progress.race_time())),
                600,
                385,
                2,
                0xffffff,
            );
            if let Some(best) = progress.best_lap_time() {
                label(
                    frame,
                    &format!("MEJOR VUELTA {}", format_time(best)),
                    600,
                    420,
                    2,
                    0x9fffd2,
                );
            }
            if let Some(last) = progress.last_lap_time() {
                label(
                    frame,
                    &format!("ULTIMA VUELTA {}", format_time(last)),
                    600,
                    440,
                    2,
                    0xc7d8ed,
                );
            }
            label(frame, "ENTER: REINTENTAR", 600, 475, 2, 0xffffff);
            label(frame, "BACKSPACE: SALIR AL ESPACIO", 600, 515, 2, 0xffdf6b);
        }
        RacePhase::Driving => {
            label(
                frame,
                "W ADELANTE  S RETROCESO  A IZQUIERDA  D DERECHA  SPACE DERRAPE  SHIFT TURBO  P PAUSA",
                600,
                HEIGHT as i32 - 24,
                1,
                0xd7e5f1,
            );
            label(
                frame,
                "R RECUPERAR  BACKSPACE SALIR",
                600,
                HEIGHT as i32 - 10,
                1,
                0xc5d7e7,
            );
        }
    }
}

fn draw_minimap(
    frame: &mut Framebuffer,
    course: &RaceCourse,
    progress: &RaceProgress,
    position: Vec3,
    yaw: f32,
    rivals: Option<&[Rival; 3]>,
) {
    const MAP_WIDTH: usize = 150;
    const MAP_HEIGHT: usize = 98;
    const LEFT: usize = WIDTH - MAP_WIDTH - 20;
    const TOP: usize = 62;
    panel(
        frame,
        LEFT - 7,
        TOP - 7,
        WIDTH - 8,
        TOP + MAP_HEIGHT + 7,
        false,
    );
    label(
        frame,
        "RADAR ORBITAL",
        (LEFT + MAP_WIDTH / 2) as i32,
        TOP as i32 - 19,
        1,
        0x76e9ff,
    );
    for sample in course.samples.iter().step_by(2) {
        let (x, y) = map_point(sample.position);
        draw_disk(frame, LEFT as i32 + x, TOP as i32 + y, 3, 0x707e80);
    }
    for checkpoint in progress.checkpoints() {
        let (x, y) = map_point(checkpoint.center);
        draw_disk(frame, LEFT as i32 + x, TOP as i32 + y, 2, 0xe0bb55);
    }
    if let Some(active) = progress.active_checkpoint() {
        let (x, y) = map_point(active.center);
        draw_ring(frame, LEFT as i32 + x, TOP as i32 + y, 6, 0xff775b);
        draw_disk(frame, LEFT as i32 + x, TOP as i32 + y, 2, 0xffd15c);
    }
    for (index, rival) in rivals.into_iter().flatten().enumerate() {
        let (x, y) = map_point(rival.controller.ship.position);
        draw_disk(
            frame,
            LEFT as i32 + x,
            TOP as i32 + y,
            4,
            [0xff6d53, 0xffc94e, 0xc177ff][index],
        );
    }
    let (x, y) = map_point(position);
    let px = LEFT as i32 + x;
    let py = TOP as i32 + y;
    draw_ring(frame, px, py, 7, 0x4ddfff);
    draw_disk(frame, px, py, 3, 0xc9f8ff);
    let (sin, cos) = yaw.sin_cos();
    draw_line(
        frame,
        px,
        py,
        px + (-cos * 8.0) as i32,
        py + (-sin * 8.0) as i32,
        0xffffff,
    );
}

fn map_point(position: Vec3) -> (i32, i32) {
    (
        ((position.x + 45.0) * 0.29) as i32,
        ((position.z + 30.0) * 0.29) as i32,
    )
}

fn draw_bar(
    frame: &mut Framebuffer,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    value: f32,
    color: u32,
) {
    for row in 0..height {
        for col in 0..width {
            let index = (y + row) as usize * frame.width + (x + col) as usize;
            frame.pixels[index] = if col as f32 / width as f32 <= value.clamp(0.0, 1.0) {
                color
            } else {
                0x25303c
            };
        }
    }
}

fn draw_space_panel(
    frame: &mut Framebuffer,
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    accent: u32,
) {
    panel(
        frame,
        left as usize,
        top as usize,
        (left + width) as usize,
        (top + height) as usize,
        false,
    );
    let corner = 12;
    for (x, y, dx, dy) in [
        (left + 2, top + 2, 1, 1),
        (left + width - 2, top + 2, -1, 1),
        (left + 2, top + height - 2, 1, -1),
        (left + width - 2, top + height - 2, -1, -1),
    ] {
        draw_line(frame, x, y, x + dx * corner, y, accent);
        draw_line(frame, x, y, x, y + dy * corner, accent);
    }
}

fn draw_ring(frame: &mut Framebuffer, cx: i32, cy: i32, radius: i32, color: u32) {
    let segments = 64;
    for index in 0..segments {
        let angle0 = index as f32 * std::f32::consts::TAU / segments as f32;
        let angle1 = (index + 1) as f32 * std::f32::consts::TAU / segments as f32;
        let point = |angle: f32| {
            (
                cx + (angle.cos() * radius as f32) as i32,
                cy + (angle.sin() * radius as f32) as i32,
            )
        };
        let (x0, y0) = point(angle0);
        let (x1, y1) = point(angle1);
        draw_line(frame, x0, y0, x1, y1, color);
    }
}

fn draw_ring_ticks(frame: &mut Framebuffer, cx: i32, cy: i32, radius: i32) {
    for index in 0..12 {
        let angle = index as f32 * std::f32::consts::TAU / 12.0;
        let outer = radius + 8;
        let inner = if index % 3 == 0 { radius + 1 } else { radius + 4 };
        draw_line(
            frame,
            cx + (angle.cos() * inner as f32) as i32,
            cy + (angle.sin() * inner as f32) as i32,
            cx + (angle.cos() * outer as f32) as i32,
            cy + (angle.sin() * outer as f32) as i32,
            if index % 3 == 0 { 0xffd15c } else { 0x4ddfff },
        );
    }
}

fn draw_disk(frame: &mut Framebuffer, cx: i32, cy: i32, radius: i32, color: u32) {
    for y in -radius..=radius {
        for x in -radius..=radius {
            if x * x + y * y > radius * radius {
                continue;
            }
            let px = cx + x;
            let py = cy + y;
            if px >= 0 && py >= 0 && px < frame.width as i32 && py < frame.height as i32 {
                frame.pixels[py as usize * frame.width + px as usize] = color;
            }
        }
    }
}

fn draw_line(frame: &mut Framebuffer, x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut error = dx + dy;
    let (mut x, mut y) = (x0, y0);
    loop {
        if x >= 0 && y >= 0 && x < frame.width as i32 && y < frame.height as i32 {
            frame.pixels[y as usize * frame.width + x as usize] = color;
        }
        if x == x1 && y == y1 {
            break;
        }
        let twice = 2 * error;
        if twice >= dy {
            error += dy;
            x += sx;
        }
        if twice <= dx {
            error += dx;
            y += sy;
        }
    }
}

fn event_label(event: ProgressEvent) -> (&'static str, u32) {
    match event {
        ProgressEvent::Checkpoint => ("CHECKPOINT", 0x8feaff),
        ProgressEvent::Lap => ("VUELTA COMPLETA", 0xffe778),
        ProgressEvent::FinalLap => ("ULTIMA VUELTA", 0xffb35c),
        ProgressEvent::Finished => ("META", 0x9fffd2),
    }
}

fn format_time(seconds: f32) -> String {
    let centiseconds = (seconds.max(0.0) * 100.0) as u32;
    format!(
        "{:02}:{:02}.{:02}",
        centiseconds / 6000,
        centiseconds / 100 % 60,
        centiseconds % 100
    )
}
