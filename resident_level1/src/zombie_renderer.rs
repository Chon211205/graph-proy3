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

use crate::zombie::{
    TipoZombie,
    Zombie,
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

pub fn render_zombies(
    dibujo: &mut RaylibDrawHandle,
    mapa: &Map,
    player: &Player,
    camera: &Camera,
    zombies: &[Zombie],

    zombie1: &Texture2D,
    zombie2: &Texture2D,
    zombie3: &Texture2D,

    zombie_v21: &Texture2D,
    zombie_v22: &Texture2D,
    zombie_v23: &Texture2D,

    zombie_v31: &Texture2D,
    zombie_v32: &Texture2D,
    zombie_v33: &Texture2D,

    offset_x: f32,
    offset_y: f32,
    escala: f32,
) {
    let mut orden:
        Vec<usize> =
        zombies
            .iter()
            .enumerate()
            .filter(
                |(_, zombie)| {
                    zombie.vivo
                },
            )
            .map(
                |(indice, _)| {
                    indice
                },
            )
            .collect();

    orden.sort_by(
        |a, b| {
            let za =
                &zombies[*a];

            let zb =
                &zombies[*b];

            let da =
                (
                    za.x - player.x
                )
                    .powi(2)
                    + (
                        za.y - player.y
                    )
                        .powi(2);

            let db =
                (
                    zb.x - player.x
                )
                    .powi(2)
                    + (
                        zb.y - player.y
                    )
                        .powi(2);

            db.partial_cmp(
                &da,
            )
                .unwrap_or(
                    std::cmp::Ordering::Equal,
                )
        },
    );

    for indice in orden {
        let zombie =
            &zombies[indice];

        let dx =
            zombie.x
                - player.x;

        let dy =
            zombie.y
                - player.y;

        let distancia =
            (
                dx * dx
                    + dy * dy
            )
                .sqrt();

        if distancia <= 1.0 {
            continue;
        }

        let angulo_zombie =
            dy.atan2(
                dx,
            );

        let diferencia =
            normalizar_angulo(
                angulo_zombie
                    - camera.angle,
            );

        if diferencia.abs()
            > FOV / 2.0
                + 0.20
        {
            continue;
        }

        let hit_pared =
            lanzar_rayo(
                mapa,
                player.x,
                player.y,
                angulo_zombie,
            );

        if hit_pared.distancia
            < distancia - 4.0
        {
            continue;
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
                ANCHO_VENTANA
                    as f32
                    / 2.0
            )
                / (
                    FOV / 2.0
                )
                    .tan();

        let screen_x =
            ANCHO_VENTANA
                as f32
                / 2.0
                + diferencia.tan()
                    * plano_proyeccion;

        let textura =
            match zombie.tipo {
                TipoZombie::Normal => {
                    if zombie.persiguiendo {
                        let frame =
                            (
                                zombie
                                    .tiempo_animacion
                                    * 6.0
                            )
                                as i32
                                % 2;

                        if frame == 0 {
                            zombie2
                        } else {
                            zombie3
                        }
                    } else {
                        zombie1
                    }
                }

                TipoZombie::Medio => {
                    if zombie.persiguiendo {
                        let frame =
                            (
                                zombie
                                    .tiempo_animacion
                                    * 6.0
                            )
                                as i32
                                % 2;

                        if frame == 0 {
                            zombie_v22
                        } else {
                            zombie_v23
                        }
                    } else {
                        zombie_v21
                    }
                }

                TipoZombie::Fuerte => {
                    if zombie.persiguiendo {
                        let frame =
                            (
                                zombie
                                    .tiempo_animacion
                                    * 5.0
                            )
                                as i32
                                % 2;

                        if frame == 0 {
                            zombie_v32
                        } else {
                            zombie_v33
                        }
                    } else {
                        zombie_v31
                    }
                }
            };

        let factor_tamano =
            match zombie.tipo {
                TipoZombie::Normal => 0.80,
                TipoZombie::Medio => 0.95,
                TipoZombie::Fuerte => 1.10,
            };

        let altura_mundo =
            TAMANO_CELDA
                * factor_tamano;

        let altura_sprite =
            altura_mundo
                / distancia_corregida
                * plano_proyeccion;

        if altura_sprite <= 1.0 {
            continue;
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
            ALTO_VENTANA
                as f32
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
                + ANCHO_VENTANA
                    as f32
                    * escala;

        let limite_superior =
            offset_y;

        let limite_inferior =
            offset_y
                + ALTO_VENTANA
                    as f32
                    * escala;

        if x + ancho_final
            <= limite_izquierdo
        {
            continue;
        }

        if x
            >= limite_derecho
        {
            continue;
        }

        if y + alto_final
            <= limite_superior
        {
            continue;
        }

        if y
            >= limite_inferior
        {
            continue;
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
}