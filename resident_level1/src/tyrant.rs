use crate::map::{
    Map,
    TAMANO_CELDA,
};
use crate::player::Player;

use std::collections::{
    HashMap,
    VecDeque,
};

pub struct Tyrant {
    pub x: f32,
    pub y: f32,
    pub persiguiendo: bool,
    pub tiempo_animacion: f32,
    tiempo_ultimo_ataque: f32,
    tiempo_pathfinding: f32,
    tiempo_disparo: f32,
    camino: Vec<(i32, i32)>,
}

impl Tyrant {
    pub fn new(
        x: f32,
        y: f32,
    ) -> Self {
        Self {
            x,
            y,
            persiguiendo: true,
            tiempo_animacion: 0.0,
            tiempo_ultimo_ataque: 0.0,
            tiempo_pathfinding: 0.0,
            tiempo_disparo: 0.0,
            camino: Vec::new(),
        }
    }

    pub fn update(
        &mut self,
        player: &Player,
        mapa: &Map,
        delta_time: f32,
    ) -> i32 {
        const VELOCIDAD: f32 = 28.0;
        const DISTANCIA_ATAQUE: f32 = 30.0;
        const INTERVALO_ATAQUE: f32 = 1.5;
        const DANO: i32 = 30;

        self.tiempo_ultimo_ataque +=
            delta_time;

        self.tiempo_pathfinding -=
            delta_time;

        self.tiempo_disparo +=
            delta_time;

        let dx =
            player.x - self.x;

        let dy =
            player.y - self.y;

        let distancia =
            (
                dx * dx
                    + dy * dy
            )
                .sqrt();

        self.persiguiendo =
            distancia
                > DISTANCIA_ATAQUE;

        if distancia
            <= DISTANCIA_ATAQUE
        {
            if self.tiempo_ultimo_ataque
                >= INTERVALO_ATAQUE
            {
                self.tiempo_ultimo_ataque =
                    0.0;

                return DANO;
            }

            return 0;
        }

        if self.tiempo_pathfinding
            <= 0.0
        {
            self.camino =
                calcular_camino(
                    mapa,
                    self.x,
                    self.y,
                    player.x,
                    player.y,
                );

            self.tiempo_pathfinding =
                0.30;
        }

        if let Some(
            &(fila, columna),
        ) =
            self.camino.first()
        {
            let objetivo_x =
                columna as f32
                    * TAMANO_CELDA
                    + TAMANO_CELDA / 2.0;

            let objetivo_y =
                fila as f32
                    * TAMANO_CELDA
                    + TAMANO_CELDA / 2.0;

            let dx_objetivo =
                objetivo_x - self.x;

            let dy_objetivo =
                objetivo_y - self.y;

            let distancia_objetivo =
                (
                    dx_objetivo
                        * dx_objetivo
                        + dy_objetivo
                            * dy_objetivo
                )
                    .sqrt();

            if distancia_objetivo
                < 4.0
            {
                if !self.camino
                    .is_empty()
                {
                    self.camino
                        .remove(0);
                }
            } else if distancia_objetivo
                > 0.001
            {
                let dir_x =
                    dx_objetivo
                        / distancia_objetivo;

                let dir_y =
                    dy_objetivo
                        / distancia_objetivo;

                let movimiento =
                    VELOCIDAD
                        * delta_time;

                let nuevo_x =
                    self.x
                        + dir_x
                            * movimiento;

                let nuevo_y =
                    self.y
                        + dir_y
                            * movimiento;

                if !mapa.es_pared(
                    nuevo_x,
                    self.y,
                ) {
                    self.x =
                        nuevo_x;
                }

                if !mapa.es_pared(
                    self.x,
                    nuevo_y,
                ) {
                    self.y =
                        nuevo_y;
                }

                self.tiempo_animacion +=
                    delta_time;
            }
        }

        0
    }

    pub fn intentar_disparar_misil(
        &mut self,
        player: &Player,
    ) -> Option<ProyectilNemesis> {
        const INTERVALO_DISPARO: f32 = 3.0;
        const DISTANCIA_MINIMA: f32 = 65.0;
        const DISTANCIA_MAXIMA: f32 = 450.0;

        let dx = player.x - self.x;
        let dy = player.y - self.y;
        let distancia = (dx * dx + dy * dy).sqrt();

        if self.tiempo_disparo < INTERVALO_DISPARO
            || distancia < DISTANCIA_MINIMA
            || distancia > DISTANCIA_MAXIMA
        {
            return None;
        }

        self.tiempo_disparo = 0.0;

        Some(ProyectilNemesis::new(
            self.x,
            self.y,
            dx / distancia,
            dy / distancia,
        ))
    }
}

pub struct ProyectilNemesis {
    pub x: f32,
    pub y: f32,
    direccion_x: f32,
    direccion_y: f32,
    pub vivo: bool,
}

impl ProyectilNemesis {
    fn new(
        x: f32,
        y: f32,
        direccion_x: f32,
        direccion_y: f32,
    ) -> Self {
        Self {
            x,
            y,
            direccion_x,
            direccion_y,
            vivo: true,
        }
    }

    pub fn update(
        &mut self,
        player: &Player,
        mapa: &Map,
        delta_time: f32,
    ) -> i32 {
        const VELOCIDAD: f32 = 90.0;
        const RADIO_IMPACTO: f32 = 16.0;
        const DANO: i32 = 25;

        let nuevo_x = self.x
            + self.direccion_x * VELOCIDAD * delta_time;
        let nuevo_y = self.y
            + self.direccion_y * VELOCIDAD * delta_time;

        if mapa.es_pared(nuevo_x, nuevo_y) {
            self.vivo = false;
            return 0;
        }

        self.x = nuevo_x;
        self.y = nuevo_y;

        let dx = player.x - self.x;
        let dy = player.y - self.y;

        if (dx * dx + dy * dy).sqrt() <= RADIO_IMPACTO {
            self.vivo = false;
            return DANO;
        }

        0
    }
}

fn calcular_camino(
    mapa: &Map,
    inicio_x: f32,
    inicio_y: f32,
    destino_x: f32,
    destino_y: f32,
) -> Vec<(i32, i32)> {
    let inicio_columna =
        (
            inicio_x
                / TAMANO_CELDA
        )
            .floor()
            as i32;

    let inicio_fila =
        (
            inicio_y
                / TAMANO_CELDA
        )
            .floor()
            as i32;

    let destino_columna =
        (
            destino_x
                / TAMANO_CELDA
        )
            .floor()
            as i32;

    let destino_fila =
        (
            destino_y
                / TAMANO_CELDA
        )
            .floor()
            as i32;

    let inicio =
        (
            inicio_fila,
            inicio_columna,
        );

    let destino =
        (
            destino_fila,
            destino_columna,
        );

    if inicio == destino {
        return Vec::new();
    }

    let mut cola =
        VecDeque::new();

    let mut anterior:
        HashMap<
            (i32, i32),
            (i32, i32),
        > =
        HashMap::new();

    cola.push_back(
        inicio,
    );

    anterior.insert(
        inicio,
        inicio,
    );

    let direcciones = [
        (-1, 0),
        (1, 0),
        (0, -1),
        (0, 1),
    ];

    while let Some(
        actual,
    ) =
        cola.pop_front()
    {
        if actual == destino {
            break;
        }

        for (
            df,
            dc,
        ) in direcciones
        {
            let nuevo =
                (
                    actual.0 + df,
                    actual.1 + dc,
                );

            if nuevo.0 < 0
                || nuevo.1 < 0
                || nuevo.0
                    >= mapa.alto()
                        as i32
                || nuevo.1
                    >= mapa.ancho()
                        as i32
            {
                continue;
            }

            if anterior
                .contains_key(
                    &nuevo,
                )
            {
                continue;
            }

            let celda =
                mapa.celda(
                    nuevo.0,
                    nuevo.1,
                );

            if !es_transitable(
                celda,
            )
                && nuevo
                    != destino
            {
                continue;
            }

            anterior.insert(
                nuevo,
                actual,
            );

            cola.push_back(
                nuevo,
            );
        }
    }

    if !anterior
        .contains_key(
            &destino,
        )
    {
        return Vec::new();
    }

    let mut camino =
        Vec::new();

    let mut actual =
        destino;

    while actual
        != inicio
    {
        camino.push(
            actual,
        );

        actual =
            anterior[
                &actual
            ];
    }

    camino.reverse();

    camino
}

fn es_transitable(
    celda: char,
) -> bool {
    matches!(
        celda,
        ' '
            | 'C'
            | 'P'
            | 'O'
            | 'K'
            | 'A'
            | 'H'
            | 'Q'
            | 'V'
            | 'I'
            | 'E'
    )
}
