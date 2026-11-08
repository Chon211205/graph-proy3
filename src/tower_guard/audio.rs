use super::state::AudioCue;

pub struct TowerAudio;

impl TowerAudio {
    pub fn load() -> Self {
        Self
    }

    pub fn play(&self, cue: AudioCue) {
        let (alias, file) = match cue {
            AudioCue::Shot => ("re_shot", "shoot.mp3"),
            AudioCue::Impact => ("re_impact", "zombie.mp3"),
            AudioCue::Damage => ("re_damage", "damage.mp3"),
            AudioCue::BossAlert => ("re_boss", "tyrant.mp3"),
            AudioCue::Reload => ("re_reload", "reload.mp3"),
            AudioCue::Door => ("re_door", "door.mp3"),
            AudioCue::ZombieDie => ("re_die", "zombiedie.mp3"),
            AudioCue::NoAmmo => ("re_empty", "noammo.mp3"),
            AudioCue::Axe => ("re_axe", "axe.mp3"),
            AudioCue::Flame => ("re_flame", "flamethrow.mp3"),
            AudioCue::Key => ("re_key", "key.mp3"),
            AudioCue::Heal => ("re_heal", "heal.mp3"),
            AudioCue::CollectAmmo => ("re_ammo", "collectammo.mp3"),
        };
        command(&format!("close {alias}"));
        command(&format!(
            "open \"assets/minigames/tower_guardian/audio/{file}\" type mpegvideo alias {alias}"
        ));
        command(&format!("play {alias} from 0"));
    }

    pub fn stop_music(&self) {
        command("close re_mansion");
    }
}

#[cfg(target_os = "windows")]
fn command(value: &str) {
    #[link(name = "winmm")]
    unsafe extern "system" {
        fn mciSendStringW(
            command: *const u16,
            result: *mut u16,
            result_len: u32,
            callback: usize,
        ) -> u32;
    }
    let wide = value.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    unsafe {
        mciSendStringW(wide.as_ptr(), std::ptr::null_mut(), 0, 0);
    }
}

#[cfg(not(target_os = "windows"))]
fn command(_: &str) {}
