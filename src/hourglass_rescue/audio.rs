#[derive(Clone, Copy)]
pub(super) enum Cue {
    Capsule,
    Damage,
    Inversion,
}

#[cfg_attr(test, allow(dead_code))]
pub(super) struct Audio {
    capsule: Vec<u16>,
    damage: Vec<u16>,
    inversion: Vec<u16>,
}

impl Audio {
    pub(super) fn load() -> Self {
        let path = |value: &str| value.encode_utf16().chain(Some(0)).collect();
        Self {
            capsule: path("assets/minigames/tower_guardian/audio/impact.wav"),
            damage: path("assets/minigames/tower_guardian/audio/damage.wav"),
            inversion: path("assets/minigames/tower_guardian/audio/boss-alert.wav"),
        }
    }

    pub(super) fn play(&self, cue: Cue) {
        #[cfg(test)]
        let _ = cue;

        #[cfg(not(test))]
        {
            let path = match cue {
                Cue::Capsule => &self.capsule,
                Cue::Damage => &self.damage,
                Cue::Inversion => &self.inversion,
            };

            #[cfg(target_os = "windows")]
            unsafe {
                #[link(name = "winmm")]
                unsafe extern "system" {
                    fn PlaySoundW(
                        sound: *const u16,
                        module: *mut core::ffi::c_void,
                        flags: u32,
                    ) -> i32;
                }

                const SND_ASYNC: u32 = 0x0001;
                const SND_NODEFAULT: u32 = 0x0002;
                const SND_FILENAME: u32 = 0x0002_0000;
                PlaySoundW(
                    path.as_ptr(),
                    std::ptr::null_mut(),
                    SND_ASYNC | SND_NODEFAULT | SND_FILENAME,
                );
            }

            #[cfg(not(target_os = "windows"))]
            let _ = path;
        }
    }
}
