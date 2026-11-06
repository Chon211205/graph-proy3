use crate::{HEIGHT, WIDTH, draw_selection_label, framebuffer::Framebuffer};

use super::{BossKind, Phase, TowerGame};

pub(super) fn draw_hud(game: &TowerGame, frame: &mut Framebuffer) {
    let center_x = (frame.width / 2) as i32;
    let scale = (2.0 * frame.width as f32 / WIDTH as f32)
        .clamp(1.0, 2.4)
        .round() as i32;
    let y_scale = frame.height as f32 / HEIGHT as f32;
    let y = |base: i32| (base as f32 * y_scale) as i32;

    if game.lost_defense_flash > 0.0 {
        draw_damage_overlay(frame, game.lost_defense_flash / super::DAMAGE_FLASH_SECONDS);
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

    if matches!(game.phase, Phase::Victory | Phase::Defeat) {
        draw_result(game, frame, center_x, y_scale, scale.max(1));
    }
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

fn draw_damage_overlay(frame: &mut Framebuffer, remaining: f32) {
    let strength = remaining.clamp(0.0, 1.0).powf(0.72);
    let band_x = (frame.width as f32 * 0.20).max(1.0);
    let band_y = (frame.height as f32 * 0.18).max(1.0);
    for y in 0..frame.height {
        let edge_y = y.min(frame.height.saturating_sub(1) - y) as f32;
        for x in 0..frame.width {
            let edge_x = x.min(frame.width.saturating_sub(1) - x) as f32;
            let distance = (edge_x / band_x).min(edge_y / band_y).clamp(0.0, 1.0);
            let falloff = 1.0 - distance;
            if falloff <= 0.0 {
                continue;
            }
            let alpha = strength * falloff * falloff * 0.66;
            let index = y * frame.width + x;
            frame.pixels[index] = tint_toward_blood(frame.pixels[index], alpha);
        }
    }

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
    for (cx, cy, rx, ry) in stains {
        draw_blood_stain(frame, cx * w, cy * h, rx * w, ry * h, strength);
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
            let alpha = strength * (0.20 + (1.0 - radius / rough_edge) * 0.55);
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

fn draw_result(game: &TowerGame, frame: &mut Framebuffer, center_x: i32, y_scale: f32, scale: i32) {
    let title = if game.phase == Phase::Victory {
        "VICTORIA - TORRE DEFENDIDA"
    } else {
        "DERROTA - DEFENSA AGOTADA"
    };
    let y = |base: i32| (base as f32 * y_scale) as i32;
    draw_panel(frame, 0x090e12, 0.18, 0.30, 0.82, 0.88);
    draw_selection_label(
        frame,
        title,
        center_x,
        y(HEIGHT as i32 * 34 / 100),
        scale.max(1),
        if game.phase == Phase::Victory {
            0xffd17b
        } else {
            0xff8173
        },
    );
    let accuracy = format!(
        "PUNTUACION {}   PRECISION {}%",
        game.stats.score,
        game.stats.accuracy()
    );
    draw_selection_label(
        frame,
        &accuracy,
        center_x,
        y(HEIGHT as i32 * 43 / 100),
        scale.max(1),
        0xf1e8d9,
    );
    let totals = format!(
        "ELIMINADOS {}   TIROS A LA CABEZA {}   DISPAROS {}",
        game.stats.kills, game.stats.headshots, game.stats.shots
    );
    draw_selection_label(
        frame,
        &totals,
        center_x,
        y(HEIGHT as i32 * 51 / 100),
        scale.max(1),
        0xc6d4d2,
    );
    draw_button(
        frame,
        center_x,
        y(HEIGHT as i32 * 73 / 100),
        "REINTENTAR  ENTER",
        0xffba58,
    );
    draw_button(
        frame,
        center_x,
        y(HEIGHT as i32 * 83 / 100),
        "VOLVER AL ESPACIO  BACKSPACE",
        0xd0d9d5,
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

fn draw_button(frame: &mut Framebuffer, x: i32, y: i32, label: &str, color: u32) {
    let width = (label.len() as i32 * 12 + 40).min((frame.width as i32 * 3) / 4);
    fill(frame, x - width / 2, y - 12, width, 38, 0x20262a);
    border(
        frame,
        x - width / 2,
        y - 12,
        x + width / 2,
        y + 26,
        0xb47b42,
    );
    draw_selection_label(frame, label, x, y - 5, 2, color);
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
