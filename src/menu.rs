use minifb::{Key, Window};

use crate::{
    HEIGHT, WIDTH, draw_selection_label as text, draw_selection_panel as panel, draw_skybox,
    framebuffer::Framebuffer,
};

#[derive(Clone, Copy, PartialEq)]
pub enum Action {
    Play,
    Hangar,
    Quit,
}

fn background(frame: &mut Framebuffer, title: &str, subtitle: &str) {
    frame.clear(0);
    draw_skybox(frame, 0.0);
    text(frame, "STAR COMMAND", 600, 58, 5, 0x99e226);
    text(frame, title, 600, 125, 3, 0xf0f3fa);
    text(frame, subtitle, 600, 172, 2, 0xb7a1df);
}

pub fn instructions(window: &mut Window, frame: &mut Framebuffer) -> Result<(), minifb::Error> {
    crate::input::clear();
    window.update();
    window.set_title("Instrucciones | Esc o Enter: volver");
    while window.is_open() {
        if crate::input::pressed(Key::Escape)
            || crate::input::pressed(Key::Enter)
            || crate::input::pressed(Key::F1)
        {
            break;
        }
        background(
            frame,
            "MANUAL DE VUELO",
            "EXPLORA EL ESPACIO Y COMPLETA EL NIVEL SUBMARINO",
        );
        panel(frame, 55, 215, 580, 565, false);
        panel(frame, 605, 215, 1145, 565, false);
        text(frame, "PILOTA TU VEHICULO", 318, 240, 2, 0x99e226);
        text(frame, "MISION Y SISTEMA", 875, 240, 2, 0x99e226);
        let left = [
            "W / S O ARRIBA / ABAJO: ALTURA",
            "A / D: GIRAR",
            "Q / E: ALABEO",
            "ESPACIO: ACELERAR",
            "CTRL: FRENAR",
            "SHIFT: VELOCIDAD DE LUZ",
            "ENTER: DISPARAR",
            "MANTEN ENTER PARA CARGAR",
        ];
        let right = [
            "1: IR AL PLANETA ACUATICO",
            "ENTER EN ORBITA: NIVEL ACUATICO",
            "BACKSPACE: SALIR DE ORBITA/NIVEL",
            "R: REINICIAR POSICION",
            "T: BUZZ NAVE / CARRO",
            "F11: PANTALLA COMPLETA",
            "P: GUARDAR CAPTURA",
            "ESC: PAUSA   F1: AYUDA",
        ];
        for (i, line) in left.iter().enumerate() {
            text(frame, line, 318, 285 + i as i32 * 32, 2, 0xd5deec);
        }
        for (i, line) in right.iter().enumerate() {
            text(frame, line, 875, 285 + i as i32 * 32, 2, 0xd5deec);
        }
        text(
            frame,
            "BUZZ SE TRANSFORMA EN SUBMARINO AL ENTRAR AL AGUA",
            600,
            602,
            2,
            0xb7a1df,
        );
        text(
            frame,
            "ESQUIVA OBSTACULOS Y LLEGA AL FINAL DEL RECORRIDO",
            600,
            638,
            2,
            0xd5deec,
        );
        text(frame, "ENTER / ESC: VOLVER", 600, 716, 2, 0x99e226);
        crate::input::clear();
        window.update_with_buffer(&frame.pixels, WIDTH, HEIGHT)?;
    }
    Ok(())
}

pub fn show(
    window: &mut Window,
    frame: &mut Framebuffer,
    paused: bool,
) -> Result<Action, minifb::Error> {
    crate::input::clear();
    window.update();
    let options: &[&str] = if paused {
        &["CONTINUAR", "CAMBIAR NAVE", "INSTRUCCIONES", "SALIR"]
    } else {
        &["JUGAR", "INSTRUCCIONES", "SALIR"]
    };
    let mut selected = 0;
    while window.is_open() {
        window.set_title(if paused {
            "Pausa | Flechas: elegir | Enter: confirmar"
        } else {
            "Star Command | Menu principal"
        });
        if crate::input::pressed(Key::Escape) {
            return Ok(if paused { Action::Play } else { Action::Quit });
        }
        if crate::input::pressed(Key::Up) || crate::input::pressed(Key::W) {
            selected = (selected + options.len() - 1) % options.len();
        }
        if crate::input::pressed(Key::Down) || crate::input::pressed(Key::S) {
            selected = (selected + 1) % options.len();
        }
        if crate::input::pressed(Key::Enter) {
            match options[selected] {
                "JUGAR" | "CONTINUAR" => return Ok(Action::Play),
                "CAMBIAR NAVE" => return Ok(Action::Hangar),
                "SALIR" => return Ok(Action::Quit),
                _ => instructions(window, frame)?,
            }
        }
        background(
            frame,
            if paused {
                "JUEGO EN PAUSA"
            } else {
                "AVENTURA ESPACIAL"
            },
            "AL INFINITO Y MAS ALLA",
        );
        for (i, label) in options.iter().enumerate() {
            let y = 260 + i * 90;
            panel(frame, 330, y, 870, y + 68, i == selected);
            text(
                frame,
                label,
                600,
                y as i32 + 23,
                3,
                if i == selected { 0x99e226 } else { 0xa9bbd2 },
            );
        }
        text(
            frame,
            "FLECHAS / W S: ELEGIR    ENTER: CONFIRMAR",
            600,
            680,
            2,
            0xd5deec,
        );
        text(
            frame,
            "CINCO NAVES - BUZZ CON TRES TRANSFORMACIONES",
            600,
            730,
            2,
            0xb7a1df,
        );
        crate::input::clear();
        window.update_with_buffer(&frame.pixels, WIDTH, HEIGHT)?;
    }
    Ok(Action::Quit)
}
