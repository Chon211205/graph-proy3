//! Preserve short key taps even when press and release arrive in the same frame.
use minifb::{InputCallback, Key, Window};
use std::cell::RefCell;
#[cfg(windows)]
use std::{ffi::c_void, sync::OnceLock};

struct Keys {
    down: [bool; 512],
    pressed: [bool; 512],
}

impl Default for Keys {
    fn default() -> Self {
        Self {
            down: [false; 512],
            pressed: [false; 512],
        }
    }
}

impl Keys {
    fn event(&mut self, key: Key, down: bool) {
        let index = key as usize;
        if down && !self.down[index] {
            self.pressed[index] = true;
        }
        self.down[index] = down;
    }
    fn take(&mut self, key: Key) -> bool {
        std::mem::replace(&mut self.pressed[key as usize], false)
    }
}

thread_local! { static KEYS: RefCell<Keys> = RefCell::new(Keys::default()); }
struct Callback;
impl InputCallback for Callback {
    fn add_char(&mut self, _: u32) {}
    fn set_key_state(&mut self, key: Key, down: bool) {
        KEYS.with_borrow_mut(|keys| keys.event(key, down));
    }
}

pub fn attach(window: &mut Window) {
    KEYS.with_borrow_mut(|keys| *keys = Keys::default());
    window.set_input_callback(Box::new(Callback));
}

pub fn pressed(key: Key) -> bool {
    KEYS.with_borrow_mut(|keys| keys.take(key))
}
pub fn clear() {
    KEYS.with_borrow_mut(|keys| keys.pressed.fill(false));
}

/// Frame-stable Xbox/XInput state. Keyboard input remains available and is
/// intentionally combined with this state by the game loop.
#[derive(Clone, Copy)]
pub struct Gamepad {
    index: u32,
    pub connected: bool,
    pub move_x: f32,
    pub move_y: f32,
    pub motion_x: f32,
    pub motion_y: f32,
    pub accelerate: bool,
    pub brake: bool,
    pub fire: bool,
    pub boost: bool,
    pub roll_left: bool,
    pub roll_right: bool,
    buttons: u16,
    pressed: u16,
}

impl Default for Gamepad {
    fn default() -> Self {
        Self::new(0)
    }
}

impl Gamepad {
    pub const DPAD_UP: u16 = 0x0001;
    pub const DPAD_DOWN: u16 = 0x0002;
    pub const DPAD_LEFT: u16 = 0x0004;
    pub const DPAD_RIGHT: u16 = 0x0008;
    pub const A: u16 = 0x1000;
    pub const B: u16 = 0x2000;
    pub const X: u16 = 0x4000;
    pub const Y: u16 = 0x8000;
    pub const START: u16 = 0x0010;
    pub const SELECT: u16 = 0x0020;

    pub fn new(index: u32) -> Self {
        Self {
            index,
            connected: false,
            move_x: 0.0,
            move_y: 0.0,
            motion_x: 0.0,
            motion_y: 0.0,
            accelerate: false,
            brake: false,
            fire: false,
            boost: false,
            roll_left: false,
            roll_right: false,
            buttons: 0,
            pressed: 0,
        }
    }

    pub fn update(&mut self) {
        let Some(raw) = platform_gamepad_state(self.index) else {
            let index = self.index;
            *self = Self::new(index);
            return;
        };
        self.connected = true;
        self.pressed = raw.buttons & !self.buttons;
        self.buttons = raw.buttons;
        self.move_x = normalize_stick(raw.left_x);
        self.move_y = normalize_stick(raw.left_y);
        self.motion_x = normalize_stick(raw.motion_x);
        self.motion_y = normalize_stick(raw.motion_y);
        self.accelerate = raw.right_trigger > 32 || self.down(Self::A);
        self.brake = raw.left_trigger > 32 || self.down(Self::B);
        self.fire = self.down(Self::X);
        self.boost = self.down(0x0040) || self.down(0x0080);
        self.roll_left = self.down(0x0100);
        self.roll_right = self.down(0x0200);
    }

    pub fn down(&self, button: u16) -> bool {
        self.buttons & button != 0
    }

    pub fn pressed(&self, button: u16) -> bool {
        self.pressed & button != 0
    }

    pub fn horizontal(&self) -> i8 {
        // Steering always comes from the left stick. The right stick is kept
        // available for camera controls and must never override movement due
        // to small neutral noise.
        digital_axis(self.move_x)
    }

    pub fn vertical(&self) -> i8 {
        digital_axis(self.move_y)
    }

    pub fn motion_active(&self) -> bool {
        self.motion_x.abs() > 0.22 || self.motion_y.abs() > 0.22
    }
}

fn digital_axis(value: f32) -> i8 {
    if value > 0.35 {
        1
    } else if value < -0.35 {
        -1
    } else {
        0
    }
}

fn normalize_stick(value: i16) -> f32 {
    const DEAD_ZONE: f32 = 7_849.0;
    let value = value as f32;
    let magnitude = value.abs();
    if magnitude <= DEAD_ZONE {
        0.0
    } else {
        value.signum() * ((magnitude - DEAD_ZONE) / (32_767.0 - DEAD_ZONE)).clamp(0.0, 1.0)
    }
}

#[derive(Clone, Copy)]
struct RawGamepad {
    buttons: u16,
    left_x: i16,
    left_y: i16,
    motion_x: i16,
    motion_y: i16,
    left_trigger: u8,
    right_trigger: u8,
}

fn merge_raw_gamepad(target: &mut Option<RawGamepad>, source: RawGamepad) {
    if let Some(current) = target {
        current.buttons |= source.buttons;
        if source.left_x.unsigned_abs() > current.left_x.unsigned_abs() {
            current.left_x = source.left_x;
        }
        if source.left_y.unsigned_abs() > current.left_y.unsigned_abs() {
            current.left_y = source.left_y;
        }
        if source.motion_x.unsigned_abs() > current.motion_x.unsigned_abs() {
            current.motion_x = source.motion_x;
        }
        if source.motion_y.unsigned_abs() > current.motion_y.unsigned_abs() {
            current.motion_y = source.motion_y;
        }
        current.left_trigger = current.left_trigger.max(source.left_trigger);
        current.right_trigger = current.right_trigger.max(source.right_trigger);
    } else {
        *target = Some(source);
    }
}

#[cfg(windows)]
#[repr(C)]
#[derive(Default)]
struct XInputGamepad {
    buttons: u16,
    left_trigger: u8,
    right_trigger: u8,
    thumb_lx: i16,
    thumb_ly: i16,
    thumb_rx: i16,
    thumb_ry: i16,
}

#[cfg(windows)]
#[repr(C)]
#[derive(Default)]
struct XInputState {
    packet_number: u32,
    gamepad: XInputGamepad,
}

#[cfg(windows)]
type XInputGetState = unsafe extern "system" fn(u32, *mut XInputState) -> u32;

#[cfg(windows)]
fn load_xinput() -> Option<XInputGetState> {
    unsafe extern "system" {
        fn LoadLibraryA(name: *const u8) -> *mut c_void;
        fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
    }
    for dll in [
        b"xinput1_4.dll\0".as_slice(),
        b"xinput1_3.dll\0".as_slice(),
        b"xinput9_1_0.dll\0".as_slice(),
    ] {
        let module = unsafe { LoadLibraryA(dll.as_ptr()) };
        if module.is_null() {
            continue;
        }
        let address = unsafe { GetProcAddress(module, c"XInputGetState".as_ptr().cast()) };
        if !address.is_null() {
            // Windows guarantees this exported symbol has the XInput signature.
            return Some(unsafe { std::mem::transmute::<*mut c_void, XInputGetState>(address) });
        }
    }
    None
}

#[cfg(windows)]
fn platform_gamepad_state(index: u32) -> Option<RawGamepad> {
    static GET_STATE: OnceLock<Option<XInputGetState>> = OnceLock::new();
    let mut combined = None;
    if let Some(get_state) = *GET_STATE.get_or_init(load_xinput) {
        let mut connected = 0;
        for physical_slot in 0..4 {
            let mut state = XInputState::default();
            if unsafe { get_state(physical_slot, &mut state) } != 0 {
                continue;
            }
            if connected == index {
                merge_raw_gamepad(
                    &mut combined,
                    RawGamepad {
                        buttons: state.gamepad.buttons,
                        left_x: state.gamepad.thumb_lx,
                        left_y: state.gamepad.thumb_ly,
                        motion_x: state.gamepad.thumb_rx,
                        motion_y: state.gamepad.thumb_ry,
                        left_trigger: state.gamepad.left_trigger,
                        right_trigger: state.gamepad.right_trigger,
                    },
                );
                break;
            }
            connected += 1;
        }
    }
    if let Some(generic) = winmm_gamepad_state(index) {
        // The same physical device may appear in both APIs. Merging matching
        // ordinal slots keeps it on one player while allowing an active HID
        // controller to override an idle virtual XInput device.
        merge_raw_gamepad(&mut combined, generic);
    }
    combined
}

#[cfg(windows)]
#[repr(C)]
struct JoyInfoEx {
    size: u32,
    flags: u32,
    x: u32,
    y: u32,
    z: u32,
    r: u32,
    u: u32,
    v: u32,
    buttons: u32,
    button_number: u32,
    pov: u32,
    reserved1: u32,
    reserved2: u32,
}

#[cfg(windows)]
type JoyGetPosEx = unsafe extern "system" fn(u32, *mut JoyInfoEx) -> u32;

#[cfg(windows)]
#[repr(C)]
struct JoyCapsW {
    manufacturer_id: u16,
    product_id: u16,
    product_name: [u16; 32],
    axis_ranges_and_counts: [u32; 19],
    registry_key: [u16; 32],
    oem_driver: [u16; 260],
}

#[cfg(windows)]
type JoyGetDevCapsW = unsafe extern "system" fn(usize, *mut JoyCapsW, u32) -> u32;

#[cfg(windows)]
fn load_winmm_joystick() -> Option<JoyGetPosEx> {
    unsafe extern "system" {
        fn LoadLibraryA(name: *const u8) -> *mut c_void;
        fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
    }
    let module = unsafe { LoadLibraryA(c"winmm.dll".as_ptr().cast()) };
    if module.is_null() {
        return None;
    }
    let address = unsafe { GetProcAddress(module, c"joyGetPosEx".as_ptr().cast()) };
    (!address.is_null())
        .then(|| unsafe { std::mem::transmute::<*mut c_void, JoyGetPosEx>(address) })
}

#[cfg(windows)]
fn load_winmm_caps() -> Option<JoyGetDevCapsW> {
    unsafe extern "system" {
        fn LoadLibraryA(name: *const u8) -> *mut c_void;
        fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
    }
    let module = unsafe { LoadLibraryA(c"winmm.dll".as_ptr().cast()) };
    if module.is_null() {
        return None;
    }
    let address = unsafe { GetProcAddress(module, c"joyGetDevCapsW".as_ptr().cast()) };
    (!address.is_null())
        .then(|| unsafe { std::mem::transmute::<*mut c_void, JoyGetDevCapsW>(address) })
}

#[cfg(windows)]
fn is_dualsense(slot: u32) -> bool {
    static GET_CAPS: OnceLock<Option<JoyGetDevCapsW>> = OnceLock::new();
    let Some(get_caps) = *GET_CAPS.get_or_init(load_winmm_caps) else {
        return false;
    };
    let mut caps: JoyCapsW = unsafe { std::mem::zeroed() };
    if unsafe {
        get_caps(
            slot as usize,
            &mut caps,
            std::mem::size_of::<JoyCapsW>() as u32,
        )
    } != 0
    {
        return false;
    }
    let end = caps
        .product_name
        .iter()
        .position(|&character| character == 0)
        .unwrap_or(caps.product_name.len());
    let name = String::from_utf16_lossy(&caps.product_name[..end]).to_ascii_lowercase();
    name.contains("dualsense") || name.contains("wireless controller")
}

fn button_map(dualsense: bool) -> [u16; 12] {
    if dualsense {
        // Physical PlayStation order translated to the same semantic layout
        // used by Xbox: Cross=A, Circle=B, Square=X, Triangle=Y.
        [
            Gamepad::X,
            Gamepad::A,
            Gamepad::B,
            Gamepad::Y,
            0x0100,
            0x0200,
            Gamepad::B,
            Gamepad::A,
            0x0020,
            Gamepad::START,
            0x0040,
            0x0080,
        ]
    } else {
        // Common DirectInput order also places L2/R2 at buttons 7/8 and
        // Select/Start at 9/10. Xbox pads normally use the XInput path, so
        // this fallback follows the native PlayStation face-button order.
        [
            Gamepad::X,
            Gamepad::A,
            Gamepad::B,
            Gamepad::Y,
            0x0100,
            0x0200,
            Gamepad::B,
            Gamepad::A,
            0x0020,
            Gamepad::START,
            0x0040,
            0x0080,
        ]
    }
}

#[cfg(windows)]
fn winmm_gamepad_state(index: u32) -> Option<RawGamepad> {
    static GET_POS: OnceLock<Option<JoyGetPosEx>> = OnceLock::new();
    let get_pos = (*GET_POS.get_or_init(load_winmm_joystick))?;
    let mut connected = 0;
    for physical_slot in 0..16 {
        let mut state = JoyInfoEx {
            size: std::mem::size_of::<JoyInfoEx>() as u32,
            flags: 0x00ff,
            x: 32_767,
            y: 32_767,
            z: 32_767,
            r: 32_767,
            u: 32_767,
            v: 32_767,
            buttons: 0,
            button_number: 0,
            pov: 0xffff,
            reserved1: 0,
            reserved2: 0,
        };
        if unsafe { get_pos(physical_slot, &mut state) } != 0 {
            continue;
        }
        if connected != index {
            connected += 1;
            continue;
        }
        let mut buttons = 0_u16;
        // Generic DirectInput order: primary buttons 1-4, shoulders 5-6,
        // menu buttons 7-8 and stick buttons 9-12.
        let dualsense = is_dualsense(physical_slot);
        let map = button_map(dualsense);
        for (index, mapped) in map.into_iter().enumerate() {
            if state.buttons & (1 << index) != 0 {
                buttons |= mapped;
            }
        }
        if state.pov != 0xffff {
            let angle = state.pov % 36_000;
            if angle >= 31_500 || angle <= 4_500 {
                buttons |= Gamepad::DPAD_UP;
            }
            if (4_500..=13_500).contains(&angle) {
                buttons |= Gamepad::DPAD_RIGHT;
            }
            if (13_500..=22_500).contains(&angle) {
                buttons |= Gamepad::DPAD_DOWN;
            }
            if (22_500..=31_500).contains(&angle) {
                buttons |= Gamepad::DPAD_LEFT;
            }
        }
        let (motion_x, motion_y) = if dualsense {
            // Native DualSense exposes right-stick X/Y on Z/R and the two
            // analog triggers on U/V. Y axes are inverted by WinMM.
            (
                centered_axis(state.z),
                centered_axis(65_535_u32.saturating_sub(state.r)),
            )
        } else {
            (
                centered_axis(state.r),
                centered_axis(65_535_u32.saturating_sub(state.u)),
            )
        };
        return Some(RawGamepad {
            buttons,
            left_x: centered_axis(state.x),
            left_y: centered_axis(65_535_u32.saturating_sub(state.y)),
            motion_x,
            motion_y,
            // Native WinMM reports DualSense trigger axes differently across
            // USB and Bluetooth (often half-pressed at rest). L2/R2 are also
            // exposed as reliable digital buttons, so use those instead.
            left_trigger: 0,
            right_trigger: 0,
        });
    }
    None
}

#[cfg(windows)]
fn centered_axis(value: u32) -> i16 {
    (value.min(65_535) as i32 - 32_767).clamp(i16::MIN as i32, i16::MAX as i32) as i16
}

#[cfg(not(windows))]
fn platform_gamepad_state(_index: u32) -> Option<RawGamepad> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn short_tap_survives_release_and_is_consumed_once() {
        let mut keys = Keys::default();
        keys.event(Key::Enter, true);
        keys.event(Key::Enter, false);
        assert!(keys.take(Key::Enter));
        assert!(!keys.take(Key::Enter));
    }
    #[test]
    fn held_key_does_not_reactivate_after_changing_screens() {
        let mut keys = Keys::default();
        keys.event(Key::Escape, true);
        assert!(keys.take(Key::Escape));
        keys.event(Key::Escape, true);
        assert!(!keys.take(Key::Escape));
        keys.event(Key::Escape, false);
        keys.event(Key::Escape, true);
        assert!(keys.take(Key::Escape));
    }

    #[test]
    fn stick_dead_zone_prevents_unwanted_drift() {
        assert_eq!(normalize_stick(7_000), 0.0);
        assert_eq!(digital_axis(normalize_stick(20_000)), 1);
        assert_eq!(digital_axis(normalize_stick(-20_000)), -1);
    }

    #[test]
    fn active_hid_input_overrides_an_idle_virtual_xinput_slot() {
        let mut combined = Some(RawGamepad {
            buttons: 0,
            left_x: 0,
            left_y: 0,
            motion_x: 0,
            motion_y: 0,
            left_trigger: 0,
            right_trigger: 0,
        });
        merge_raw_gamepad(
            &mut combined,
            RawGamepad {
                buttons: Gamepad::X,
                left_x: 22_000,
                left_y: -18_000,
                motion_x: 0,
                motion_y: 0,
                left_trigger: 0,
                right_trigger: 200,
            },
        );
        let combined = combined.unwrap();
        assert_eq!(combined.buttons, Gamepad::X);
        assert_eq!(combined.left_x, 22_000);
        assert_eq!(combined.left_y, -18_000);
        assert_eq!(combined.right_trigger, 200);
    }

    #[test]
    fn local_players_keep_independent_controller_slots() {
        let first = Gamepad::new(0);
        let second = Gamepad::new(1);
        assert_eq!(first.index, 0);
        assert_eq!(second.index, 1);
        assert!(!first.connected && !second.connected);
    }

    #[test]
    fn dualsense_face_buttons_match_xbox_actions() {
        let playstation = button_map(true);
        assert_eq!(playstation[0], Gamepad::X); // Square = Xbox X / fire
        assert_eq!(playstation[1], Gamepad::A); // Cross = Xbox A / accept
        assert_eq!(playstation[2], Gamepad::B); // Circle = Xbox B / back
        assert_eq!(playstation[3], Gamepad::Y); // Triangle = Xbox Y / action
        assert_eq!(playstation[9], Gamepad::START); // Options = Menu
        assert_eq!(playstation[6], Gamepad::B); // L2 = brake
        assert_eq!(playstation[7], Gamepad::A); // R2 = accelerate
    }

    #[test]
    fn directinput_triggers_can_never_open_pause() {
        for map in [button_map(false), button_map(true)] {
            assert_ne!(map[6], Gamepad::START); // L2
            assert_ne!(map[7], Gamepad::START); // R2
            assert_eq!(map[9], Gamepad::START); // Options/Menu
        }
    }

    #[test]
    fn generic_directinput_square_fires_like_dualsense_square() {
        assert_eq!(button_map(false)[0], Gamepad::X);
        assert_eq!(button_map(true)[0], Gamepad::X);
    }

    #[test]
    fn right_stick_noise_never_moves_the_ship() {
        let mut pad = Gamepad::new(0);
        pad.move_x = 0.0;
        pad.move_y = 0.0;
        pad.motion_x = 0.85;
        pad.motion_y = -0.85;
        assert_eq!(pad.horizontal(), 0);
        assert_eq!(pad.vertical(), 0);
    }
}
