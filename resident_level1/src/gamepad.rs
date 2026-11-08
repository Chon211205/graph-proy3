use raylib::prelude::*;

pub const ZONA_MUERTA: f32 = 0.20;
const MAX_GAMEPADS: i32 = 16;

fn indices(rl: &RaylibHandle) -> impl Iterator<Item = i32> + '_ {
    (0..MAX_GAMEPADS).filter(|indice| {
        rl.is_gamepad_available(*indice)
    })
}

pub fn conectado(rl: &RaylibHandle) -> bool {
    indices(rl).next().is_some()
}

pub fn presionado(
    rl: &RaylibHandle,
    boton: GamepadButton,
) -> bool {
    indices(rl)
        .any(|indice| {
            rl.is_gamepad_button_pressed(indice, boton)
        })
}

pub fn pulsado(
    rl: &RaylibHandle,
    boton: GamepadButton,
) -> bool {
    indices(rl)
        .any(|indice| {
            rl.is_gamepad_button_down(indice, boton)
        })
}

pub fn eje(
    rl: &RaylibHandle,
    axis: GamepadAxis,
) -> f32 {
    let valor = indices(rl)
        .map(|indice| {
            rl.get_gamepad_axis_movement(indice, axis)
        })
        .max_by(|a, b| {
            a.abs()
                .partial_cmp(&b.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(0.0);

    if valor.abs() < ZONA_MUERTA {
        0.0
    } else {
        valor
    }
}

pub fn aceptar(rl: &RaylibHandle) -> bool {
    presionado(
        rl,
        GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_DOWN,
    )
}

pub fn volver(rl: &RaylibHandle) -> bool {
    presionado(
        rl,
        GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_RIGHT,
    )
}

pub fn arriba(rl: &RaylibHandle) -> bool {
    presionado(
        rl,
        GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_UP,
    )
}

pub fn abajo(rl: &RaylibHandle) -> bool {
    presionado(
        rl,
        GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_DOWN,
    )
}

pub fn izquierda(rl: &RaylibHandle) -> bool {
    presionado(
        rl,
        GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_LEFT,
    )
}

pub fn derecha(rl: &RaylibHandle) -> bool {
    presionado(
        rl,
        GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_RIGHT,
    )
}
