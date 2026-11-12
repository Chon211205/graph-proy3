//! Looped background music for the planet minigames.
//!
//! Playback uses Windows MCI so the project does not need an additional
//! runtime dependency. The existing minigame audio modules remain responsible
//! for short sound effects.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Track {
    CoreGuardian,
    Aquatic,
    Racing,
    TowerGuard,
    Hourglass,
}

impl Track {
    fn path(self) -> &'static str {
        match self {
            Self::CoreGuardian => "assets/minigames/music/core_guardian.mp3",
            Self::Aquatic => "assets/minigames/music/aquatic.wav",
            Self::Racing => "assets/minigames/music/racing.wav",
            Self::TowerGuard => "assets/minigames/music/tower_guard.wav",
            Self::Hourglass => "assets/minigames/music/hourglass.wav",
        }
    }

    fn kind(self) -> &'static str {
        if self == Self::CoreGuardian {
            "mpegvideo"
        } else {
            "waveaudio"
        }
    }
}

const ALIAS: &str = "planet_minigame_bgm";
const VOLUME_STEP: u32 = 100;

pub struct MusicPlayer {
    track: Option<Track>,
    volume: u32,
    muted: bool,
}

impl MusicPlayer {
    pub fn new() -> Self {
        Self {
            track: None,
            volume: 700,
            muted: false,
        }
    }

    pub fn update(&mut self, next: Option<Track>) {
        if self.track == next {
            return;
        }
        self.stop();
        let Some(track) = next else { return };
        let path = track.path().replace('/', "\\");
        command(&format!(
            "open \"{path}\" type {} alias {ALIAS}",
            track.kind()
        ));
        command(&format!(
            "setaudio {ALIAS} volume to {}",
            self.effective_volume()
        ));
        command(&format!("play {ALIAS} repeat"));
        self.track = Some(track);
    }

    pub fn stop(&mut self) {
        command(&format!("stop {ALIAS}"));
        command(&format!("close {ALIAS}"));
        self.track = None;
    }

    pub fn volume_percent(&self) -> u32 {
        self.volume / 10
    }

    pub fn muted(&self) -> bool {
        self.muted
    }

    pub fn adjust(&mut self, increase: bool) {
        if increase {
            self.volume = (self.volume + VOLUME_STEP).min(1_000);
        } else {
            self.volume = self.volume.saturating_sub(VOLUME_STEP);
        }
        self.apply_volume();
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        self.apply_volume();
    }

    fn effective_volume(&self) -> u32 {
        if self.muted { 0 } else { self.volume }
    }

    fn apply_volume(&self) {
        if self.track.is_some() {
            command(&format!(
                "setaudio {ALIAS} volume to {}",
                self.effective_volume()
            ));
        }
    }
}

impl Drop for MusicPlayer {
    fn drop(&mut self) {
        self.stop();
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
