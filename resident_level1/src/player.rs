use crate::map::{
    Map,
    TAMANO_CELDA,
};

use raylib::prelude::*;
use crate::gamepad;

pub struct Player {
    pub x: f32,
    pub y: f32,

    spawn_x: f32,
    spawn_y: f32,

    velocidad: f32,
}

impl Player {
    pub fn new(
        mapa: &Map,
    ) -> Self {
        let (
            spawn_x,
            spawn_y,
        ) =
            if let Some(
                (
                    x,
                    y,
                ),
            ) = mapa.buscar_jugador()
            {
                (
                    x,
                    y,
                )
            } else {
                (
                    TAMANO_CELDA
                        * 1.5,
                    TAMANO_CELDA
                        * 1.5,
                )
            };

        Self {
            x: spawn_x,
            y: spawn_y,

            spawn_x,
            spawn_y,

            velocidad: 90.0,
        }
    }

    pub fn update(
        &mut self,
        ventana: &RaylibHandle,
        mapa: &Map,
        angulo: f32,
        delta_time: f32,
    ) {
        let velocidad_frame =
            self.velocidad
                * delta_time;

        let frente_x =
            angulo.cos();

        let frente_y =
            angulo.sin();

        let derecha_x =
            -frente_y;

        let derecha_y =
            frente_x;

        let mut movimiento_x =
            0.0;

        let mut movimiento_y =
            0.0;

        let stick_x = gamepad::eje(
            ventana,
            GamepadAxis::GAMEPAD_AXIS_LEFT_X,
        );

        let stick_y = gamepad::eje(
            ventana,
            GamepadAxis::GAMEPAD_AXIS_LEFT_Y,
        );

        movimiento_x +=
            derecha_x * stick_x
                - frente_x * stick_y;

        movimiento_y +=
            derecha_y * stick_x
                - frente_y * stick_y;

        if ventana.is_key_down(
            KeyboardKey::KEY_W,
        ) {
            movimiento_x +=
                frente_x;

            movimiento_y +=
                frente_y;
        }

        if ventana.is_key_down(
            KeyboardKey::KEY_S,
        ) {
            movimiento_x -=
                frente_x;

            movimiento_y -=
                frente_y;
        }

        if ventana.is_key_down(
            KeyboardKey::KEY_D,
        ) {
            movimiento_x +=
                derecha_x;

            movimiento_y +=
                derecha_y;
        }

        if ventana.is_key_down(
            KeyboardKey::KEY_A,
        ) {
            movimiento_x -=
                derecha_x;

            movimiento_y -=
                derecha_y;
        }

        let longitud =
            (
                movimiento_x
                    * movimiento_x
                    + movimiento_y
                        * movimiento_y
            )
                .sqrt();

        if longitud > 0.0 {
            movimiento_x /=
                longitud;

            movimiento_y /=
                longitud;
        }

        let nuevo_x =
            self.x
                + movimiento_x
                    * velocidad_frame;

        let nuevo_y =
            self.y
                + movimiento_y
                    * velocidad_frame;

        if puede_moverse(
            mapa,
            nuevo_x,
            self.y,
        ) {
            self.x =
                nuevo_x;
        }

        if puede_moverse(
            mapa,
            self.x,
            nuevo_y,
        ) {
            self.y =
                nuevo_y;
        }
    }

    pub fn reset(
        &mut self,
    ) {
        self.x =
            self.spawn_x;

        self.y =
            self.spawn_y;
    }
}

fn puede_moverse(
    mapa: &Map,
    x: f32,
    y: f32,
) -> bool {
    const RADIO_JUGADOR: f32 =
        6.0;

    if mapa.es_pared(
        x - RADIO_JUGADOR,
        y - RADIO_JUGADOR,
    ) {
        return false;
    }

    if mapa.es_pared(
        x + RADIO_JUGADOR,
        y - RADIO_JUGADOR,
    ) {
        return false;
    }

    if mapa.es_pared(
        x - RADIO_JUGADOR,
        y + RADIO_JUGADOR,
    ) {
        return false;
    }

    if mapa.es_pared(
        x + RADIO_JUGADOR,
        y + RADIO_JUGADOR,
    ) {
        return false;
    }

    true
}
