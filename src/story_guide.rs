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
                name: "JARDINES DE OXIDO",
                signal: "La carrera oculta una esfera. Gana para recuperar su señal.",
                objective: "GANA EL GRAN PREMIO ORBITAL",
                reward_bit: 0b0000001,
            },
            Self::PinkCore => Briefing {
                name: "NOVA ROSA",
                signal: "El guardian protege una esfera y mantiene abierta la grieta.",
                objective: "DERROTA AL GUARDIAN DEL NUCLEO",
                reward_bit: 0b0000100,
            },
            Self::AquaticRuins => Briefing {
                name: "ABISMO AZUL",
                signal: "Las ruinas registraron la primera onda del agujero negro.",
                objective:
                    "SOBREVIVE A LA PERSECUCION Y DERROTA AL MEGALODON PARA RECUPERAR LA ESFERA",
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

struct Sprite {
    width: usize,
    rgba: Vec<[u8; 4]>,
    visible: (usize, usize, usize, usize),
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
        let width = frame.width;
        let height = frame.height;
        let scale = 1;
        let panel_w = (width as f32 * 0.38)
            .min(500.0)
            .min(width.saturating_sub(36) as f32) as usize;
        let left = ((width as f32 * 0.018) as usize).min(width.saturating_sub(panel_w));
        let robot_column_w = panel_w / 4;
        let text_x = left + robot_column_w + 14;
        let text_w = left + panel_w - text_x - 14;
        let completed = collected(phase_planet, collected_mask);
        let description = if completed {
            "Frecuencia asegurada. La red responde con mas fuerza."
        } else {
            briefing.signal
        };
        let objective = if completed {
            "ESFERA RECUPERADA"
        } else {
            briefing.objective
        };
        let description_lines = wrap_text(description, text_w, scale);
        let objective_lines = wrap_text(objective, text_w, scale);

        // Build the vertical layout from measured text blocks, then place the
        // separated control strip after the mission objective.
        let mut cursor = 11usize;
        let header_y = cursor;
        cursor += 7 + 5;
        let title_y = cursor;
        let title_scale = 2;
        cursor += 7 * title_scale as usize + 6;
        let description_y = cursor;
        cursor += description_lines.len() * 11 + 7;
        let separator_y = cursor;
        cursor += 1 + 6;
        let mission_y = cursor;
        cursor += 7 + 4;
        let objective_y = cursor;
        cursor += objective_lines.len() * 11 + 8;
        let controls_y = cursor;
        const CONTROLS_H: usize = 30;
        let panel_h = controls_y + CONTROLS_H + 1;
        let bottom_margin = (height as f32 * 0.025).max(18.0) as usize;
        let top = height.saturating_sub(panel_h + bottom_margin);

        panel(frame, left, top, panel_w, panel_h, controls_y);
        let divider_x = left + robot_column_w;
        vertical_rule(frame, divider_x, top + 10, top + controls_y - 1, 0x31536c);

        let robot_box_w = robot_column_w.saturating_sub(22);
        let robot_box_h = controls_y.saturating_sub(18);
        if let Some(robot) = &self.robot {
            draw_sprite_fit(robot, frame, left + 11, top + 8, robot_box_w, robot_box_h);
        } else {
            let size = robot_box_w.min(robot_box_h).min(68);
            robot_fallback(
                frame,
                left + (robot_column_w.saturating_sub(size)) / 2,
                top + (controls_y.saturating_sub(size)) / 2,
                size,
            );
        }

        draw_left(
            frame,
            "NEXO // TRANSMISION RECIBIDA",
            text_x,
            top + header_y,
            1,
            0x68e8ff,
        );
        draw_left(
            frame,
            briefing.name,
            text_x,
            top + title_y,
            title_scale,
            0xf4efdf,
        );
        draw_lines_left(
            frame,
            &description_lines,
            text_x,
            top + description_y,
            1,
            11,
            0xd7e5ed,
        );
        horizontal_rule(frame, text_x, top + separator_y, text_w, 0x31536c);
        draw_left(frame, "MISION", text_x, top + mission_y, 1, 0xffbd62);
        draw_lines_left(
            frame,
            &objective_lines,
            text_x,
            top + objective_y,
            1,
            11,
            if completed { 0x7dffc4 } else { 0xe6edf0 },
        );

        draw_controls(frame, left, top + controls_y, panel_w, CONTROLS_H);
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
    let pixels: Vec<[u8; 4]> = data[..info.buffer_size()]
        .chunks_exact(channels)
        .map(|p| match info.color_type {
            png::ColorType::Rgba => [p[0], p[1], p[2], p[3]],
            png::ColorType::Rgb => [p[0], p[1], p[2], 255],
            png::ColorType::GrayscaleAlpha => [p[0], p[0], p[0], p[1]],
            png::ColorType::Grayscale => [p[0], p[0], p[0], 255],
            png::ColorType::Indexed => unreachable!(),
        })
        .collect();
    let mut min_x = info.width as usize;
    let mut min_y = info.height as usize;
    let mut max_x = 0usize;
    let mut max_y = 0usize;
    for y in 0..info.height as usize {
        for x in 0..info.width as usize {
            if pixels[y * info.width as usize + x][3] > 8 {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x + 1);
                max_y = max_y.max(y + 1);
            }
        }
    }
    let visible = if min_x < max_x && min_y < max_y {
        (min_x, min_y, max_x, max_y)
    } else {
        (0, 0, info.width as usize, info.height as usize)
    };
    Some(Sprite {
        width: info.width as usize,
        rgba: pixels,
        visible,
    })
}

fn panel(
    frame: &mut Framebuffer,
    left: usize,
    top: usize,
    width: usize,
    height: usize,
    controls_y: usize,
) {
    let right = left.saturating_add(width).min(frame.width);
    let bottom = top.saturating_add(height).min(frame.height);
    for y in top..bottom {
        for x in left..right {
            let edge = x < left + 1 || x + 1 >= right || y < top + 1 || y + 1 >= bottom;
            let i = y * frame.width + x;
            frame.pixels[i] = if edge { 0x49677f } else { 0x061321 };
        }
    }
    horizontal_rule(
        frame,
        left + 1,
        top + 1,
        width.saturating_sub(2).min(88),
        0x46e6ff,
    );
    let controls_top = top.saturating_add(controls_y).min(bottom);
    for y in controls_top..bottom.saturating_sub(1) {
        for x in left.saturating_add(1)..right.saturating_sub(1) {
            frame.pixels[y * frame.width + x] = 0x0a1b2a;
        }
    }
    horizontal_rule(
        frame,
        left + 1,
        controls_top,
        width.saturating_sub(2),
        0x31536c,
    );
}

fn vertical_rule(frame: &mut Framebuffer, x: usize, top: usize, bottom: usize, color: u32) {
    if x >= frame.width {
        return;
    }
    for y in top.min(frame.height)..bottom.min(frame.height) {
        frame.pixels[y * frame.width + x] = color;
    }
}

fn horizontal_rule(frame: &mut Framebuffer, x: usize, y: usize, width: usize, color: u32) {
    if y >= frame.height || x >= frame.width {
        return;
    }
    let end = x.saturating_add(width).min(frame.width);
    for px in x..end {
        frame.pixels[y * frame.width + px] = color;
    }
}

fn draw_sprite_fit(
    sprite: &Sprite,
    frame: &mut Framebuffer,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
) {
    let (min_x, min_y, max_x, max_y) = sprite.visible;
    let source_w = max_x.saturating_sub(min_x);
    let source_h = max_y.saturating_sub(min_y);
    if source_w == 0 || source_h == 0 || w == 0 || h == 0 {
        return;
    }
    let fit = (w as f32 / source_w as f32).min(h as f32 / source_h as f32);
    let draw_w = ((source_w as f32 * fit).round() as usize).max(1).min(w);
    let draw_h = ((source_h as f32 * fit).round() as usize).max(1).min(h);
    let draw_x = x + (w - draw_w) / 2;
    let draw_y = y + (h - draw_h) / 2;
    for dy in 0..draw_h {
        let py = draw_y + dy;
        if py >= frame.height {
            break;
        }
        let sy = min_y + (dy * source_h / draw_h).min(source_h - 1);
        for dx in 0..draw_w {
            let px = draw_x + dx;
            if px >= frame.width {
                break;
            }
            let sx = min_x + (dx * source_w / draw_w).min(source_w - 1);
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

fn measure_text(text: &str, scale: i32) -> usize {
    if text.is_empty() {
        0
    } else {
        (text.chars().count() * 6 - 1) * scale.max(1) as usize
    }
}

fn draw_left(frame: &mut Framebuffer, text: &str, x: usize, y: usize, scale: i32, color: u32) {
    let width = measure_text(text, scale);
    draw_selection_label(frame, text, (x + width / 2) as i32, y as i32, scale, color);
}

fn draw_lines_left(
    frame: &mut Framebuffer,
    lines: &[String],
    x: usize,
    y: usize,
    scale: i32,
    line_height: usize,
    color: u32,
) {
    for (index, line) in lines.iter().enumerate() {
        draw_left(frame, line, x, y + index * line_height, scale, color);
    }
}

/// Word wrapping measured in the same 6-pixel glyph advance used by the HUD
/// font. A small raggedness cost balances lines and discourages orphan words.
fn wrap_text(text: &str, max_width: usize, scale: i32) -> Vec<String> {
    let normalized = text.to_ascii_uppercase();
    let words: Vec<&str> = normalized.split_whitespace().collect();
    if words.is_empty() {
        return Vec::new();
    }
    let max_width = max_width.max(1);
    let n = words.len();
    let mut costs = vec![u64::MAX; n + 1];
    let mut next_word = vec![n; n];
    costs[n] = 0;
    for start in (0..n).rev() {
        let mut line = String::new();
        for end in start..n {
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(words[end]);
            let measured = measure_text(&line, scale);
            if measured > max_width && end > start {
                break;
            }
            let slack = max_width.saturating_sub(measured) as u64;
            let is_single_word_orphan = end == start && (start > 0 || end + 1 < n);
            let orphan_penalty = if is_single_word_orphan { 220_000 } else { 0 };
            let line_cost = 1_000_000 + slack * slack + orphan_penalty;
            let candidate = line_cost.saturating_add(costs[end + 1]);
            if candidate < costs[start] {
                costs[start] = candidate;
                next_word[start] = end + 1;
            }
        }
    }
    let mut lines = Vec::new();
    let mut start = 0;
    while start < n {
        let end = next_word[start].max(start + 1).min(n);
        lines.push(words[start..end].join(" "));
        start = end;
    }
    lines
}

fn blend(under: u32, over: u32, alpha: u8) -> u32 {
    let a = alpha as u32;
    let inv = 255 - a;
    let r = ((((under >> 16) & 255) * inv + ((over >> 16) & 255) * a) / 255) & 255;
    let g = ((((under >> 8) & 255) * inv + ((over >> 8) & 255) * a) / 255) & 255;
    let b = (((under & 255) * inv + (over & 255) * a) / 255) & 255;
    (r << 16) | (g << 8) | b
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

fn draw_controls(frame: &mut Framebuffer, left: usize, top: usize, panel_w: usize, height: usize) {
    let key_x = left + 12;
    let key_w = 43usize;
    let key_h = 17usize.min(height.saturating_sub(8));
    let key_y = top + (height.saturating_sub(key_h)) / 2;
    fill_rect(frame, key_x, key_y, key_w, key_h, 0xffbd62);
    draw_left(
        frame,
        "ENTER",
        key_x + 7,
        key_y + (key_h.saturating_sub(7)) / 2,
        1,
        0x1a1b22,
    );
    draw_left(
        frame,
        "INICIAR MISION",
        key_x + key_w + 9,
        key_y + (key_h.saturating_sub(7)) / 2,
        1,
        0xf4efdf,
    );

    let secondary = "BACKSPACE · VOLVER";
    let secondary_w = measure_text(secondary, 1);
    let right = left + panel_w - 13;
    draw_left(
        frame,
        secondary,
        right.saturating_sub(secondary_w),
        key_y + (key_h.saturating_sub(7)) / 2,
        1,
        0x9aabba,
    );
}

fn fill_rect(frame: &mut Framebuffer, x: usize, y: usize, width: usize, height: usize, color: u32) {
    let right = x.saturating_add(width).min(frame.width);
    let bottom = y.saturating_add(height).min(frame.height);
    for py in y.min(frame.height)..bottom {
        for px in x.min(frame.width)..right {
            frame.pixels[py * frame.width + px] = color;
        }
    }
}
