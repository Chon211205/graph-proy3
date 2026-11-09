mod audio;
mod camera;
mod damage_effect;
mod framebuffer;
mod gamepad;
mod hud;
mod interaction;
mod inventory;
mod licker;
mod licker_renderer;
mod map;
mod map_renderer;
mod menu;
mod player;
mod puzzle;
mod raycaster;
mod sprite_renderer;
mod texture_data;
mod tyrant;
mod tyrant_renderer;
mod weapon;
mod zombie;
mod zombie_renderer;

use audio::{AudioManager, ModoMusica};
use camera::Camera;
use damage_effect::DamageEffect;
use framebuffer::Framebuffer;
use hud::render_hud;

use interaction::{
    disparar,
    interactuar,
    procesar_muerte_zombie,
    recoger_objetos_cercanos,
    InteractionResult,
    ShotResult,
};

use inventory::Inventory;

use licker::{
    EstadoLicker,
    Licker,
    TipoLicker,
};

use licker_renderer::render_lickers;

use map::{
    Map,
    TipoSpawnZombie,
    TAMANO_CELDA,
};

use map_renderer::{
    render_leyenda_minimapa,
    render_minimap,
};

use menu::{
    AccionMenu,
    AccionSeleccionNivel,
    EstadoJuego,
    Menu,
    NivelSeleccionado,
};

use player::Player;
use puzzle::Puzzle;

use raycaster::{
    lanzar_rayo,
    render_3d,
    ALTO_VENTANA,
    ANCHO_VENTANA,
};

use sprite_renderer::{
    render_ammo_sprites,
    render_antivirus_sprite,
    render_flamethrow_ammo_sprites,
    render_final_objective_sprites,
    render_heal_sprites,
    render_key_sprite,
};

use texture_data::TextureData;

use tyrant::{ProyectilNemesis, Tyrant};
use tyrant_renderer::{render_misiles_nemesis, render_tyrant};

type Nemesis = Tyrant;

use weapon::{
    atacar_con_hacha,
    puede_bloquear_ataque,
    ArmaActual,
};

use zombie::{
    TipoZombie,
    Zombie,
};

use zombie_renderer::render_zombies;

use rand::Rng;
use raylib::audio::RaylibAudio;
use raylib::prelude::*;

const DISTANCIA_MIN_SPAWN: f32 = 120.0;
const DISTANCIA_MAX_SPAWN: f32 = 380.0;
const DISTANCIA_ENTRE_ZOMBIES: f32 = 35.0;

const META_NORMAL: i32 = 20;
const META_MEDIO: i32 = 15;
const META_FUERTE: i32 = 5;

const DANO_PISTOLA_LICKER: i32 = 50;
const DANO_HACHA_LICKER: i32 = 75;
const MULTIPLICADOR_HEADSHOT_LICKER: i32 = 2;
const DANO_LANZALLAMAS: i32 = 15;
const ALCANCE_LANZALLAMAS: f32 = 125.0;
const META_ENEMIGOS_FINAL: i32 = 12;
const META_PIEZAS_RADIO: i32 = 3;

const DURACION_SONIDO_MUERTE: f32 = 0.8;

enum ResultadoDisparoLicker {
    Impacto { vida_restante: i32 },
    Headshot { vida_restante: i32 },
    Muerte,
    MuerteHeadshot,
}

fn crear_zombies(
    mapa: &mut Map,
) -> Vec<Zombie> {
    mapa.extraer_zombies()
        .into_iter()
        .map(|(x, y, tipo)| {
            match tipo {
                TipoSpawnZombie::Normal => {
                    Zombie::new(x, y)
                }

                TipoSpawnZombie::ConLlave => {
                    Zombie::new_con_llave(
                        x,
                        y,
                    )
                }

                TipoSpawnZombie::Medio => {
                    Zombie::new_medio(
                        x,
                        y,
                    )
                }

                TipoSpawnZombie::Fuerte => {
                    Zombie::new_fuerte(
                        x,
                        y,
                    )
                }
            }
        })
        .collect()
}

fn crear_lickers(
    mapa: &mut Map,
) -> Vec<Licker> {
    let mut lickers =
        Vec::new();

    for fila in 0..mapa.alto() {
        for columna in 0..mapa.ancho() {
            let tipo_spawn = mapa.celda(
                fila as i32,
                columna as i32,
            );

            if matches!(tipo_spawn, 'R' | 'S' | 'T') {
                let x =
                    columna as f32
                        * TAMANO_CELDA
                        + TAMANO_CELDA / 2.0;

                let y =
                    fila as f32
                        * TAMANO_CELDA
                        + TAMANO_CELDA / 2.0;

                let licker = match tipo_spawn {
                    'R' => Licker::new(x, y),
                    'S' => Licker::new_medio(x, y),
                    'T' => Licker::new_fuerte(x, y),
                    _ => unreachable!(),
                };

                lickers.push(licker);

                mapa.cambiar_celda(
                    fila as i32,
                    columna as i32,
                    ' ',
                );
            }
        }
    }

    lickers
}

fn buscar_tyrant(
    mapa: &mut Map,
) -> Option<Tyrant> {
    for fila in 0..mapa.alto() {
        for columna in 0..mapa.ancho() {
            if mapa.celda(
                fila as i32,
                columna as i32,
            ) == 'Y'
            {
                let x =
                    columna as f32
                        * TAMANO_CELDA
                        + TAMANO_CELDA / 2.0;

                let y =
                    fila as f32
                        * TAMANO_CELDA
                        + TAMANO_CELDA / 2.0;

                mapa.cambiar_celda(
                    fila as i32,
                    columna as i32,
                    ' ',
                );

                return Some(
                    Tyrant::new(
                        x,
                        y,
                    ),
                );
            }
        }
    }

    None
}

fn calcular_distancia(
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
) -> f32 {
    let dx =
        x2 - x1;

    let dy =
        y2 - y1;

    (
        dx * dx
            + dy * dy
    )
        .sqrt()
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

fn contar_vivos_tipo(
    zombies: &[Zombie],
    tipo: TipoZombie,
) -> usize {
    zombies
        .iter()
        .filter(|zombie| {
            zombie.vivo
                && zombie.tipo == tipo
        })
        .count()
}

fn actualizar_bajas_por_tipo(
    normal_antes: usize,
    medio_antes: usize,
    fuerte_antes: usize,
    zombies: &[Zombie],
    bajas_normal: &mut i32,
    bajas_medio: &mut i32,
    bajas_fuerte: &mut i32,
) {
    let normal_despues =
        contar_vivos_tipo(
            zombies,
            TipoZombie::Normal,
        );

    let medio_despues =
        contar_vivos_tipo(
            zombies,
            TipoZombie::Medio,
        );

    let fuerte_despues =
        contar_vivos_tipo(
            zombies,
            TipoZombie::Fuerte,
        );

    if normal_despues < normal_antes {
        *bajas_normal +=
            (
                normal_antes
                    - normal_despues
            ) as i32;
    }

    if medio_despues < medio_antes {
        *bajas_medio +=
            (
                medio_antes
                    - medio_despues
            ) as i32;
    }

    if fuerte_despues < fuerte_antes {
        *bajas_fuerte +=
            (
                fuerte_antes
                    - fuerte_despues
            ) as i32;
    }
}

fn objetivo_completo(
    bajas_normal: i32,
    bajas_medio: i32,
    bajas_fuerte: i32,
) -> bool {
    bajas_normal >= META_NORMAL
        && bajas_medio >= META_MEDIO
        && bajas_fuerte >= META_FUERTE
}

fn max_zombies_para_horda(
    numero_horda: i32,
) -> usize {
    let extra =
        (
            numero_horda / 5
        )
            .max(0)
            as usize
            * 2;

    (18 + extra).min(30)
}

fn cantidad_horda(
    numero_horda: i32,
) -> usize {
    match numero_horda {
        1 => 7,
        2 => 9,
        3 => 11,
        4 => 13,
        5 => 14,

        _ => {
            (
                14
                    + (
                        numero_horda - 5
                    )
                        .max(0)
                        as usize
            )
                .min(18)
        }
    }
}

fn buscar_spawn_horda(
    mapa: &Map,
    player: &Player,
    zombies: &[Zombie],
) -> Option<(f32, f32)> {
    if mapa.ancho() <= 2
        || mapa.alto() <= 2
    {
        return None;
    }

    let mut rng =
        rand::thread_rng();

    for _ in 0..800 {
        let columna =
            rng.gen_range(
                1..mapa.ancho() - 1,
            );

        let fila =
            rng.gen_range(
                1..mapa.alto() - 1,
            );

        let celda =
            mapa.celda(
                fila as i32,
                columna as i32,
            );

        if !matches!(
            celda,
            ' ' | 'C' | 'P' | 'O'
        ) {
            continue;
        }

        let x =
            columna as f32
                * TAMANO_CELDA
                + TAMANO_CELDA / 2.0;

        let y =
            fila as f32
                * TAMANO_CELDA
                + TAMANO_CELDA / 2.0;

        let distancia =
            calcular_distancia(
                player.x,
                player.y,
                x,
                y,
            );

        if distancia
            < DISTANCIA_MIN_SPAWN
        {
            continue;
        }

        if distancia
            > DISTANCIA_MAX_SPAWN
        {
            continue;
        }

        let ocupado =
            zombies
                .iter()
                .filter(|z| z.vivo)
                .any(|zombie| {
                    calcular_distancia(
                        zombie.x,
                        zombie.y,
                        x,
                        y,
                    ) < DISTANCIA_ENTRE_ZOMBIES
                });

        if ocupado {
            continue;
        }

        return Some(
            (
                x,
                y,
            ),
        );
    }

    None
}

fn crear_zombie_horda(
    numero_horda: i32,
    indice: usize,
    x: f32,
    y: f32,
) -> Zombie {
    match numero_horda {
        1 => {
            Zombie::new(
                x,
                y,
            )
        }

        2 => {
            if indice % 4 == 0 {
                Zombie::new_medio(
                    x,
                    y,
                )
            } else {
                Zombie::new(
                    x,
                    y,
                )
            }
        }

        3 | 4 => {
            match indice % 6 {
                0 => {
                    Zombie::new_fuerte(
                        x,
                        y,
                    )
                }

                1 | 2 => {
                    Zombie::new_medio(
                        x,
                        y,
                    )
                }

                _ => {
                    Zombie::new(
                        x,
                        y,
                    )
                }
            }
        }

        _ => {
            match indice % 5 {
                0 | 1 => {
                    Zombie::new_fuerte(
                        x,
                        y,
                    )
                }

                2 | 3 => {
                    Zombie::new_medio(
                        x,
                        y,
                    )
                }

                _ => {
                    Zombie::new(
                        x,
                        y,
                    )
                }
            }
        }
    }
}

fn generar_horda(
    zombies: &mut Vec<Zombie>,
    mapa: &Map,
    player: &Player,
    numero_horda: i32,
) -> usize {
    let vivos =
        zombies
            .iter()
            .filter(|zombie| {
                zombie.vivo
            })
            .count();

    let maximo =
        max_zombies_para_horda(
            numero_horda,
        );

    if vivos >= maximo {
        return 0;
    }

    let cantidad =
        cantidad_horda(
            numero_horda,
        )
            .min(
                maximo
                    .saturating_sub(
                        vivos,
                    ),
            );

    let mut generados =
        0;

    for indice in 0..cantidad {
        let Some((x, y)) =
            buscar_spawn_horda(
                mapa,
                player,
                zombies,
            )
        else {
            continue;
        };

        let mut zombie =
            crear_zombie_horda(
                numero_horda,
                indice,
                x,
                y,
            );

        zombie.persiguiendo =
            true;

        zombies.push(
            zombie,
        );

        generados +=
            1;
    }

    generados
}

fn generar_refuerzos_final(
    zombies: &mut Vec<Zombie>,
    mapa: &Map,
    player: &Player,
) -> usize {
    const CANTIDAD_REFUERZOS: usize = 4;

    let mut generados =
        0;

    for indice in 0..CANTIDAD_REFUERZOS {
        let Some((x, y)) =
            buscar_spawn_horda(
                mapa,
                player,
                zombies,
            )
        else {
            continue;
        };

        let mut zombie =
            crear_zombie_horda(
                2,
                indice,
                x,
                y,
            );

        zombie.persiguiendo =
            true;

        zombies.push(zombie);
        generados += 1;
    }

    generados
}

fn disparar_licker(
    lickers: &mut [Licker],
    player: &Player,
    camera: &Camera,
    mapa: &mut Map,
) -> Option<ResultadoDisparoLicker> {
    const TOLERANCIA_ANGULO: f32 =
        0.10;

    let mut mejor_indice =
        None;

    let mut mejor_distancia =
        f32::MAX;

    for (
        indice,
        licker,
    ) in lickers
        .iter()
        .enumerate()
    {
        if !licker.vivo {
            continue;
        }

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

        if distancia <= 0.001 {
            continue;
        }

        let angulo =
            dy.atan2(dx);

        let diferencia =
            normalizar_angulo(
                angulo
                    - camera.angle,
            );

        if diferencia.abs()
            > TOLERANCIA_ANGULO
        {
            continue;
        }

        let hit =
            lanzar_rayo(
                mapa,
                player.x,
                player.y,
                angulo,
            );

        if hit.distancia
            < distancia - 6.0
        {
            continue;
        }

        let distancia_corregida =
            (
                distancia
                    * diferencia.cos()
            )
                .max(1.0);

        let factor_tamano =
            match licker.estado {
                EstadoLicker::Suelo => {
                    0.80
                }

                EstadoLicker::Trepando => {
                    0.85
                }

                EstadoLicker::Pared => {
                    0.85
                }

                EstadoLicker::Techo => {
                    0.82
                }

                EstadoLicker::Cayendo => {
                    0.90
                }
            };

        let altura_sprite =
            (
                TAMANO_CELDA
                    * ALTO_VENTANA
                        as f32
                    / distancia_corregida
            )
                * factor_tamano;

        let altura_mundo =
            TAMANO_CELDA
                * licker.altura;

        let desplazamiento =
            altura_mundo
                * ALTO_VENTANA
                    as f32
                / distancia_corregida;

        let horizonte =
            ALTO_VENTANA
                as f32
                / 2.0
                + camera
                    .vertical_offset
                    as f32;

        let parte_inferior =
            horizonte
                + altura_sprite
                    * 0.50
                - desplazamiento;

        let parte_superior =
            parte_inferior
                - altura_sprite;

        let mira_y =
            ALTO_VENTANA
                as f32
                / 2.0;

        if mira_y
            < parte_superior
            || mira_y
                > parte_inferior
        {
            continue;
        }

        if distancia
            < mejor_distancia
        {
            mejor_distancia =
                distancia;

            mejor_indice =
                Some(indice);
        }
    }

    let Some(indice) =
        mejor_indice
    else {
        return None;
    };

    let estaba_vivo =
        lickers[indice].vivo;

    let es_headshot =
        detectar_headshot_licker(
            &lickers[indice],
            mejor_distancia,
            camera,
        );

    let dano =
        if es_headshot {
            DANO_PISTOLA_LICKER
                * MULTIPLICADOR_HEADSHOT_LICKER
        } else {
            DANO_PISTOLA_LICKER
        };

    lickers[indice]
        .recibir_dano(
            dano,
        );

    let murio =
        estaba_vivo
            && !lickers[indice].vivo;

    if murio {
        soltar_objeto_licker(
            &lickers[indice],
            mapa,
        );
    }

    if murio {
        if es_headshot {
            Some(ResultadoDisparoLicker::MuerteHeadshot)
        } else {
            Some(ResultadoDisparoLicker::Muerte)
        }
    } else if es_headshot {
        Some(ResultadoDisparoLicker::Headshot {
            vida_restante: lickers[indice].vida,
        })
    } else {
        Some(ResultadoDisparoLicker::Impacto {
            vida_restante: lickers[indice].vida,
        })
    }
}

fn lickers_de_cada_tipo_derrotados(
    lickers: &[Licker],
) -> bool {
    [
        TipoLicker::Normal,
        TipoLicker::Medio,
        TipoLicker::Fuerte,
    ]
    .iter()
    .all(|tipo| {
        lickers.iter().any(|licker| {
            !licker.vivo
                && licker.tipo == *tipo
        })
    })
}

fn buscar_nemesis(
    mapa: &mut Map,
) -> Option<Nemesis> {
    for fila in 0..mapa.alto() {
        for columna in 0..mapa.ancho() {
            if mapa.celda(fila as i32, columna as i32) == 'N' {
                let x = columna as f32 * TAMANO_CELDA
                    + TAMANO_CELDA / 2.0;
                let y = fila as f32 * TAMANO_CELDA
                    + TAMANO_CELDA / 2.0;

                mapa.cambiar_celda(
                    fila as i32,
                    columna as i32,
                    ' ',
                );

                return Some(Nemesis::new(x, y));
            }
        }
    }

    None
}

fn detectar_headshot_licker(
    licker: &Licker,
    distancia: f32,
    camera: &Camera,
) -> bool {
    if distancia <= 0.001 {
        return false;
    }

    let factor_tamano = match licker.estado {
        EstadoLicker::Suelo => 0.80,
        EstadoLicker::Trepando | EstadoLicker::Pared => 0.85,
        EstadoLicker::Techo => 0.82,
        EstadoLicker::Cayendo => 0.90,
    };

    let altura_proyectada =
        (TAMANO_CELDA * ALTO_VENTANA as f32 / distancia)
            * factor_tamano;

    let desplazamiento =
        TAMANO_CELDA
            * licker.altura
            * ALTO_VENTANA as f32
            / distancia;

    let horizonte =
        ALTO_VENTANA as f32 / 2.0
            + camera.vertical_offset as f32;

    let parte_inferior =
        horizonte
            + altura_proyectada * 0.50
            - desplazamiento;

    let parte_superior =
        parte_inferior
            - altura_proyectada;

    let limite_cabeza =
        parte_superior
            + altura_proyectada * 0.30;

    let mira_y = ALTO_VENTANA as f32 / 2.0;

    mira_y >= parte_superior
        && mira_y <= limite_cabeza
}

fn atacar_licker_hacha(
    lickers: &mut [Licker],
    player: &Player,
    camera: &Camera,
    mapa: &mut Map,
) -> Option<bool> {
    const DISTANCIA_HACHA: f32 =
        45.0;

    const TOLERANCIA: f32 =
        0.45;

    let mut mejor_indice =
        None;

    let mut mejor_distancia =
        f32::MAX;

    for (
        indice,
        licker,
    ) in lickers
        .iter()
        .enumerate()
    {
        if !licker.vivo {
            continue;
        }

        if !matches!(
            licker.estado,
            EstadoLicker::Suelo
                | EstadoLicker::Cayendo
        ) {
            continue;
        }

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

        if distancia
            > DISTANCIA_HACHA
        {
            continue;
        }

        let angulo =
            dy.atan2(dx);

        let diferencia =
            normalizar_angulo(
                angulo
                    - camera.angle,
            );

        if diferencia.abs()
            > TOLERANCIA
        {
            continue;
        }

        let hit =
            lanzar_rayo(
                mapa,
                player.x,
                player.y,
                angulo,
            );

        if hit.distancia
            < distancia - 4.0
        {
            continue;
        }

        if distancia
            < mejor_distancia
        {
            mejor_distancia =
                distancia;

            mejor_indice =
                Some(indice);
        }
    }

    let Some(indice) =
        mejor_indice
    else {
        return None;
    };

    let estaba_vivo =
        lickers[indice].vivo;

    lickers[indice]
        .recibir_dano(
            DANO_HACHA_LICKER,
        );

    let murio =
        estaba_vivo
            && !lickers[indice].vivo;

    if murio {
        soltar_objeto_licker(
            &lickers[indice],
            mapa,
        );
    }

    Some(murio)
}

fn soltar_objeto_licker(
    licker: &Licker,
    mapa: &mut Map,
) {
    let columna =
        (licker.x / TAMANO_CELDA)
            .floor() as i32;

    let fila =
        (licker.y / TAMANO_CELDA)
            .floor() as i32;

    let tirada =
        rand::thread_rng()
            .gen_range(0..100);

    if mapa.nivel() == 4 {
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
                'Q',
            );
        }

        return;
    }

    if mapa.nivel() == 3 {
        if tirada < 40 {
            mapa.cambiar_celda(
                fila,
                columna,
                'K',
            );
        } else if tirada < 55 {
            mapa.cambiar_celda(
                fila,
                columna,
                'H',
            );
        } else if tirada < 70 {
            mapa.cambiar_celda(
                fila,
                columna,
                'Q',
            );
        }

        return;
    }

    if tirada < 25 {
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
    } else if tirada < 70 {
        mapa.cambiar_celda(
            fila,
            columna,
            'Q',
        );
    }
}

fn atacar_con_lanzallamas(
    zombies: &mut [Zombie],
    lickers: &mut [Licker],
    player: &Player,
    camera: &Camera,
    mapa: &mut Map,
) -> i32 {
    let mut bajas = 0;

    for zombie in zombies.iter_mut() {
        if zombie.vivo
            && objetivo_en_llama(
                player,
                camera,
                mapa,
                zombie.x,
                zombie.y,
            )
        {
            zombie.recibir_dano(DANO_LANZALLAMAS);

            if !zombie.vivo {
                procesar_muerte_zombie(zombie, mapa, false);
                bajas += 1;
            }
        }
    }

    for licker in lickers.iter_mut() {
        if licker.vivo
            && objetivo_en_llama(
                player,
                camera,
                mapa,
                licker.x,
                licker.y,
            )
        {
            licker.recibir_dano(DANO_LANZALLAMAS);

            if !licker.vivo {
                soltar_objeto_licker(licker, mapa);
                bajas += 1;
            }
        }
    }

    bajas
}

fn objetivo_en_llama(
    player: &Player,
    camera: &Camera,
    mapa: &Map,
    x: f32,
    y: f32,
) -> bool {
    let dx = x - player.x;
    let dy = y - player.y;
    let distancia = (dx * dx + dy * dy).sqrt();

    if distancia > ALCANCE_LANZALLAMAS || distancia <= 0.001 {
        return false;
    }

    let diferencia =
        normalizar_angulo(dy.atan2(dx) - camera.angle);

    if diferencia.abs() > 0.32 {
        return false;
    }

    let hit = lanzar_rayo(
        mapa,
        player.x,
        player.y,
        dy.atan2(dx),
    );

    hit.distancia >= distancia - 4.0
}

fn detener_sonidos_enemigos(
    sonidos: &AudioManager<'_>,
) {
    sonidos
        .detener_todos_enemigos();
}

fn numero_mapa_inicial(
    nivel: NivelSeleccionado,
) -> i32 {
    match nivel {
        NivelSeleccionado::Mansion => {
            1
        }

        NivelSeleccionado::Laboratorio => {
            3
        }

        NivelSeleccionado::Final => {
            4
        }
    }
}

fn nombre_nivel(
    nivel: NivelSeleccionado,
) -> &'static str {
    match nivel {
        NivelSeleccionado::Mansion => {
            "MANSION"
        }

        NivelSeleccionado::Laboratorio => {
            "LABORATORIO"
        }

        NivelSeleccionado::Final => {
            "HELIPUERTO FINAL"
        }
    }
}

fn cargar_partida(
    nivel: NivelSeleccionado,
    mapa: &mut Map,
    zombies: &mut Vec<Zombie>,
    lickers: &mut Vec<Licker>,
    tyrant: &mut Option<Tyrant>,
    nemesis: &mut Option<Nemesis>,
    misiles_nemesis: &mut Vec<ProyectilNemesis>,
    player: &mut Player,
    camera: &mut Camera,
    inventory: &mut Inventory,
    puzzle: &mut Puzzle,
    damage_effect: &mut DamageEffect,
) -> i32 {
    let numero =
        numero_mapa_inicial(
            nivel,
        );

    *mapa =
        Map::new(
            numero,
        );

    *zombies =
        crear_zombies(
            mapa,
        );

    *lickers =
        crear_lickers(
            mapa,
        );

    *tyrant =
        buscar_tyrant(
            mapa,
        );

    *nemesis =
        buscar_nemesis(
            mapa,
        );

    misiles_nemesis.clear();

    *player =
        Player::new(
            mapa,
        );

    *camera =
        Camera::new();

    *inventory =
        Inventory::new();

    *puzzle =
        Puzzle::new();

    *damage_effect =
        DamageEffect::new();

    numero
}

fn cambiar_nivel_mansion(
    nivel_destino: i32,
    portal_destino: char,
    indice_portal: usize,
    mapa: &mut Map,
    zombies: &mut Vec<Zombie>,
    lickers: &mut Vec<Licker>,
    tyrant: &mut Option<Tyrant>,
    nemesis: &mut Option<Nemesis>,
    misiles_nemesis: &mut Vec<ProyectilNemesis>,
    player: &mut Player,
    camera: &mut Camera,
    puzzle: &mut Puzzle,
    damage_effect: &mut DamageEffect,
) {
    *mapa =
        Map::new(
            nivel_destino,
        );

    *zombies =
        crear_zombies(
            mapa,
        );

    *lickers =
        crear_lickers(
            mapa,
        );

    *tyrant =
        buscar_tyrant(
            mapa,
        );

    *nemesis =
        buscar_nemesis(
            mapa,
        );

    misiles_nemesis.clear();

    *player =
        Player::new(
            mapa,
        );

    if let Some((x, y)) =
        mapa.posicion_entrada_portal(
            portal_destino,
            indice_portal,
        )
    {
        player.x =
            x;

        player.y =
            y;
    }

    *camera =
        Camera::new();

    *puzzle =
        Puzzle::new();

    *damage_effect =
        DamageEffect::new();
}

fn main() {
    let mut estado_juego =
        EstadoJuego::Jugando;

    let mut menu =
        Menu::new();

    let mut nivel_seleccionado =
        NivelSeleccionado::Mansion;

    let mut nivel_actual =
        1;

    let mut pagina_historia =
        0_usize;

    let mut modo_musica =
        ModoMusica::Normal;

    let mut enemigos_matados =
        0_i32;

    let mut bajas_normal =
        0_i32;

    let mut bajas_medio =
        0_i32;

    let mut bajas_fuerte =
        0_i32;

    let mut objetivo_completado =
        false;

    let mut antivirus_recogido =
        false;

    let mut interruptores_final =
        0_i32;

    let mut evacuacion_completada =
        false;

    let mut pantalla_great =
        false;

    let mut numero_horda =
        1_i32;

    let mut siguiente_horda =
        4_i32;

    let mut tiempo_cambio_nivel =
        0.0_f32;

    let mut mapa =
        Map::new(
            nivel_actual,
        );

    let mut zombies =
        crear_zombies(
            &mut mapa,
        );

    let mut lickers =
        crear_lickers(
            &mut mapa,
        );

    let mut tyrant =
        buscar_tyrant(
            &mut mapa,
        );

    let mut nemesis =
        buscar_nemesis(
            &mut mapa,
        );

    let mut misiles_nemesis:
        Vec<ProyectilNemesis> = Vec::new();

    let mut player =
        Player::new(
            &mapa,
        );

    let mut camera =
        Camera::new();

    let mut inventory =
        Inventory::new();

    let mut puzzle =
        Puzzle::new();

    let mut damage_effect =
        DamageEffect::new();

    let mut arma_equipada =
        ArmaActual::Pistola;

    let mut mensaje =
        "MANSION"
            .to_string();

    let mut vida_jugador =
        100;

    let mut balas_cargador =
        8;

    let mut balas_reserva =
        24;

    let mut municion_lanzallamas =
        100;

    let mut tiempo_llama =
        0.0_f32;

    let mut tiempo_disparo =
        0.0_f32;

    let mut tiempo_hachazo =
        0.0_f32;

    let mut recargando =
        false;

    let mut tiempo_recarga =
        0.0_f32;

    let mut tiempo_sonido_muerte_zombie =
        0.0_f32;

    let mut tiempo_sonido_muerte_licker =
        0.0_f32;

    const DURACION_RECARGA: f32 =
        0.8;

    const CAMBIO_FRAME_RECARGA: f32 =
        0.4;

    const DURACION_HACHAZO: f32 =
        0.22;

    let mut framebuffer =
        Framebuffer::new(
            ANCHO_VENTANA,
            ALTO_VENTANA,
        );

    framebuffer
        .set_background_color(
            Color::BLACK,
        );

    let (
        mut ventana,
        thread,
    ) =
        raylib::init()
            .size(
                ANCHO_VENTANA,
                ALTO_VENTANA,
            )
            .resizable()
            .title(
                "Survival Horror Arcade",
            )
            .build();

    ventana
        .toggle_fullscreen();

    ventana
        .set_target_fps(
            60,
        );

    ventana
        .disable_cursor();

    let audio =
        RaylibAudio::
            init_audio_device()
                .expect(
                    "No se pudo iniciar el audio",
                );

    let sonidos =
        AudioManager::new(
            &audio,
        );

    let pistol1 =
        ventana
            .load_texture(
                &thread,
                "assets/textures/pistol1.png",
            )
            .unwrap();

    let pistol2 =
        ventana
            .load_texture(
                &thread,
                "assets/textures/pistol2.png",
            )
            .unwrap();

    let pistol3 =
        ventana
            .load_texture(
                &thread,
                "assets/textures/pistol3.png",
            )
            .unwrap();

    let pistol_r =
        ventana
            .load_texture(
                &thread,
                "assets/textures/pistolR.png",
            )
            .unwrap();

    let pistol_r2 =
        ventana
            .load_texture(
                &thread,
                "assets/textures/pistolR2.png",
            )
            .unwrap();

    let axe1 =
        ventana
            .load_texture(
                &thread,
                "assets/textures/axe1.png",
            )
            .unwrap();

    let axe2 =
        ventana
            .load_texture(
                &thread,
                "assets/textures/axe2.png",
            )
            .unwrap();

    let axe3 =
        ventana
            .load_texture(
                &thread,
                "assets/textures/axe3.png",
            )
            .unwrap();

    let key_texture =
        ventana
            .load_texture(
                &thread,
                "assets/textures/key.png",
            )
            .unwrap();

    let lab_key_texture =
        ventana
            .load_texture(
                &thread,
                "assets/textures/labkey.png",
            )
            .expect(
                "No se pudo cargar labkey.png",
            );

    let ammo_texture =
        ventana
            .load_texture(
                &thread,
                "assets/textures/ammo.png",
            )
            .unwrap();

    let heal_texture =
        ventana
            .load_texture(
                &thread,
                "assets/textures/heal.png",
            )
            .unwrap();

    let zombie1 =
        ventana
            .load_texture(
                &thread,
                "assets/textures/zombie1.png",
            )
            .unwrap();

    let zombie2 =
        ventana
            .load_texture(
                &thread,
                "assets/textures/zombie2.png",
            )
            .unwrap();

    let zombie3 =
        ventana
            .load_texture(
                &thread,
                "assets/textures/zombie3.png",
            )
            .unwrap();

    let zombie_v21 =
        ventana
            .load_texture(
                &thread,
                "assets/textures/zombieV21.png",
            )
            .unwrap();

    let zombie_v22 =
        ventana
            .load_texture(
                &thread,
                "assets/textures/zombieV22.png",
            )
            .unwrap();

    let zombie_v23 =
        ventana
            .load_texture(
                &thread,
                "assets/textures/zombiev23.png",
            )
            .unwrap();

    let zombie_v31 =
        ventana
            .load_texture(
                &thread,
                "assets/textures/zombieV31.png",
            )
            .unwrap();

    let zombie_v32 =
        ventana
            .load_texture(
                &thread,
                "assets/textures/zombieV32.png",
            )
            .unwrap();

    let zombie_v33 =
        ventana
            .load_texture(
                &thread,
                "assets/textures/zombieV33.png",
            )
            .unwrap();

    let tyrant1 =
        ventana
            .load_texture(
                &thread,
                "assets/textures/tyrant1.png",
            )
            .unwrap();

    let tyrant2 =
        ventana
            .load_texture(
                &thread,
                "assets/textures/tyrant2.png",
            )
            .unwrap();

    let tyrant3 =
        ventana
            .load_texture(
                &thread,
                "assets/textures/tyrant3.png",
            )
            .unwrap();

    let licker1 =
        ventana
            .load_texture(
                &thread,
                "assets/textures/licker1.png",
            )
            .unwrap();

    let licker2 =
        ventana
            .load_texture(
                &thread,
                "assets/textures/licker2.png",
            )
            .unwrap();

    let licker3 =
        ventana
            .load_texture(
                &thread,
                "assets/textures/licker3.png",
            )
            .unwrap();

    let flamethrow_ammo_texture = ventana
        .load_texture(
            &thread,
            "assets/textures/flamethrowammo.png",
        )
        .unwrap();

    let antivirus_texture = ventana
        .load_texture(
            &thread,
            "assets/textures/antivirus.png",
        )
        .unwrap();

    let radio_part_texture = ventana
        .load_texture(
            &thread,
            "assets/textures/radiopart.png",
        )
        .unwrap();

    let flamethrow1 = ventana
        .load_texture(&thread, "assets/textures/flamethrow.png")
        .unwrap();
    let flamethrow2 = ventana
        .load_texture(&thread, "assets/textures/flamethrow2.png")
        .unwrap();
    let flamethrow3 = ventana
        .load_texture(&thread, "assets/textures/flamethrow3.png")
        .unwrap();

    let nemesis1 = ventana
        .load_texture(
            &thread,
            "assets/textures/nemesis1.png",
        )
        .unwrap();

    let nemesis2 = ventana
        .load_texture(
            &thread,
            "assets/textures/nemesis2.png",
        )
        .unwrap();

    let nemesis3 = ventana
        .load_texture(
            &thread,
            "assets/textures/nemesis3.png",
        )
        .unwrap();

    let nemesis_shoot = ventana
        .load_texture(
            &thread,
            "assets/textures/nemesisshoot.png",
        )
        .unwrap();

    let licker_v21 = ventana
        .load_texture(&thread, "assets/textures/lickerV21.png")
        .unwrap();
    let licker_v22 = ventana
        .load_texture(&thread, "assets/textures/lickerV22.png")
        .unwrap();
    let licker_v23 = ventana
        .load_texture(&thread, "assets/textures/lickerV23.png")
        .unwrap();

    let licker_v31 = ventana
        .load_texture(&thread, "assets/textures/lickerV31.png")
        .unwrap();
    let licker_v32 = ventana
        .load_texture(&thread, "assets/textures/lickerV32.png")
        .unwrap();
    let licker_v33 = ventana
        .load_texture(&thread, "assets/textures/lickerV33.png")
        .unwrap();

    let mut wall_image =
        Image::load_image(
            "assets/textures/wall.png",
        )
        .unwrap();

    let mut window_image =
        Image::load_image(
            "assets/textures/window.png",
        )
        .unwrap();

    let mut door_image =
        Image::load_image(
            "assets/textures/door.png",
        )
        .unwrap();

    let mut up_image =
        Image::load_image(
            "assets/textures/up.png",
        )
        .unwrap();

    let mut down_image =
        Image::load_image(
            "assets/textures/down.png",
        )
        .unwrap();

    let mut floor_image =
        Image::load_image(
            "assets/textures/floor.png",
        )
        .unwrap();

    let mut floor2_image =
        Image::load_image(
            "assets/textures/floor2.png",
        )
        .unwrap();

    let mut roof_image =
        Image::load_image(
            "assets/textures/roof.png",
        )
        .unwrap();

    let mut lab_wall_image =
        Image::load_image(
            "assets/textures/labwall.png",
        )
        .unwrap();

    let mut lab_door_image =
        Image::load_image(
            "assets/textures/labdoor.png",
        )
        .unwrap();

    let mut lab_floor_image =
        Image::load_image(
            "assets/textures/labfloor.png",
        )
        .unwrap();

    let mut rooftop_floor_image =
        Image::load_image(
            "assets/textures/rooftopfloor.png",
        )
        .unwrap();

    let mut metal_crate_image =
        Image::load_image(
            "assets/textures/metalcrate.png",
        )
        .unwrap();

    let mut lab_roof_image =
        Image::load_image(
            "assets/textures/labroof.png",
        )
        .unwrap();

    let mut citynight_image =
        Image::load_image(
            "assets/textures/citynight.png",
        )
        .unwrap();

    let textura_pared =
        TextureData::from_image(
            &mut wall_image,
        );

    let textura_ventana =
        TextureData::from_image(
            &mut window_image,
        );

    let textura_puerta =
        TextureData::from_image(
            &mut door_image,
        );

    let textura_subir =
        TextureData::from_image(
            &mut up_image,
        );

    let textura_bajar =
        TextureData::from_image(
            &mut down_image,
        );

    let textura_suelo =
        TextureData::from_image(
            &mut floor_image,
        );

    let textura_suelo2 =
        TextureData::from_image(
            &mut floor2_image,
        );

    let textura_techo =
        TextureData::from_image(
            &mut roof_image,
        );

    let textura_lab_pared =
        TextureData::from_image(
            &mut lab_wall_image,
        );

    let textura_lab_puerta =
        TextureData::from_image(
            &mut lab_door_image,
        );

    let textura_lab_suelo =
        TextureData::from_image(
            &mut lab_floor_image,
        );

    let textura_rooftop_suelo =
        TextureData::from_image(
            &mut rooftop_floor_image,
        );

    let textura_caja_metal =
        TextureData::from_image(
            &mut metal_crate_image,
        );

    let textura_lab_techo =
        TextureData::from_image(
            &mut lab_roof_image,
        );

    let textura_citynight =
        TextureData::from_image(
            &mut citynight_image,
        );

    let mut textura_framebuffer =
        ventana
            .load_texture_from_image(
                &thread,
                framebuffer.image(),
            )
            .unwrap();

    while !ventana
        .window_should_close()
    {
        let delta_time =
            ventana
                .get_frame_time();

        if tiempo_sonido_muerte_zombie
            > 0.0
        {
            tiempo_sonido_muerte_zombie -=
                delta_time;

            if tiempo_sonido_muerte_zombie
                <= 0.0
            {
                sonidos
                    .detener_zombie_muere();

                tiempo_sonido_muerte_zombie =
                    0.0;
            }
        }

        if tiempo_sonido_muerte_licker
            > 0.0
        {
            tiempo_sonido_muerte_licker -=
                delta_time;

            if tiempo_sonido_muerte_licker
                <= 0.0
            {
                sonidos
                    .detener_licker_muere();

                tiempo_sonido_muerte_licker =
                    0.0;
            }
        }

        if estado_juego
            == EstadoJuego::Jugando
            && vida_jugador > 0
        {
            sonidos
                .actualizar_musica(
                    nivel_seleccionado
                        != NivelSeleccionado::Mansion,
                    modo_musica,
                );
        }

        match estado_juego {
            EstadoJuego::Menu => {
                detener_sonidos_enemigos(
                    &sonidos,
                );

                ventana
                    .enable_cursor();

                match menu.update(
                    &ventana,
                ) {
                    AccionMenu::Jugar => {
                        nivel_seleccionado =
                            NivelSeleccionado::Mansion;

                        nivel_actual =
                            cargar_partida(
                                nivel_seleccionado,
                                &mut mapa,
                                &mut zombies,
                                &mut lickers,
                                &mut tyrant,
                                &mut nemesis,
                                &mut misiles_nemesis,
                                &mut player,
                                &mut camera,
                                &mut inventory,
                                &mut puzzle,
                                &mut damage_effect,
                            );

                        vida_jugador =
                            100;

                        balas_cargador =
                            8;

                        balas_reserva =
                            24;

                        municion_lanzallamas =
                            100;

                        arma_equipada =
                            ArmaActual::Pistola;

                        enemigos_matados =
                            0;

                        bajas_normal =
                            0;

                        bajas_medio =
                            0;

                        bajas_fuerte =
                            0;

                        objetivo_completado =
                            false;

                        antivirus_recogido =
                            false;

                        interruptores_final = 0;
                        evacuacion_completada = false;

                        pantalla_great =
                            false;

                        numero_horda =
                            1;

                        siguiente_horda =
                            4;

                        tiempo_cambio_nivel =
                            0.0;

                        tiempo_disparo =
                            0.0;

                        tiempo_hachazo =
                            0.0;

                        tiempo_recarga =
                            0.0;

                        tiempo_sonido_muerte_zombie =
                            0.0;

                        tiempo_sonido_muerte_licker =
                            0.0;

                        recargando =
                            false;

                        mensaje =
                            "MANSION"
                                .to_string();

                        pagina_historia =
                            0;

                        estado_juego =
                            EstadoJuego::Historia;

                        sonidos
                            .detener_musica();

                        ventana
                            .enable_cursor();
                    }

                    AccionMenu::SeleccionarNivel => {
                        estado_juego =
                            EstadoJuego::SeleccionNivel;
                    }

                    AccionMenu::Controles => {
                        estado_juego =
                            EstadoJuego::Controles;
                    }

                    AccionMenu::Opciones => {
                        estado_juego =
                            EstadoJuego::Opciones;
                    }

                    AccionMenu::Salir => {
                        sonidos
                            .detener_musica();

                        break;
                    }

                    AccionMenu::Ninguna => {}
                }

                let mut dibujo =
                    ventana
                        .begin_drawing(
                            &thread,
                        );

                menu.render_menu(
                    &mut dibujo,
                );

                continue;
            }

            EstadoJuego::SeleccionNivel => {
                detener_sonidos_enemigos(
                    &sonidos,
                );

                ventana
                    .enable_cursor();

                match menu
                    .update_seleccion_nivel(
                        &ventana,
                    )
                {
                    AccionSeleccionNivel::Elegir(
                        nivel,
                    ) => {
                        nivel_seleccionado =
                            nivel;

                        nivel_actual =
                            cargar_partida(
                                nivel_seleccionado,
                                &mut mapa,
                                &mut zombies,
                                &mut lickers,
                                &mut tyrant,
                                &mut nemesis,
                                &mut misiles_nemesis,
                                &mut player,
                                &mut camera,
                                &mut inventory,
                                &mut puzzle,
                                &mut damage_effect,
                            );

                        vida_jugador =
                            100;

                        balas_cargador =
                            8;

                        balas_reserva =
                            24;

                        municion_lanzallamas =
                            100;

                        arma_equipada =
                            ArmaActual::Pistola;

                        enemigos_matados =
                            0;

                        bajas_normal =
                            0;

                        bajas_medio =
                            0;

                        bajas_fuerte =
                            0;

                        objetivo_completado =
                            false;

                        antivirus_recogido =
                            false;

                        interruptores_final = 0;
                        evacuacion_completada = false;

                        pantalla_great =
                            false;

                        numero_horda =
                            1;

                        siguiente_horda =
                            4;

                        tiempo_cambio_nivel =
                            0.0;

                        tiempo_disparo =
                            0.0;

                        tiempo_hachazo =
                            0.0;

                        tiempo_recarga =
                            0.0;

                        tiempo_sonido_muerte_zombie =
                            0.0;

                        tiempo_sonido_muerte_licker =
                            0.0;

                        recargando =
                            false;

                        mensaje =
                            nombre_nivel(
                                nivel_seleccionado,
                            )
                                .to_string();

                        sonidos
                            .detener_musica();

                        pagina_historia =
                            0;

                        estado_juego =
                            EstadoJuego::Historia;

                        ventana
                            .enable_cursor();
                    }

                    AccionSeleccionNivel::Volver => {
                        estado_juego =
                            EstadoJuego::Menu;
                    }

                    AccionSeleccionNivel::Ninguna => {}
                }

                let mut dibujo =
                    ventana
                        .begin_drawing(
                            &thread,
                        );

                menu.render_seleccion_nivel(
                    &mut dibujo,
                );

                continue;
            }

            EstadoJuego::Controles => {
                detener_sonidos_enemigos(
                    &sonidos,
                );

                ventana
                    .enable_cursor();

                if ventana
                    .is_key_pressed(
                        KeyboardKey::KEY_BACKSPACE,
                    )
                    || gamepad::volver(&ventana)
                {
                    estado_juego =
                        EstadoJuego::Menu;
                }

                let control_conectado =
                    gamepad::conectado(&ventana);

                let mut dibujo =
                    ventana
                        .begin_drawing(
                            &thread,
                        );

                menu.render_controles(
                    &mut dibujo,
                    control_conectado,
                );

                continue;
            }

            EstadoJuego::Historia => {
                detener_sonidos_enemigos(
                    &sonidos,
                );

                if modo_musica == ModoMusica::BadBlood {
                    sonidos.actualizar_musica(
                        false,
                        modo_musica,
                    );
                } else {
                    sonidos.detener_musica();
                }

                ventana
                    .enable_cursor();

                if ventana.is_key_pressed(
                    KeyboardKey::KEY_BACKSPACE,
                ) || gamepad::volver(&ventana) {
                    pagina_historia = 0;
                    estado_juego = EstadoJuego::Menu;
                } else if ventana.is_key_pressed(
                    KeyboardKey::KEY_ENTER,
                ) || gamepad::aceptar(&ventana) {
                    if pagina_historia == 0 {
                        pagina_historia = 1;
                    } else {
                        pagina_historia = 0;
                        estado_juego = EstadoJuego::Jugando;
                        sonidos.iniciar_musica(
                            nivel_seleccionado
                                != NivelSeleccionado::Mansion,
                            modo_musica,
                        );
                        ventana.disable_cursor();
                    }
                }

                let mut dibujo =
                    ventana.begin_drawing(&thread);

                menu.render_historia(
                    &mut dibujo,
                    pagina_historia,
                    nivel_seleccionado,
                );

                continue;
            }

            EstadoJuego::HistoriaFinal => {
                detener_sonidos_enemigos(&sonidos);
                sonidos.detener_musica();
                ventana.enable_cursor();

                if ventana.is_key_pressed(
                    KeyboardKey::KEY_ENTER,
                ) || gamepad::aceptar(&ventana)
                {
                    pantalla_great = true;
                    estado_juego = EstadoJuego::Jugando;
                } else if ventana.is_key_pressed(
                    KeyboardKey::KEY_BACKSPACE,
                ) || gamepad::volver(&ventana)
                {
                    estado_juego = EstadoJuego::Menu;
                }

                let mut dibujo =
                    ventana.begin_drawing(&thread);

                menu.render_historia_final(&mut dibujo);

                continue;
            }

            EstadoJuego::Opciones => {
                sonidos.detener_musica();
                ventana.enable_cursor();

                if menu.update_opciones(
                    &ventana,
                    &mut modo_musica,
                ) {
                    estado_juego = EstadoJuego::Menu;
                }

                let mut dibujo =
                    ventana.begin_drawing(&thread);

                menu.render_opciones(
                    &mut dibujo,
                    modo_musica,
                );

                continue;
            }

            EstadoJuego::Jugando => {}
        }

        if pantalla_great {
            detener_sonidos_enemigos(
                &sonidos,
            );

            sonidos
                .actualizar_musica(
                    nivel_seleccionado
                        != NivelSeleccionado::Mansion,
                    modo_musica,
                );

            ventana
                .enable_cursor();

            if ventana.is_key_pressed(KeyboardKey::KEY_ENTER)
                || gamepad::aceptar(&ventana)
            {
                std::process::exit(10);
            }
            if ventana.is_key_pressed(KeyboardKey::KEY_BACKSPACE)
                || gamepad::volver(&ventana)
            {
                break;
            }

            if ventana
                .is_key_pressed(
                    KeyboardKey::KEY_ENTER,
                )
                || gamepad::aceptar(&ventana)
            {
                if nivel_seleccionado
                    != NivelSeleccionado::Final
                {
                    nivel_seleccionado =
                        match nivel_seleccionado {
                            NivelSeleccionado::Mansion =>
                                NivelSeleccionado::Laboratorio,
                            NivelSeleccionado::Laboratorio =>
                                NivelSeleccionado::Final,
                            NivelSeleccionado::Final => unreachable!(),
                        };

                    nivel_actual = cargar_partida(
                        nivel_seleccionado,
                        &mut mapa,
                        &mut zombies,
                        &mut lickers,
                        &mut tyrant,
                        &mut nemesis,
                        &mut misiles_nemesis,
                        &mut player,
                        &mut camera,
                        &mut inventory,
                        &mut puzzle,
                        &mut damage_effect,
                    );

                    vida_jugador = 100;
                    balas_cargador = 8;
                    balas_reserva = 24;
                    municion_lanzallamas = 100;
                    arma_equipada = ArmaActual::Pistola;
                    enemigos_matados = 0;
                    bajas_normal = 0;
                    bajas_medio = 0;
                    bajas_fuerte = 0;
                    objetivo_completado = false;
                    antivirus_recogido = false;
                    interruptores_final = 0;
                    evacuacion_completada = false;
                    pantalla_great = false;
                    numero_horda = 1;
                    siguiente_horda = 4;
                    recargando = false;
                    mensaje = nombre_nivel(nivel_seleccionado)
                        .to_string();
                    pagina_historia = 0;
                    estado_juego = EstadoJuego::Historia;
                    if modo_musica == ModoMusica::Normal {
                        sonidos.detener_musica();
                    }
                    ventana.enable_cursor();
                } else {
                    pantalla_great = false;
                    estado_juego = EstadoJuego::Menu;
                    sonidos.detener_musica();
                    ventana.enable_cursor();
                }

                continue;
            }

            if ventana
                .is_key_pressed(
                    KeyboardKey::KEY_BACKSPACE,
                )
                || gamepad::volver(&ventana)
            {
                pantalla_great =
                    false;

                estado_juego =
                    EstadoJuego::Menu;

                sonidos
                    .detener_musica();

                ventana
                    .enable_cursor();

                continue;
            }

            let mut dibujo =
                ventana
                    .begin_drawing(
                        &thread,
                    );

            dibujo
                .clear_background(
                    Color::new(
                        8,
                        8,
                        8,
                        255,
                    ),
                );

            let sw =
                dibujo
                    .get_screen_width();

            let sh =
                dibujo
                    .get_screen_height();

            let titulo =
                "GREAT!";

            let subtitulo =
                "OBJETIVO COMPLETADO";

            let normal =
                format!(
                    "ZOMBIES NORMALES: {}/{}",
                    bajas_normal
                        .min(META_NORMAL),
                    META_NORMAL,
                );

            let medio =
                format!(
                    "ZOMBIES MEDIOS: {}/{}",
                    bajas_medio
                        .min(META_MEDIO),
                    META_MEDIO,
                );

            let fuerte =
                format!(
                    "ZOMBIES FUERTES: {}/{}",
                    bajas_fuerte
                        .min(META_FUERTE),
                    META_FUERTE,
                );

            let arcade =
                if nivel_seleccionado
                    == NivelSeleccionado::Final
                {
                    "ENTER - VER EPILOGO"
                } else {
                    "ENTER - SIGUIENTE NIVEL"
                };

            let volver =
                "BACKSPACE - MENU PRINCIPAL";

            let titulo_w =
                dibujo.measure_text(
                    titulo,
                    80,
                );

            dibujo.draw_text(
                titulo,
                sw / 2
                    - titulo_w / 2,
                sh / 2
                    - 190,
                80,
                Color::GOLD,
            );

            let sub_w =
                dibujo.measure_text(
                    subtitulo,
                    28,
                );

            dibujo.draw_text(
                subtitulo,
                sw / 2
                    - sub_w / 2,
                sh / 2
                    - 95,
                28,
                Color::WHITE,
            );

            let normal_w =
                dibujo.measure_text(
                    &normal,
                    24,
                );

            dibujo.draw_text(
                &normal,
                sw / 2
                    - normal_w / 2,
                sh / 2
                    - 30,
                24,
                Color::LIGHTGRAY,
            );

            let medio_w =
                dibujo.measure_text(
                    &medio,
                    24,
                );

            dibujo.draw_text(
                &medio,
                sw / 2
                    - medio_w / 2,
                sh / 2
                    + 5,
                24,
                Color::LIGHTGRAY,
            );

            let fuerte_w =
                dibujo.measure_text(
                    &fuerte,
                    24,
                );

            dibujo.draw_text(
                &fuerte,
                sw / 2
                    - fuerte_w / 2,
                sh / 2
                    + 40,
                24,
                Color::LIGHTGRAY,
            );

            let arcade_w =
                dibujo.measure_text(
                    arcade,
                    24,
                );

            dibujo.draw_text(
                arcade,
                sw / 2
                    - arcade_w / 2,
                sh / 2
                    + 115,
                24,
                Color::GREEN,
            );

            let volver_w =
                dibujo.measure_text(
                    volver,
                    20,
                );

            dibujo.draw_text(
                volver,
                sw / 2
                    - volver_w / 2,
                sh / 2
                    + 155,
                20,
                Color::GRAY,
            );

            continue;
        }

        if ventana
            .is_key_pressed(
                KeyboardKey::KEY_BACKSPACE,
            )
            || gamepad::volver(&ventana)
        {
            detener_sonidos_enemigos(
                &sonidos,
            );

            sonidos
                .detener_musica();

            break;
        }

        tiempo_cambio_nivel =
            (
                tiempo_cambio_nivel
                    - delta_time
            )
                .max(0.0);

        tiempo_disparo =
            (
                tiempo_disparo
                    - delta_time
            )
                .max(0.0);

        tiempo_hachazo =
            (
                tiempo_hachazo
                    - delta_time
            )
                .max(0.0);

        tiempo_llama =
            (tiempo_llama - delta_time)
                .max(0.0);

        if recargando
            && vida_jugador > 0
        {
            tiempo_recarga -=
                delta_time;

            if tiempo_recarga <= 0.0 {
                recargando =
                    false;

                tiempo_recarga =
                    0.0;

                let faltantes =
                    8
                        - balas_cargador;

                let cantidad =
                    faltantes
                        .min(
                            balas_reserva,
                        );

                balas_cargador +=
                    cantidad;

                balas_reserva -=
                    cantidad;

                mensaje =
                    "Arma recargada"
                        .to_string();
            }
        }

        damage_effect
            .update(
                delta_time,
            );

        if vida_jugador > 0 {
            camera.update(
                &ventana,
                delta_time,
                2.0,
            );

            player.update(
                &ventana,
                &mapa,
                camera.angle,
                delta_time,
            );
        }

        let bloqueando =
            vida_jugador > 0
                && arma_equipada == ArmaActual::Hacha
                && tiempo_hachazo <= 0.0
                && (ventana.is_mouse_button_down(
                        MouseButton::MOUSE_BUTTON_RIGHT,
                    )
                    || gamepad::pulsado(
                        &ventana,
                        GamepadButton::GAMEPAD_BUTTON_LEFT_TRIGGER_2,
                    ));

        if vida_jugador > 0 {
            for zombie
                in &mut zombies
            {
                let dano =
                    zombie.update(
                        &player,
                        &mapa,
                        delta_time,
                    );

                if dano > 0 {
                    let bloqueo_valido =
                        bloqueando
                            && puede_bloquear_ataque(
                                zombie,
                                &player,
                                &camera,
                            );

                    let dano_final =
                        if bloqueo_valido {
                            3
                        } else {
                            dano
                        };

                    vida_jugador -=
                        dano_final;

                    vida_jugador =
                        vida_jugador
                            .max(0);

                    if bloqueo_valido {
                        sonidos
                            .bloqueo_hacha();

                        mensaje =
                            "Ataque bloqueado"
                                .to_string();
                    } else {
                        sonidos
                            .dano();

                        mensaje =
                            format!(
                                "Dano recibido: {}",
                                dano_final,
                            );
                    }

                    damage_effect
                        .activar();
                }
            }
        }

        if vida_jugador > 0 {
            for licker
                in &mut lickers
            {
                let dano =
                    licker.update(
                        &player,
                        &mapa,
                        delta_time,
                        nivel_seleccionado
                            != NivelSeleccionado::Final,
                    );

                if dano > 0 {
                    let dano_final =
                        if bloqueando {
                            5
                        } else {
                            dano
                        };

                    vida_jugador -=
                        dano_final;

                    vida_jugador =
                        vida_jugador
                            .max(0);

                    if bloqueando {
                        sonidos
                            .bloqueo_hacha();

                        mensaje =
                            "Bloqueaste al LICKER"
                                .to_string();
                    } else {
                        sonidos
                            .dano();

                        mensaje =
                            format!(
                                "LICKER -{} VIDA",
                                dano_final,
                            );
                    }

                    damage_effect
                        .activar();
                }
            }
        }

        let hay_normal =
            zombies
                .iter()
                .any(|zombie| {
                    zombie.vivo
                        && zombie.persiguiendo
                        && zombie.tipo
                            == TipoZombie::Normal
                });

        let hay_medio =
            zombies
                .iter()
                .any(|zombie| {
                    zombie.vivo
                        && zombie.persiguiendo
                        && zombie.tipo
                            == TipoZombie::Medio
                });

        let hay_fuerte =
            zombies
                .iter()
                .any(|zombie| {
                    zombie.vivo
                        && zombie.persiguiendo
                        && zombie.tipo
                            == TipoZombie::Fuerte
                });

        if vida_jugador > 0
            && hay_normal
        {
            sonidos
                .zombie();
        } else {
            sonidos
                .detener_zombie();
        }

        if vida_jugador > 0
            && hay_medio
        {
            sonidos
                .zombie_medio();
        } else {
            sonidos
                .detener_zombie_medio();
        }

        if vida_jugador > 0
            && hay_fuerte
        {
            sonidos
                .zombie_fuerte();
        } else {
            sonidos
                .detener_zombie_fuerte();
        }

        let licker_cerca =
            lickers
                .iter()
                .any(|licker| {
                    licker.vivo
                        && calcular_distancia(
                            player.x,
                            player.y,
                            licker.x,
                            licker.y,
                        ) <= 450.0
                });

        if vida_jugador > 0
            && licker_cerca
        {
            sonidos
                .licker();
        } else {
            sonidos
                .detener_licker();
        }

        if vida_jugador > 0 {
            if let Some(
                tyrant_actual,
            ) =
                tyrant.as_mut()
            {
                let distancia_tyrant =
                    calcular_distancia(
                        player.x,
                        player.y,
                        tyrant_actual.x,
                        tyrant_actual.y,
                    );

                if distancia_tyrant
                    <= 450.0
                {
                    sonidos
                        .tyrant();
                } else {
                    sonidos
                        .detener_tyrant();
                }

                let dano =
                    tyrant_actual.update(
                        &player,
                        &mapa,
                        delta_time,
                    );

                if dano > 0 {
                    let dano_final =
                        if bloqueando {
                            8
                        } else {
                            dano
                        };

                    vida_jugador -=
                        dano_final;

                    vida_jugador =
                        vida_jugador
                            .max(0);

                    if bloqueando {
                        sonidos
                            .bloqueo_hacha();

                        mensaje =
                            "Bloqueaste al TYRANT"
                                .to_string();
                    } else {
                        sonidos
                            .dano();

                        mensaje =
                            format!(
                                "TYRANT -{} VIDA",
                                dano_final,
                            );
                    }

                    damage_effect
                        .activar();
                }
            } else {
                sonidos
                    .detener_tyrant();
            }
        } else {
            sonidos
                .detener_tyrant();
        }

        if vida_jugador > 0 {
            if let Some(nemesis_actual) =
                nemesis.as_mut()
            {
                let distancia_nemesis =
                    calcular_distancia(
                        player.x,
                        player.y,
                        nemesis_actual.x,
                        nemesis_actual.y,
                    );

                if distancia_nemesis <= 450.0 {
                    sonidos.nemesis();
                } else {
                    sonidos.detener_nemesis();
                }

                let dano = nemesis_actual.update(
                    &player,
                    &mapa,
                    delta_time,
                );

                if let Some(misil) =
                    nemesis_actual
                        .intentar_disparar_misil(
                            &player,
                        )
                {
                    misiles_nemesis.push(misil);
                    sonidos.disparo_nemesis();
                }

                if dano > 0 {
                    let dano_final =
                        if bloqueando { 8 } else { dano };

                    vida_jugador -= dano_final;
                    vida_jugador = vida_jugador.max(0);

                    if bloqueando {
                        sonidos.bloqueo_hacha();
                        mensaje =
                            "Bloqueaste a NEMESIS".to_string();
                    } else {
                        sonidos.dano();
                        mensaje = format!(
                            "NEMESIS -{} VIDA",
                            dano_final,
                        );
                    }

                    damage_effect.activar();
                }
            } else {
                sonidos.detener_nemesis();
            }
        } else {
            sonidos.detener_nemesis();
        }

        if vida_jugador > 0 {
            let mut dano_misiles = 0;

            for misil in &mut misiles_nemesis {
                if misil.vivo {
                    dano_misiles += misil.update(
                        &player,
                        &mapa,
                        delta_time,
                    );
                }
            }

            misiles_nemesis.retain(|misil| misil.vivo);

            if dano_misiles > 0 {
                let dano_final =
                    if bloqueando { 8 } else { dano_misiles };

                vida_jugador =
                    (vida_jugador - dano_final).max(0);

                if bloqueando {
                    sonidos.bloqueo_hacha();
                    mensaje =
                        "Bloqueaste el misil de NEMESIS".to_string();
                } else {
                    sonidos.dano();
                    mensaje = format!(
                        "MISIL DE NEMESIS -{} VIDA",
                        dano_final,
                    );
                }

                damage_effect.activar();
            }
        }

        if vida_jugador > 0 {
            match recoger_objetos_cercanos(
                &mut mapa,
                &player,
                &mut inventory,
                &mut puzzle,
                vida_jugador,
            ) {
                InteractionResult::LlaveRecogida => {
                    sonidos
                        .llave();

                    mensaje =
                        "Recogiste una llave"
                            .to_string();
                }

                InteractionResult::MunicionRecogida(
                    cantidad,
                ) => {
                    sonidos
                        .recoger_municion();

                    balas_reserva +=
                        cantidad;

                    mensaje =
                        format!(
                            "+{} balas",
                            cantidad,
                        );
                }

                InteractionResult::MunicionLanzallamasRecogida(
                    cantidad,
                ) => {
                    sonidos.recoger_municion();
                    municion_lanzallamas += cantidad;
                    mensaje = format!(
                        "+{} combustible",
                        cantidad,
                    );
                }

                InteractionResult::AntivirusRecogido => {
                    antivirus_recogido = true;
                    mensaje =
                        "ANTIVIRUS CONSEGUIDO".to_string();
                }

                InteractionResult::PiezaRadioRecogida => {
                    interruptores_final =
                        (interruptores_final + 1)
                            .min(META_PIEZAS_RADIO);
                    mensaje = format!(
                        "PIEZAS DE RADIO {}/{}",
                        interruptores_final,
                        META_PIEZAS_RADIO,
                    );
                }

                InteractionResult::RadioEncontrada => {
                    let enemigos_listos =
                        enemigos_matados >= META_ENEMIGOS_FINAL;
                    let lickers_listos =
                        lickers_de_cada_tipo_derrotados(&lickers);
                    let piezas_listas =
                        interruptores_final >= META_PIEZAS_RADIO;

                    if enemigos_listos
                        && lickers_listos
                        && piezas_listas
                    {
                        evacuacion_completada = true;
                        mensaje =
                            "HELICOPTERO EN CAMINO".to_string();
                    } else {
                        mensaje = format!(
                            "FALTA: BAJAS {}/{} | RADIO {}/{} | LICKERS {}",
                            enemigos_matados.min(META_ENEMIGOS_FINAL),
                            META_ENEMIGOS_FINAL,
                            interruptores_final,
                            META_PIEZAS_RADIO,
                            if lickers_listos { "OK" } else { "3 TIPOS" },
                        );
                    }
                }

                InteractionResult::CuracionRecogida(
                    cantidad,
                ) => {
                    sonidos
                        .curacion();

                    vida_jugador =
                        (
                            vida_jugador
                                + cantidad
                        )
                            .min(100);

                    mensaje =
                        format!(
                            "+{} vida",
                            cantidad,
                        );
                }

                _ => {}
            }
        }

        if vida_jugador > 0 {
            if ventana
                .is_key_pressed(
                    KeyboardKey::KEY_ONE,
                )
                || gamepad::arriba(&ventana)
            {
                arma_equipada =
                    ArmaActual::Pistola;

                tiempo_hachazo =
                    0.0;

                mensaje =
                    "Pistola equipada"
                        .to_string();
            }

            if ventana
                .is_key_pressed(
                    KeyboardKey::KEY_TWO,
                )
                || gamepad::izquierda(&ventana)
            {
                arma_equipada =
                    ArmaActual::Hacha;

                recargando =
                    false;

                tiempo_recarga =
                    0.0;

                tiempo_disparo =
                    0.0;

                mensaje =
                    "Hacha equipada"
                        .to_string();
            }

            if ventana
                .is_key_pressed(
                    KeyboardKey::KEY_THREE,
                )
                || gamepad::derecha(&ventana)
            {
                arma_equipada =
                    ArmaActual::Lanzallamas;
                recargando = false;
                tiempo_recarga = 0.0;
                mensaje =
                    "Lanzallamas equipado".to_string();
            }

            if ventana
                .is_key_pressed(
                    KeyboardKey::KEY_E,
                )
                || gamepad::aceptar(&ventana)
            {
                match interactuar(
                    &mut mapa,
                    &player,
                    &camera,
                    &mut inventory,
                    &mut puzzle,
                ) {
                    InteractionResult::PuertaAbierta => {
                        sonidos
                            .puerta();

                        mensaje =
                            "Abriste la puerta"
                                .to_string();
                    }

                    InteractionResult::PuertaCerrada => {
                        mensaje =
                            "Necesitas una llave"
                                .to_string();
                    }

                    InteractionResult::SubirNivel(
                        indice,
                    ) => {
                        if nivel_seleccionado
                            == NivelSeleccionado::Mansion
                            && nivel_actual == 1
                            && tiempo_cambio_nivel
                                <= 0.0
                        {
                            detener_sonidos_enemigos(
                                &sonidos,
                            );

                            nivel_actual =
                                2;

                            cambiar_nivel_mansion(
                                2,
                                'B',
                                indice,
                                &mut mapa,
                                &mut zombies,
                                &mut lickers,
                                &mut tyrant,
                                &mut nemesis,
                                &mut misiles_nemesis,
                                &mut player,
                                &mut camera,
                                &mut puzzle,
                                &mut damage_effect,
                            );

                            tiempo_cambio_nivel =
                                1.0;

                            tiempo_disparo =
                                0.0;

                            tiempo_hachazo =
                                0.0;

                            tiempo_recarga =
                                0.0;

                            tiempo_sonido_muerte_zombie =
                                0.0;

                            tiempo_sonido_muerte_licker =
                                0.0;

                            recargando =
                                false;

                            mensaje =
                                "MANSION - PISO 2"
                                    .to_string();
                        }
                    }

                    InteractionResult::BajarNivel(
                        indice,
                    ) => {
                        if nivel_seleccionado
                            == NivelSeleccionado::Mansion
                            && nivel_actual == 2
                            && tiempo_cambio_nivel
                                <= 0.0
                        {
                            detener_sonidos_enemigos(
                                &sonidos,
                            );

                            nivel_actual =
                                1;

                            cambiar_nivel_mansion(
                                1,
                                'X',
                                indice,
                                &mut mapa,
                                &mut zombies,
                                &mut lickers,
                                &mut tyrant,
                                &mut nemesis,
                                &mut misiles_nemesis,
                                &mut player,
                                &mut camera,
                                &mut puzzle,
                                &mut damage_effect,
                            );

                            tiempo_cambio_nivel =
                                1.0;

                            tiempo_disparo =
                                0.0;

                            tiempo_hachazo =
                                0.0;

                            tiempo_recarga =
                                0.0;

                            tiempo_sonido_muerte_zombie =
                                0.0;

                            tiempo_sonido_muerte_licker =
                                0.0;

                            recargando =
                                false;

                            mensaje =
                                "MANSION - PISO 1"
                                    .to_string();
                        }
                    }

                    _ => {}
                }
            }
        }

        let apuntando =
            vida_jugador > 0
                && matches!(
                    arma_equipada,
                    ArmaActual::Pistola
                        | ArmaActual::Lanzallamas
                )
                && !recargando
                && (ventana.is_mouse_button_down(
                        MouseButton::MOUSE_BUTTON_RIGHT,
                    )
                    || gamepad::pulsado(
                        &ventana,
                        GamepadButton::GAMEPAD_BUTTON_LEFT_TRIGGER_2,
                    ));

        let disparando_llama =
            vida_jugador > 0
                && arma_equipada
                    == ArmaActual::Lanzallamas
                && (ventana.is_mouse_button_down(
                        MouseButton::MOUSE_BUTTON_LEFT,
                    )
                    || gamepad::pulsado(
                        &ventana,
                        GamepadButton::GAMEPAD_BUTTON_RIGHT_TRIGGER_2,
                    ))
                && municion_lanzallamas > 0;

        if disparando_llama {
            sonidos.lanzallamas();

            if tiempo_llama <= 0.0 {
                tiempo_llama = 0.10;
                municion_lanzallamas -= 1;

                let normal_antes =
                    contar_vivos_tipo(&zombies, TipoZombie::Normal);
                let medio_antes =
                    contar_vivos_tipo(&zombies, TipoZombie::Medio);
                let fuerte_antes =
                    contar_vivos_tipo(&zombies, TipoZombie::Fuerte);

                let bajas = atacar_con_lanzallamas(
                    &mut zombies,
                    &mut lickers,
                    &player,
                    &camera,
                    &mut mapa,
                );

                actualizar_bajas_por_tipo(
                    normal_antes,
                    medio_antes,
                    fuerte_antes,
                    &zombies,
                    &mut bajas_normal,
                    &mut bajas_medio,
                    &mut bajas_fuerte,
                );

                if bajas > 0 {
                    enemigos_matados += bajas;
                    sonidos.detener_zombie_muere();
                    sonidos.zombie_muere();
                    tiempo_sonido_muerte_zombie =
                        DURACION_SONIDO_MUERTE;
                    mensaje = format!(
                        "LLAMAS - BAJAS {}",
                        enemigos_matados,
                    );
                }
            }
        } else {
            sonidos.detener_lanzallamas();

            if vida_jugador > 0
                && arma_equipada == ArmaActual::Lanzallamas
                && municion_lanzallamas <= 0
                && (ventana.is_mouse_button_pressed(
                        MouseButton::MOUSE_BUTTON_LEFT,
                    )
                    || gamepad::presionado(
                        &ventana,
                        GamepadButton::GAMEPAD_BUTTON_RIGHT_TRIGGER_2,
                    ))
            {
                sonidos.sin_municion();
                mensaje =
                    "Sin combustible".to_string();
            }
        }

        if vida_jugador > 0
            && (ventana
                    .is_mouse_button_pressed(
                        MouseButton::MOUSE_BUTTON_LEFT,
                    )
                || gamepad::presionado(
                    &ventana,
                    GamepadButton::GAMEPAD_BUTTON_RIGHT_TRIGGER_2,
                ))
        {
            match arma_equipada {
                ArmaActual::Pistola => {
                    if !recargando {
                        if balas_cargador > 0 {
                            balas_cargador -=
                                1;

                            tiempo_disparo =
                                0.12;

                            sonidos
                                .disparo();

                            let normal_antes =
                                contar_vivos_tipo(
                                    &zombies,
                                    TipoZombie::Normal,
                                );

                            let medio_antes =
                                contar_vivos_tipo(
                                    &zombies,
                                    TipoZombie::Medio,
                                );

                            let fuerte_antes =
                                contar_vivos_tipo(
                                    &zombies,
                                    TipoZombie::Fuerte,
                                );

                            let vivos_antes =
                                zombies
                                    .iter()
                                    .filter(|z| z.vivo)
                                    .count();

                            let resultado =
                                disparar(
                                    &mut zombies,
                                    &player,
                                    &camera,
                                    &mut mapa,
                                );

                            actualizar_bajas_por_tipo(
                                normal_antes,
                                medio_antes,
                                fuerte_antes,
                                &zombies,
                                &mut bajas_normal,
                                &mut bajas_medio,
                                &mut bajas_fuerte,
                            );

                            let vivos_despues =
                                zombies
                                    .iter()
                                    .filter(|z| z.vivo)
                                    .count();

                            if vivos_despues
                                < vivos_antes
                            {
                                enemigos_matados +=
                                    (
                                        vivos_antes
                                            - vivos_despues
                                    ) as i32;

                                sonidos
                                    .detener_zombie_muere();

                                sonidos
                                    .zombie_muere();

                                tiempo_sonido_muerte_zombie =
                                    DURACION_SONIDO_MUERTE;
                            }

                            let fallo_zombie =
                                matches!(
                                    resultado,
                                    ShotResult::Miss
                                );

                            if fallo_zombie {
                                if let Some(resultado_licker) =
                                    disparar_licker(
                                        &mut lickers,
                                        &player,
                                        &camera,
                                        &mut mapa,
                                    )
                                {
                                    match resultado_licker {
                                        ResultadoDisparoLicker::Muerte
                                        | ResultadoDisparoLicker::MuerteHeadshot => {
                                            enemigos_matados += 1;

                                            sonidos.detener_licker_muere();
                                            sonidos.licker_muere();
                                            tiempo_sonido_muerte_licker =
                                                DURACION_SONIDO_MUERTE;

                                            mensaje = match resultado_licker {
                                                ResultadoDisparoLicker::MuerteHeadshot => format!(
                                                    "HEADSHOT - LICKER ELIMINADO - BAJAS {}",
                                                    enemigos_matados,
                                                ),
                                                _ => format!(
                                                    "LICKER ELIMINADO - BAJAS {}",
                                                    enemigos_matados,
                                                ),
                                            };
                                        }
                                        ResultadoDisparoLicker::Headshot { vida_restante } => {
                                            mensaje = format!(
                                                "HEADSHOT AL LICKER - {} HP",
                                                vida_restante,
                                            );
                                        }
                                        ResultadoDisparoLicker::Impacto { vida_restante } => {
                                            mensaje = format!(
                                                "Impacto al LICKER - {} HP",
                                                vida_restante,
                                            );
                                        }
                                    }
                                } else {
                                    mensaje =
                                        "Disparo fallido"
                                            .to_string();
                                }
                            } else {
                                mensaje =
                                    match resultado {
                                        ShotResult::Miss => {
                                            "Disparo fallido"
                                                .to_string()
                                        }

                                        ShotResult::Hit {
                                            vida_restante,
                                        } => {
                                            format!(
                                                "Impacto - {} HP",
                                                vida_restante,
                                            )
                                        }

                                        ShotResult::HeadshotHit {
                                            vida_restante,
                                        } => {
                                            format!(
                                                "HEADSHOT x2 - {} HP",
                                                vida_restante,
                                            )
                                        }

                                        ShotResult::Kill => {
                                            format!(
                                                "BAJAS: {}",
                                                enemigos_matados,
                                            )
                                        }

                                        ShotResult::KillConLlave => {
                                            format!(
                                                "BAJAS: {} - LLAVE",
                                                enemigos_matados,
                                            )
                                        }

                                        ShotResult::HeadshotKill => {
                                            format!(
                                                "HEADSHOT x2 - BAJA {}",
                                                enemigos_matados,
                                            )
                                        }

                                        ShotResult::HeadshotKillConLlave => {
                                            format!(
                                                "HEADSHOT x2 - BAJA {} - LLAVE",
                                                enemigos_matados,
                                            )
                                        }
                                    };
                            }
                        } else {
                            sonidos
                                .sin_municion();

                            mensaje =
                                "Sin municion"
                                    .to_string();
                        }
                    }
                }

                ArmaActual::Hacha => {
                    if tiempo_hachazo <= 0.0
                        && !bloqueando
                    {
                        tiempo_hachazo =
                            DURACION_HACHAZO;

                        sonidos
                            .hachazo();

                        let normal_antes =
                            contar_vivos_tipo(
                                &zombies,
                                TipoZombie::Normal,
                            );

                        let medio_antes =
                            contar_vivos_tipo(
                                &zombies,
                                TipoZombie::Medio,
                            );

                        let fuerte_antes =
                            contar_vivos_tipo(
                                &zombies,
                                TipoZombie::Fuerte,
                            );

                        let vivos_antes =
                            zombies
                                .iter()
                                .filter(|z| z.vivo)
                                .count();

                        let resultado =
                            atacar_con_hacha(
                                &mut zombies,
                                &player,
                                &camera,
                                &mut mapa,
                            );

                        actualizar_bajas_por_tipo(
                            normal_antes,
                            medio_antes,
                            fuerte_antes,
                            &zombies,
                            &mut bajas_normal,
                            &mut bajas_medio,
                            &mut bajas_fuerte,
                        );

                        let vivos_despues =
                            zombies
                                .iter()
                                .filter(|z| z.vivo)
                                .count();

                        if vivos_despues
                            < vivos_antes
                        {
                            enemigos_matados +=
                                (
                                    vivos_antes
                                        - vivos_despues
                                ) as i32;

                            sonidos
                                .detener_zombie_muere();

                            sonidos
                                .zombie_muere();

                            tiempo_sonido_muerte_zombie =
                                DURACION_SONIDO_MUERTE;
                        }

                        let fallo_zombie =
                            matches!(
                                resultado,
                                ShotResult::Miss
                            );

                        if fallo_zombie {
                            if let Some(murio) =
                                atacar_licker_hacha(
                                    &mut lickers,
                                    &player,
                                    &camera,
                                    &mut mapa,
                                )
                            {
                                if murio {
                                    enemigos_matados +=
                                        1;

                                    sonidos
                                        .detener_licker_muere();

                                    sonidos
                                        .licker_muere();

                                    tiempo_sonido_muerte_licker =
                                        DURACION_SONIDO_MUERTE;

                                    mensaje =
                                        format!(
                                            "LICKER ELIMINADO - BAJAS {}",
                                            enemigos_matados,
                                        );
                                } else {
                                    mensaje =
                                        "Golpe al LICKER"
                                            .to_string();
                                }
                            } else {
                                mensaje =
                                    "Hachazo fallido"
                                        .to_string();
                            }
                        } else {
                            mensaje =
                                match resultado {
                                    ShotResult::Miss => {
                                        "Hachazo fallido"
                                            .to_string()
                                    }

                                    ShotResult::Hit {
                                        vida_restante,
                                    } => {
                                        format!(
                                            "Golpe - {} HP",
                                            vida_restante,
                                        )
                                    }

                                    ShotResult::Kill
                                    | ShotResult::HeadshotKill => {
                                        format!(
                                            "BAJAS: {}",
                                            enemigos_matados,
                                        )
                                    }

                                    ShotResult::KillConLlave
                                    | ShotResult::HeadshotKillConLlave => {
                                        format!(
                                            "BAJAS: {} - LLAVE",
                                            enemigos_matados,
                                        )
                                    }

                                    ShotResult::HeadshotHit {
                                        vida_restante,
                                    } => {
                                        format!(
                                            "Golpe - {} HP",
                                            vida_restante,
                                        )
                                    }
                                };
                        }
                    }
                }

                ArmaActual::Lanzallamas => {
                    // El disparo continuo se procesa mientras se mantiene el botón.
                }
            }
        }

        if nivel_seleccionado
            == NivelSeleccionado::Final
            && !evacuacion_completada
            && !zombies.iter().any(|zombie| zombie.vivo)
            && !lickers.iter().any(|licker| licker.vivo)
        {
            let generados =
                generar_refuerzos_final(
                    &mut zombies,
                    &mapa,
                    &player,
                );

            if generados > 0 {
                mensaje = format!(
                    "REFUERZOS ENEMIGOS - {} NUEVOS",
                    generados,
                );
            }
        }

        let meta_alcanzada =
            match nivel_seleccionado {
                NivelSeleccionado::Laboratorio => antivirus_recogido,
                NivelSeleccionado::Final => {
                    evacuacion_completada
                        && enemigos_matados >= META_ENEMIGOS_FINAL
                        && lickers_de_cada_tipo_derrotados(&lickers)
                        && interruptores_final >= META_PIEZAS_RADIO
                }
                NivelSeleccionado::Mansion => {
                    objetivo_completo(
                        bajas_normal,
                        bajas_medio,
                        bajas_fuerte,
                    )
                }
            };

        if !objetivo_completado
            && meta_alcanzada
        {
            objetivo_completado =
                true;

            if nivel_seleccionado
                == NivelSeleccionado::Final
            {
                pantalla_great = false;
                estado_juego = EstadoJuego::HistoriaFinal;
            } else {
                pantalla_great = true;
            }

            detener_sonidos_enemigos(
                &sonidos,
            );

            mensaje =
                "OBJETIVO COMPLETADO"
                    .to_string();

            ventana
                .enable_cursor();

            continue;
        }

        while vida_jugador > 0
            && (
                nivel_seleccionado
                    == NivelSeleccionado::Mansion
                || objetivo_completado
            )
            && enemigos_matados
                >= siguiente_horda
        {
            let generados =
                generar_horda(
                    &mut zombies,
                    &mapa,
                    &player,
                    numero_horda,
                );

            if generados > 0 {
                mensaje =
                    format!(
                        "HORDA {} - {} ENEMIGOS",
                        numero_horda,
                        generados,
                    );
            }

            numero_horda +=
                1;

            siguiente_horda +=
                6;
        }

        if vida_jugador > 0
            && (ventana
                    .is_key_pressed(
                        KeyboardKey::KEY_R,
                    )
                || gamepad::presionado(
                    &ventana,
                    GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_LEFT,
                ))
            && arma_equipada
                == ArmaActual::Pistola
            && !recargando
        {
            if balas_cargador == 8 {
                mensaje =
                    "Cargador lleno"
                        .to_string();
            } else if balas_reserva
                <= 0
            {
                mensaje =
                    "Sin balas de reserva"
                        .to_string();
            } else {
                recargando =
                    true;

                tiempo_recarga =
                    DURACION_RECARGA;

                sonidos
                    .recarga();

                mensaje =
                    "Recargando..."
                        .to_string();
            }
        }

        if ventana
            .is_key_pressed(
                KeyboardKey::KEY_F5,
            )
        {
            detener_sonidos_enemigos(
                &sonidos,
            );

            sonidos
                .detener_musica();

            nivel_actual =
                cargar_partida(
                    nivel_seleccionado,
                    &mut mapa,
                    &mut zombies,
                    &mut lickers,
                    &mut tyrant,
                    &mut nemesis,
                    &mut misiles_nemesis,
                    &mut player,
                    &mut camera,
                    &mut inventory,
                    &mut puzzle,
                    &mut damage_effect,
                );

            enemigos_matados =
                0;

            bajas_normal =
                0;

            bajas_medio =
                0;

            bajas_fuerte =
                0;

            objetivo_completado =
                false;

            antivirus_recogido =
                false;

            interruptores_final = 0;
            evacuacion_completada = false;

            pantalla_great =
                false;

            numero_horda =
                1;

            siguiente_horda =
                4;

            tiempo_cambio_nivel =
                0.0;

            arma_equipada =
                ArmaActual::Pistola;

            vida_jugador =
                100;

            balas_cargador =
                8;

            balas_reserva =
                24;

            municion_lanzallamas =
                100;

            tiempo_disparo =
                0.0;

            tiempo_hachazo =
                0.0;

            tiempo_recarga =
                0.0;

            tiempo_sonido_muerte_zombie =
                0.0;

            tiempo_sonido_muerte_licker =
                0.0;

            recargando =
                false;

            mensaje =
                nombre_nivel(
                    nivel_seleccionado,
                )
                    .to_string();

            sonidos
                .iniciar_musica(
                    nivel_seleccionado
                        != NivelSeleccionado::Mansion,
                    modo_musica,
                );

            ventana
                .disable_cursor();
        }

        if ventana
            .is_key_pressed(
                KeyboardKey::KEY_F11,
            )
        {
            ventana
                .toggle_fullscreen();
        }

        if ventana
            .is_key_pressed(
                KeyboardKey::KEY_TAB,
            )
        {
            ventana
                .enable_cursor();
        }

        framebuffer.clear();

        let es_laboratorio =
            nivel_seleccionado
                != NivelSeleccionado::Mansion;

        let pared_actual =
            if es_laboratorio {
                &textura_lab_pared
            } else {
                &textura_pared
            };

        let puerta_actual =
            if es_laboratorio {
                &textura_lab_puerta
            } else {
                &textura_puerta
            };

        let suelo_actual =
            if nivel_seleccionado
                == NivelSeleccionado::Final
            {
                &textura_rooftop_suelo
            } else if es_laboratorio {
                &textura_lab_suelo
            } else {
                &textura_suelo
            };

        let suelo2_actual =
            if nivel_seleccionado
                == NivelSeleccionado::Final
            {
                &textura_rooftop_suelo
            } else if es_laboratorio {
                &textura_lab_suelo
            } else {
                &textura_suelo2
            };

        let techo_actual =
            if es_laboratorio {
                &textura_lab_techo
            } else {
                &textura_techo
            };

        render_3d(
            &mut framebuffer,
            &mapa,
            &player,
            &camera,

            pared_actual,
            &textura_caja_metal,
            &textura_ventana,
            puerta_actual,

            &textura_subir,
            &textura_bajar,

            suelo_actual,
            suelo2_actual,

            techo_actual,
            if nivel_seleccionado
                == NivelSeleccionado::Final
            {
                Some(&textura_citynight)
            } else {
                None
            },
        );

        render_minimap(
            &mut framebuffer,
            &mapa,
            &player,
            &camera,
        );

        textura_framebuffer
            .update_texture(
                framebuffer.pixels(),
            )
            .unwrap();

        let pantalla_ancho =
            ventana
                .get_screen_width()
                as f32;

        let pantalla_alto =
            ventana
                .get_screen_height()
                as f32;

        let escala =
            (
                pantalla_ancho
                    / ANCHO_VENTANA
                        as f32
            )
                .min(
                    pantalla_alto
                        / ALTO_VENTANA
                            as f32,
                );

        let ancho_render =
            ANCHO_VENTANA
                as f32
                * escala;

        let alto_render =
            ALTO_VENTANA
                as f32
                * escala;

        let offset_x =
            (
                pantalla_ancho
                    - ancho_render
            )
                / 2.0;

        let offset_y =
            (
                pantalla_alto
                    - alto_render
            )
                / 2.0;

        let textura_arma =
            match arma_equipada {
                ArmaActual::Pistola => {
                    if recargando {
                        if tiempo_recarga
                            > CAMBIO_FRAME_RECARGA
                        {
                            &pistol_r2
                        } else {
                            &pistol_r
                        }
                    } else if tiempo_disparo > 0.0 {
                        &pistol3
                    } else if apuntando {
                        &pistol2
                    } else {
                        &pistol1
                    }
                }

                ArmaActual::Hacha => {
                    if bloqueando {
                        &axe3
                    } else if tiempo_hachazo > 0.0 {
                        &axe2
                    } else {
                        &axe1
                    }
                }

                ArmaActual::Lanzallamas => {
                    if disparando_llama {
                        &flamethrow3
                    } else if apuntando {
                        &flamethrow2
                    } else {
                        &flamethrow1
                    }
                }
            };

        let escala_base_arma =
            match arma_equipada {
                ArmaActual::Pistola => {
                    if recargando {
                        0.28
                    } else if tiempo_disparo > 0.0 {
                        0.34
                    } else if apuntando {
                        0.36
                    } else {
                        0.32
                    }
                }

                ArmaActual::Hacha => {
                    if bloqueando {
                        0.42
                    } else if tiempo_hachazo > 0.0 {
                        0.37
                    } else {
                        0.34
                    }
                }

                ArmaActual::Lanzallamas => {
                    if disparando_llama {
                        0.37
                    } else if apuntando {
                        0.36
                    } else {
                        0.34
                    }
                }
            };

        let escala_arma =
            escala_base_arma
                * escala;

        let arma_ancho =
            textura_arma
                .width()
                as f32
                * escala_arma;

        let arma_alto =
            textura_arma
                .height()
                as f32
                * escala_arma;

        let retroceso =
            if arma_equipada
                == ArmaActual::Pistola
                && tiempo_disparo > 0.0
                && !recargando
            {
                8.0 * escala
            } else {
                0.0
            };

        let arma_x =
            offset_x
                + ancho_render / 2.0
                - arma_ancho / 2.0;

        let arma_y =
            offset_y
                + alto_render
                - arma_alto
                - retroceso;

        let mut dibujo =
            ventana
                .begin_drawing(
                    &thread,
                );

        dibujo
            .clear_background(
                Color::BLACK,
            );

        dibujo.draw_texture_pro(
            &textura_framebuffer,

            Rectangle::new(
                0.0,
                0.0,
                ANCHO_VENTANA
                    as f32,
                ALTO_VENTANA
                    as f32,
            ),

            Rectangle::new(
                offset_x,
                offset_y,
                ancho_render,
                alto_render,
            ),

            Vector2::new(
                0.0,
                0.0,
            ),

            0.0,

            Color::WHITE,
        );

        unsafe {
            raylib::ffi::BeginScissorMode(
                offset_x
                    .round()
                    as i32,

                offset_y
                    .round()
                    as i32,

                ancho_render
                    .round()
                    .max(1.0)
                    as i32,

                alto_render
                    .round()
                    .max(1.0)
                    as i32,
            );
        }

        let textura_llave_actual =
            if es_laboratorio {
                &lab_key_texture
            } else {
                &key_texture
            };

        render_key_sprite(
            &mut dibujo,
            &mapa,
            &player,
            &camera,
            textura_llave_actual,
            offset_x,
            offset_y,
            escala,
        );

        render_ammo_sprites(
            &mut dibujo,
            &mapa,
            &player,
            &camera,
            &ammo_texture,
            offset_x,
            offset_y,
            escala,
        );

        render_heal_sprites(
            &mut dibujo,
            &mapa,
            &player,
            &camera,
            &heal_texture,
            offset_x,
            offset_y,
            escala,
        );

        render_zombies(
            &mut dibujo,
            &mapa,
            &player,
            &camera,
            &zombies,

            &zombie1,
            &zombie2,
            &zombie3,

            &zombie_v21,
            &zombie_v22,
            &zombie_v23,

            &zombie_v31,
            &zombie_v32,
            &zombie_v33,

            offset_x,
            offset_y,
            escala,
        );

        render_lickers(
            &mut dibujo,
            &mapa,
            &player,
            &camera,
            &lickers,

            &licker1,
            &licker2,
            &licker3,

            &licker_v21,
            &licker_v22,
            &licker_v23,

            &licker_v31,
            &licker_v32,
            &licker_v33,

            offset_x,
            offset_y,
            escala,
        );

        if let Some(
            tyrant_actual,
        ) =
            tyrant.as_ref()
        {
            render_tyrant(
                &mut dibujo,
                &mapa,
                &player,
                &camera,
                tyrant_actual,

                &tyrant1,
                &tyrant2,
                &tyrant3,

                offset_x,
                offset_y,
                escala,
            );
        }

        if let Some(nemesis_actual) =
            nemesis.as_ref()
        {
            render_tyrant(
                &mut dibujo,
                &mapa,
                &player,
                &camera,
                nemesis_actual,

                &nemesis1,
                &nemesis2,
                &nemesis3,

                offset_x,
                offset_y,
                escala,
            );
        }

        render_misiles_nemesis(
            &mut dibujo,
            &mapa,
            &player,
            &camera,
            &misiles_nemesis,
            &nemesis_shoot,
            offset_x,
            offset_y,
            escala,
        );

        render_flamethrow_ammo_sprites(
            &mut dibujo,
            &mapa,
            &player,
            &camera,
            &flamethrow_ammo_texture,
            offset_x,
            offset_y,
            escala,
        );

        render_antivirus_sprite(
            &mut dibujo,
            &mapa,
            &player,
            &camera,
            &antivirus_texture,
            offset_x,
            offset_y,
            escala,
        );

        render_final_objective_sprites(
            &mut dibujo,
            &mapa,
            &player,
            &camera,
            &radio_part_texture,
            &radio_part_texture,
            offset_x,
            offset_y,
            escala,
        );

        if vida_jugador > 0 {
            dibujo
                .draw_texture_ex(
                    textura_arma,

                    Vector2::new(
                        arma_x,
                        arma_y,
                    ),

                    0.0,

                    escala_arma,

                    Color::WHITE,
                );

            if apuntando {
                let mira_x =
                    offset_x
                        + ancho_render
                            / 2.0;

                let mira_y =
                    offset_y
                        + alto_render
                            / 2.0;

                dibujo
                    .draw_circle(
                        mira_x
                            as i32,

                        mira_y
                            as i32,

                        3.0
                            * escala
                                .max(1.0),

                        Color::RED,
                    );
            }
        }

        unsafe {
            raylib::ffi::
                EndScissorMode();
        }

        render_hud(
            &mut dibujo,
            vida_jugador,
            balas_cargador,
            balas_reserva,
            municion_lanzallamas,
            &inventory,
            &mensaje,
            offset_x,
            offset_y,
            ancho_render,
            alto_render,
            escala,
        );

        render_leyenda_minimapa(
            &mut dibujo,
            &mapa,
            offset_x,
            offset_y,
            escala,
        );

        let vivos_zombies =
            zombies
                .iter()
                .filter(|z| z.vivo)
                .count();

        let vivos_lickers =
            lickers
                .iter()
                .filter(|l| l.vivo)
                .count();

        let nombre =
            nombre_nivel(
                nivel_seleccionado,
            );

        let texto_arcade =
            if nivel_seleccionado
                == NivelSeleccionado::Mansion
            {
                format!(
                    "{} | PISO {} | HORDA {} | BAJAS {} | Z {} | L {}",
                    nombre,
                    nivel_actual,
                    numero_horda,
                    enemigos_matados,
                    vivos_zombies,
                    vivos_lickers,
                )
            } else {
                if objetivo_completado {
                    format!(
                        "{} | MODO HORDAS {} | BAJAS {} | Z {} | L {}",
                        nombre,
                        numero_horda,
                        enemigos_matados,
                        vivos_zombies,
                        vivos_lickers,
                    )
                } else {
                    let mision =
                        if nivel_seleccionado
                            == NivelSeleccionado::Final
                        {
                            "RADIO Y RESCATE"
                        } else {
                            "ANTIVIRUS"
                        };

                    format!(
                        "{} | MISION {} | BAJAS {} | Z {} | L {}",
                        nombre,
                        mision,
                        enemigos_matados,
                        vivos_zombies,
                        vivos_lickers,
                    )
                }
            };

        // La informacion de la mision vive a la derecha del minimapa para no
        // mezclarse con su leyenda.
        let panel_mision_x =
            offset_x + 150.0 * escala;
        let panel_mision_y =
            offset_y + 43.0 * escala;
        let panel_mision_ancho =
            (offset_x + ancho_render - panel_mision_x - 8.0 * escala)
                .max(120.0 * escala);

        dibujo.draw_rectangle(
            panel_mision_x as i32,
            panel_mision_y as i32,
            panel_mision_ancho as i32,
            (39.0 * escala) as i32,
            Color::new(0, 0, 0, 175),
        );
        dibujo.draw_rectangle(
            panel_mision_x as i32,
            panel_mision_y as i32,
            (3.0 * escala) as i32,
            (39.0 * escala) as i32,
            Color::new(128, 30, 34, 255),
        );

        let mut fuente_estado =
            (13.0 * escala) as i32;
        while fuente_estado > 9
            && dibujo.measure_text(&texto_arcade, fuente_estado)
                > (panel_mision_ancho - 14.0 * escala) as i32
        {
            fuente_estado -= 1;
        }

        dibujo.draw_text(
            &texto_arcade,
            (panel_mision_x + 8.0 * escala) as i32,
            (panel_mision_y + 4.0 * escala) as i32,
            fuente_estado,
            Color::RAYWHITE,
        );

        if !objetivo_completado {
            let texto_objetivo =
                match nivel_seleccionado {
                    NivelSeleccionado::Laboratorio =>
                        "OBJETIVO | ENCUENTRA EL ANTIVIRUS AL FINAL DEL LABORATORIO"
                            .to_string(),
                    NivelSeleccionado::Final => format!(
                        "OBJETIVO | BAJAS {}/{} | RADIO {}/{} | LICKERS: {} | LLAMA AL HELICOPTERO",
                        enemigos_matados.min(META_ENEMIGOS_FINAL),
                        META_ENEMIGOS_FINAL,
                        interruptores_final,
                        META_PIEZAS_RADIO,
                        if lickers_de_cada_tipo_derrotados(&lickers) {
                            "OK"
                        } else {
                            "FALTAN TIPOS"
                        },
                    ),
                    NivelSeleccionado::Mansion => format!(
                        "OBJETIVO | NORMAL {}/{} | MEDIO {}/{} | FUERTE {}/{}",
                        bajas_normal.min(META_NORMAL),
                        META_NORMAL,
                        bajas_medio.min(META_MEDIO),
                        META_MEDIO,
                        bajas_fuerte.min(META_FUERTE),
                        META_FUERTE,
                    ),
                };

            let mut fuente_objetivo =
                (12.0 * escala) as i32;
            while fuente_objetivo > 8
                && dibujo.measure_text(&texto_objetivo, fuente_objetivo)
                    > (panel_mision_ancho - 14.0 * escala) as i32
            {
                fuente_objetivo -= 1;
            }

            dibujo.draw_text(
                &texto_objetivo,
                (panel_mision_x + 8.0 * escala) as i32,
                (panel_mision_y + 22.0 * escala) as i32,
                fuente_objetivo,
                Color::GOLD,
            );
        } else {
            dibujo.draw_text(
                "MODO ARCADE",
                (panel_mision_x + 8.0 * escala) as i32,
                (panel_mision_y + 22.0 * escala) as i32,
                (12.0 * escala) as i32,
                Color::GOLD,
            );
        }

        damage_effect
            .render(
                &mut dibujo,
            );

        if vida_jugador <= 0 {
            sonidos
                .detener_musica();

            detener_sonidos_enemigos(
                &sonidos,
            );

            let sw =
                dibujo
                    .get_screen_width();

            let sh =
                dibujo
                    .get_screen_height();

            dibujo
                .draw_rectangle(
                    0,
                    0,
                    sw,
                    sh,

                    Color::new(
                        0,
                        0,
                        0,
                        220,
                    ),
                );

            let titulo =
                if objetivo_completado {
                    "MISION CUMPLIDA"
                } else {
                    "GAME OVER"
                };

            let stats =
                format!(
                    "{} - HORDA {} - {} BAJAS",
                    nombre,
                    numero_horda,
                    enemigos_matados,
                );

            let objetivo =
                if objetivo_completado {
                    match nivel_seleccionado {
                        NivelSeleccionado::Laboratorio =>
                            "ANTIVIRUS CONSEGUIDO".to_string(),
                        NivelSeleccionado::Final =>
                            "HELICOPTERO SOLICITADO".to_string(),
                        NivelSeleccionado::Mansion =>
                            "MISION DE LA MANSION COMPLETADA".to_string(),
                    }
                } else {
                    format!(
                        "N {}/{}   M {}/{}   F {}/{}",
                        bajas_normal.min(META_NORMAL),
                        META_NORMAL,
                        bajas_medio.min(META_MEDIO),
                        META_MEDIO,
                        bajas_fuerte.min(META_FUERTE),
                        META_FUERTE,
                    )
                };

            let reiniciar =
                if objetivo_completado
                    && nivel_seleccionado
                        != NivelSeleccionado::Final
                {
                    "ENTER - SIGUIENTE NIVEL | F5 - JUGAR DE NUEVO"
                } else if objetivo_completado {
                    "ULTIMO NIVEL COMPLETADO | F5 - JUGAR DE NUEVO"
                } else {
                    "F5 - JUGAR DE NUEVO"
                };

            let menu_texto =
                "BACKSPACE - MENU PRINCIPAL";

            let titulo_w =
                dibujo.measure_text(
                    titulo,
                    64,
                );

            dibujo.draw_text(
                titulo,
                sw / 2
                    - titulo_w / 2,
                sh / 2
                    - 130,
                64,
                Color::RED,
            );

            let stats_w =
                dibujo.measure_text(
                    &stats,
                    26,
                );

            dibujo.draw_text(
                &stats,
                sw / 2
                    - stats_w / 2,
                sh / 2
                    - 35,
                26,
                Color::WHITE,
            );

            let objetivo_w =
                dibujo.measure_text(
                    &objetivo,
                    22,
                );

            dibujo.draw_text(
                &objetivo,
                sw / 2
                    - objetivo_w / 2,
                sh / 2
                    + 5,
                22,
                Color::GOLD,
            );

            let reiniciar_w =
                dibujo.measure_text(
                    reiniciar,
                    22,
                );

            dibujo.draw_text(
                reiniciar,
                sw / 2
                    - reiniciar_w / 2,
                sh / 2
                    + 60,
                22,
                Color::LIGHTGRAY,
            );

            let menu_w =
                dibujo.measure_text(
                    menu_texto,
                    20,
                );

            dibujo.draw_text(
                menu_texto,
                sw / 2
                    - menu_w / 2,
                sh / 2
                    + 95,
                20,
                Color::GRAY,
            );
        }
    }

    sonidos
        .detener_musica();

    detener_sonidos_enemigos(
        &sonidos,
    );
}
