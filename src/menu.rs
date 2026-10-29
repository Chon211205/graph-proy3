use std::{fs::File, sync::OnceLock};

use minifb::{Key, Window};

use crate::{
    HEIGHT, WIDTH, draw_selection_label as text, draw_selection_panel as panel, draw_skybox,
    framebuffer::Framebuffer,
};

static MENU_BACKGROUND: OnceLock<Option<Vec<u32>>> = OnceLock::new();

fn load_background() -> Option<Vec<u32>> {
    let file = File::open("assets/menu_background.png").ok()?;
    let mut decoder = png::Decoder::new(file);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let mut source = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut source).ok()?;
    let source = &source[..info.buffer_size()];
    let channels = match info.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Indexed => return None,
    };
    let width = info.width as usize;
    let height = info.height as usize;
    let crop_width = (height * WIDTH / HEIGHT).min(width);
    let crop_left = (width - crop_width) / 2;
    let mut pixels = vec![0; WIDTH * HEIGHT];
    for y in 0..HEIGHT {
        let source_y = y * height / HEIGHT;
        for x in 0..WIDTH {
            let source_x = crop_left + x * crop_width / WIDTH;
            let offset = (source_y * width + source_x) * channels;
            let (r, g, b) = match info.color_type {
                png::ColorType::Rgb | png::ColorType::Rgba =>
                    (source[offset], source[offset + 1], source[offset + 2]),
                png::ColorType::Grayscale | png::ColorType::GrayscaleAlpha =>
                    (source[offset], source[offset], source[offset]),
                png::ColorType::Indexed => unreachable!(),
            };
            pixels[y * WIDTH + x] = ((r as u32) << 16) | ((g as u32) << 8) | b as u32;
        }
    }
    Some(pixels)
}

#[derive(Clone, Copy, PartialEq)]
pub enum Action {
    Play,
    Hangar,
    Quit,
}

pub(crate) fn draw_backdrop(frame: &mut Framebuffer) {
    frame.clear(0);
    if let Some(pixels) = MENU_BACKGROUND.get_or_init(load_background) {
        frame.pixels.copy_from_slice(pixels);
    } else {
        draw_skybox(frame, 0.0);
    }
}

fn left_text(frame: &mut Framebuffer, label: &str, left: i32, top: i32, scale: i32, color: u32) {
    let width = label.len() as i32 * 6 * scale - scale;
    text(frame, label, left + width / 2, top, scale, color);
}

fn brand(frame: &mut Framebuffer, compact: bool) {
    if compact {
        left_text(frame, "STAR COMMAND", 82, 38, 6, 0xf0eee7);
        left_text(frame, "MANUAL DE VUELO", 84, 102, 3, 0xb6c7d8);
    } else {
        left_text(frame, "STAR", 82, 82, 12, 0xf0eee7);
        left_text(frame, "COMMAND", 82, 176, 12, 0xf0eee7);
        for x in 84..130 {
            frame.pixels[335 * WIDTH + x] = 0xe8e5dc;
        }
        left_text(frame, "AVENTURA ESPACIAL", 150, 326, 3, 0xf0eee7);
        left_text(frame, "AL INFINITO Y MAS ALLA", 150, 365, 2, 0x91a9bd);
    }
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
        draw_backdrop(frame);
        brand(frame, true);
        left_text(frame, "EXPLORA EL ESPACIO Y COMPLETA EL NIVEL SUBMARINO", 85, 155, 2, 0x91a9bd);
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
            "T: BUZZ / RAYO NAVE-CARRO",
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
            "BUZZ / RAYO: SUBMARINO AL ENTRAR AL AGUA",
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
        left_text(frame, "ENTER / ESC: VOLVER", 85, 736, 2, 0xffd447);
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
        draw_backdrop(frame);
        brand(frame, false);
        if paused {
            left_text(frame, "JUEGO EN PAUSA", 85, 405, 3, 0x91a9bd);
        }
        for (i, label) in options.iter().enumerate() {
            let y = (if paused { 460 } else { 455 }) + i as i32 * 58;
            let color = if i == selected { 0xf0eee7 } else { 0x91a9bd };
            if i == selected {
                for marker_y in y + 3..y + 27 {
                    for marker_x in 84..94 {
                        frame.pixels[marker_y as usize * WIDTH + marker_x] = 0xffc928;
                    }
                }
            }
            left_text(frame, label, 116, y, 3, color);
            if i == selected {
                let width = label.len() as i32 * 18 - 3;
                for x in 116..116 + width {
                    frame.pixels[(y as usize + 36) * WIDTH + x as usize] = 0xffc928;
                }
            }
        }
        left_text(frame, "UP DOWN / W S  ELEGIR     ENTER  CONFIRMAR", 84, 710, 1, 0xf0eee7);
        crate::input::clear();
        window.update_with_buffer(&frame.pixels, WIDTH, HEIGHT)?;
    }
    Ok(Action::Quit)
}
