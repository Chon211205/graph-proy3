use crate::map::Map;
use crate::player::Player;
use crate::raycaster::lanzar_rayo;

#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
)]
pub enum TipoZombie {
    Normal,
    Medio,
    Fuerte,
}

pub struct Zombie {
    pub x: f32,
    pub y: f32,
    pub vida: i32,
    pub velocidad: f32,
    pub vivo: bool,
    pub persiguiendo: bool,
    pub tiempo_animacion: f32,
    pub puede_dropear_llave: bool,
    pub tipo: TipoZombie,
    tiempo_ultimo_ataque: f32,
}

impl Zombie {
    pub fn new(
        x: f32,
        y: f32,
    ) -> Self {
        Self {
            x,
            y,
            vida: 100,
            velocidad: 22.0,
            vivo: true,
            persiguiendo: false,
            tiempo_animacion: 0.0,
            puede_dropear_llave: false,
            tipo: TipoZombie::Normal,
            tiempo_ultimo_ataque: 0.0,
        }
    }

    pub fn new_con_llave(
        x: f32,
        y: f32,
    ) -> Self {
        Self {
            x,
            y,
            vida: 100,
            velocidad: 22.0,
            vivo: true,
            persiguiendo: false,
            tiempo_animacion: 0.0,
            puede_dropear_llave: true,
            tipo: TipoZombie::Normal,
            tiempo_ultimo_ataque: 0.0,
        }
    }

    pub fn new_medio(
        x: f32,
        y: f32,
    ) -> Self {
        Self {
            x,
            y,
            vida: 175,
            velocidad: 27.0,
            vivo: true,
            persiguiendo: false,
            tiempo_animacion: 0.0,
            puede_dropear_llave: false,
            tipo: TipoZombie::Medio,
            tiempo_ultimo_ataque: 0.0,
        }
    }

    pub fn new_fuerte(
        x: f32,
        y: f32,
    ) -> Self {
        Self {
            x,
            y,
            vida: 300,
            velocidad: 24.0,
            vivo: true,
            persiguiendo: false,
            tiempo_animacion: 0.0,
            puede_dropear_llave: false,
            tipo: TipoZombie::Fuerte,
            tiempo_ultimo_ataque: 0.0,
        }
    }

    pub fn recibir_dano(
        &mut self,
        cantidad: i32,
    ) {
        if !self.vivo {
            return;
        }

        self.vida -= cantidad;

        if self.vida <= 0 {
            self.vida = 0;
            self.vivo = false;
            self.persiguiendo = false;
        }
    }

    pub fn update(
        &mut self,
        player: &Player,
        mapa: &Map,
        delta_time: f32,
    ) -> i32 {
        if !self.vivo {
            return 0;
        }

        self.tiempo_ultimo_ataque +=
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

        let rango_deteccion =
            match self.tipo {
                TipoZombie::Normal => {
                    250.0
                }

                TipoZombie::Medio => {
                    320.0
                }

                TipoZombie::Fuerte => {
                    380.0
                }
            };

        let distancia_ataque =
            match self.tipo {
                TipoZombie::Normal => {
                    22.0
                }

                TipoZombie::Medio => {
                    24.0
                }

                TipoZombie::Fuerte => {
                    26.0
                }
            };

        if distancia
            <= rango_deteccion
            && self
                .tiene_linea_vision(
                    player,
                    mapa,
                    distancia,
                )
        {
            self.persiguiendo =
                true;
        }

        if !self.persiguiendo {
            return 0;
        }

        if distancia
            > distancia_ataque
        {
            if distancia > 0.001 {
                let direccion_x =
                    dx / distancia;

                let direccion_y =
                    dy / distancia;

                let movimiento =
                    self.velocidad
                        * delta_time;

                let nuevo_x =
                    self.x
                        + direccion_x
                            * movimiento;

                let nuevo_y =
                    self.y
                        + direccion_y
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
            }

            self.tiempo_animacion +=
                delta_time;

            return 0;
        }

        if self.tiempo_ultimo_ataque
            >= 1.0
        {
            self.tiempo_ultimo_ataque =
                0.0;

            return match self.tipo {
                TipoZombie::Normal => 10,
                TipoZombie::Medio => 15,
                TipoZombie::Fuerte => 25,
            };
        }

        0
    }

    fn tiene_linea_vision(
        &self,
        player: &Player,
        mapa: &Map,
        distancia_jugador: f32,
    ) -> bool {
        if distancia_jugador
            <= 0.001
        {
            return true;
        }

        let angulo =
            (
                player.y
                    - self.y
            )
                .atan2(
                    player.x
                        - self.x,
                );

        let hit =
            lanzar_rayo(
                mapa,
                self.x,
                self.y,
                angulo,
            );

        hit.distancia
            >= distancia_jugador
                - 4.0
    }
}