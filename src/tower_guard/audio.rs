use raylib::core::audio::{RaylibAudio, Sound};

use super::state::AudioCue;

pub struct TowerAudio<'audio> {
    shot: Option<Sound<'audio>>,
    impact: Option<Sound<'audio>>,
    damage: Option<Sound<'audio>>,
    boss_alert: Option<Sound<'audio>>,
}

impl<'audio> TowerAudio<'audio> {
    pub fn load(device: &'audio RaylibAudio) -> Self {
        let load = |path: &str| device.new_sound(path).ok();
        let shot = load("assets/minigames/tower_guardian/audio/shot.wav");
        let impact = load("assets/minigames/tower_guardian/audio/impact.wav");
        let damage = load("assets/minigames/tower_guardian/audio/damage.wav");
        let boss_alert = load("assets/minigames/tower_guardian/audio/boss-alert.wav");
        for sound in [&shot, &impact, &damage, &boss_alert].into_iter().flatten() {
            sound.set_volume(0.32);
        }
        Self {
            shot,
            impact,
            damage,
            boss_alert,
        }
    }

    pub fn play(&self, cue: AudioCue) {
        let sound = match cue {
            AudioCue::Shot => &self.shot,
            AudioCue::Impact => &self.impact,
            AudioCue::Damage => &self.damage,
            AudioCue::BossAlert => &self.boss_alert,
        };
        if let Some(sound) = sound {
            sound.play();
        }
    }
}
