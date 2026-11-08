use crate::camera::Camera;
use crate::framebuffer::Framebuffer;
use crate::map::{
    Map,
    TAMANO_CELDA,
};
use crate::player::Player;

use raylib::prelude::*;

fn pintar_rectangulo(
    framebuffer: &mut Framebuffer,
    x: i32,
    y: i32,
    ancho: i32,
    alto: i32,
    color: Color,
) {
    for py in y..y + alto {
        for px in x..x + ancho {
            framebuffer.point_color(
                px,
                py,
                color,
            );
        }
    }
}

fn pintar_borde_rectangulo(
    framebuffer: &mut Framebuffer,
    x: i32,
    y: i32,
    ancho: i32,
    alto: i32,
    color: Color,
) {
    for px in x..x + ancho {
        framebuffer.point_color(
            px,
            y,
            color,
        );

        framebuffer.point_color(
            px,
            y + alto - 1,
            color,
        );
    }

    for py in y..y + alto {
        framebuffer.point_color(
            x,
            py,
            color,
        );

        framebuffer.point_color(
            x + ancho - 1,
            py,
            color,
        );
    }
}

fn pintar_circulo(
    framebuffer: &mut Framebuffer,
    centro_x: i32,
    centro_y: i32,
    radio: i32,
    color: Color,
) {
    for y in -radio..=radio {
        for x in -radio..=radio {
            if x * x
                + y * y
                <= radio * radio
            {
                framebuffer.point_color(
                    centro_x + x,
                    centro_y + y,
                    color,
                );
            }
        }
    }
}

fn pintar_linea(
    framebuffer: &mut Framebuffer,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    color: Color,
) {
    let mut x0 =
        x0;

    let mut y0 =
        y0;

    let dx =
        (
            x1 - x0
        )
            .abs();

    let sx =
        if x0 < x1 {
            1
        } else {
            -1
        };

    let dy =
        -(
            y1 - y0
        )
            .abs();

    let sy =
        if y0 < y1 {
            1
        } else {
            -1
        };

    let mut error =
        dx + dy;

    loop {
        framebuffer.point_color(
            x0,
            y0,
            color,
        );

        if x0 == x1
            && y0 == y1
        {
            break;
        }

        let error2 =
            2 * error;

        if error2 >= dy {
            error +=
                dy;

            x0 +=
                sx;
        }

        if error2 <= dx {
            error +=
                dx;

            y0 +=
                sy;
        }
    }
}

pub fn render_minimap(
    framebuffer: &mut Framebuffer,
    mapa: &Map,
    player: &Player,
    camera: &Camera,
) {
    if mapa.ancho() == 0
        || mapa.alto() == 0
    {
        return;
    }

    let max_ancho =
        125.0;

    let max_alto =
        90.0;

    let escala_x =
        max_ancho
            / mapa.ancho()
                as f32;

    let escala_y =
        max_alto
            / mapa.alto()
                as f32;

    let escala =
        escala_x
            .min(
                escala_y,
            )
            .max(
                1.0,
            );

    let origen_x =
        12;

    let origen_y =
        12;

    let ancho =
        (
            mapa.ancho()
                as f32
                * escala
        )
            .ceil()
            as i32;

    let alto =
        (
            mapa.alto()
                as f32
                * escala
        )
            .ceil()
            as i32;

    pintar_rectangulo(
        framebuffer,
        origen_x - 5,
        origen_y - 5,
        ancho + 10,
        alto + 10,
        Color::BLACK,
    );

    pintar_borde_rectangulo(
        framebuffer,
        origen_x - 5,
        origen_y - 5,
        ancho + 10,
        alto + 10,
        Color::GRAY,
    );

    for fila in 0..mapa.alto() {
        for columna in 0..mapa.ancho() {
            let celda =
                mapa.celda(
                    fila as i32,
                    columna as i32,
                );

            let color =
                match celda {
                    '#' => {
                        Some(
                            Color::LIGHTGRAY,
                        )
                    }

                    'W' if mapa.nivel() != 1 && mapa.nivel() != 2 => {
                        Some(
                            Color::SKYBLUE,
                        )
                    }

                    'D' => {
                        Some(
                            Color::BROWN,
                        )
                    }

                    'O' => {
                        Some(
                            Color::DARKBROWN,
                        )
                    }

                    'C' if mapa.nivel() != 1 && mapa.nivel() != 2 => {
                        Some(
                            Color::MAROON,
                        )
                    }

                    'K' => {
                        Some(
                            Color::YELLOW,
                        )
                    }

                    'A' => {
                        Some(
                            Color::ORANGE,
                        )
                    }

                    'H' => {
                        Some(
                            Color::GREEN,
                        )
                    }

                    'X' => {
                        Some(
                            Color::BLUE,
                        )
                    }

                    'B' => {
                        Some(
                            Color::PURPLE,
                        )
                    }

                    'J' => {
                        Some(Color::DARKGRAY)
                    }

                    'Q' => {
                        Some(Color::PINK)
                    }

                    'V' | 'I' => {
                        Some(Color::GOLD)
                    }

                    'E' => {
                        Some(Color::WHITE)
                    }

                    _ => {
                        None
                    }
                };

            let Some(color) =
                color
            else {
                continue;
            };

            let x =
                origen_x
                    + (
                        columna
                            as f32
                            * escala
                    )
                        as i32;

            let y =
                origen_y
                    + (
                        fila
                            as f32
                            * escala
                    )
                        as i32;

            let tamano =
                escala
                    .ceil()
                    .max(
                        1.0,
                    )
                    as i32;

            pintar_rectangulo(
                framebuffer,
                x,
                y,
                tamano,
                tamano,
                color,
            );
        }
    }

    let jugador_x =
        origen_x
            as f32
            + (
                player.x
                    / TAMANO_CELDA
            )
                * escala;

    let jugador_y =
        origen_y
            as f32
            + (
                player.y
                    / TAMANO_CELDA
            )
                * escala;

    pintar_circulo(
        framebuffer,
        jugador_x
            .round()
            as i32,
        jugador_y
            .round()
            as i32,
        2,
        Color::RED,
    );

    let largo =
        6.0;

    let direccion_x =
        jugador_x
            + camera
                .angle
                .cos()
                * largo;

    let direccion_y =
        jugador_y
            + camera
                .angle
                .sin()
                * largo;

    pintar_linea(
        framebuffer,
        jugador_x
            .round()
            as i32,
        jugador_y
            .round()
            as i32,
        direccion_x
            .round()
            as i32,
        direccion_y
            .round()
            as i32,
        Color::YELLOW,
    );
}

pub fn render_leyenda_minimapa(
    dibujo: &mut RaylibDrawHandle,
    mapa: &Map,
    offset_x: f32,
    offset_y: f32,
    escala: f32,
) {
    let mut entradas = vec![
        (Color::RED, "Jugador"),
        (Color::YELLOW, "Direccion"),
    ];

    match mapa.nivel() {
        3 => {
            entradas.extend([
                (Color::LIGHTGRAY, "Pared"),
                (Color::BROWN, "Puerta"),
                (Color::YELLOW, "Llave laboratorio"),
                (Color::ORANGE, "Municion"),
                (Color::GREEN, "Curacion"),
                (Color::DARKBROWN, "Puerta abierta"),
                (Color::GOLD, "Antivirus"),
            ]);
        }
        4 => {
            entradas.extend([
                (Color::DARKGRAY, "Caja metalica"),
                (Color::ORANGE, "Municion"),
                (Color::GREEN, "Curacion"),
                (Color::PINK, "Combustible"),
                (Color::GOLD, "Pieza radio"),
                (Color::WHITE, "Llamar helicoptero"),
            ]);
        }
        _ => {
            entradas.extend([
                (Color::LIGHTGRAY, "Pared"),
                (Color::BROWN, "Puerta"),
                (Color::YELLOW, "Llave"),
                (Color::ORANGE, "Municion"),
                (Color::GREEN, "Curacion"),
                (Color::BLUE, "Subir"),
                (Color::PURPLE, "Bajar"),
                (Color::DARKBROWN, "Puerta abierta"),
                (Color::PINK, "Combustible"),
            ]);
        }
    }

    let escala = escala.max(0.75);
    let x = offset_x + 7.0 * escala;
    let y = offset_y + 110.0 * escala;
    let ancho = 264.0 * escala;
    let filas = ((entradas.len() + 1) / 2) as f32;
    let alto = (39.0 + filas * 18.0) * escala;

    // Sombra para separar la leyenda del escenario.
    dibujo.draw_rectangle(
        (x + 3.0 * escala) as i32,
        (y + 4.0 * escala) as i32,
        ancho as i32,
        alto as i32,
        Color::new(0, 0, 0, 120),
    );

    dibujo.draw_rectangle(
        x as i32,
        y as i32,
        ancho as i32,
        alto as i32,
        Color::new(9, 12, 16, 225),
    );

    // Marco exterior e interior con estilo de interfaz de supervivencia.
    dibujo.draw_rectangle_lines(
        x as i32,
        y as i32,
        ancho as i32,
        alto as i32,
        Color::new(128, 30, 34, 255),
    );
    dibujo.draw_rectangle_lines(
        (x + 2.0 * escala) as i32,
        (y + 2.0 * escala) as i32,
        (ancho - 4.0 * escala) as i32,
        (alto - 4.0 * escala) as i32,
        Color::new(52, 58, 64, 230),
    );

    dibujo.draw_rectangle(
        (x + 3.0 * escala) as i32,
        (y + 3.0 * escala) as i32,
        (ancho - 6.0 * escala) as i32,
        (20.0 * escala) as i32,
        Color::new(74, 17, 21, 245),
    );

    dibujo.draw_text(
        "GUIA DEL MAPA",
        (x + 10.0 * escala) as i32,
        (y + 6.0 * escala) as i32,
        (12.0 * escala) as i32,
        Color::RAYWHITE,
    );

    let nombre_nivel = match mapa.nivel() {
        1 | 2 => "MANSION",
        3 => "LABORATORIO",
        4 => "HELIPUERTO",
        _ => "NIVEL",
    };
    let ancho_nombre = dibujo.measure_text(nombre_nivel, (10.0 * escala) as i32);
    dibujo.draw_text(
        nombre_nivel,
        (x + ancho - 10.0 * escala - ancho_nombre as f32) as i32,
        (y + 7.0 * escala) as i32,
        (10.0 * escala) as i32,
        Color::new(224, 184, 126, 255),
    );

    dibujo.draw_line(
        (x + 8.0 * escala) as i32,
        (y + 29.0 * escala) as i32,
        (x + ancho - 8.0 * escala) as i32,
        (y + 29.0 * escala) as i32,
        Color::new(93, 98, 102, 180),
    );

    for (indice, (color, nombre)) in entradas.iter().enumerate() {
        let columna = indice % 2;
        let fila = indice / 2;
        let entrada_x =
            x + (10.0 + columna as f32 * 127.0) * escala;
        let entrada_y =
            y + (37.0 + fila as f32 * 18.0) * escala;

        dibujo.draw_rectangle(
            (entrada_x - 1.0 * escala) as i32,
            (entrada_y - 1.0 * escala) as i32,
            (12.0 * escala) as i32,
            (12.0 * escala) as i32,
            Color::new(220, 220, 220, 210),
        );

        dibujo.draw_rectangle(
            entrada_x as i32,
            entrada_y as i32,
            (10.0 * escala) as i32,
            (10.0 * escala) as i32,
            *color,
        );

        dibujo.draw_text(
            nombre,
            (entrada_x + 16.0 * escala) as i32,
            (entrada_y - 2.0 * escala) as i32,
            (11.0 * escala) as i32,
            Color::new(224, 226, 228, 255),
        );
    }
}
