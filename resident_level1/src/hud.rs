use crate::inventory::Inventory;
use raylib::prelude::*;

pub fn render_hud(
    dibujo: &mut RaylibDrawHandle,
    vida_jugador: i32,
    balas_cargador: i32,
    balas_reserva: i32,
    municion_lanzallamas: i32,
    inventory: &Inventory,
    mensaje: &str,
    offset_x: f32,
    offset_y: f32,
    ancho_render: f32,
    alto_render: f32,
    escala: f32,
) {
    let escala_ui =
        escala.clamp(
            1.0,
            1.25,
        );

    let margen =
        16.0 * escala_ui;

    let tamano_texto =
        (
            18.0
                * escala_ui
        ) as i32;

    let tamano_mensaje =
        (
            16.0
                * escala_ui
        ) as i32;

    let tamano_fps =
        (
            18.0
                * escala_ui
        ) as i32;

    let x_izquierda =
        offset_x
            + margen;

    let y_inferior =
        offset_y
            + alto_render
            - margen
            - tamano_texto
                as f32;

    let texto_vida =
        format!(
            "Vida: {}",
            vida_jugador,
        );

    dibujo.draw_text(
        &texto_vida,
        x_izquierda
            as i32,
        y_inferior
            as i32,
        tamano_texto,
        if vida_jugador > 50 {
            Color::GREEN
        } else if vida_jugador > 25 {
            Color::YELLOW
        } else {
            Color::RED
        },
    );

    let texto_municion =
        format!(
            "Municion: {}/{}",
            balas_cargador,
            balas_reserva,
        );

    let ancho_vida =
        dibujo.measure_text(
            &texto_vida,
            tamano_texto,
        );

    let x_municion =
        x_izquierda
            + ancho_vida
                as f32
            + 35.0
                * escala_ui;

    dibujo.draw_text(
        &texto_municion,
        x_municion
            as i32,
        y_inferior
            as i32,
        tamano_texto,
        Color::WHITE,
    );

    let texto_combustible =
        format!("LLAMA: {}", municion_lanzallamas);

    dibujo.draw_text(
        &texto_combustible,
        x_municion as i32,
        (y_inferior - 28.0 * escala_ui) as i32,
        tamano_texto,
        Color::ORANGE,
    );

    if inventory.tiene_llave() {
        let ancho_municion =
            dibujo.measure_text(
                &texto_municion,
                tamano_texto,
            );

        let x_llave =
            x_municion
                + ancho_municion
                    as f32
                + 35.0
                    * escala_ui;

        dibujo.draw_text(
            "Llave",
            x_llave
                as i32,
            y_inferior
                as i32,
            tamano_texto,
            Color::YELLOW,
        );
    }

    let fps =
        dibujo.get_fps();

    let texto_fps =
        format!(
            "FPS: {}",
            fps,
        );

    let ancho_fps =
        dibujo.measure_text(
            &texto_fps,
            tamano_fps,
        );

    dibujo.draw_text(
        &texto_fps,
        (
            offset_x
                + ancho_render
                - margen
                - ancho_fps
                    as f32
        ) as i32,
        (
            offset_y
                + margen
        ) as i32,
        tamano_fps,
        Color::GREEN,
    );

    if !mensaje.is_empty() {
        let ancho_mensaje =
            dibujo.measure_text(
                mensaje,
                tamano_mensaje,
            );

        let x_mensaje =
            offset_x
                + ancho_render
                    / 2.0
                - ancho_mensaje
                    as f32
                    / 2.0;

        let y_mensaje =
            offset_y
                + margen;

        dibujo.draw_text(
            mensaje,
            x_mensaje
                as i32,
            y_mensaje
                as i32,
            tamano_mensaje,
            Color::WHITE,
        );
    }
}
