use crate::camera::Camera;
use crate::map::{
    Map,
    TAMANO_CELDA,
};
use crate::player::Player;
use crate::raycaster::{
    lanzar_rayo,
    ALTO_VENTANA,
    ANCHO_VENTANA,
};

use raylib::prelude::*;
use std::f32::consts::PI;

const FOV: f32 =
    PI / 3.0;

fn normalizar_angulo(
    mut angulo: f32,
) -> f32 {
    while angulo > PI {
        angulo -=
            2.0 * PI;
    }

    while angulo < -PI {
        angulo +=
            2.0 * PI;
    }

    angulo
}

pub fn render_key_sprite(
    dibujo: &mut RaylibDrawHandle,
    mapa: &Map,
    player: &Player,
    camera: &Camera,
    textura: &Texture2D,
    offset_x: f32,
    offset_y: f32,
    escala: f32,
) {
    for fila in 0..mapa.alto() {
        for columna in 0..mapa.ancho() {
            if mapa.celda(
                fila as i32,
                columna as i32,
            ) != 'K'
            {
                continue;
            }

            render_objeto(
                dibujo,
                mapa,
                player,
                camera,
                fila,
                columna,
                textura,
                0.30,
                offset_x,
                offset_y,
                escala,
            );
        }
    }
}

pub fn render_ammo_sprites(
    dibujo: &mut RaylibDrawHandle,
    mapa: &Map,
    player: &Player,
    camera: &Camera,
    textura: &Texture2D,
    offset_x: f32,
    offset_y: f32,
    escala: f32,
) {
    for fila in 0..mapa.alto() {
        for columna in 0..mapa.ancho() {
            if mapa.celda(
                fila as i32,
                columna as i32,
            ) != 'A'
            {
                continue;
            }

            render_objeto(
                dibujo,
                mapa,
                player,
                camera,
                fila,
                columna,
                textura,
                0.38,
                offset_x,
                offset_y,
                escala,
            );
        }
    }
}

pub fn render_heal_sprites(
    dibujo: &mut RaylibDrawHandle,
    mapa: &Map,
    player: &Player,
    camera: &Camera,
    textura: &Texture2D,
    offset_x: f32,
    offset_y: f32,
    escala: f32,
) {
    for fila in 0..mapa.alto() {
        for columna in 0..mapa.ancho() {
            if mapa.celda(
                fila as i32,
                columna as i32,
            ) != 'H'
            {
                continue;
            }

            render_objeto(
                dibujo,
                mapa,
                player,
                camera,
                fila,
                columna,
                textura,
                0.32,
                offset_x,
                offset_y,
                escala,
            );
        }
    }
}

fn render_objeto(
    dibujo: &mut RaylibDrawHandle,
    mapa: &Map,
    player: &Player,
    camera: &Camera,
    fila: usize,
    columna: usize,
    textura: &Texture2D,
    factor_tamano: f32,
    offset_x: f32,
    offset_y: f32,
    escala: f32,
) {
    let objeto_x =
        columna as f32
            * TAMANO_CELDA
            + TAMANO_CELDA
                / 2.0;

    let objeto_y =
        fila as f32
            * TAMANO_CELDA
            + TAMANO_CELDA
                / 2.0;

    let dx =
        objeto_x
            - player.x;

    let dy =
        objeto_y
            - player.y;

    let distancia =
        (
            dx * dx
                + dy * dy
        )
            .sqrt();

    if distancia <= 1.0 {
        return;
    }

    let angulo_objeto =
        dy.atan2(
            dx,
        );

    let diferencia =
        normalizar_angulo(
            angulo_objeto
                - camera.angle,
        );

    if diferencia.abs()
        > FOV / 2.0
            + 0.20
    {
        return;
    }

    let hit_pared =
        lanzar_rayo(
            mapa,
            player.x,
            player.y,
            angulo_objeto,
        );

    if hit_pared.distancia
        < distancia - 3.0
    {
        return;
    }

    let distancia_corregida =
        (
            distancia
                * diferencia.cos()
        )
            .max(
                1.0,
            );

    let plano_proyeccion =
        (
            ANCHO_VENTANA as f32
                / 2.0
        )
            / (
                FOV / 2.0
            )
                .tan();

    let screen_x =
        ANCHO_VENTANA as f32
            / 2.0
            + diferencia.tan()
                * plano_proyeccion;

    let altura_mundo =
        TAMANO_CELDA
            * factor_tamano;

    let altura_sprite =
        altura_mundo
            / distancia_corregida
            * plano_proyeccion;

    if altura_sprite <= 1.0 {
        return;
    }

    let proporcion =
        textura.width()
            as f32
            / textura.height()
                as f32;

    let ancho_sprite =
        altura_sprite
            * proporcion;

    let centro_y =
        ALTO_VENTANA as f32
            / 2.0
            + camera.vertical_offset
                as f32;

    let suelo =
        centro_y
            + (
                TAMANO_CELDA
                    / 2.0
            )
                / distancia_corregida
                * plano_proyeccion;

    let x =
        offset_x
            + (
                screen_x
                    - ancho_sprite
                        / 2.0
            )
                * escala;

    let y =
        offset_y
            + (
                suelo
                    - altura_sprite
            )
                * escala;

    let ancho_final =
        ancho_sprite
            * escala;

    let alto_final =
        altura_sprite
            * escala;

    let limite_izquierdo =
        offset_x;

    let limite_derecho =
        offset_x
            + ANCHO_VENTANA as f32
                * escala;

    let limite_superior =
        offset_y;

    let limite_inferior =
        offset_y
            + ALTO_VENTANA as f32
                * escala;

    if x + ancho_final
        <= limite_izquierdo
    {
        return;
    }

    if x
        >= limite_derecho
    {
        return;
    }

    if y + alto_final
        <= limite_superior
    {
        return;
    }

    if y
        >= limite_inferior
    {
        return;
    }

    dibujo.draw_texture_pro(
        textura,
        Rectangle::new(
            0.0,
            0.0,
            textura.width()
                as f32,
            textura.height()
                as f32,
        ),
        Rectangle::new(
            x,
            y,
            ancho_final,
            alto_final,
        ),
        Vector2::new(
            0.0,
            0.0,
        ),
        0.0,
        Color::WHITE,
    );
}

pub fn render_flamethrow_ammo_sprites(
    dibujo: &mut RaylibDrawHandle,
    mapa: &Map,
    player: &Player,
    camera: &Camera,
    textura: &Texture2D,
    offset_x: f32,
    offset_y: f32,
    escala: f32,
) {
    for fila in 0..mapa.alto() {
        for columna in 0..mapa.ancho() {
            if mapa.celda(fila as i32, columna as i32) == 'Q' {
                render_objeto(
                    dibujo,
                    mapa,
                    player,
                    camera,
                    fila,
                    columna,
                    textura,
                    0.38,
                    offset_x,
                    offset_y,
                    escala,
                );
            }
        }
    }
}

pub fn render_antivirus_sprite(
    dibujo: &mut RaylibDrawHandle,
    mapa: &Map,
    player: &Player,
    camera: &Camera,
    textura: &Texture2D,
    offset_x: f32,
    offset_y: f32,
    escala: f32,
) {
    for fila in 0..mapa.alto() {
        for columna in 0..mapa.ancho() {
            if mapa.celda(fila as i32, columna as i32) == 'V' {
                render_objeto(
                    dibujo,
                    mapa,
                    player,
                    camera,
                    fila,
                    columna,
                    textura,
                    0.36,
                    offset_x,
                    offset_y,
                    escala,
                );
            }
        }
    }
}

pub fn render_final_objective_sprites(
    dibujo: &mut RaylibDrawHandle,
    mapa: &Map,
    player: &Player,
    camera: &Camera,
    textura_pieza_radio: &Texture2D,
    textura_radio: &Texture2D,
    offset_x: f32,
    offset_y: f32,
    escala: f32,
) {
    for fila in 0..mapa.alto() {
        for columna in 0..mapa.ancho() {
            let (textura, tamano) =
                match mapa.celda(fila as i32, columna as i32) {
                    'I' => (textura_pieza_radio, 0.32),
                    'E' => (textura_radio, 0.42),
                    _ => continue,
                };

            render_objeto(
                dibujo,
                mapa,
                player,
                camera,
                fila,
                columna,
                textura,
                tamano,
                offset_x,
                offset_y,
                escala,
            );
        }
    }
}
