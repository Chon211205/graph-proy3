use crate::camera::Camera;
use crate::inventory::Inventory;
use crate::map::{
    Map,
    TAMANO_CELDA,
};
use crate::player::Player;
use crate::puzzle::Puzzle;

use crate::raycaster::{
    ALTO_VENTANA,
};

use crate::zombie::{
    TipoZombie,
    Zombie,
};

use rand::Rng;

pub enum InteractionResult {
    None,

    LlaveRecogida,

    MunicionRecogida(
        i32,
    ),

    MunicionLanzallamasRecogida(
        i32,
    ),

    AntivirusRecogido,

    PiezaRadioRecogida,

    RadioEncontrada,

    CuracionRecogida(
        i32,
    ),

    PuertaAbierta,

    PuertaCerrada,

    SubirNivel(
        usize,
    ),

    BajarNivel(
        usize,
    ),
}

pub enum ShotResult {
    Miss,

    Hit {
        vida_restante: i32,
    },

    Kill,

    KillConLlave,

    HeadshotHit {
        vida_restante: i32,
    },

    HeadshotKill,

    HeadshotKillConLlave,
}

pub fn recoger_objetos_cercanos(
    mapa: &mut Map,
    player: &Player,
    inventory: &mut Inventory,
    _puzzle: &mut Puzzle,
    vida_jugador: i32,
) -> InteractionResult {
    let columna =
        (
            player.x
                / TAMANO_CELDA
        )
            .floor()
            as i32;

    let fila =
        (
            player.y
                / TAMANO_CELDA
        )
            .floor()
            as i32;

    let celda =
        mapa.celda(
            fila,
            columna,
        );

    match celda {
        'K' => {
            mapa.cambiar_celda(
                fila,
                columna,
                ' ',
            );

            inventory
                .recoger_llave();

            InteractionResult::
                LlaveRecogida
        }

        'A' => {
            mapa.cambiar_celda(
                fila,
                columna,
                ' ',
            );

            InteractionResult::
                MunicionRecogida(
                    8,
                )
        }

        'H' => {
            if vida_jugador
                >= 100
            {
                InteractionResult::
                    None
            } else {
                mapa.cambiar_celda(
                    fila,
                    columna,
                    ' ',
                );

                InteractionResult::
                    CuracionRecogida(
                        25,
                    )
            }
        }

        'Q' => {
            mapa.cambiar_celda(
                fila,
                columna,
                ' ',
            );

            InteractionResult::
                MunicionLanzallamasRecogida(
                    25,
                )
        }

        'V' => {
            mapa.cambiar_celda(
                fila,
                columna,
                ' ',
            );

            InteractionResult::AntivirusRecogido
        }

        'I' => {
            mapa.cambiar_celda(
                fila,
                columna,
                ' ',
            );

            InteractionResult::PiezaRadioRecogida
        }

        'E' => {
            InteractionResult::RadioEncontrada
        }

        _ => {
            InteractionResult::
                None
        }
    }
}

pub fn interactuar(
    mapa: &mut Map,
    player: &Player,
    camera: &Camera,
    inventory: &mut Inventory,
    _puzzle: &mut Puzzle,
) -> InteractionResult {
    let distancias = [
        12.0,
        16.0,
        20.0,
        24.0,
        28.0,
        32.0,
        36.0,
        40.0,
    ];

    for distancia in distancias {
        let x =
            player.x
                + camera
                    .angle
                    .cos()
                    * distancia;

        let y =
            player.y
                + camera
                    .angle
                    .sin()
                    * distancia;

        let columna =
            (
                x
                    / TAMANO_CELDA
            )
                .floor()
                as i32;

        let fila =
            (
                y
                    / TAMANO_CELDA
            )
                .floor()
                as i32;

        let celda =
            mapa.celda(
                fila,
                columna,
            );

        match celda {
            'D' => {
                if inventory
                    .tiene_llave()
                {
                    mapa.cambiar_celda(
                        fila,
                        columna,
                        'O',
                    );

                    inventory
                        .usar_llave();

                    return InteractionResult::
                        PuertaAbierta;
                }

                return InteractionResult::
                    PuertaCerrada;
            }

            'X' => {
                let indice =
                    mapa.indice_portal_en(
                        'X',
                        fila,
                        columna,
                    )
                    .unwrap_or(
                        0,
                    );

                return InteractionResult::
                    SubirNivel(
                        indice,
                    );
            }

            'B' => {
                let indice =
                    mapa.indice_portal_en(
                        'B',
                        fila,
                        columna,
                    )
                    .unwrap_or(
                        0,
                    );

                return InteractionResult::
                    BajarNivel(
                        indice,
                    );
            }

            '#' | 'W' => {
                return InteractionResult::
                    None;
            }

            _ => {}
        }
    }

    InteractionResult::None
}

pub fn disparar(
    zombies: &mut [Zombie],
    player: &Player,
    camera: &Camera,
    mapa: &mut Map,
) -> ShotResult {
    const DANO_CUERPO: i32 =
        50;

    const MULTIPLICADOR_HEADSHOT: i32 =
        2;

    const TOLERANCIA_ANGULO: f32 =
        0.08;

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
            > TOLERANCIA_ANGULO
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
        return ShotResult::
            Miss;
    };

    let es_headshot =
        detectar_headshot(
            &zombies[
                indice
            ],
            mejor_distancia,
            camera,
        );

    let dano =
        if es_headshot {
            DANO_CUERPO
                * MULTIPLICADOR_HEADSHOT
        } else {
            DANO_CUERPO
        };

    let zombie =
        &mut zombies[
            indice
        ];

    zombie.recibir_dano(
        dano,
    );

    if zombie.vivo {
        if es_headshot {
            return ShotResult::
                HeadshotHit {
                    vida_restante:
                        zombie.vida,
                };
        }

        return ShotResult::
            Hit {
                vida_restante:
                    zombie.vida,
            };
    }

    procesar_muerte_zombie(
        zombie,
        mapa,
        es_headshot,
    )
}

fn detectar_headshot(
    zombie: &Zombie,
    distancia: f32,
    camera: &Camera,
) -> bool {
    if distancia
        <= 0.001
    {
        return false;
    }

    let escala_zombie =
        match zombie.tipo {
            TipoZombie::Normal => {
                0.80
            }

            TipoZombie::Medio => {
                0.95
            }

            TipoZombie::Fuerte => {
                1.10
            }
        };

    let altura_proyectada =
        (
            TAMANO_CELDA
                * ALTO_VENTANA
                    as f32
                / distancia
        )
            * escala_zombie;

    let horizonte =
        ALTO_VENTANA
            as f32
            / 2.0
            + camera
                .vertical_offset
                as f32;

    let parte_inferior =
        horizonte
            + altura_proyectada
                * 0.50;

    let parte_superior =
        parte_inferior
            - altura_proyectada;

    let altura_cabeza =
        altura_proyectada
            * 0.30;

    let limite_cabeza =
        parte_superior
            + altura_cabeza;

    let mira_y =
        ALTO_VENTANA
            as f32
            / 2.0;

    mira_y
        >= parte_superior
        && mira_y
            <= limite_cabeza
}

pub fn procesar_muerte_zombie(
    zombie: &Zombie,
    mapa: &mut Map,
    headshot: bool,
) -> ShotResult {
    let columna =
        (
            zombie.x
                / TAMANO_CELDA
        )
            .floor()
            as i32;

    let fila =
        (
            zombie.y
                / TAMANO_CELDA
        )
            .floor()
            as i32;

    if mapa.nivel() != 4
        && zombie
            .puede_dropear_llave
    {
        mapa.cambiar_celda(
            fila,
            columna,
            'K',
        );

        if headshot {
            return ShotResult::
                HeadshotKillConLlave;
        }

        return ShotResult::
            KillConLlave;
    }

    let mut rng =
        rand::thread_rng();

    let tirada =
        rng.gen_range(
            0..100,
        );

    if mapa.nivel() == 3 {
        if tirada < 40 {
            mapa.cambiar_celda(
                fila,
                columna,
                'K',
            );
        } else if tirada < 50 {
            mapa.cambiar_celda(
                fila,
                columna,
                'H',
            );
        } else if tirada < 60 {
            mapa.cambiar_celda(
                fila,
                columna,
                'A',
            );
        } else if tirada < 70 {
            mapa.cambiar_celda(
                fila,
                columna,
                'Q',
            );
        }

        if headshot {
            return ShotResult::HeadshotKill;
        }

        return ShotResult::Kill;
    }

    if tirada < 25 {
        mapa.cambiar_celda(
            fila,
            columna,
            'H',
        );
    } else if tirada < 50 {
        mapa.cambiar_celda(
            fila,
            columna,
            'A',
        );
    } else if tirada < 70 {
        mapa.cambiar_celda(
            fila,
            columna,
            'Q',
        );
    }

    if headshot {
        ShotResult::
            HeadshotKill
    } else {
        ShotResult::
            Kill
    }
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
        x +=
            paso_x;

        y +=
            paso_y;

        if mapa.es_pared(
            x,
            y,
        ) {
            return false;
        }
    }

    true
}
