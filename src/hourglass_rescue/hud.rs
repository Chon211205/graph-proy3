use crate::ship::Ship;
use crate::{draw_selection_label, framebuffer::Framebuffer};

use super::scene::CAPSULE_POSITIONS;
use super::ui::{bar, draw_overlay, panel, shield_icon};
use super::{CAPSULE_COUNT, EXIT_POSITION, HourglassRescue, Phase};

pub(super) fn draw(game: &HourglassRescue, frame: &mut Framebuffer, ship: &Ship) {
    let w = frame.width as i32;
    let h = frame.height as i32;
    let scale = (frame.width as f32 / 600.0).round().clamp(1.0, 3.0) as i32;
    if matches!(game.phase, Phase::Won | Phase::Lost) {
        super::results::draw(game, frame, scale);
        return;
    }
    if game.phase == Phase::Paused {
        draw_overlay(frame, 0x080711, 0.12);
        panel(frame, 0.18, 0.28, 0.82, 0.72, 0x12101e, 0x8e66bd);
        draw_selection_label(
            frame,
            "MISION EN PAUSA",
            w / 2,
            (h as f32 * 0.39) as i32,
            scale * 2,
            0xffd17a,
        );
        draw_selection_label(
            frame,
            "P / ESC  CONTINUAR",
            w / 2,
            (h as f32 * 0.51) as i32,
            scale,
            0xd7eaf0,
        );
        draw_selection_label(
            frame,
            "R  REINICIAR",
            w / 2,
            (h as f32 * 0.57) as i32,
            scale,
            0xd7eaf0,
        );
        draw_selection_label(
            frame,
            "BACKSPACE  VOLVER A VESPER",
            w / 2,
            (h as f32 * 0.63) as i32,
            scale,
            0xffbd67,
        );
        draw_selection_label(
            frame,
            "W/S AVANZAR  A/D GIRAR  ESPACIO/CTRL ALTURA  E RESCATAR",
            w / 2,
            (h as f32 * 0.68) as i32,
            (scale / 2).max(1),
            0xaac9d4,
        );
        return;
    }

    panel(frame, 0.018, 0.022, 0.23, 0.12, 0x15121e, 0xb98552);
    draw_selection_label(
        frame,
        "RESCATADAS",
        (w as f32 * 0.12) as i32,
        (h as f32 * 0.037) as i32,
        scale,
        0xf2dfc4,
    );
    draw_selection_label(
        frame,
        &format!("{}/5", game.rescued),
        (w as f32 * 0.12) as i32,
        (h as f32 * 0.074) as i32,
        scale * 2,
        0xffce74,
    );

    panel(frame, 0.77, 0.022, 0.982, 0.12, 0x15121e, 0xb55fc0);
    let seconds = game.remaining().ceil() as u32;
    draw_selection_label(
        frame,
        &format!("{:02}:{:02}", seconds / 60, seconds % 60),
        (w as f32 * 0.876) as i32,
        (h as f32 * 0.047) as i32,
        scale * 2,
        0xf5efff,
    );
    let gravity_label = if game.inversion_complete {
        "GRAVEDAD: ARRIBA"
    } else {
        "GRAVEDAD: ABAJO"
    };
    draw_selection_label(
        frame,
        gravity_label,
        (w as f32 * 0.876) as i32,
        (h as f32 * 0.091) as i32,
        scale,
        0xdbc7ef,
    );

    draw_selection_label(
        frame,
        "RESCATE A CONTRATIEMPO",
        w / 2,
        (h as f32 * 0.025) as i32,
        scale,
        0xffd88b,
    );
    if !game.inversion_complete {
        let left = game.inversion_countdown().ceil() as u32;
        draw_selection_label(
            frame,
            &format!("INVERSION EN {:02}:{:02}", left / 60, left % 60),
            w / 2,
            (h as f32 * 0.065) as i32,
            scale,
            if game.phase == Phase::Warning {
                0xffad67
            } else {
                0xe0c7f5
            },
        );
    } else {
        draw_selection_label(
            frame,
            "CAMARA SUPERIOR ABIERTA",
            w / 2,
            (h as f32 * 0.065) as i32,
            scale,
            0x9deafa,
        );
    }

    for index in 0..3 {
        let color = if index < game.shields {
            0x73e8ef
        } else {
            0x53434f
        };
        let x = 32 + index as i32 * 34;
        let y = h - (scale * 19 + 18);
        shield_icon(frame, x, y, scale * 2, color);
    }
    panel(frame, 0.018, 0.83, 0.28, 0.965, 0x15121e, 0x795c9c);
    draw_selection_label(
        frame,
        "IMPULSO",
        (w as f32 * 0.15) as i32,
        (h as f32 * 0.846) as i32,
        scale,
        0xd7c2ed,
    );
    bar(
        frame,
        (w as f32 * 0.035) as i32,
        (h as f32 * 0.88) as i32,
        (w as f32 * 0.23) as i32,
        (scale * 5).max(5),
        game.boost_reserve / 2.2,
        0x6bd9ee,
    );
    draw_selection_label(
        frame,
        "SHIFT: IMPULSO",
        (w as f32 * 0.15) as i32,
        (h as f32 * 0.913) as i32,
        scale,
        0xaccbd8,
    );

    let target = if game.rescued as usize == CAPSULE_COUNT {
        Some((EXIT_POSITION, "PORTAL"))
    } else {
        game.rescue_target
            .map(|index| (CAPSULE_POSITIONS[index], "SEÑAL DE RESCATE"))
    };
    if let Some((position, label)) = target {
        let distance = game.target_distance_for_hud(ship).unwrap_or(f32::INFINITY);
        // Long-range labels used to point straight at each capsule from spawn.
        // The signal only resolves once the pilot has searched the region.
        let signal_range = if game.rescued as usize == CAPSULE_COUNT {
            140.0
        } else {
            85.0
        };
        let (signal_label, signal_color) = if distance <= signal_range {
            ("RADAR: CONTACTO", 0x8ce7ef)
        } else if distance <= 180.0 {
            ("RADAR: SENAL INTERMITENTE", 0xffd17a)
        } else if distance <= 300.0 {
            ("RADAR: ECO DEBIL", 0xc5a9d9)
        } else {
            ("RADAR: EXPLORANDO", 0x8b839c)
        };
        draw_selection_label(
            frame,
            signal_label,
            (w as f32 * 0.82) as i32,
            (h as f32 * 0.82) as i32,
            scale,
            signal_color,
        );
        let camera = game.camera(ship);
        if distance <= signal_range {
            if let Some((x, y)) =
                super::rendering::project(&camera, position, frame.width, frame.height)
            {
                if x >= 80 && x < w - 80 && y >= 50 && y < h - 70 {
                    let distance = distance.round() as u32;
                    draw_selection_label(
                        frame,
                        &format!("{}  {:03}M", label, distance),
                        x,
                        y + scale * 8,
                        (scale / 2).max(1),
                        0xffe08f,
                    );
                }
            }
        }
    }
    if game.rescue_progress > 0.0 {
        draw_selection_label(
            frame,
            "MANTENE E PARA RESCATAR",
            (w as f32 * 0.83) as i32,
            (h as f32 * 0.875) as i32,
            scale,
            0x8ce7ef,
        );
        bar(
            frame,
            (w as f32 * 0.72) as i32,
            (h as f32 * 0.915) as i32,
            (w as f32 * 0.22) as i32,
            (scale * 5).max(5),
            game.rescue_progress_ratio(),
            0x74e0e9,
        );
    }

    if game.feedback_timer > 0.0
        && !game.feedback.is_empty()
        && game.feedback != "RESCATA LAS CINCO CAPSULAS Y ALCANZA LA SALIDA"
    {
        draw_selection_label(
            frame,
            game.feedback,
            w / 2,
            (h as f32 * 0.17) as i32,
            scale,
            0xffd28b,
        );
    }
    match game.phase {
        Phase::Intro => {
            draw_overlay(frame, 0x080711, 0.34);
            panel(frame, 0.20, 0.34, 0.80, 0.66, 0x171320, 0xc08659);
            draw_selection_label(
                frame,
                "RESCATA LAS CINCO CAPSULAS",
                w / 2,
                (h as f32 * 0.405) as i32,
                scale * 2,
                0xffd58a,
            );
            draw_selection_label(
                frame,
                "W/S AVANZAR  A/D GIRAR  E MANTENER PARA RESCATAR",
                w / 2,
                (h as f32 * 0.48) as i32,
                (scale / 2).max(1),
                0xe5dced,
            );
            draw_selection_label(
                frame,
                "ESPACIO/CTRL ALTURA  SHIFT IMPULSO  P PAUSA",
                w / 2,
                (h as f32 * 0.535) as i32,
                (scale / 2).max(1),
                0xbdeaf0,
            );
            draw_selection_label(
                frame,
                "LA INVERSION ABRIRA EL SANTUARIO  //  BACKSPACE: ESPACIO",
                w / 2,
                (h as f32 * 0.60) as i32,
                (scale / 2).max(1),
                0xd5c4e7,
            );
        }
        Phase::Inverting => {
            draw_overlay(frame, 0x3f1646, 0.22);
            draw_selection_label(
                frame,
                "INVERSION DE GRAVEDAD",
                w / 2,
                (h as f32 * 0.45) as i32,
                scale * 2,
                0xf2c0ff,
            );
            draw_selection_label(
                frame,
                "SUJETA EL RUMBO",
                w / 2,
                (h as f32 * 0.52) as i32,
                scale,
                0xffffff,
            );
        }
        _ => {}
    }
}
