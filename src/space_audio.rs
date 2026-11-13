/// Play the short ship blaster cue without blocking the game loop.
///
/// The WAV is embedded at compile time and stays alive for the asynchronous
/// WinMM playback call. Other platforms and unit-test builds remain silent.
pub(crate) fn play_ship_shot() {
    #[cfg(all(target_os = "windows", not(test)))]
    unsafe {
        #[link(name = "winmm")]
        unsafe extern "system" {
            fn PlaySoundW(sound: *const u16, module: *mut core::ffi::c_void, flags: u32) -> i32;
        }

        const SND_ASYNC: u32 = 0x0001;
        const SND_MEMORY: u32 = 0x0004;
        const SND_NODEFAULT: u32 = 0x0002;
        static SHIP_SHOT: &[u8] = include_bytes!("../assets/sounds/ship_blaster.wav");

        // PlaySound expects a pointer to the RIFF/WAVE bytes for SND_MEMORY.
        PlaySoundW(
            SHIP_SHOT.as_ptr().cast(),
            std::ptr::null_mut(),
            SND_ASYNC | SND_MEMORY | SND_NODEFAULT,
        );
    }
}
