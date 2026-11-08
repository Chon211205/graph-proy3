use raylib::audio::{
    Music,
    RaylibAudio,
    Sound,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ModoMusica {
    Normal,
    BadBlood,
}

pub struct AudioManager<'a> {
    shoot: Sound<'a>,
    reload: Sound<'a>,
    door: Sound<'a>,
    noammo: Sound<'a>,

    zombie: Sound<'a>,
    zombie2: Sound<'a>,
    zombie3: Sound<'a>,

    damage: Sound<'a>,
    key: Sound<'a>,
    zombiedie: Sound<'a>,
    collectammo: Sound<'a>,

    axe: Sound<'a>,
    axeblock: Sound<'a>,
    heal: Sound<'a>,

    tyrant: Sound<'a>,
    nemesis: Sound<'a>,
    nemesisshoot: Sound<'a>,
    flamethrow: Sound<'a>,

    licker: Sound<'a>,
    lickerdie: Sound<'a>,

    mansion: Music<'a>,
    laboratorio: Music<'a>,
    badblood: Music<'a>,
}

impl<'a> AudioManager<'a> {
    pub fn new(
        audio: &'a RaylibAudio,
    ) -> Self {
        let mansion =
            audio
                .new_music(
                    "assets/sounds/mansion.mp3",
                )
                .expect(
                    "No se pudo cargar mansion.mp3",
                );

        mansion.set_volume(
            0.30,
        );

        let laboratorio =
            audio
                .new_music(
                    "assets/sounds/lab.mp3",
                )
                .expect(
                    "No se pudo cargar lab.mp3",
                );

        laboratorio.set_volume(
            0.30,
        );

        let badblood = audio
            .new_music("assets/sounds/badblood.mp3")
            .expect("No se pudo cargar badblood.mp3");

        badblood.set_volume(0.35);

        Self {
            shoot: audio
                .new_sound(
                    "assets/sounds/shoot.mp3",
                )
                .expect(
                    "No se pudo cargar shoot.mp3",
                ),

            reload: audio
                .new_sound(
                    "assets/sounds/reload.mp3",
                )
                .expect(
                    "No se pudo cargar reload.mp3",
                ),

            door: audio
                .new_sound(
                    "assets/sounds/door.mp3",
                )
                .expect(
                    "No se pudo cargar door.mp3",
                ),

            noammo: audio
                .new_sound(
                    "assets/sounds/noammo.mp3",
                )
                .expect(
                    "No se pudo cargar noammo.mp3",
                ),

            zombie: audio
                .new_sound(
                    "assets/sounds/zombie.mp3",
                )
                .expect(
                    "No se pudo cargar zombie.mp3",
                ),

            zombie2: audio
                .new_sound(
                    "assets/sounds/zombie2.mp3",
                )
                .expect(
                    "No se pudo cargar zombie2.mp3",
                ),

            zombie3: audio
                .new_sound(
                    "assets/sounds/zombie3.mp3",
                )
                .expect(
                    "No se pudo cargar zombie3.mp3",
                ),

            damage: audio
                .new_sound(
                    "assets/sounds/damage.mp3",
                )
                .expect(
                    "No se pudo cargar damage.mp3",
                ),

            key: audio
                .new_sound(
                    "assets/sounds/key.mp3",
                )
                .expect(
                    "No se pudo cargar key.mp3",
                ),

            zombiedie: audio
                .new_sound(
                    "assets/sounds/zombiedie.mp3",
                )
                .expect(
                    "No se pudo cargar zombiedie.mp3",
                ),

            collectammo: audio
                .new_sound(
                    "assets/sounds/collectammo.mp3",
                )
                .expect(
                    "No se pudo cargar collectammo.mp3",
                ),

            axe: audio
                .new_sound(
                    "assets/sounds/axe.mp3",
                )
                .expect(
                    "No se pudo cargar axe.mp3",
                ),

            axeblock: audio
                .new_sound(
                    "assets/sounds/axeblock.mp3",
                )
                .expect(
                    "No se pudo cargar axeblock.mp3",
                ),

            heal: audio
                .new_sound(
                    "assets/sounds/heal.mp3",
                )
                .expect(
                    "No se pudo cargar heal.mp3",
                ),

            tyrant: audio
                .new_sound(
                    "assets/sounds/tyrant.mp3",
                )
                .expect(
                    "No se pudo cargar tyrant.mp3",
                ),

            nemesis: audio
                .new_sound(
                    "assets/sounds/nemesis.mp3",
                )
                .expect(
                    "No se pudo cargar nemesis.mp3",
                ),

            nemesisshoot: audio
                .new_sound(
                    "assets/sounds/nemesisshoot.mp3",
                )
                .expect(
                    "No se pudo cargar nemesisshoot.mp3",
                ),

            flamethrow: audio
                .new_sound(
                    "assets/sounds/flamethrow.mp3",
                )
                .expect(
                    "No se pudo cargar flamethrow.mp3",
                ),

            licker: audio
                .new_sound(
                    "assets/sounds/licker.mp3",
                )
                .expect(
                    "No se pudo cargar licker.mp3",
                ),

            lickerdie: audio
                .new_sound(
                    "assets/sounds/lickerdie.mp3",
                )
                .expect(
                    "No se pudo cargar lickerdie.mp3",
                ),

            mansion,
            laboratorio,
            badblood,
        }
    }

    pub fn actualizar_musica(
        &self,
        es_laboratorio: bool,
        modo: ModoMusica,
    ) {
        if modo == ModoMusica::BadBlood {
            self.badblood.update_stream();

            if self.mansion.is_stream_playing() {
                self.mansion.stop_stream();
            }

            if self.laboratorio.is_stream_playing() {
                self.laboratorio.stop_stream();
            }

            if !self.badblood.is_stream_playing() {
                self.badblood.play_stream();
            }

            return;
        }

        if self.badblood.is_stream_playing() {
            self.badblood.stop_stream();
        }

        if es_laboratorio {
            self.laboratorio.update_stream();

            if self.mansion.is_stream_playing() {
                self.mansion.stop_stream();
            }

            if !self.laboratorio.is_stream_playing() {
                self.laboratorio.play_stream();
            }
        } else {
            self.mansion.update_stream();

            if self.laboratorio.is_stream_playing() {
                self.laboratorio.stop_stream();
            }

            if !self.mansion.is_stream_playing() {
                self.mansion.play_stream();
            }
        }
    }

    pub fn iniciar_musica(
        &self,
        es_laboratorio: bool,
        modo: ModoMusica,
    ) {
        self.actualizar_musica(
            es_laboratorio,
            modo,
        );
    }

    pub fn detener_musica(
        &self,
    ) {
        if self.mansion
            .is_stream_playing()
        {
            self.mansion
                .stop_stream();
        }

        if self.laboratorio
            .is_stream_playing()
        {
            self.laboratorio
                .stop_stream();
        }

        if self.badblood.is_stream_playing() {
            self.badblood.stop_stream();
        }
    }

    pub fn disparo(
        &self,
    ) {
        self.shoot.play();
    }

    pub fn recarga(
        &self,
    ) {
        self.reload.play();
    }

    pub fn puerta(
        &self,
    ) {
        self.door.play();
    }

    pub fn sin_municion(
        &self,
    ) {
        self.noammo.play();
    }

    pub fn zombie(
        &self,
    ) {
        if !self.zombie
            .is_playing()
        {
            self.zombie.play();
        }
    }

    pub fn detener_zombie(
        &self,
    ) {
        if self.zombie
            .is_playing()
        {
            self.zombie.stop();
        }
    }

    pub fn zombie_medio(
        &self,
    ) {
        if !self.zombie2
            .is_playing()
        {
            self.zombie2.play();
        }
    }

    pub fn detener_zombie_medio(
        &self,
    ) {
        if self.zombie2
            .is_playing()
        {
            self.zombie2.stop();
        }
    }

    pub fn zombie_fuerte(
        &self,
    ) {
        if !self.zombie3
            .is_playing()
        {
            self.zombie3.play();
        }
    }

    pub fn detener_zombie_fuerte(
        &self,
    ) {
        if self.zombie3
            .is_playing()
        {
            self.zombie3.stop();
        }
    }

    pub fn dano(
        &self,
    ) {
        self.damage.play();
    }

    pub fn llave(
        &self,
    ) {
        self.key.play();
    }

    pub fn zombie_muere(
        &self,
    ) {
        self.zombiedie.play();
    }

    pub fn detener_zombie_muere(
        &self,
    ) {
        if self.zombiedie
            .is_playing()
        {
            self.zombiedie.stop();
        }
    }

    pub fn recoger_municion(
        &self,
    ) {
        self.collectammo.play();
    }

    pub fn hachazo(
        &self,
    ) {
        self.axe.play();
    }

    pub fn bloqueo_hacha(
        &self,
    ) {
        self.axeblock.play();
    }

    pub fn curacion(
        &self,
    ) {
        self.heal.play();
    }

    pub fn tyrant(
        &self,
    ) {
        if !self.tyrant
            .is_playing()
        {
            self.tyrant.play();
        }
    }

    pub fn detener_tyrant(
        &self,
    ) {
        if self.tyrant
            .is_playing()
        {
            self.tyrant.stop();
        }
    }

    pub fn nemesis(
        &self,
    ) {
        if !self.nemesis
            .is_playing()
        {
            self.nemesis.play();
        }
    }

    pub fn detener_nemesis(
        &self,
    ) {
        if self.nemesis
            .is_playing()
        {
            self.nemesis.stop();
        }
    }

    pub fn disparo_nemesis(
        &self,
    ) {
        self.nemesisshoot.play();
    }

    pub fn lanzallamas(
        &self,
    ) {
        if !self.flamethrow.is_playing() {
            self.flamethrow.play();
        }
    }

    pub fn detener_lanzallamas(
        &self,
    ) {
        if self.flamethrow.is_playing() {
            self.flamethrow.stop();
        }
    }

    pub fn licker(
        &self,
    ) {
        if !self.licker
            .is_playing()
        {
            self.licker.play();
        }
    }

    pub fn detener_licker(
        &self,
    ) {
        if self.licker
            .is_playing()
        {
            self.licker.stop();
        }
    }

    pub fn licker_muere(
        &self,
    ) {
        self.lickerdie.play();
    }

    pub fn detener_licker_muere(
        &self,
    ) {
        if self.lickerdie
            .is_playing()
        {
            self.lickerdie.stop();
        }
    }

    pub fn detener_todos_enemigos(
        &self,
    ) {
        self.detener_zombie();
        self.detener_zombie_medio();
        self.detener_zombie_fuerte();

        self.detener_tyrant();
        self.detener_nemesis();
        self.detener_lanzallamas();

        self.detener_licker();

        self.detener_zombie_muere();
        self.detener_licker_muere();
    }
}
