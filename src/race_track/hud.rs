use crate::{
    HEIGHT, WIDTH, draw_selection_label as label, draw_selection_panel as panel,
    framebuffer::Framebuffer, obj_loader::Vec3,
};

use super::{
    RacePhase, TILE_SIZE,
    drive::DriveState,
    progress::{ProgressEvent, RaceProgress},
};

pub(super) fn draw(
    frame: &mut Framebuffer,
    tiles: &[Vec<u8>],
    progress: &RaceProgress,
    drive: &DriveState,
    position: Vec3,
    yaw: f32,
    phase: RacePhase,
    countdown: f32,
) {
    label(frame, "JARDIN RUST", 600, 20, 2, 0xffe778);
    let lap = format!(
        "VUELTA {}/{}",
        progress.current_lap(),
        progress.total_laps()
    );
    let checkpoint = format!(
        "CHECKPOINT {}/{}",
        progress.current_checkpoint() + 1,
        progress.checkpoint_count()
    );
    label(frame, &lap, 98, 28, 2, 0xffffff);
    label(frame, &checkpoint, 600, 58, 1, 0xaad6ff);

    let speed = format!("{:03.0} KM/H", drive.speed.abs() * 3.6);
    label(frame, &speed, 105, 85, 2, 0x85f1ff);
    let timer = format!("TIEMPO {}", format_time(progress.race_time()));
    label(frame, &timer, 105, 122, 1, 0xffffff);
    let lap_timer = format!("VUELTA {}", format_time(progress.lap_time()));
    label(frame, &lap_timer, 105, 140, 1, 0xc7d8ed);
    if let Some(best) = progress.best_lap_time() {
        let best = format!("MEJOR {}", format_time(best));
        label(frame, &best, 105, 158, 1, 0xffdf6b);
    }

    draw_bar(
        frame,
        38,
        HEIGHT as i32 - 82,
        172,
        12,
        drive.drift_charge,
        0xffb34f,
    );
    label(
        frame,
        "TURBO DE DERRAPE",
        124,
        HEIGHT as i32 - 103,
        1,
        0xffd6a0,
    );
    draw_bar(
        frame,
        38,
        HEIGHT as i32 - 54,
        172,
        9,
        drive.boost_timer.clamp(0.0, 1.4) / 1.4,
        0x4ce7ff,
    );
    label(frame, "IMPULSO", 124, HEIGHT as i32 - 73, 1, 0xb6f5ff);

    draw_minimap(frame, tiles, progress, position, yaw);
    if let Some(event) = progress.event() {
        let (message, color) = event_label(event);
        label(frame, message, 600, 108, 2, color);
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
            panel(frame, 480, 285, 720, 490, true);
            label(frame, "PREPARATE", 600, 325, 2, 0xffffff);
            label(frame, number, 600, 390, 5, 0xffe35c);
        }
        RacePhase::Paused => {
            panel(frame, 295, 278, 905, 510, true);
            label(frame, "CARRERA EN PAUSA", 600, 322, 3, 0xffffff);
            label(frame, "P / ENTER: CONTINUAR", 600, 390, 2, 0x8feaff);
            label(frame, "BACKSPACE: SALIR AL ESPACIO", 600, 438, 2, 0xffdf6b);
        }
        RacePhase::Finished => {
            panel(frame, 285, 250, 915, 545, true);
            label(frame, "CARRERA COMPLETADA", 600, 295, 3, 0xffe778);
            label(
                frame,
                &format!("TIEMPO {}", format_time(progress.race_time())),
                600,
                365,
                2,
                0xffffff,
            );
            if let Some(last) = progress.last_lap_time() {
                label(
                    frame,
                    &format!("ULTIMA VUELTA {}", format_time(last)),
                    600,
                    390,
                    2,
                    0xc7d8ed,
                );
            }
            if let Some(best) = progress.best_lap_time() {
                label(
                    frame,
                    &format!("MEJOR VUELTA {}", format_time(best)),
                    600,
                    405,
                    2,
                    0x9fffd2,
                );
            }
            label(frame, "ENTER: REINTENTAR", 600, 460, 2, 0xffffff);
            label(frame, "BACKSPACE: VOLVER AL ESPACIO", 600, 500, 2, 0xffdf6b);
        }
        RacePhase::Driving => {
            label(
                frame,
                "W/UP ACELERA  S/DOWN FRENA  A/D O FLECHAS GIRA",
                600,
                HEIGHT as i32 - 27,
                1,
                0xd7e5f1,
            );
            label(
                frame,
                "SPACE DERRAPE  SHIFT TURBO  R RECUPERAR  P PAUSA  BACKSPACE SALIR",
                600,
                HEIGHT as i32 - 12,
                1,
                0xd7e5f1,
            );
        }
    }
}

fn draw_minimap(
    frame: &mut Framebuffer,
    tiles: &[Vec<u8>],
    progress: &RaceProgress,
    position: Vec3,
    yaw: f32,
) {
    const SCALE: usize = 2;
    const LEFT: usize = WIDTH - 65 * SCALE - 24;
    const TOP: usize = 62;
    panel(
        frame,
        LEFT - 7,
        TOP - 7,
        WIDTH - 10,
        TOP + 41 * SCALE + 7,
        false,
    );
    for (row, line) in tiles.iter().enumerate() {
        for (col, tile) in line.iter().enumerate() {
            let color = match tile {
                b'#' | b'H' | b'S' | b'D' => 0x263342,
                b'W' => 0x24668b,
                b'R' => 0xffc83d,
                b'P' | b'K' | b'L' | b'M' => 0x88949b,
                _ => 0x285637,
            };
            for dy in 0..SCALE {
                for dx in 0..SCALE {
                    let x = LEFT + col * SCALE + dx;
                    let y = TOP + row * SCALE + dy;
                    if x < WIDTH && y < HEIGHT {
                        frame.pixels[y * frame.width + x] = color;
                    }
                }
            }
        }
    }
    for pair in progress.checkpoints().windows(2) {
        let a = grid_point(pair[0].center);
        let b = grid_point(pair[1].center);
        draw_line(
            frame,
            LEFT as i32 + a.0,
            TOP as i32 + a.1,
            LEFT as i32 + b.0,
            TOP as i32 + b.1,
            0xecd35d,
        );
    }
    if let Some(active) = progress.active_checkpoint() {
        let (x, y) = grid_point(active.center);
        draw_disk(frame, LEFT as i32 + x, TOP as i32 + y, 3, 0xff5c56);
    }
    let player_x = (position.x / TILE_SIZE * SCALE as f32) as i32;
    let player_y = (position.z / TILE_SIZE * SCALE as f32) as i32;
    let px = LEFT as i32 + player_x;
    let py = TOP as i32 + player_y;
    draw_disk(frame, px, py, 3, 0x71f3ff);
    let (sin, cos) = yaw.sin_cos();
    draw_line(
        frame,
        px,
        py,
        px + (-cos * 10.0) as i32,
        py + (-sin * 10.0) as i32,
        0xffffff,
    );
}

fn grid_point(point: Vec3) -> (i32, i32) {
    (
        (point.x / TILE_SIZE * 2.0) as i32,
        (point.z / TILE_SIZE * 2.0) as i32,
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
