use raylib::prelude::*;
use crate::audio::ModoMusica;
use crate::gamepad;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EstadoJuego {
    Menu,
    SeleccionNivel,
    Historia,
    HistoriaFinal,
    Jugando,
    Controles,
    Opciones,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AccionMenu {
    Ninguna,
    Jugar,
    SeleccionarNivel,
    Controles,
    Opciones,
    Salir,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NivelSeleccionado {
    Mansion,
    Laboratorio,
    Final,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AccionSeleccionNivel {
    Ninguna,
    Elegir(NivelSeleccionado),
    Volver,
}

pub struct Menu {
    opcion_menu: usize,
    opcion_nivel: usize,
}

impl Menu {
    pub fn new() -> Self {
        Self {
            opcion_menu: 0,
            opcion_nivel: 0,
        }
    }

    pub fn update(
        &mut self,
        rl: &RaylibHandle,
    ) -> AccionMenu {
        const TOTAL_OPCIONES: usize = 5;

        if rl.is_key_pressed(
            KeyboardKey::KEY_UP,
        ) || gamepad::arriba(rl) {
            if self.opcion_menu == 0 {
                self.opcion_menu =
                    TOTAL_OPCIONES - 1;
            } else {
                self.opcion_menu -= 1;
            }
        }

        if rl.is_key_pressed(
            KeyboardKey::KEY_DOWN,
        ) || gamepad::abajo(rl) {
            self.opcion_menu =
                (
                    self.opcion_menu + 1
                ) % TOTAL_OPCIONES;
        }

        if rl.is_key_pressed(
            KeyboardKey::KEY_ENTER,
        ) || gamepad::aceptar(rl) {
            return match self.opcion_menu {
                0 => AccionMenu::Jugar,

                1 => {
                    AccionMenu::SeleccionarNivel
                }

                2 => {
                    AccionMenu::Controles
                }

                3 => {
                    AccionMenu::Opciones
                }

                4 => {
                    AccionMenu::Salir
                }

                _ => {
                    AccionMenu::Ninguna
                }
            };
        }

        AccionMenu::Ninguna
    }

    pub fn update_opciones(
        &self,
        rl: &RaylibHandle,
        modo: &mut ModoMusica,
    ) -> bool {
        if rl.is_key_pressed(KeyboardKey::KEY_LEFT)
            || rl.is_key_pressed(KeyboardKey::KEY_RIGHT)
            || rl.is_key_pressed(KeyboardKey::KEY_ENTER)
            || gamepad::izquierda(rl)
            || gamepad::derecha(rl)
            || gamepad::aceptar(rl)
        {
            *modo = match *modo {
                ModoMusica::Normal => ModoMusica::BadBlood,
                ModoMusica::BadBlood => ModoMusica::Normal,
            };
        }

        rl.is_key_pressed(KeyboardKey::KEY_BACKSPACE)
            || gamepad::volver(rl)
    }

    pub fn update_seleccion_nivel(
        &mut self,
        rl: &RaylibHandle,
    ) -> AccionSeleccionNivel {
        const TOTAL_NIVELES: usize = 3;

        if rl.is_key_pressed(
            KeyboardKey::KEY_UP,
        ) || gamepad::arriba(rl) {
            if self.opcion_nivel == 0 {
                self.opcion_nivel =
                    TOTAL_NIVELES - 1;
            } else {
                self.opcion_nivel -= 1;
            }
        }

        if rl.is_key_pressed(
            KeyboardKey::KEY_DOWN,
        ) || gamepad::abajo(rl) {
            self.opcion_nivel =
                (
                    self.opcion_nivel + 1
                ) % TOTAL_NIVELES;
        }

        if rl.is_key_pressed(
            KeyboardKey::KEY_ENTER,
        ) || gamepad::aceptar(rl) {
            return match self.opcion_nivel {
                0 => {
                    AccionSeleccionNivel::Elegir(
                        NivelSeleccionado::Mansion,
                    )
                }

                1 => {
                    AccionSeleccionNivel::Elegir(
                        NivelSeleccionado::Laboratorio,
                    )
                }

                2 => {
                    AccionSeleccionNivel::Elegir(
                        NivelSeleccionado::Final,
                    )
                }

                _ => {
                    AccionSeleccionNivel::Ninguna
                }
            };
        }

        if rl.is_key_pressed(
            KeyboardKey::KEY_BACKSPACE,
        ) || gamepad::volver(rl) {
            return AccionSeleccionNivel::Volver;
        }

        AccionSeleccionNivel::Ninguna
    }

    pub fn render_menu(
        &self,
        d: &mut RaylibDrawHandle,
    ) {
        let sw =
            d.get_screen_width();

        let sh =
            d.get_screen_height();

        d.clear_background(
            Color::new(
                5,
                5,
                8,
                255,
            ),
        );

        for y in 0..sh {
            let factor =
                y as f32
                    / sh.max(1) as f32;

            let intensidad =
                (
                    16.0
                        * (
                            1.0 - factor
                        )
                ) as u8;

            d.draw_line(
                0,
                y,
                sw,
                y,
                Color::new(
                    intensidad,
                    0,
                    0,
                    255,
                ),
            );
        }

        let titulo =
            "NIGHTMARE";

        let subtitulo =
            "SURVIVAL";

        let titulo_tamano =
            64;

        let subtitulo_tamano =
            26;

        let titulo_ancho =
            d.measure_text(
                titulo,
                titulo_tamano,
            );

        let subtitulo_ancho =
            d.measure_text(
                subtitulo,
                subtitulo_tamano,
            );

        d.draw_text(
            titulo,
            sw / 2
                - titulo_ancho / 2,
            sh / 2
                - 230,
            titulo_tamano,
            Color::RED,
        );

        d.draw_text(
            subtitulo,
            sw / 2
                - subtitulo_ancho / 2,
            sh / 2
                - 160,
            subtitulo_tamano,
            Color::LIGHTGRAY,
        );

        let opciones = [
            "JUGAR",
            "SELECCIONAR NIVEL",
            "CONTROLES",
            "OPCIONES",
            "SALIR",
        ];

        for (
            indice,
            texto,
        ) in opciones
            .iter()
            .enumerate()
        {
            let seleccionado =
                indice == self.opcion_menu;

            let tamano =
                if seleccionado {
                    32
                } else {
                    27
                };

            let color =
                if seleccionado {
                    Color::GOLD
                } else {
                    Color::WHITE
                };

            let prefijo =
                if seleccionado {
                    "> "
                } else {
                    "  "
                };

            let opcion =
                format!(
                    "{}{}",
                    prefijo,
                    texto,
                );

            let ancho =
                d.measure_text(
                    &opcion,
                    tamano,
                );

            let y =
                sh / 2
                    - 55
                    + indice as i32
                        * 55;

            d.draw_text(
                &opcion,
                sw / 2
                    - ancho / 2,
                y,
                tamano,
                color,
            );
        }

        let ayuda =
            "FLECHAS - MOVER   ENTER - SELECCIONAR";

        let ayuda_ancho =
            d.measure_text(
                ayuda,
                18,
            );

        d.draw_text(
            ayuda,
            sw / 2
                - ayuda_ancho / 2,
            sh - 55,
            18,
            Color::GRAY,
        );
    }

    pub fn render_historia(
        &self,
        d: &mut RaylibDrawHandle,
        pagina: usize,
        nivel: NivelSeleccionado,
    ) {
        let sw =
            d.get_screen_width();

        let sh =
            d.get_screen_height();

        d.clear_background(
            Color::new(3, 4, 6, 255),
        );

        for y in 0..sh {
            let intensidad =
                (18.0 * (1.0 - y as f32 / sh.max(1) as f32))
                    as u8;

            d.draw_line(
                0,
                y,
                sw,
                y,
                Color::new(intensidad, 0, 0, 255),
            );
        }

        let (titulo, lineas, color_titulo) =
            match (nivel, pagina) {
                (NivelSeleccionado::Final, 0) => (
                    "ULTIMA TRANSMISION",
                    vec![
                        "Con el antivirus asegurado, Nohc logro llegar",
                        "al helipuerto para solicitar una evacuacion.",
                        "",
                        "Tyrant y Nemesis lo siguieron hasta la azotea.",
                        "La unica salida es reparar la radio",
                        "y resistir hasta que llegue el helicoptero.",
                    ],
                    Color::RED,
                ),
                (NivelSeleccionado::Final, _) => (
                    "NIVEL: HELIPUERTO FINAL",
                    vec![
                        "OBJETIVO",
                        "",
                        "Elimina 12 enemigos y derrota",
                        "al menos un licker de cada tipo.",
                        "Reune las 3 piezas de radio y usala",
                        "para llamar al helicoptero.",
                    ],
                    Color::GOLD,
                ),
                (NivelSeleccionado::Laboratorio, 0) => (
                    "DESCUBRIMIENTO SUBTERRANEO",
                    vec![
                        "Al explorar la mansion, el agente Nohc",
                        "descubrio un laboratorio subterraneo.",
                        "",
                        "En sus instalaciones se encuentra un antivirus",
                        "que podria servir para desarrollar una cura",
                        "contra el brote del virus T en Peten.",
                    ],
                    Color::RED,
                ),
                (NivelSeleccionado::Laboratorio, _) => (
                    "NIVEL: LABORATORIO",
                    vec![
                        "OBJETIVO",
                        "",
                        "Explora el laboratorio subterraneo.",
                        "Encuentra el antivirus ubicado al final",
                        "y escapa con el para desarrollar la cura.",
                    ],
                    Color::GOLD,
                ),
                (_, 0) => (
                    "ARCHIVO CONFIDENCIAL",
                    vec![
                        "Se encontro una nueva sede de Umbrella Corps:",
                        "una mansion abandonada en Guatemala.",
                        "",
                        "El agente Nohc fue enviado a investigar.",
                        "Al llegar, descubrio una casa llena de",
                        "armas biologicas.",
                    ],
                    Color::RED,
                ),
                (_, _) => (
                    "NIVEL: MANSION",
                    vec![
                        "OBJETIVO",
                        "",
                        "Elimina 20 zombis normales,",
                        "15 zombis medios y 5 zombis fuertes.",
                        "",
                        "Sobrevive y descubre que oculta la mansion.",
                    ],
                    Color::GOLD,
                ),
            };

        let tamano_titulo =
            42;

        let ancho_titulo =
            d.measure_text(titulo, tamano_titulo);

        d.draw_text(
            titulo,
            sw / 2 - ancho_titulo / 2,
            sh / 2 - 180,
            tamano_titulo,
            color_titulo,
        );

        for (indice, linea) in lineas.iter().enumerate() {
            let tamano =
                if *linea == "OBJETIVO" { 28 } else { 24 };

            let ancho =
                d.measure_text(linea, tamano);

            d.draw_text(
                linea,
                sw / 2 - ancho / 2,
                sh / 2 - 85 + indice as i32 * 36,
                tamano,
                if *linea == "OBJETIVO" {
                    Color::GOLD
                } else {
                    Color::RAYWHITE
                },
            );
        }

        let continuar =
            if pagina == 0 {
                "ENTER - CONTINUAR"
            } else {
                "ENTER - INICIAR MISION"
            };

        let ancho_continuar =
            d.measure_text(continuar, 20);

        d.draw_text(
            continuar,
            sw / 2 - ancho_continuar / 2,
            sh - 75,
            20,
            Color::LIGHTGRAY,
        );

        d.draw_text(
            "BACKSPACE - VOLVER",
            25,
            sh - 40,
            16,
            Color::GRAY,
        );
    }

    pub fn render_historia_final(
        &self,
        d: &mut RaylibDrawHandle,
    ) {
        let sw = d.get_screen_width();
        let sh = d.get_screen_height();

        d.clear_background(Color::new(2, 3, 5, 255));

        for y in 0..sh {
            let intensidad =
                (20.0 * (1.0 - y as f32 / sh.max(1) as f32))
                    as u8;

            d.draw_line(
                0,
                y,
                sw,
                y,
                Color::new(0, intensidad / 3, intensidad, 255),
            );
        }

        let titulo = "EPILOGO";
        let tamano_titulo = 48;
        let ancho_titulo =
            d.measure_text(titulo, tamano_titulo);

        d.draw_text(
            titulo,
            sw / 2 - ancho_titulo / 2,
            sh / 2 - 180,
            tamano_titulo,
            Color::GOLD,
        );

        let lineas = [
            "Nohc logro escapar con el antivirus.",
            "",
            "Lo llevo a un laboratorio del Gobierno",
            "de los Estados Unidos para crear una cura",
            "y evitar mas tragedias relacionadas",
            "con las armas biologicas.",
            "",
            "La mision habia terminado... por ahora.",
        ];

        for (indice, linea) in lineas.iter().enumerate() {
            let ancho = d.measure_text(linea, 24);

            d.draw_text(
                linea,
                sw / 2 - ancho / 2,
                sh / 2 - 80 + indice as i32 * 34,
                24,
                Color::RAYWHITE,
            );
        }

        let continuar = "ENTER - CONTINUAR";
        let ancho_continuar = d.measure_text(continuar, 20);

        d.draw_text(
            continuar,
            sw / 2 - ancho_continuar / 2,
            sh - 65,
            20,
            Color::LIGHTGRAY,
        );
    }

    pub fn render_seleccion_nivel(
        &self,
        d: &mut RaylibDrawHandle,
    ) {
        let sw =
            d.get_screen_width();

        let sh =
            d.get_screen_height();

        d.clear_background(
            Color::new(
                5,
                5,
                8,
                255,
            ),
        );

        for y in 0..sh {
            let factor =
                y as f32
                    / sh.max(1) as f32;

            let intensidad =
                (
                    18.0
                        * (
                            1.0 - factor
                        )
                ) as u8;

            d.draw_line(
                0,
                y,
                sw,
                y,
                Color::new(
                    0,
                    intensidad / 2,
                    intensidad,
                    255,
                ),
            );
        }

        let titulo =
            "SELECCIONAR NIVEL";

        let titulo_ancho =
            d.measure_text(
                titulo,
                48,
            );

        d.draw_text(
            titulo,
            sw / 2
                - titulo_ancho / 2,
            sh / 2
                - 200,
            48,
            Color::WHITE,
        );

        let niveles = [
            (
                "MANSION",
                "Sobrevive dentro de la mansion",
            ),
            (
                "LABORATORIO",
                "Sobrevive dentro del laboratorio",
            ),
            (
                "HELIPUERTO FINAL",
                "Repara la radio y llama al helicoptero",
            ),
        ];

        for (
            indice,
            (
                nombre,
                descripcion,
            ),
        ) in niveles
            .iter()
            .enumerate()
        {
            let seleccionado =
                indice == self.opcion_nivel;

            let y =
                sh / 2
                    - 70
                    + indice as i32
                        * 120;

            let color =
                if seleccionado {
                    Color::GOLD
                } else {
                    Color::WHITE
                };

            let texto =
                if seleccionado {
                    format!(
                        "> {}",
                        nombre,
                    )
                } else {
                    format!(
                        "  {}",
                        nombre,
                    )
                };

            let ancho =
                d.measure_text(
                    &texto,
                    32,
                );

            d.draw_text(
                &texto,
                sw / 2
                    - ancho / 2,
                y,
                32,
                color,
            );

            let descripcion_ancho =
                d.measure_text(
                    descripcion,
                    18,
                );

            d.draw_text(
                descripcion,
                sw / 2
                    - descripcion_ancho / 2,
                y + 42,
                18,
                Color::GRAY,
            );
        }

        let volver =
            "BACKSPACE - VOLVER";

        let volver_ancho =
            d.measure_text(
                volver,
                18,
            );

        d.draw_text(
            volver,
            sw / 2
                - volver_ancho / 2,
            sh - 60,
            18,
            Color::GRAY,
        );
    }

    pub fn render_opciones(
        &self,
        d: &mut RaylibDrawHandle,
        modo: ModoMusica,
    ) {
        let sw = d.get_screen_width();
        let sh = d.get_screen_height();

        d.clear_background(Color::new(5, 5, 8, 255));

        let titulo = "OPCIONES";
        let titulo_ancho = d.measure_text(titulo, 48);

        d.draw_text(
            titulo,
            sw / 2 - titulo_ancho / 2,
            sh / 2 - 150,
            48,
            Color::RED,
        );

        let seleccion = match modo {
            ModoMusica::Normal => "< MUSICA: NORMAL >",
            ModoMusica::BadBlood => "< MUSICA: BAD BLOOD >",
        };

        let ancho = d.measure_text(seleccion, 30);

        d.draw_text(
            seleccion,
            sw / 2 - ancho / 2,
            sh / 2 - 25,
            30,
            Color::GOLD,
        );

        let ayuda = "IZQUIERDA / DERECHA / ENTER - CAMBIAR";
        let ayuda_ancho = d.measure_text(ayuda, 18);

        d.draw_text(
            ayuda,
            sw / 2 - ayuda_ancho / 2,
            sh / 2 + 45,
            18,
            Color::LIGHTGRAY,
        );

        d.draw_text(
            "BACKSPACE - VOLVER",
            25,
            sh - 40,
            16,
            Color::GRAY,
        );
    }

    pub fn render_controles(
        &self,
        d: &mut RaylibDrawHandle,
        control_conectado: bool,
    ) {
        let sw =
            d.get_screen_width();

        let sh =
            d.get_screen_height();

        d.clear_background(
            Color::new(
                5,
                5,
                8,
                255,
            ),
        );

        let titulo =
            "CONTROLES";

        let titulo_ancho =
            d.measure_text(
                titulo,
                48,
            );

        d.draw_text(
            titulo,
            sw / 2
                - titulo_ancho / 2,
            60,
            48,
            Color::RED,
        );

        let gamepad_1 =
            "GAMEPAD: STICK IZQ MOVER | STICK DER GIRAR | LT APUNTAR | RT ATACAR";
        let gamepad_2 =
            "A INTERACTUAR | B VOLVER | X RECARGAR | CRUCETA CAMBIAR ARMA";

        let estado_gamepad =
            if control_conectado {
                "CONTROL CONECTADO"
            } else {
                "CONTROL NO DETECTADO"
            };

        let ancho_estado =
            d.measure_text(estado_gamepad, 16);

        d.draw_text(
            estado_gamepad,
            sw / 2 - ancho_estado / 2,
            92,
            16,
            if control_conectado {
                Color::GREEN
            } else {
                Color::ORANGE
            },
        );

        for (indice, texto) in [gamepad_1, gamepad_2]
            .iter()
            .enumerate()
        {
            let ancho = d.measure_text(texto, 15);
            d.draw_text(
                texto,
                sw / 2 - ancho / 2,
                112 + indice as i32 * 20,
                15,
                Color::SKYBLUE,
            );
        }

        let controles = [
            (
                "W A S D",
                "Movimiento",
            ),
            (
                "MOUSE",
                "Camara",
            ),
            (
                "CLICK DERECHO",
                "Apuntar / Bloquear",
            ),
            (
                "CLICK IZQUIERDO",
                "Disparar / Atacar",
            ),
            (
                "1",
                "Pistola",
            ),
            (
                "2",
                "Hacha",
            ),
            (
                "R",
                "Recargar",
            ),
            (
                "E",
                "Interactuar",
            ),
            (
                "TAB",
                "Liberar cursor",
            ),
            (
                "F5",
                "Reiniciar partida",
            ),
            (
                "F11",
                "Pantalla completa",
            ),
            (
                "BACKSPACE",
                "Volver al menu",
            ),
        ];

        let inicio_y =
            160;

        for (
            indice,
            (
                tecla,
                accion,
            ),
        ) in controles
            .iter()
            .enumerate()
        {
            let y =
                inicio_y
                    + indice as i32
                        * 30;

            d.draw_text(
                tecla,
                sw / 2
                    - 260,
                y,
                20,
                Color::GOLD,
            );

            d.draw_text(
                accion,
                sw / 2
                    - 20,
                y,
                20,
                Color::LIGHTGRAY,
            );
        }

        let volver =
            "BACKSPACE - VOLVER";

        let volver_ancho =
            d.measure_text(
                volver,
                18,
            );

        d.draw_text(
            volver,
            sw / 2
                - volver_ancho / 2,
            sh - 45,
            18,
            Color::GRAY,
        );
    }
}
