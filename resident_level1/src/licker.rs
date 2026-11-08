use crate::map::{
    Map,
    TAMANO_CELDA,
};
use crate::player::Player;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EstadoLicker {
    Suelo,
    Trepando,
    Pared,
    Techo,
    Cayendo,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TipoLicker {
    Normal,
    Medio,
    Fuerte,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LadoPared {
    Izquierda,
    Derecha,
    Arriba,
    Abajo,
}

pub struct Licker {
    pub x: f32,
    pub y: f32,

    pub vida: i32,
    pub vivo: bool,
    pub persiguiendo: bool,
    pub tipo: TipoLicker,

    pub estado: EstadoLicker,
    pub lado_pared: Option<LadoPared>,

    pub altura: f32,
    pub tiempo_animacion: f32,

    tiempo_ataque: f32,
    tiempo_cambio_estado: f32,
}

impl Licker {
    pub fn new(
        x: f32,
        y: f32,
    ) -> Self {
        Self::new_con_tipo(x, y, TipoLicker::Normal)
    }

    pub fn new_medio(x: f32, y: f32) -> Self {
        Self::new_con_tipo(x, y, TipoLicker::Medio)
    }

    pub fn new_fuerte(x: f32, y: f32) -> Self {
        Self::new_con_tipo(x, y, TipoLicker::Fuerte)
    }

    fn new_con_tipo(x: f32, y: f32, tipo: TipoLicker) -> Self {
        let vida = match tipo {
            TipoLicker::Normal => 180,
            TipoLicker::Medio => 260,
            TipoLicker::Fuerte => 360,
        };

        Self {
            x,
            y,

            vida,
            vivo: true,
            persiguiendo: false,
            tipo,

            estado: EstadoLicker::Suelo,
            lado_pared: None,

            altura: 0.0,
            tiempo_animacion: 0.0,

            tiempo_ataque: 0.0,
            tiempo_cambio_estado: 0.0,
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
        }
    }

    pub fn update(
        &mut self,
        player: &Player,
        mapa: &Map,
        delta_time: f32,
        puede_trepar: bool,
    ) -> i32 {
        if !self.vivo {
            return 0;
        }

        if !puede_trepar {
            self.estado =
                EstadoLicker::Suelo;
            self.lado_pared =
                None;
            self.altura =
                0.0;
            self.tiempo_cambio_estado =
                0.0;
        }

        self.tiempo_animacion +=
            delta_time;

        self.tiempo_ataque +=
            delta_time;

        self.tiempo_cambio_estado +=
            delta_time;

        match self.estado {
            EstadoLicker::Suelo => {
                self.update_suelo(
                    player,
                    mapa,
                    delta_time,
                    puede_trepar,
                )
            }

            EstadoLicker::Trepando => {
                self.update_trepando(
                    delta_time,
                );

                0
            }

            EstadoLicker::Pared => {
                self.update_pared(
                    player,
                    delta_time,
                );

                0
            }

            EstadoLicker::Techo => {
                self.update_techo(
                    player,
                    mapa,
                    delta_time,
                )
            }

            EstadoLicker::Cayendo => {
                self.update_cayendo(
                    player,
                    delta_time,
                )
            }
        }
    }

    fn update_suelo(
        &mut self,
        player: &Player,
        mapa: &Map,
        delta_time: f32,
        puede_trepar: bool,
    ) -> i32 {
        const VELOCIDAD: f32 = 35.0;
        const DISTANCIA_ATAQUE: f32 = 26.0;
        const DANO: i32 = 18;

        let dx =
            player.x - self.x;

        let dy =
            player.y - self.y;

        let distancia =
            (dx * dx + dy * dy)
                .sqrt();

        if distancia
            <= DISTANCIA_ATAQUE
        {
            self.persiguiendo = false;
            if self.tiempo_ataque
                >= 1.1
            {
                self.tiempo_ataque =
                    0.0;

                return DANO;
            }

            return 0;
        }

        self.persiguiendo = true;

        if puede_trepar
            && self.tiempo_cambio_estado
                >= 1.5
        {
            if let Some(lado) =
                detectar_pared_cercana(
                    mapa,
                    self.x,
                    self.y,
                )
            {
                self.estado =
                    EstadoLicker::Trepando;

                self.lado_pared =
                    Some(lado);

                self.tiempo_cambio_estado =
                    0.0;

                return 0;
            }
        }

        mover_hacia(
            &mut self.x,
            &mut self.y,
            player.x,
            player.y,
            VELOCIDAD,
            delta_time,
            mapa,
        );

        0
    }

    fn update_trepando(
        &mut self,
        delta_time: f32,
    ) {
        const VELOCIDAD_SUBIDA: f32 = 1.4;

        self.altura +=
            VELOCIDAD_SUBIDA
                * delta_time;

        if self.altura
            >= 0.65
        {
            self.altura =
                0.65;

            self.estado =
                EstadoLicker::Pared;

            self.tiempo_cambio_estado =
                0.0;
        }
    }

    fn update_pared(
        &mut self,
        player: &Player,
        delta_time: f32,
    ) {
        const VELOCIDAD_PARED: f32 = 24.0;

        let Some(lado) =
            self.lado_pared
        else {
            self.estado =
                EstadoLicker::Suelo;

            self.altura =
                0.0;

            return;
        };

        match lado {
            LadoPared::Izquierda
            | LadoPared::Derecha => {
                let diferencia =
                    player.y - self.y;

                if diferencia.abs()
                    > 2.0
                {
                    self.y +=
                        diferencia.signum()
                            * VELOCIDAD_PARED
                            * delta_time;
                }
            }

            LadoPared::Arriba
            | LadoPared::Abajo => {
                let diferencia =
                    player.x - self.x;

                if diferencia.abs()
                    > 2.0
                {
                    self.x +=
                        diferencia.signum()
                            * VELOCIDAD_PARED
                            * delta_time;
                }
            }
        }

        self.altura +=
            0.55
                * delta_time;

        if self.altura >= 1.0 {
            self.altura =
                1.0;

            self.estado =
                EstadoLicker::Techo;

            self.lado_pared =
                None;

            self.tiempo_cambio_estado =
                0.0;
        }
    }

    fn update_techo(
        &mut self,
        player: &Player,
        mapa: &Map,
        delta_time: f32,
    ) -> i32 {
        const VELOCIDAD_TECHO: f32 = 42.0;
        const DISTANCIA_CAIDA: f32 = 65.0;

        let dx =
            player.x - self.x;

        let dy =
            player.y - self.y;

        let distancia =
            (dx * dx + dy * dy)
                .sqrt();

        if distancia
            <= DISTANCIA_CAIDA
            && self.tiempo_cambio_estado
                >= 1.0
        {
            self.estado =
                EstadoLicker::Cayendo;

            self.tiempo_cambio_estado =
                0.0;

            return 0;
        }

        mover_hacia(
            &mut self.x,
            &mut self.y,
            player.x,
            player.y,
            VELOCIDAD_TECHO,
            delta_time,
            mapa,
        );

        0
    }

    fn update_cayendo(
        &mut self,
        player: &Player,
        delta_time: f32,
    ) -> i32 {
        const VELOCIDAD_CAIDA: f32 = 2.8;
        const DISTANCIA_GOLPE: f32 = 35.0;
        const DANO_CAIDA: i32 = 28;

        self.altura -=
            VELOCIDAD_CAIDA
                * delta_time;

        if self.altura > 0.0 {
            return 0;
        }

        self.altura =
            0.0;

        self.estado =
            EstadoLicker::Suelo;

        self.tiempo_cambio_estado =
            0.0;

        let dx =
            player.x - self.x;

        let dy =
            player.y - self.y;

        let distancia =
            (dx * dx + dy * dy)
                .sqrt();

        if distancia
            <= DISTANCIA_GOLPE
        {
            self.tiempo_ataque =
                0.0;

            return DANO_CAIDA;
        }

        0
    }
}

fn detectar_pared_cercana(
    mapa: &Map,
    x: f32,
    y: f32,
) -> Option<LadoPared> {
    let distancia =
        TAMANO_CELDA
            * 0.55;

    let izquierda =
        mapa.es_pared(
            x - distancia,
            y,
        );

    let derecha =
        mapa.es_pared(
            x + distancia,
            y,
        );

    let arriba =
        mapa.es_pared(
            x,
            y - distancia,
        );

    let abajo =
        mapa.es_pared(
            x,
            y + distancia,
        );

    if izquierda {
        return Some(
            LadoPared::Izquierda,
        );
    }

    if derecha {
        return Some(
            LadoPared::Derecha,
        );
    }

    if arriba {
        return Some(
            LadoPared::Arriba,
        );
    }

    if abajo {
        return Some(
            LadoPared::Abajo,
        );
    }

    None
}

fn mover_hacia(
    x: &mut f32,
    y: &mut f32,
    objetivo_x: f32,
    objetivo_y: f32,
    velocidad: f32,
    delta_time: f32,
    mapa: &Map,
) {
    let dx =
        objetivo_x - *x;

    let dy =
        objetivo_y - *y;

    let distancia =
        (dx * dx + dy * dy)
            .sqrt();

    if distancia <= 0.001 {
        return;
    }

    let dir_x =
        dx / distancia;

    let dir_y =
        dy / distancia;

    let movimiento =
        velocidad
            * delta_time;

    let nuevo_x =
        *x
            + dir_x
                * movimiento;

    let nuevo_y =
        *y
            + dir_y
                * movimiento;

    if !mapa.es_pared(
        nuevo_x,
        *y,
    ) {
        *x =
            nuevo_x;
    }

    if !mapa.es_pared(
        *x,
        nuevo_y,
    ) {
        *y =
            nuevo_y;
    }
}
