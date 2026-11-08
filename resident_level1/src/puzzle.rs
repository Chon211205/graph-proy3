pub struct Puzzle {
    pub llave_recogida: bool,
    pub puerta_abierta: bool,
}

impl Puzzle {
    pub fn new() -> Self {
        Self {
            llave_recogida: false,
            puerta_abierta: false,
        }
    }

    pub fn recoger_llave(&mut self) {
        self.llave_recogida = true;
    }

    pub fn abrir_puerta(&mut self) {
        self.puerta_abierta = true;
    }
}