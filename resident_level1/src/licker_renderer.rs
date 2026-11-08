use crate::camera::Camera;
use crate::licker::{
    EstadoLicker,
    LadoPared,
    Licker,
    TipoLicker,
};
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

const FOV: f32 =
    std::f32::consts::PI / 3.0;

pub fn render_lickers(
    d: &mut RaylibDrawHandle,
    mapa: &Map,
    player: &Player,
    camera: &Camera,
    lickers: &[Licker],
    licker1: &Texture2D,
    licker2: &Texture2D,
    licker3: &Texture2D,
    licker_v21: &Texture2D,
    licker_v22: &Texture2D,
    licker_v23: &Texture2D,
    licker_v31: &Texture2D,
    licker_v32: &Texture2D,
    licker_v33: &Texture2D,
    offset_x: f32,
    offset_y: f32,
    escala_pantalla: f32,
) {
    let mut visibles:
        Vec<(usize, f32)> =
        lickers
            .iter()
            .enumerate()
            .filter(|(_, licker)| {
                licker.vivo
            })
            .map(|(indice, licker)| {
                let dx =
                    licker.x
                        - player.x;

                let dy =
                    licker.y
                        - player.y;

                let distancia =
                    (
                        dx * dx
                            + dy * dy
                    )
                        .sqrt();

                (
                    indice,
                    distancia,
                )
            })
            .collect();

    visibles.sort_by(
        |a, b| {
            b.1
                .partial_cmp(
                    &a.1,
                )
                .unwrap_or(
                    std::cmp::Ordering::Equal,
                )
        },
    );

    for (
        indice,
        distancia,
    ) in visibles
    {
        render_licker(
            d,
            mapa,
            player,
            camera,
            &lickers[indice],
            distancia,
            licker1,
            licker2,
            licker3,
            licker_v21,
            licker_v22,
            licker_v23,
            licker_v31,
            licker_v32,
            licker_v33,
            offset_x,
            offset_y,
            escala_pantalla,
        );
    }
}

fn render_licker(
    d: &mut RaylibDrawHandle,
    mapa: &Map,
    player: &Player,
    camera: &Camera,
    licker: &Licker,
    distancia: f32,
    licker1: &Texture2D,
    licker2: &Texture2D,
    licker3: &Texture2D,
    licker_v21: &Texture2D,
    licker_v22: &Texture2D,
    licker_v23: &Texture2D,
    licker_v31: &Texture2D,
    licker_v32: &Texture2D,
    licker_v33: &Texture2D,
    offset_x: f32,
    offset_y: f32,
    escala_pantalla: f32,
) {
    if distancia <= 0.001 {
        return;
    }

    let dx =
        licker.x
            - player.x;

    let dy =
        licker.y
            - player.y;

    let angulo_licker =
        dy.atan2(dx);

    let diferencia =
        normalizar_angulo(
            angulo_licker
                - camera.angle,
        );

    if diferencia.abs()
        > FOV / 2.0 + 0.30
    {
        return;
    }

    let hit =
        lanzar_rayo(
            mapa,
            player.x,
            player.y,
            angulo_licker,
        );

    if hit.distancia
        < distancia - 6.0
    {
        return;
    }

    let textura =
        seleccionar_textura(
            licker,
            licker1,
            licker2,
            licker3,
            licker_v21,
            licker_v22,
            licker_v23,
            licker_v31,
            licker_v32,
            licker_v33,
        );

    let distancia_corregida =
        (
            distancia
                * diferencia.cos()
        )
            .max(1.0);

    let factor_tamano =
        match licker.estado {
            EstadoLicker::Suelo => 0.80,
            EstadoLicker::Trepando => 0.85,
            EstadoLicker::Pared => 0.85,
            EstadoLicker::Techo => 0.82,
            EstadoLicker::Cayendo => 0.90,
        };

    let altura_sprite =
        (
            TAMANO_CELDA
                * ALTO_VENTANA
                    as f32
                / distancia_corregida
        )
            * factor_tamano;

    if altura_sprite <= 1.0 {
        return;
    }

    let relacion =
        textura.width()
            as f32
            / textura
                .height()
                .max(1)
                as f32;

    let ancho_sprite =
        altura_sprite
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

    let altura_mundo =
        TAMANO_CELDA
            * licker.altura;

    let desplazamiento_altura =
        altura_mundo
            * ALTO_VENTANA
                as f32
            / distancia_corregida;

    let parte_inferior =
        horizonte
            + altura_sprite
                * 0.50
            - desplazamiento_altura;

    let pantalla_y =
        parte_inferior
            - altura_sprite;

    let destino_x =
        offset_x
            + (
                pantalla_x
                    - ancho_sprite
                        / 2.0
            )
                * escala_pantalla;

    let destino_y =
        offset_y
            + pantalla_y
                * escala_pantalla;

    let destino_ancho =
        ancho_sprite
            * escala_pantalla;

    let destino_alto =
        altura_sprite
            * escala_pantalla;

    if destino_x
            + destino_ancho
        < offset_x
    {
        return;
    }

    if destino_x
        > offset_x
            + ANCHO_VENTANA
                as f32
                * escala_pantalla
    {
        return;
    }

    if destino_y
            + destino_alto
        < offset_y
    {
        return;
    }

    if destino_y
        > offset_y
            + ALTO_VENTANA
                as f32
                * escala_pantalla
    {
        return;
    }

    let rotacion =
        rotacion_licker(
            licker,
        );

    let centro_destino_x =
        destino_x
            + destino_ancho
                / 2.0;

    let centro_destino_y =
        destino_y
            + destino_alto
                / 2.0;

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
            centro_destino_x,
            centro_destino_y,
            destino_ancho,
            destino_alto,
        ),

        Vector2::new(
            destino_ancho
                / 2.0,
            destino_alto
                / 2.0,
        ),

        rotacion,

        Color::WHITE,
    );
}

fn seleccionar_textura<'a>(
    licker: &Licker,
    licker1: &'a Texture2D,
    licker2: &'a Texture2D,
    licker3: &'a Texture2D,
    licker_v21: &'a Texture2D,
    licker_v22: &'a Texture2D,
    licker_v23: &'a Texture2D,
    licker_v31: &'a Texture2D,
    licker_v32: &'a Texture2D,
    licker_v33: &'a Texture2D,
) -> &'a Texture2D {
    if licker.tipo != TipoLicker::Normal {
        let (parado, persecucion_1, persecucion_2) = match licker.tipo {
            TipoLicker::Medio => (licker_v21, licker_v22, licker_v23),
            TipoLicker::Fuerte => (licker_v31, licker_v32, licker_v33),
            TipoLicker::Normal => unreachable!(),
        };

        if !licker.persiguiendo {
            return parado;
        }

        let frame = (licker.tiempo_animacion * 6.0) as i32 % 2;
        return if frame == 0 { persecucion_1 } else { persecucion_2 };
    }

    match licker.estado {
        EstadoLicker::Suelo => {
            let frame =
                (
                    licker
                        .tiempo_animacion
                        * 7.0
                )
                    as i32
                    % 2;

            if frame == 0 {
                licker2
            } else {
                licker3
            }
        }

        EstadoLicker::Trepando => {
            let frame =
                (
                    licker
                        .tiempo_animacion
                        * 6.0
                )
                    as i32
                    % 2;

            if frame == 0 {
                licker1
            } else {
                licker2
            }
        }

        EstadoLicker::Pared => {
            let frame =
                (
                    licker
                        .tiempo_animacion
                        * 5.0
                )
                    as i32
                    % 2;

            if frame == 0 {
                licker1
            } else {
                licker2
            }
        }

        EstadoLicker::Techo => {
            let frame =
                (
                    licker
                        .tiempo_animacion
                        * 5.0
                )
                    as i32
                    % 2;

            if frame == 0 {
                licker2
            } else {
                licker3
            }
        }

        EstadoLicker::Cayendo => {
            licker3
        }
    }
}

fn rotacion_licker(
    licker: &Licker,
) -> f32 {
    match licker.estado {
        EstadoLicker::Suelo => {
            0.0
        }

        EstadoLicker::Trepando
        | EstadoLicker::Pared => {
            match licker.lado_pared {
                Some(
                    LadoPared::Izquierda,
                ) => {
                    90.0
                }

                Some(
                    LadoPared::Derecha,
                ) => {
                    -90.0
                }

                Some(
                    LadoPared::Arriba,
                ) => {
                    0.0
                }

                Some(
                    LadoPared::Abajo,
                ) => {
                    180.0
                }

                None => {
                    0.0
                }
            }
        }

        EstadoLicker::Techo => {
            180.0
        }

        EstadoLicker::Cayendo => {
            0.0
        }
    }
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
