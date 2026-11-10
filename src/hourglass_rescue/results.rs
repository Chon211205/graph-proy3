use crate::{draw_selection_label, framebuffer::Framebuffer};

use super::ui::{draw_overlay, panel};
use super::{HourglassRescue, Phase};

pub(super) fn draw(game: &HourglassRescue, frame: &mut Framebuffer, scale: i32) {
    let w = frame.width as i32;
    let h = frame.height as i32;
    let won = game.phase == Phase::Won;
    draw_overlay(frame, if won { 0x080c15 } else { 0x12070f }, 0.88);
    panel(
        frame,
        0.07,
        0.07,
        0.93,
        0.93,
        0x10131b,
        if won { 0x8bdde9 } else { 0xb94c5b },
    );
    draw_selection_label(
        frame,
        "REPORTE DE MISION  //  PLANETA DEL RELOJ",
        w / 2,
        (h as f32 * 0.13) as i32,
        scale,
        0xe2c28c,
    );
    draw_selection_label(
        frame,
        if won {
            "ESFERA DE 5 ESTRELLAS CONSEGUIDA"
        } else {
            "MISION FALLIDA"
        },
        w / 2,
        (h as f32 * 0.25) as i32,
        scale * 3,
        if won { 0x8ceafa } else { 0xf07579 },
    );
    draw_selection_label(
        frame,
        &format!("FRAGMENTOS DE ENERGIA  {}/5", game.rescued),
        w / 2,
        (h as f32 * 0.43) as i32,
        scale * 2,
        0xf2e7d2,
    );
    draw_selection_label(
        frame,
        &format!(
            "TIEMPO EMPLEADO  {:02}:{:02}",
            game.elapsed.max(0.0) as u32 / 60,
            game.elapsed.max(0.0) as u32 % 60
        ),
        w / 2,
        (h as f32 * 0.52) as i32,
        scale,
        0xc9d9e1,
    );
    draw_selection_label(
        frame,
        &format!("RELOJES RECOGIDOS  {}/{}", game.clocks_collected, super::CLOCK_COUNT),
        w / 2,
        (h as f32 * 0.58) as i32,
        scale,
        0xc9d9e1,
    );
    panel(frame, 0.25, 0.69, 0.75, 0.78, 0x211c24, 0xc58c51);
    draw_selection_label(
        frame,
        "R  REINTENTAR",
        w / 2,
        (h as f32 * 0.715) as i32,
        scale * 2,
        0xffd17c,
    );
    panel(frame, 0.20, 0.81, 0.80, 0.90, 0x1d2027, 0x8bcad2);
    draw_selection_label(
        frame,
        "BACKSPACE  VOLVER A VESPER",
        w / 2,
        (h as f32 * 0.835) as i32,
        scale,
        0xd4e6e8,
    );
}
