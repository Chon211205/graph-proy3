use crate::camera::Camera;
use crate::framebuffer::Framebuffer;
use crate::map::{
    Map,
    TAMANO_CELDA,
};
use crate::player::Player;
use crate::texture_data::TextureData;

use raylib::prelude::*;

pub const ANCHO_VENTANA: i32 = 800;
pub const ALTO_VENTANA: i32 = 600;

const FOV: f32 =
    std::f32::consts::PI / 3.0;

const DISTANCIA_MAXIMA: f32 =
    2000.0;

pub struct RayHit {
    pub distancia: f32,
    pub offset_textura: f32,
    pub golpe_vertical: bool,
    pub tipo: char,
}

pub fn lanzar_rayo(
    mapa: &Map,
    origen_x: f32,
    origen_y: f32,
    angulo: f32,
) -> RayHit {
    let dir_x =
        angulo.cos();

    let dir_y =
        angulo.sin();

    let mut mapa_x =
        (
            origen_x
                / TAMANO_CELDA
        )
            .floor()
            as i32;

    let mut mapa_y =
        (
            origen_y
                / TAMANO_CELDA
        )
            .floor()
            as i32;

    let delta_dist_x =
        if dir_x.abs() < 0.00001 {
            f32::MAX
        } else {
            (
                TAMANO_CELDA
                    / dir_x
            )
                .abs()
        };

    let delta_dist_y =
        if dir_y.abs() < 0.00001 {
            f32::MAX
        } else {
            (
                TAMANO_CELDA
                    / dir_y
            )
                .abs()
        };

    let paso_x: i32;
    let paso_y: i32;

    let mut distancia_lateral_x: f32;
    let mut distancia_lateral_y: f32;

    if dir_x < 0.0 {
        paso_x =
            -1;

        distancia_lateral_x =
            (
                origen_x
                    - mapa_x
                        as f32
                        * TAMANO_CELDA
            )
                / dir_x
                    .abs()
                    .max(
                        0.00001,
                    );
    } else {
        paso_x =
            1;

        distancia_lateral_x =
            (
                (
                    mapa_x
                        as f32
                        + 1.0
                )
                    * TAMANO_CELDA
                    - origen_x
            )
                / dir_x
                    .abs()
                    .max(
                        0.00001,
                    );
    }

    if dir_y < 0.0 {
        paso_y =
            -1;

        distancia_lateral_y =
            (
                origen_y
                    - mapa_y
                        as f32
                        * TAMANO_CELDA
            )
                / dir_y
                    .abs()
                    .max(
                        0.00001,
                    );
    } else {
        paso_y =
            1;

        distancia_lateral_y =
            (
                (
                    mapa_y
                        as f32
                        + 1.0
                )
                    * TAMANO_CELDA
                    - origen_y
            )
                / dir_y
                    .abs()
                    .max(
                        0.00001,
                    );
    }

    let mut golpe_vertical =
        false;

    let mut distancia =
        0.0;

    let mut tipo =
        '#';

    for _ in 0..2048 {
        if distancia_lateral_x
            < distancia_lateral_y
        {
            distancia =
                distancia_lateral_x;

            distancia_lateral_x +=
                delta_dist_x;

            mapa_x +=
                paso_x;

            golpe_vertical =
                true;
        } else {
            distancia =
                distancia_lateral_y;

            distancia_lateral_y +=
                delta_dist_y;

            mapa_y +=
                paso_y;

            golpe_vertical =
                false;
        }

        if distancia
            > DISTANCIA_MAXIMA
        {
            break;
        }

        let celda =
            mapa.celda(
                mapa_y,
                mapa_x,
            );

        if mapa.es_bloque_solido(
            mapa_y,
            mapa_x,
        ) {
            tipo =
                celda;

            break;
        }
    }

    let impacto_x =
        origen_x
            + dir_x
                * distancia;

    let impacto_y =
        origen_y
            + dir_y
                * distancia;

    let mut offset_textura =
        if golpe_vertical {
            impacto_y
                / TAMANO_CELDA
        } else {
            impacto_x
                / TAMANO_CELDA
        };

    offset_textura -=
        offset_textura.floor();

    if golpe_vertical
        && dir_x > 0.0
    {
        offset_textura =
            1.0
                - offset_textura;
    }

    if !golpe_vertical
        && dir_y < 0.0
    {
        offset_textura =
            1.0
                - offset_textura;
    }

    RayHit {
        distancia:
            distancia.max(
                0.0001,
            ),

        offset_textura,

        golpe_vertical,

        tipo,
    }
}

pub fn render_3d(
    framebuffer: &mut Framebuffer,
    mapa: &Map,
    player: &Player,
    camera: &Camera,

    textura_pared: &TextureData,
    textura_caja: &TextureData,
    textura_ventana: &TextureData,
    textura_puerta: &TextureData,

    textura_subir: &TextureData,
    textura_bajar: &TextureData,

    textura_suelo: &TextureData,
    textura_suelo2: &TextureData,

    textura_techo: &TextureData,
    panorama: Option<&TextureData>,
) {
    let limitar_suelo =
        panorama.is_some();

    if let Some(panorama) = panorama {
        render_panorama(framebuffer, camera, panorama);
    } else {
        render_techo(
            framebuffer,
            player,
            camera,
            textura_techo,
        );
    }

    render_suelo(
        framebuffer,
        mapa,
        player,
        camera,
        textura_suelo,
        textura_suelo2,
        limitar_suelo,
    );

    render_paredes(
        framebuffer,
        mapa,
        player,
        camera,

        textura_pared,
        textura_caja,
        textura_ventana,
        textura_puerta,

        textura_subir,
        textura_bajar,
    );
}

fn render_techo(
    framebuffer: &mut Framebuffer,
    player: &Player,
    camera: &Camera,
    textura_techo: &TextureData,
) {
    let horizonte =
        ALTO_VENTANA
            as f32
            / 2.0
            + camera
                .vertical_offset
                as f32;

    let dir_x =
        camera.angle.cos();

    let dir_y =
        camera.angle.sin();

    let plano_x =
        -dir_y
            * (
                FOV / 2.0
            )
                .tan();

    let plano_y =
        dir_x
            * (
                FOV / 2.0
            )
                .tan();

    let rayo_izq_x =
        dir_x
            - plano_x;

    let rayo_izq_y =
        dir_y
            - plano_y;

    let rayo_der_x =
        dir_x
            + plano_x;

    let rayo_der_y =
        dir_y
            + plano_y;

    let posicion_vertical =
        TAMANO_CELDA
            * ALTO_VENTANA
                as f32
            / 2.0;

    let fin_y =
        horizonte
            .min(
                ALTO_VENTANA
                    as f32,
            )
            .max(
                0.0,
            )
            as i32;

    for y in 0..fin_y {
        let p =
            horizonte
                - y as f32;

        if p.abs()
            < 0.001
        {
            continue;
        }

        let distancia_fila =
            posicion_vertical
                / p;

        let paso_x =
            distancia_fila
                * (
                    rayo_der_x
                        - rayo_izq_x
                )
                / ANCHO_VENTANA
                    as f32;

        let paso_y =
            distancia_fila
                * (
                    rayo_der_y
                        - rayo_izq_y
                )
                / ANCHO_VENTANA
                    as f32;

        let mut mundo_x =
            player.x
                + distancia_fila
                    * rayo_izq_x;

        let mut mundo_y =
            player.y
                + distancia_fila
                    * rayo_izq_y;

        for x in 0..ANCHO_VENTANA {
            let local_x =
                mundo_x / TAMANO_CELDA;
            let local_y =
                mundo_y / TAMANO_CELDA;

            let u =
                local_x - local_x.floor();
            let v =
                local_y - local_y.floor();

            let tex_x =
                (
                    u
                        * textura_techo
                            .width
                            as f32
                )
                    .floor()
                    .clamp(
                        0.0,
                        textura_techo
                            .width
                            as f32
                            - 1.0,
                    )
                    as i32;

            let tex_y =
                (
                    v
                        * textura_techo
                            .height
                            as f32
                )
                    .floor()
                    .clamp(
                        0.0,
                        textura_techo
                            .height
                            as f32
                            - 1.0,
                    )
                    as i32;

            let mut color =
                textura_techo
                    .get_pixel(
                        tex_x,
                        tex_y,
                    );

            let oscuridad =
                (
                    0.80
                        - distancia_fila
                            / 1800.0
                )
                    .clamp(
                        0.30,
                        0.80,
                    );

            color.r =
                (
                    color.r
                        as f32
                        * oscuridad
                )
                    .clamp(
                        0.0,
                        255.0,
                    )
                    as u8;

            color.g =
                (
                    color.g
                        as f32
                        * oscuridad
                )
                    .clamp(
                        0.0,
                        255.0,
                    )
                    as u8;

            color.b =
                (
                    color.b
                        as f32
                        * oscuridad
                )
                    .clamp(
                        0.0,
                        255.0,
                    )
                    as u8;

            framebuffer
                .point_color(
                    x,
                    y,
                    color,
                );

            mundo_x +=
                paso_x;

            mundo_y +=
                paso_y;
        }
    }
}

fn render_suelo(
    framebuffer: &mut Framebuffer,
    mapa: &Map,
    player: &Player,
    camera: &Camera,
    textura_suelo: &TextureData,
    textura_suelo2: &TextureData,
    limitar_al_mapa: bool,
) {
    let horizonte =
        ALTO_VENTANA
            as f32
            / 2.0
            + camera
                .vertical_offset
                as f32;

    let dir_x =
        camera.angle.cos();

    let dir_y =
        camera.angle.sin();

    let plano_x =
        -dir_y
            * (
                FOV / 2.0
            )
                .tan();

    let plano_y =
        dir_x
            * (
                FOV / 2.0
            )
                .tan();

    let rayo_izq_x =
        dir_x
            - plano_x;

    let rayo_izq_y =
        dir_y
            - plano_y;

    let rayo_der_x =
        dir_x
            + plano_x;

    let rayo_der_y =
        dir_y
            + plano_y;

    let posicion_vertical =
        TAMANO_CELDA
            * ALTO_VENTANA
                as f32
            / 2.0;

    let inicio_y =
        horizonte
            .max(
                0.0,
            )
            as i32;

    for y in inicio_y
        ..ALTO_VENTANA
    {
        let p =
            y as f32
                - horizonte;

        if p.abs()
            < 0.001
        {
            continue;
        }

        let distancia_fila =
            posicion_vertical
                / p;

        let paso_x =
            distancia_fila
                * (
                    rayo_der_x
                        - rayo_izq_x
                )
                / ANCHO_VENTANA
                    as f32;

        let paso_y =
            distancia_fila
                * (
                    rayo_der_y
                        - rayo_izq_y
                )
                / ANCHO_VENTANA
                    as f32;

        let mut mundo_x =
            player.x
                + distancia_fila
                    * rayo_izq_x;

        let mut mundo_y =
            player.y
                + distancia_fila
                    * rayo_izq_y;

        for x in 0..ANCHO_VENTANA {
            let columna =
                (
                    mundo_x
                        / TAMANO_CELDA
                )
                    .floor()
                    as i32;

            let fila =
                (
                    mundo_y
                        / TAMANO_CELDA
                )
                    .floor()
                    as i32;

            let celda =
                mapa.celda(
                    fila,
                    columna,
                );

            if limitar_al_mapa
                && (
                    fila < 0
                    || columna < 0
                    || fila >= mapa.alto() as i32
                    || columna >= mapa.ancho() as i32
                    || celda == 'G'
                )
            {
                mundo_x += paso_x;
                mundo_y += paso_y;
                continue;
            }

            let textura =
                if celda == 'C' {
                    textura_suelo2
                } else {
                    textura_suelo
                };

            let local_x =
                mundo_x / TAMANO_CELDA;
            let local_y =
                mundo_y / TAMANO_CELDA;

            let u =
                local_x - local_x.floor();
            let v =
                local_y - local_y.floor();

            let tex_x =
                (
                    u
                        * textura
                            .width
                            as f32
                )
                    .floor()
                    .clamp(
                        0.0,
                        textura.width
                            as f32
                            - 1.0,
                    )
                    as i32;

            let tex_y =
                (
                    v
                        * textura
                            .height
                            as f32
                )
                    .floor()
                    .clamp(
                        0.0,
                        textura.height
                            as f32
                            - 1.0,
                    )
                    as i32;

            let mut color =
                textura.get_pixel(
                    tex_x,
                    tex_y,
                );

            let sombra =
                (
                    1.0
                        - distancia_fila
                            / 1400.0
                )
                    .clamp(
                        0.35,
                        1.0,
                    );

            color.r =
                (
                    color.r
                        as f32
                        * sombra
                )
                    .clamp(
                        0.0,
                        255.0,
                    )
                    as u8;

            color.g =
                (
                    color.g
                        as f32
                        * sombra
                )
                    .clamp(
                        0.0,
                        255.0,
                    )
                    as u8;

            color.b =
                (
                    color.b
                        as f32
                        * sombra
                )
                    .clamp(
                        0.0,
                        255.0,
                    )
                    as u8;

            framebuffer
                .point_color(
                    x,
                    y,
                    color,
                );

            mundo_x +=
                paso_x;

            mundo_y +=
                paso_y;
        }
    }
}

fn render_paredes(
    framebuffer: &mut Framebuffer,
    mapa: &Map,
    player: &Player,
    camera: &Camera,

    textura_pared: &TextureData,
    textura_caja: &TextureData,
    textura_ventana: &TextureData,
    textura_puerta: &TextureData,

    textura_subir: &TextureData,
    textura_bajar: &TextureData,
) {
    for columna_pantalla
        in 0..ANCHO_VENTANA
    {
        let porcentaje =
            columna_pantalla
                as f32
                / ANCHO_VENTANA
                    as f32;

        let angulo_rayo =
            camera.angle
                - FOV
                    / 2.0
                + porcentaje
                    * FOV;

        let hit =
            lanzar_rayo(
                mapa,
                player.x,
                player.y,
                angulo_rayo,
            );

        let diferencia_angulo =
            angulo_rayo
                - camera.angle;

        let distancia_corregida =
            (
                hit.distancia
                    * diferencia_angulo
                        .cos()
            )
                .max(
                    0.001,
                );

        let altura_pared =
            (
                TAMANO_CELDA
                    * ALTO_VENTANA
                        as f32
                    / distancia_corregida
            )
                .max(
                    1.0,
                );

        let centro =
            ALTO_VENTANA
                as f32
                / 2.0
                + camera
                    .vertical_offset
                    as f32;

        let inicio =
            centro
                - altura_pared
                    / 2.0;

        let fin =
            centro
                + altura_pared
                    / 2.0;

        if hit.tipo == 'G' {
            continue;
        }

        let textura =
            match hit.tipo {
                'J' => {
                    textura_caja
                }

                'W' => {
                    textura_ventana
                }

                'D' => {
                    textura_puerta
                }

                'X' => {
                    textura_subir
                }

                'B' => {
                    textura_bajar
                }

                _ => {
                    textura_pared
                }
            };

        dibujar_columna_pared(
            framebuffer,
            columna_pantalla,
            inicio,
            fin,
            &hit,
            textura,
            distancia_corregida,
        );
    }
}

fn dibujar_columna_pared(
    framebuffer: &mut Framebuffer,
    x: i32,
    inicio: f32,
    fin: f32,
    hit: &RayHit,
    textura: &TextureData,
    distancia: f32,
) {
    let altura =
        fin
            - inicio;

    if altura <= 0.0 {
        return;
    }

    let tex_x =
        (
            hit.offset_textura
                * textura.width
                    as f32
        )
            .floor()
            .clamp(
                0.0,
                textura.width
                    as f32
                    - 1.0,
            )
            as i32;

    let inicio_dibujo =
        inicio
            .max(
                0.0,
            )
            as i32;

    let fin_dibujo =
        fin
            .min(
                ALTO_VENTANA
                    as f32
                    - 1.0,
            )
            as i32;

    let sombra_distancia =
        (
            1.0
                - distancia
                    / 900.0
        )
            .clamp(
                0.28,
                1.0,
            );

    let sombra_lado =
        if hit.golpe_vertical {
            1.0
        } else {
            0.82
        };

    let sombra =
        sombra_distancia
            * sombra_lado;

    for y in inicio_dibujo
        ..=fin_dibujo
    {
        let porcentaje_y =
            (
                y as f32
                    - inicio
            )
                / altura;

        let tex_y =
            (
                porcentaje_y
                    * textura.height
                        as f32
            )
                .floor()
                .clamp(
                    0.0,
                    textura.height
                        as f32
                        - 1.0,
                )
                as i32;

        let mut color =
            textura.get_pixel(
                tex_x,
                tex_y,
            );

        color.r =
            (
                color.r
                    as f32
                    * sombra
            )
                .clamp(
                    0.0,
                    255.0,
                )
                as u8;

        color.g =
            (
                color.g
                    as f32
                    * sombra
            )
                .clamp(
                    0.0,
                    255.0,
                )
                as u8;

        color.b =
            (
                color.b
                    as f32
                    * sombra
            )
                .clamp(
                    0.0,
                    255.0,
                )
                as u8;

        framebuffer
            .point_color(
                x,
                y,
                color,
            );
    }
}

fn render_panorama(
    framebuffer: &mut Framebuffer,
    camera: &Camera,
    panorama: &TextureData,
) {
    for y in 0..ALTO_VENTANA {
        let textura_y =
            ((y as f32 / ALTO_VENTANA as f32)
                * panorama.height as f32)
                .clamp(0.0, panorama.height as f32 - 1.0)
                as i32;

        for x in 0..ANCHO_VENTANA {
            let desplazamiento =
                (x as f32 / ANCHO_VENTANA as f32 - 0.5) * FOV;
            let angulo =
                (camera.angle + desplazamiento)
                    .rem_euclid(std::f32::consts::PI * 2.0);
            let textura_x =
                (angulo / (std::f32::consts::PI * 2.0)
                    * panorama.width as f32) as i32
                    % panorama.width;

            framebuffer.point_color(
                x,
                y,
                panorama.get_pixel(textura_x, textura_y),
            );
        }
    }
}
