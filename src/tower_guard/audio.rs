use super::state::AudioCue;

pub struct TowerAudio {
    shot: Vec<u16>,
    impact: Vec<u16>,
    damage: Vec<u16>,
    boss_alert: Vec<u16>,
}

impl TowerAudio {
    pub fn load() -> Self {
        let path = |value: &str| value.encode_utf16().chain(Some(0)).collect();
        Self {
            shot: path("assets/minigames/tower_guardian/audio/shot.wav"),
            impact: path("assets/minigames/tower_guardian/audio/impact.wav"),
            damage: path("assets/minigames/tower_guardian/audio/damage.wav"),
            boss_alert: path("assets/minigames/tower_guardian/audio/boss-alert.wav"),
        }
    }

    pub fn play(&self, cue: AudioCue) {
        let path = match cue {
            AudioCue::Shot => &self.shot,
            AudioCue::Impact => &self.impact,
            AudioCue::Damage => &self.damage,
            AudioCue::BossAlert => &self.boss_alert,
        };

        #[cfg(target_os = "windows")]
        unsafe {
            #[link(name = "winmm")]
            unsafe extern "system" {
                fn PlaySoundW(sound: *const u16, module: *mut core::ffi::c_void, flags: u32)
                -> i32;
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
    }
}
