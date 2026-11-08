use raylib::prelude::*;
use crate::gamepad;

pub struct Camera {
    pub angle: f32,
    pub vertical_offset: i32,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            angle: 0.0,
            vertical_offset: 0,
        }
    }

pub fn update(
    &mut self,
    window: &RaylibHandle,
    delta_time: f32,
    rotation_speed: f32,
) {
    let mouse_delta =
        window.get_mouse_delta();

    let sensibilidad_x = 0.003;
    let sensibilidad_y = 0.6;

    self.angle +=
        mouse_delta.x
            * sensibilidad_x;

    self.angle +=
        gamepad::eje(
            window,
            GamepadAxis::GAMEPAD_AXIS_RIGHT_X,
        ) * rotation_speed
            * delta_time
            * 1.8;

    // Stick derecho vertical: permite apuntar hacia arriba y abajo.
    // En raylib el eje Y es positivo al mover el stick hacia abajo,
    // igual que el desplazamiento vertical del mouse.
    let mirada_vertical_control =
        gamepad::eje(
            window,
            GamepadAxis::GAMEPAD_AXIS_RIGHT_Y,
        );

    if mirada_vertical_control != 0.0 {
        let movimiento_vertical =
            mirada_vertical_control
                * rotation_speed
                * delta_time
                * 95.0;

        // vertical_offset usa enteros; el minimo evita perder movimientos
        // suaves del stick por el redondeo de cada fotograma.
        self.vertical_offset +=
            movimiento_vertical
                .signum() as i32
                * movimiento_vertical
                    .abs()
                    .max(1.0) as i32;
    }

    self.vertical_offset +=
        (mouse_delta.y * sensibilidad_y)
            as i32;

    if window.is_key_down(
        KeyboardKey::KEY_J,
    ) {
        self.angle -=
            rotation_speed * delta_time;
    }

    if window.is_key_down(
        KeyboardKey::KEY_L,
    ) {
        self.angle +=
            rotation_speed * delta_time;
    }

    self.vertical_offset =
        self.vertical_offset.clamp(
            -150,
            150,
        );

    self.angle =
        normalize_angle(
            self.angle,
        );
}

    pub fn reset(&mut self) {
        self.angle = 0.0;
        self.vertical_offset = 0;
    }
}

fn normalize_angle(
    mut angle: f32,
) -> f32 {
    let full_rotation =
        2.0 * std::f32::consts::PI;

    while angle < 0.0 {
        angle += full_rotation;
    }

    while angle >= full_rotation {
        angle -= full_rotation;
    }

    angle
}
