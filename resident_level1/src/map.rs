pub const TAMANO_CELDA: f32 = 25.0;

#[derive(Clone, Copy)]
pub enum TipoSpawnZombie {
    Normal,
    ConLlave,
    Medio,
    Fuerte,
}

pub struct Map {
    celdas: Vec<Vec<char>>,
    ancho: usize,
    alto: usize,
    nivel: i32,
}

impl Map {
    pub fn new(nivel: i32) -> Self {
        let contenido = match nivel {
            1 => include_str!("../mapa_nivel1.txt"),
            2 => include_str!("../mapa_nivel2.txt"),
            3 => include_str!("../lab.txt"),
            4 => include_str!("../mapa_final.txt"),
            _ => include_str!("../mapa_nivel1.txt"),
        };

        Self::desde_texto(contenido, nivel)
    }

    fn desde_texto(
        contenido: &str,
        nivel: i32,
    ) -> Self {
        let lineas: Vec<String> = contenido
            .lines()
            .map(|linea| {
                linea
                    .trim_end_matches('\r')
                    .to_string()
            })
            .collect();

        let alto = lineas.len();

        let ancho = lineas
            .iter()
            .map(|linea| {
                linea.chars().count()
            })
            .max()
            .unwrap_or(0);

        let mut celdas =
            Vec::with_capacity(alto);

        for linea in lineas {
            let mut fila: Vec<char> =
                linea.chars().collect();

            while fila.len() < ancho {
                fila.push('#');
            }

            celdas.push(fila);
        }

        Self {
            celdas,
            ancho,
            alto,
            nivel,
        }
    }

    pub fn nivel(&self) -> i32 {
        self.nivel
    }

    pub fn ancho(&self) -> usize {
        self.ancho
    }

    pub fn alto(&self) -> usize {
        self.alto
    }

    pub fn celda(
        &self,
        fila: i32,
        columna: i32,
    ) -> char {
        if fila < 0 || columna < 0 {
            return '#';
        }

        let fila =
            fila as usize;

        let columna =
            columna as usize;

        if fila >= self.alto
            || columna >= self.ancho
        {
            return '#';
        }

        self.celdas[fila][columna]
    }

    pub fn cambiar_celda(
        &mut self,
        fila: i32,
        columna: i32,
        nueva: char,
    ) {
        if fila < 0 || columna < 0 {
            return;
        }

        let fila =
            fila as usize;

        let columna =
            columna as usize;

        if fila >= self.alto
            || columna >= self.ancho
        {
            return;
        }

        self.celdas[fila][columna] =
            nueva;
    }

    pub fn es_bloque_solido(
        &self,
        fila: i32,
        columna: i32,
    ) -> bool {
        let celda =
            self.celda(
                fila,
                columna,
            );

        matches!(
            celda,
            '#'
                | 'D'
                | 'W'
                | 'X'
                | 'B'
                | 'J'
                | 'G'
        )
    }

    pub fn es_pared(
        &self,
        x: f32,
        y: f32,
    ) -> bool {
        let columna =
            (
                x / TAMANO_CELDA
            )
                .floor()
                as i32;

        let fila =
            (
                y / TAMANO_CELDA
            )
                .floor()
                as i32;

        self.es_bloque_solido(
            fila,
            columna,
        )
    }

    pub fn buscar_jugador(
        &self,
    ) -> Option<(f32, f32)> {
        for fila in 0..self.alto {
            for columna in 0..self.ancho {
                if self.celdas[fila][columna]
                    == 'P'
                {
                    let x =
                        columna as f32
                            * TAMANO_CELDA
                            + TAMANO_CELDA / 2.0;

                    let y =
                        fila as f32
                            * TAMANO_CELDA
                            + TAMANO_CELDA / 2.0;

                    return Some((x, y));
                }
            }
        }

        None
    }

    pub fn extraer_zombies(
        &mut self,
    ) -> Vec<(
        f32,
        f32,
        TipoSpawnZombie,
    )> {
        let mut zombies =
            Vec::new();

        for fila in 0..self.alto {
            for columna in 0..self.ancho {
                let tipo =
                    match self.celdas[fila][columna] {
                        'Z' => {
                            Some(
                                TipoSpawnZombie::Normal,
                            )
                        }

                        'L' => {
                            Some(
                                TipoSpawnZombie::ConLlave,
                            )
                        }

                        'M' => {
                            Some(
                                TipoSpawnZombie::Medio,
                            )
                        }

                        'F' => {
                            Some(
                                TipoSpawnZombie::Fuerte,
                            )
                        }

                        _ => None,
                    };

                if let Some(tipo) = tipo {
                    let x =
                        columna as f32
                            * TAMANO_CELDA
                            + TAMANO_CELDA / 2.0;

                    let y =
                        fila as f32
                            * TAMANO_CELDA
                            + TAMANO_CELDA / 2.0;

                    zombies.push((
                        x,
                        y,
                        tipo,
                    ));

                    self.celdas[fila][columna] =
                        ' ';
                }
            }
        }

        zombies
    }

    pub fn buscar_portales(
        &self,
        simbolo: char,
    ) -> Vec<(usize, usize)> {
        let mut portales =
            Vec::new();

        for fila in 0..self.alto {
            for columna in 0..self.ancho {
                if self.celdas[fila][columna]
                    == simbolo
                {
                    portales.push((
                        fila,
                        columna,
                    ));
                }
            }
        }

        portales.sort_by_key(
            |&(_, columna)| {
                columna
            },
        );

        portales
    }

    pub fn indice_portal_en(
        &self,
        simbolo: char,
        fila: i32,
        columna: i32,
    ) -> Option<usize> {
        if fila < 0
            || columna < 0
        {
            return None;
        }

        let fila =
            fila as usize;

        let columna =
            columna as usize;

        let portales =
            self.buscar_portales(
                simbolo,
            );

        portales
            .iter()
            .position(
                |&(
                    portal_fila,
                    portal_columna,
                )| {
                    portal_fila == fila
                        && portal_columna
                            == columna
                },
            )
    }

    pub fn posicion_portal(
        &self,
        simbolo: char,
        indice: usize,
    ) -> Option<(f32, f32)> {
        let portales =
            self.buscar_portales(
                simbolo,
            );

        let &(
            fila,
            columna,
        ) =
            portales.get(indice)?;

        let x =
            columna as f32
                * TAMANO_CELDA
                + TAMANO_CELDA / 2.0;

        let y =
            fila as f32
                * TAMANO_CELDA
                + TAMANO_CELDA / 2.0;

        Some((x, y))
    }

    pub fn posicion_entrada_portal(
        &self,
        simbolo: char,
        indice: usize,
    ) -> Option<(f32, f32)> {
        let portales =
            self.buscar_portales(
                simbolo,
            );

        let &(
            fila,
            columna,
        ) =
            portales.get(indice)?;

        let vecinos = [
            (
                fila as i32,
                columna as i32 - 1,
            ),
            (
                fila as i32,
                columna as i32 + 1,
            ),
            (
                fila as i32 - 1,
                columna as i32,
            ),
            (
                fila as i32 + 1,
                columna as i32,
            ),
        ];

        for (
            vecino_fila,
            vecino_columna,
        ) in vecinos
        {
            let celda =
                self.celda(
                    vecino_fila,
                    vecino_columna,
                );

            if matches!(
                celda,
                ' '
                    | 'C'
                    | 'P'
                    | 'O'
            ) {
                let x =
                    vecino_columna as f32
                        * TAMANO_CELDA
                        + TAMANO_CELDA / 2.0;

                let y =
                    vecino_fila as f32
                        * TAMANO_CELDA
                        + TAMANO_CELDA / 2.0;

                return Some((x, y));
            }
        }

        None
    }

    pub fn guardar_txt(
        &self,
        ruta: &str,
    ) -> std::io::Result<()> {
        use std::fs::File;
        use std::io::Write;

        let mut archivo =
            File::create(ruta)?;

        for fila in &self.celdas {
            let linea: String =
                fila
                    .iter()
                    .collect();

            writeln!(
                archivo,
                "{}",
                linea,
            )?;
        }

        Ok(())
    }
}
