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
use crate::tyrant::{ProyectilNemesis, Tyrant};

use raylib::prelude::*;

const FOV: f32 =
    std::f32::consts::PI
        / 3.0;

pub fn render_tyrant(
    d: &mut RaylibDrawHandle,
    mapa: &Map,
    player: &Player,
    camera: &Camera,
    tyrant: &Tyrant,
    tyrant1: &Texture2D,
    tyrant2: &Texture2D,
    tyrant3: &Texture2D,
    offset_x: f32,
    offset_y: f32,
    escala_pantalla: f32,
) {
    let dx =
        tyrant.x
            - player.x;

    let dy =
        tyrant.y
            - player.y;

    let distancia =
        (
            dx * dx
                + dy * dy
        )
            .sqrt();

    if distancia
        <= 0.001
    {
        return;
    }

    let angulo_tyrant =
        dy.atan2(
            dx,
        );

    let mut diferencia =
        normalizar_angulo(
            angulo_tyrant
                - camera.angle,
        );

    if diferencia.abs()
        > FOV / 2.0
            + 0.25
    {
        return;
    }

    let hit =
        lanzar_rayo(
            mapa,
            player.x,
            player.y,
            angulo_tyrant,
        );

    if hit.distancia
        < distancia - 6.0
    {
        return;
    }

    let textura =
        if tyrant.persiguiendo {
            let frame =
                (
                    tyrant.tiempo_animacion
                        * 5.0
                )
                    as i32
                    % 2;

            if frame == 0 {
                tyrant2
            } else {
                tyrant3
            }
        } else {
            tyrant1
        };

    let distancia_corregida =
        (
            distancia
                * diferencia.cos()
        )
            .max(
                1.0,
            );

    let altura =
        (
            TAMANO_CELDA
                * ALTO_VENTANA
                    as f32
                / distancia_corregida
        )
            * 1.35;

    if altura
        <= 1.0
    {
        return;
    }

    let relacion =
        textura.width()
            as f32
            / textura.height()
                .max(1)
                as f32;

    let ancho =
        altura
            * relacion;

    let centro_x =
        ANCHO_VENTANA
            as f32
            / 2.0;

    let pantalla_x =
        centro_x
            + (
                diferencia
                    / (
                        FOV / 2.0
                    )
            )
                * centro_x;

    let horizonte =
        ALTO_VENTANA
            as f32
            / 2.0
            + camera
                .vertical_offset
                as f32;

    let parte_inferior =
        horizonte
            + altura
                * 0.50;

    let pantalla_y =
        parte_inferior
            - altura;

    let destino_x =
        offset_x
            + (
                pantalla_x
                    - ancho / 2.0
            )
                * escala_pantalla;

    let destino_y =
        offset_y
            + pantalla_y
                * escala_pantalla;

    let destino_ancho =
        ancho
            * escala_pantalla;

    let destino_alto =
        altura
            * escala_pantalla;

    if destino_x
            + destino_ancho
        < offset_x
        || destino_x
            > offset_x
                + ANCHO_VENTANA
                    as f32
                    * escala_pantalla
    {
        return;
    }

    d.draw_texture_pro(
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
            destino_x,
            destino_y,
            destino_ancho,
            destino_alto,
        ),
        Vector2::new(
            0.0,
            0.0,
        ),
        0.0,
        Color::WHITE,
    );

}

fn normalizar_angulo(
    mut angulo: f32,
) -> f32 {
    while angulo
        > std::f32::consts::PI
    {
        angulo -=
            std::f32::consts::PI
                * 2.0;
    }

    while angulo
        < -std::f32::consts::PI
    {
        angulo +=
            std::f32::consts::PI
                * 2.0;
    }

    angulo
}

pub fn render_misiles_nemesis(
    d: &mut RaylibDrawHandle,
    mapa: &Map,
    player: &Player,
    camera: &Camera,
    misiles: &[ProyectilNemesis],
    textura: &Texture2D,
    offset_x: f32,
    offset_y: f32,
    escala_pantalla: f32,
) {
    for misil in misiles.iter().filter(|misil| misil.vivo) {
        let dx = misil.x - player.x;
        let dy = misil.y - player.y;
        let distancia = (dx * dx + dy * dy).sqrt();

        if distancia <= 0.001 {
            continue;
        }

        let angulo = dy.atan2(dx);
        let diferencia =
            normalizar_angulo(angulo - camera.angle);

        if diferencia.abs() > FOV / 2.0 + 0.20 {
            continue;
        }

        let hit = lanzar_rayo(
            mapa,
            player.x,
            player.y,
            angulo,
        );

        if hit.distancia < distancia - 4.0 {
            continue;
        }

        let distancia_corregida =
            (distancia * diferencia.cos()).max(1.0);
        let altura =
            TAMANO_CELDA * ALTO_VENTANA as f32
                / distancia_corregida
                * 0.30;
        let ancho = altura
            * textura.width() as f32
            / textura.height().max(1) as f32;
        let centro_x = ANCHO_VENTANA as f32 / 2.0;
        let pantalla_x = centro_x
            + diferencia / (FOV / 2.0) * centro_x;
        let horizonte = ALTO_VENTANA as f32 / 2.0
            + camera.vertical_offset as f32;

        d.draw_texture_pro(
            textura,
            Rectangle::new(
                0.0,
                0.0,
                textura.width() as f32,
                textura.height() as f32,
            ),
            Rectangle::new(
                offset_x + (pantalla_x - ancho / 2.0) * escala_pantalla,
                offset_y + (horizonte - altura / 2.0) * escala_pantalla,
                ancho * escala_pantalla,
                altura * escala_pantalla,
            ),
            Vector2::new(0.0, 0.0),
            0.0,
            Color::WHITE,
        );
    }
}
