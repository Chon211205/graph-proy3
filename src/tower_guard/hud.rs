use crate::{HEIGHT, WIDTH, draw_selection_label, framebuffer::Framebuffer};

use super::{BossKind, Phase, TowerGame, sprites::SpriteBank};

pub(super) fn draw_hud(game: &TowerGame, frame: &mut Framebuffer, sprites: &SpriteBank) {
    let center_x = (frame.width / 2) as i32;
    let scale = (2.0 * frame.width as f32 / WIDTH as f32)
        .clamp(1.0, 2.4)
        .round() as i32;
    let y_scale = frame.height as f32 / HEIGHT as f32;
    let y = |base: i32| (base as f32 * y_scale) as i32;

    // Results replace the whole play HUD so the title, score and actions read
    // as one dedicated screen at every window size.
    if matches!(game.phase, Phase::Victory | Phase::Defeat) {
        draw_result(game, frame, center_x, scale.max(1), sprites);
        return;
    }

    let injury = (5_u8.saturating_sub(game.defenses) as f32 / 5.0).powf(1.35) * 0.78;
    let impact = game.lost_defense_flash / super::DAMAGE_FLASH_SECONDS;
    if injury > 0.0 || impact > 0.0 {
        draw_damage_overlay(frame, injury, impact);
    }

    draw_selection_label(
        frame,
        "TORRE: ULTIMA GUARDIA",
        center_x,
        y(18),
        scale.max(1),
        0xffd78a,
    );
    let round = format!("RONDA {}/5", game.round);
    draw_selection_label(frame, &round, 80, y(18), scale.max(1), 0xf0e6cf);
    let defense = format!("DEFENSA {}/5", game.defenses);
    draw_selection_label(
        frame,
        &defense,
        (frame.width as i32) - 90,
        y(18),
        scale.max(1),
        0xff8873,
    );

    if let Some(boss) = &game.boss {
        let name = if boss.kind == BossKind::Hat {
            "JEFE DEL SOMBRERO"
        } else {
            "JEFE LANZACOHETES"
        };
        draw_selection_label(frame, name, center_x, y(52), scale.max(1), 0xffbd75);
        let bar_width = (frame.width as f32 * 0.34) as i32;
        let left = center_x - bar_width / 2;
        draw_bar(
            frame,
            left,
            y(77),
            bar_width,
            (10.0 * y_scale).max(5.0) as i32,
            boss.health as f32 / boss.max_health as f32,
            0xc84038,
        );
    } else {
        let group = format!(
            "GRUPO {:02} / {:02}",
            game.group_index + 1,
            super::director::group_count(game.round)
        );
        draw_selection_label(frame, &group, center_x, y(52), scale.max(1), 0xc3d0cc);
        if game.phase == Phase::Intro {
            draw_selection_label(
                frame,
                "CONTACTOS APROXIMANDOSE",
                center_x,
                y(119),
                scale.max(1),
                0xffc46a,
            );
        }
    }

    let ammo = if game.phase == Phase::Boss {
        format!("CARGADOR {} / 6", game.boss_ammo)
    } else {
        format!("DISPAROS {} / 3", game.normal_ammo)
    };
    draw_selection_label(frame, &ammo, 92, y(86), scale.max(1), 0xffd78a);
    let stats = format!(
        "PUNTOS {:06}   ELIMINADOS {}   ESCAPARON {}",
        game.stats.score, game.stats.kills, game.stats.escapes
    );
    draw_selection_label(frame, &stats, center_x, y(86), scale.max(1), 0xd2d9d4);

    if game.reload_timer > 0.0 {
        draw_selection_label(
            frame,
            "RECARGANDO",
            center_x,
            y(119),
            scale.max(1),
            0xffc46a,
        );
        let bar_width = (frame.width as f32 * 0.14) as i32;
        draw_bar(
            frame,
            center_x - bar_width / 2,
            y(143),
            bar_width,
            5,
            1.0 - game.reload_timer / super::boss::BOSS_RELOAD_SECONDS,
            0xffb248,
        );
    }
    if game.feedback_timer > 0.0 && !game.feedback.is_empty() {
        draw_selection_label(
            frame,
            &game.feedback,
            center_x,
            y(126),
            scale.max(1),
            if game.lost_defense_flash > 0.0 {
                0xff8b76
            } else {
                0xffd397
            },
        );
    }

    let controls = if game.boss.is_some() {
        "MOUSE: APUNTAR   CLIC / ESPACIO: DISPARAR   R: RECARGAR   P: PAUSA"
    } else {
        "MOUSE / FLECHAS: APUNTAR   CLIC / ESPACIO: DISPARAR   P: PAUSA"
    };
    draw_selection_label(
        frame,
        controls,
        center_x,
        y(HEIGHT as i32 - 27),
        scale.max(1),
        0xb8c9c9,
    );
    draw_reticle(
        frame,
        game.aim.0 * frame.width as f32,
        game.aim.1 * frame.height as f32,
        game.hit_marker_timer > 0.0,
        game.hit_marker_headshot,
        game.muzzle_timer > 0.0,
    );

    if game.lost_defense_flash > 0.0 {
        draw_selection_label(
            frame,
            "PUESTO IMPACTADO",
            center_x,
            y(HEIGHT as i32 / 2 - 35),
            scale.max(1),
            0xff6f5f,
        );
    }
}

fn draw_damage_overlay(frame: &mut Framebuffer, injury: f32, impact: f32) {
    let strength = injury.clamp(0.0, 0.78);
    let impact = impact.clamp(0.0, 1.0).powf(0.72);
    let w = frame.width as f32;
    let h = frame.height as f32;
    let stains = [
        (0.015, 0.17, 0.040, 0.085),
        (0.045, 0.39, 0.023, 0.052),
        (0.020, 0.70, 0.034, 0.075),
        (0.075, 0.94, 0.031, 0.048),
        (0.985, 0.23, 0.036, 0.074),
        (0.955, 0.53, 0.025, 0.059),
        (0.985, 0.82, 0.039, 0.080),
        (0.39, 0.015, 0.034, 0.054),
        (0.70, 0.020, 0.026, 0.046),
    ];
    let blood = (strength + impact * 0.88).clamp(0.0, 1.0);
    let stain_count = (2.0 + blood * (stains.len() as f32 - 2.0)).ceil() as usize;
    let size_scale = 0.8 + blood * 0.45;
    for (cx, cy, rx, ry) in stains.into_iter().take(stain_count) {
        draw_blood_stain(
            frame,
            cx * w,
            cy * h,
            rx * w * size_scale,
            ry * h * size_scale,
            blood,
        );
    }
}

fn draw_blood_stain(frame: &mut Framebuffer, cx: f32, cy: f32, rx: f32, ry: f32, strength: f32) {
    let left = (cx - rx).floor().max(0.0) as usize;
    let right = (cx + rx).ceil().min(frame.width as f32) as usize;
    let top = (cy - ry).floor().max(0.0) as usize;
    let bottom = (cy + ry).ceil().min(frame.height as f32) as usize;
    for y in top..bottom {
        for x in left..right {
            let dx = (x as f32 + 0.5 - cx) / rx;
            let dy = (y as f32 + 0.5 - cy) / ry;
            let angle = dy.atan2(dx);
            let rough_edge = 0.84 + (angle * 5.0).sin() * 0.08 + (angle * 9.0).cos() * 0.05;
            let radius = (dx * dx + dy * dy).sqrt();
            if radius > rough_edge {
                continue;
            }
            let alpha = strength * (0.14 + (1.0 - radius / rough_edge) * 0.68);
            let index = y * frame.width + x;
            frame.pixels[index] = tint_toward_blood(frame.pixels[index], alpha);
        }
    }
}

fn tint_toward_blood(color: u32, alpha: f32) -> u32 {
    let channel = |shift: u32, target: u32| {
        let base = ((color >> shift) & 0xff) as f32;
        let mixed = base + (target as f32 - base) * alpha.clamp(0.0, 1.0);
        mixed.round() as u32
    };
    (channel(16, 156) << 16) | (channel(8, 8) << 8) | channel(0, 20)
}

fn draw_result(
    game: &TowerGame,
    frame: &mut Framebuffer,
    center_x: i32,
    scale: i32,
    sprites: &SpriteBank,
) {
    let won = game.phase == Phase::Victory;
    let width = frame.width as f32;
    let height = frame.height as f32;
    let px = |ratio: f32| (ratio * width) as i32;
    let py = |ratio: f32| (ratio * height) as i32;
    let accent = if won { 0xffc56c } else { 0xd94d4d };

    fill(frame, 0, 0, frame.width as i32, frame.height as i32, 0x08090d);
    // Framed telemetry treatment, with restrained corner marks and a warm
    // status stripe to match the tower's industrial amber/red palette.
    draw_panel(frame, 0x0d1117, 0.035, 0.035, 0.965, 0.965);
    fill(frame, px(0.08), py(0.055), px(0.84), (height * 0.004).max(2.0) as i32, accent);
    draw_selection_label(
        frame,
        if won { "INFORME DE MISION  //  OBJETIVO COMPLETADO" } else { "INFORME DE MISION  //  DEFENSA CAIDA" },
        center_x,
        py(0.085),
        scale.max(1),
        0xd3b17c,
    );
    sprites.draw_result_title(
        frame,
        won,
        center_x as f32,
        height * 0.16,
        height * 0.14,
    );
    let card_top = py(0.405);
    let card_height = py(0.105);
    let card_width = width * 0.265;
    let gap = width * 0.025;
    let labels = ["PUNTUACION", "PRECISION", "ELIMINADOS", "TIROS A LA CABEZA", "DISPAROS", "ESCAPARON"];
    let values = [
        format!("{:06}", game.stats.score),
        format!("{}%", game.stats.accuracy()),
        game.stats.kills.to_string(),
        game.stats.headshots.to_string(),
        game.stats.shots.to_string(),
        game.stats.escapes.to_string(),
    ];
    let stat_scale = (scale as f32 * 0.82).round().clamp(1.0, 3.0) as i32;
    for index in 0..6 {
        let column = index % 3;
        let row = index / 3;
        let card_center = center_x as f32 + (column as f32 - 1.0) * (card_width + gap);
        let top = card_top + row as i32 * py(0.13);
        draw_panel(
            frame,
            0x15181d,
            (card_center - card_width / 2.0) / width,
            top as f32 / height,
            (card_center + card_width / 2.0) / width,
            (top + card_height) as f32 / height,
        );
        draw_selection_label(
            frame,
            labels[index],
            card_center as i32,
            top + py(0.018),
            stat_scale,
            0xd0a763,
        );
        draw_selection_label(
            frame,
            &values[index],
            card_center as i32,
            top + py(0.060),
            (stat_scale + 1).min(3),
            0xe6e8e4,
        );
    }
    let button_scale = (scale as f32 * 0.9).round().clamp(1.0, 3.0) as i32;
    draw_button(
        frame,
        center_x,
        py(0.755),
        "REINTENTAR  [ ENTER ]",
        0xffc56c,
        button_scale,
        accent,
    );
    draw_button(
        frame,
        center_x,
        py(0.865),
        "VOLVER AL ESPACIO  [ BACKSPACE ]",
        0xd6e0dd,
        button_scale,
        accent,
    );
}

fn draw_panel(frame: &mut Framebuffer, color: u32, left: f32, top: f32, right: f32, bottom: f32) {
    let l = (left * frame.width as f32) as usize;
    let t = (top * frame.height as f32) as usize;
    let r = (right * frame.width as f32) as usize;
    let b = (bottom * frame.height as f32) as usize;
    for y in t.min(frame.height)..b.min(frame.height) {
        for x in l.min(frame.width)..r.min(frame.width) {
            frame.pixels[y * frame.width + x] = color;
        }
    }
    border(frame, l as i32, t as i32, r as i32, b as i32, 0xb47b42);
}

fn draw_bar(
    frame: &mut Framebuffer,
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    ratio: f32,
    color: u32,
) {
    fill(frame, left, top, width, height, 0x26191a);
    fill(
        frame,
        left,
        top,
        (width as f32 * ratio.clamp(0.0, 1.0)) as i32,
        height,
        color,
    );
}

fn draw_button(
    frame: &mut Framebuffer,
    x: i32,
    y: i32,
    label: &str,
    color: u32,
    scale: i32,
    accent: u32,
) {
    let text_width = label.len() as i32 * 6 * scale - scale;
    let width = (text_width + scale * 12).min((frame.width as i32 * 3) / 4);
    let height = scale * 11 + 12;
    fill(frame, x - width / 2, y - height / 2, width, height, 0x20262a);
    fill(frame, x - width / 2 + 2, y - height / 2 + 2, 4, height - 4, accent);
    border(
        frame,
        x - width / 2,
        y - height / 2,
        x + width / 2,
        y + height / 2,
        accent,
    );
    draw_selection_label(frame, label, x, y - (scale * 3), scale, color);
}

fn draw_reticle(frame: &mut Framebuffer, x: f32, y: f32, hit: bool, headshot: bool, fired: bool) {
    let (x, y) = (x as i32, y as i32);
    let color = if hit {
        if headshot { 0xffd36e } else { 0xf1eee0 }
    } else {
        0xf0e6ce
    };
    let radius = if hit { 17 } else { 12 };
    line(frame, x - radius, y, x - 4, y, color);
    line(frame, x + 4, y, x + radius, y, color);
    line(frame, x, y - radius, x, y - 4, color);
    line(frame, x, y + 4, x, y + radius, color);
    for d in -2..=2 {
        put(frame, x + d, y - radius - 3, color);
        put(frame, x + d, y + radius + 3, color);
    }
    if fired {
        for dy in -2..=2 {
            for dx in -2..=2 {
                put(frame, x + dx, y + dy, 0xffd26c);
            }
        }
    }
}

fn fill(frame: &mut Framebuffer, left: i32, top: i32, width: i32, height: i32, color: u32) {
    for y in top.max(0)..(top + height).min(frame.height as i32) {
        for x in left.max(0)..(left + width).min(frame.width as i32) {
            frame.pixels[y as usize * frame.width + x as usize] = color;
        }
    }
}
fn border(frame: &mut Framebuffer, left: i32, top: i32, right: i32, bottom: i32, color: u32) {
    line(frame, left, top, right, top, color);
    line(frame, left, bottom, right, bottom, color);
    line(frame, left, top, left, bottom, color);
    line(frame, right, top, right, bottom, color);
}
fn line(frame: &mut Framebuffer, x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
    let steps = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
    for n in 0..=steps {
        let t = n as f32 / steps as f32;
        put(
            frame,
            (x0 as f32 + (x1 - x0) as f32 * t) as i32,
            (y0 as f32 + (y1 - y0) as f32 * t) as i32,
            color,
        );
    }
}
fn put(frame: &mut Framebuffer, x: i32, y: i32, color: u32) {
    if x >= 0 && y >= 0 && x < frame.width as i32 && y < frame.height as i32 {
        frame.pixels[y as usize * frame.width + x as usize] = color;
    }
}
