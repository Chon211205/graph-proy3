use crate::{draw_selection_label, framebuffer::Framebuffer, galaxy::Galaxy};
use std::{fs::File, io::BufReader};

const ROBOT_PATH: &str = "assets/ui/space_guide_robot.png";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Planet {
    GardenRust,
    PinkCore,
    AquaticRuins,
    Hourglass,
    Tower,
    LavaCastle,
    LavaSanctum,
}

#[derive(Clone, Copy)]
pub(crate) struct Briefing {
    pub name: &'static str,
    pub signal: &'static str,
    pub objective: &'static str,
    reward_bit: u8,
}

impl Planet {
    pub(crate) const fn briefing(self) -> Briefing {
        match self {
            Self::GardenRust => Briefing {
                name: "JARDIN RUST",
                signal: "La carrera oculta una esfera. Gana para recuperar su señal.",
                objective: "GANA EL GRAN PREMIO ORBITAL",
                reward_bit: 0b0000001,
            },
            Self::PinkCore => Briefing {
                name: "PLANETA ROSADO",
                signal: "El guardian protege una esfera y mantiene abierta la grieta.",
                objective: "DERROTA AL GUARDIAN DEL NUCLEO",
                reward_bit: 0b0000100,
            },
            Self::AquaticRuins => Briefing {
                name: "RUINAS ACUATICAS",
                signal: "Las ruinas registraron la primera onda del agujero negro.",
                objective: "DERROTA AL TIBURON Y RECUPERA LA ESFERA",
                reward_bit: 0b0001000,
            },
            Self::Hourglass => Briefing {
                name: "RELOJ DE ARENA",
                signal: "La inversion temporal amenaza las capsulas de rescate.",
                objective: "RESCATA LAS CAPSULAS Y FIJA LA SENAL",
                reward_bit: 0b0010000,
            },
            Self::Tower => Briefing {
                name: "CASA TORRE",
                signal: "La torre fue tomada. Dentro sigue activa otra resonancia.",
                objective: "SUPERA LA ULTIMA GUARDIA",
                reward_bit: 0b0000010,
            },
            Self::LavaCastle => Briefing {
                name: "CASTILLO DE LAVA",
                signal: "Nova se desgarra alrededor de esta fortaleza antigua.",
                objective: "VENCE AL GUARDIAN Y ABRE EL CASTILLO",
                reward_bit: 0b0100000,
            },
            Self::LavaSanctum => Briefing {
                name: "SANTUARIO DE NOVA",
                signal: "El sello interior concentra la ultima frecuencia de Nova.",
                objective: "REUNE LOS FRAGMENTOS Y ROMPE EL SELLO",
                reward_bit: 0b1000000,
            },
        }
    }
}

pub(crate) fn planet_for_orbit(
    galaxy: Galaxy,
    orbit_index: Option<usize>,
    race_locked: bool,
    pink_locked: bool,
) -> Option<Planet> {
    match galaxy {
        Galaxy::Home if race_locked => Some(Planet::GardenRust),
        Galaxy::Home if pink_locked => Some(Planet::PinkCore),
        Galaxy::Home => Some(Planet::AquaticRuins),
        Galaxy::Vesper => match orbit_index {
            Some(0) => Some(Planet::Hourglass),
            Some(1) => Some(Planet::Tower),
            _ => None,
        },
        Galaxy::Nova => Some(Planet::LavaCastle),
    }
}

pub(crate) fn collected(planet: Planet, mask: u8) -> bool {
    mask & planet.briefing().reward_bit != 0
}

pub(crate) fn sphere_count(mask: u8) -> u32 {
    (mask & 0b0111_1111).count_ones()
}

struct Sprite {
    width: usize,
    height: usize,
    rgba: Vec<[u8; 4]>,
}

pub(crate) struct Guide {
    robot: Option<Sprite>,
}

impl Guide {
    pub(crate) fn load() -> Self {
        Self {
            robot: load_sprite(ROBOT_PATH),
        }
    }

    pub(crate) fn draw(&self, frame: &mut Framebuffer, planet: Planet, collected_mask: u8) {
        let phase_planet = if planet == Planet::LavaCastle && collected(planet, collected_mask) {
            Planet::LavaSanctum
        } else {
            planet
        };
        let briefing = phase_planet.briefing();
        let count = sphere_count(collected_mask);
        let width = frame.width;
        let height = frame.height;
        let scale = ((height as f32 / 800.0).round() as i32).clamp(1, 2);
        let left = (width as f32 * 0.018) as usize;
        let top = (height as f32 * 0.755) as usize;
        let panel_w = (width as f32 * 0.37) as usize;
        let panel_h = (height as f32 * 0.225) as usize;

        panel(frame, left, top, panel_w, panel_h);
        if let Some(robot) = &self.robot {
            draw_sprite(
                robot,
                frame,
                left + 10,
                top + panel_h.saturating_sub(98) / 2,
                72,
                86,
            );
        } else {
            robot_fallback(frame, left + 18, top + 34, 55);
        }

        let text_x = left + (panel_w * 25 / 100);
        let text_center = left + panel_w * 63 / 100;
        draw_selection_label(
            frame,
            "NEXO // GUIA DE VUELO",
            text_center as i32,
            (top + 15) as i32,
            scale,
            0x68e8ff,
        );
        draw_selection_label(
            frame,
            briefing.name,
            text_center as i32,
            (top + 42) as i32,
            scale + 1,
            0xffd27b,
        );

        let completed = collected(phase_planet, collected_mask);
        let line = if completed {
            "Frecuencia asegurada. La red responde con mas fuerza."
        } else {
            briefing.signal
        };
        draw_wrapped(
            frame,
            line,
            text_x + 4,
            top + 73,
            (panel_w * 70 / 100) as i32,
            scale,
            0xd7efff,
        );

        let status = if completed {
            "ESFERA RECUPERADA"
        } else {
            briefing.objective
        };
        draw_selection_label(
            frame,
            status,
            text_center as i32,
            (top + panel_h - 42) as i32,
            scale,
            if completed { 0x7dffc4 } else { 0x9deaff },
        );
        let chapter = if count == 7 {
            "NEXO COMPLETO // VUELVE A VESPER"
        } else {
            "FRECUENCIAS DEL NEXO"
        };
        let footer = format!("{chapter} // {count}/7");
        draw_selection_label(
            frame,
            &footer,
            text_center as i32,
            (top + panel_h - 19) as i32,
            scale,
            0xb6cce0,
        );
    }
}

fn load_sprite(path: &str) -> Option<Sprite> {
    let file = File::open(path).ok()?;
    let mut decoder = png::Decoder::new(BufReader::new(file));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let mut data = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut data).ok()?;
    let channels = match info.color_type {
        png::ColorType::Rgba => 4,
        png::ColorType::Rgb => 3,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Grayscale => 1,
        png::ColorType::Indexed => return None,
    };
    let pixels = data[..info.buffer_size()]
        .chunks_exact(channels)
        .map(|p| match info.color_type {
            png::ColorType::Rgba => [p[0], p[1], p[2], p[3]],
            png::ColorType::Rgb => [p[0], p[1], p[2], 255],
            png::ColorType::GrayscaleAlpha => [p[0], p[0], p[0], p[1]],
            png::ColorType::Grayscale => [p[0], p[0], p[0], 255],
            png::ColorType::Indexed => unreachable!(),
        })
        .collect();
    Some(Sprite {
        width: info.width as usize,
        height: info.height as usize,
        rgba: pixels,
    })
}

fn panel(frame: &mut Framebuffer, left: usize, top: usize, width: usize, height: usize) {
    let right = left.saturating_add(width).min(frame.width);
    let bottom = top.saturating_add(height).min(frame.height);
    for y in top..bottom {
        for x in left..right {
            let edge = x < left + 2 || x + 2 >= right || y < top + 2 || y + 2 >= bottom;
            let i = y * frame.width + x;
            frame.pixels[i] = if edge {
                0x46e6ff
            } else {
                blend(frame.pixels[i], 0x06172c, 225)
            };
        }
    }
}

fn draw_sprite(sprite: &Sprite, frame: &mut Framebuffer, x: usize, y: usize, w: usize, h: usize) {
    if sprite.width == 0 || sprite.height == 0 || w == 0 || h == 0 {
        return;
    }
    for dy in 0..h {
        let py = y + dy;
        if py >= frame.height {
            break;
        }
        let sy = (dy * sprite.height / h).min(sprite.height - 1);
        for dx in 0..w {
            let px = x + dx;
            if px >= frame.width {
                break;
            }
            let sx = (dx * sprite.width / w).min(sprite.width - 1);
            let [r, g, b, a] = sprite.rgba[sy * sprite.width + sx];
            if a == 0 {
                continue;
            }
            let i = py * frame.width + px;
            frame.pixels[i] = blend(
                frame.pixels[i],
                ((r as u32) << 16) | ((g as u32) << 8) | b as u32,
                a,
            );
        }
    }
}

fn blend(under: u32, over: u32, alpha: u8) -> u32 {
    let a = alpha as u32;
    let inv = 255 - a;
    let r = ((((under >> 16) & 255) * inv + ((over >> 16) & 255) * a) / 255) & 255;
    let g = ((((under >> 8) & 255) * inv + ((over >> 8) & 255) * a) / 255) & 255;
    let b = (((under & 255) * inv + (over & 255) * a) / 255) & 255;
    (r << 16) | (g << 8) | b
}

fn draw_wrapped(
    frame: &mut Framebuffer,
    text: &str,
    x: usize,
    y: usize,
    max_width: i32,
    scale: i32,
    color: u32,
) {
    let text = text.to_ascii_uppercase();
    let max_chars = (max_width / (6 * scale.max(1))).max(1) as usize;
    let mut line = String::new();
    let mut row = 0;
    for word in text.split_whitespace() {
        if !line.is_empty() && line.len() + 1 + word.len() > max_chars {
            draw_selection_label(
                frame,
                &line,
                (x + max_width as usize / 2) as i32,
                (y + row * 20 * scale as usize) as i32,
                scale,
                color,
            );
            line.clear();
            row += 1;
            if row >= 2 {
                break;
            }
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() && row < 2 {
        draw_selection_label(
            frame,
            &line,
            (x + max_width as usize / 2) as i32,
            (y + row * 20 * scale as usize) as i32,
            scale,
            color,
        );
    }
}

fn robot_fallback(frame: &mut Framebuffer, x: usize, y: usize, size: usize) {
    for dy in 0..size {
        for dx in 0..size {
            if dx < 5 || dx + 5 >= size || dy < 4 || dy + 4 >= size {
                continue;
            }
            let i = (y + dy).min(frame.height - 1) * frame.width + (x + dx).min(frame.width - 1);
            frame.pixels[i] = if dy < size / 2 { 0xbbefff } else { 0x38dff4 };
        }
    }
}
