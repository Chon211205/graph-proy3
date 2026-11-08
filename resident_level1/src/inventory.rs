pub struct Inventory {
    tiene_llave: bool,
}

impl Inventory {
    pub fn new() -> Self {
        Self {
            tiene_llave: false,
        }
    }

    pub fn recoger_llave(&mut self) {
        self.tiene_llave = true;
    }

    pub fn tiene_llave(&self) -> bool {
        self.tiene_llave
    }

    pub fn usar_llave(&mut self) {
        self.tiene_llave = false;
    }
}