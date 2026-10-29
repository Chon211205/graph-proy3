//! Preserve short key taps even when press and release arrive in the same frame.
use minifb::{InputCallback, Key, Window};
use std::cell::RefCell;

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
}
