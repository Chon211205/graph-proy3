use crate::camera::Camera;
use crate::interaction::{
    procesar_muerte_zombie,
    ShotResult,
};
use crate::map::Map;
use crate::player::Player;
use crate::zombie::Zombie;

#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
)]
pub enum ArmaActual {
    Pistola,
    Hacha,
    Lanzallamas,
}

const DANO_HACHA: i32 = 75;
const DISTANCIA_HACHA: f32 = 45.0;
const TOLERANCIA_ANGULO_HACHA: f32 = 0.45;

pub fn atacar_con_hacha(
    zombies: &mut [Zombie],
    player: &Player,
    camera: &Camera,
    mapa: &mut Map,
) -> ShotResult {
    let mut mejor_indice:
        Option<usize> =
        None;

    let mut mejor_distancia =
        f32::MAX;

    for (
        indice,
        zombie,
    ) in zombies
        .iter()
        .enumerate()
    {
        if !zombie.vivo {
            continue;
        }

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

        if distancia
            > DISTANCIA_HACHA
        {
            continue;
        }

        let angulo_zombie =
            dy.atan2(
                dx,
            );

        let mut diferencia =
            angulo_zombie
                - camera.angle;

        while diferencia
            > std::f32::consts::PI
        {
            diferencia -=
                std::f32::consts::PI
                    * 2.0;
        }

        while diferencia
            < -std::f32::consts::PI
        {
            diferencia +=
                std::f32::consts::PI
                    * 2.0;
        }

        if diferencia.abs()
            > TOLERANCIA_ANGULO_HACHA
        {
            continue;
        }

        if !hay_linea_vision(
            mapa,
            player.x,
            player.y,
            zombie.x,
            zombie.y,
        ) {
            continue;
        }

        if distancia
            < mejor_distancia
        {
            mejor_distancia =
                distancia;

            mejor_indice =
                Some(
                    indice,
                );
        }
    }

    let Some(indice) =
        mejor_indice
    else {
        return ShotResult::Miss;
    };

    zombies[indice]
        .recibir_dano(
            DANO_HACHA,
        );

    if zombies[indice].vivo {
        return ShotResult::Hit {
            vida_restante:
                zombies[indice].vida,
        };
    }

    procesar_muerte_zombie(
        &zombies[indice],
        mapa,
        false,
    )
}

pub fn puede_bloquear_ataque(
    zombie: &Zombie,
    player: &Player,
    camera: &Camera,
) -> bool {
    if !zombie.vivo {
        return false;
    }

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

    if distancia > 45.0 {
        return false;
    }

    let angulo_zombie =
        dy.atan2(
            dx,
        );

    let mut diferencia =
        angulo_zombie
            - camera.angle;

    while diferencia
        > std::f32::consts::PI
    {
        diferencia -=
            std::f32::consts::PI
                * 2.0;
    }

    while diferencia
        < -std::f32::consts::PI
    {
        diferencia +=
            std::f32::consts::PI
                * 2.0;
    }

    diferencia.abs()
        <= std::f32::consts::PI
            / 3.0
}

fn hay_linea_vision(
    mapa: &Map,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
) -> bool {
    let dx =
        x2 - x1;

    let dy =
        y2 - y1;

    let distancia =
        (
            dx * dx
                + dy * dy
        )
            .sqrt();

    if distancia
        <= 0.001
    {
        return true;
    }

    let pasos =
        (
            distancia
                / 4.0
        )
            .ceil()
            as i32;

    if pasos <= 0 {
        return true;
    }

    let paso_x =
        dx
            / pasos
                as f32;

    let paso_y =
        dy
            / pasos
                as f32;

    let mut x =
        x1;

    let mut y =
        y1;

    for _ in 0..pasos {
        x += paso_x;
        y += paso_y;

        if mapa.es_pared(
            x,
            y,
        ) {
            return false;
        }
    }

    true
}
