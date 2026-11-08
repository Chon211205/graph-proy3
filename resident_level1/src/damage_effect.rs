use raylib::prelude::*;

const DURACION_EFECTO: f32 = 0.35;

pub struct DamageEffect {
    tiempo_restante: f32,
}

impl DamageEffect {
    pub fn new() -> Self {
        Self {
            tiempo_restante: 0.0,
        }
    }

    pub fn activar(&mut self) {
        self.tiempo_restante = DURACION_EFECTO;
    }

    pub fn update(&mut self, delta_time: f32) {
        if self.tiempo_restante <= 0.0 {
            return;
        }

        self.tiempo_restante -= delta_time;

        if self.tiempo_restante < 0.0 {
            self.tiempo_restante = 0.0;
        }
    }

    pub fn render(
        &self,
        dibujo: &mut RaylibDrawHandle,
    ) {
        if self.tiempo_restante <= 0.0 {
            return;
        }

        let intensidad =
            (self.tiempo_restante / DURACION_EFECTO)
                .clamp(0.0, 1.0);

        let alpha_fondo =
            (95.0 * intensidad) as u8;

        let alpha_bordes =
            (180.0 * intensidad) as u8;

        let ancho =
            dibujo.get_screen_width();

        let alto =
            dibujo.get_screen_height();

        dibujo.draw_rectangle(
            0,
            0,
            ancho,
            alto,
            Color::new(
                180,
                0,
                0,
                alpha_fondo,
            ),
        );

        let grosor = 35;

        dibujo.draw_rectangle(
            0,
            0,
            ancho,
            grosor,
            Color::new(
                150,
                0,
                0,
                alpha_bordes,
            ),
        );

        dibujo.draw_rectangle(
            0,
            alto - grosor,
            ancho,
            grosor,
            Color::new(
                150,
                0,
                0,
                alpha_bordes,
            ),
        );

        dibujo.draw_rectangle(
            0,
            0,
            grosor,
            alto,
            Color::new(
                150,
                0,
                0,
                alpha_bordes,
            ),
        );

        dibujo.draw_rectangle(
            ancho - grosor,
            0,
            grosor,
            alto,
            Color::new(
                150,
                0,
                0,
                alpha_bordes,
            ),
        );
    }
}