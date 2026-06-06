//! `InputSender` over **uinput** (the `evdev` crate). Creates a kernel-level
//! virtual keyboard, *below* the compositor, so it works the same on X11/Wayland
//! and is transparent to Hyprland, Wine/Proton, and EAC (see
//! `docs/input-linux.md`).
//!
//! `VirtualDevice::emit` requires `&mut self` and the `InputSender::emit` trait
//! is `&self`, so the device lives behind a `Mutex` (which also allows sharing
//! the sender across threads, e.g. the Tauri commands).

use std::sync::Mutex;

use evdev::{uinput::VirtualDevice, AttributeSet, InputEvent, KeyCode, KeyEvent as EvKeyEvent};
use plinth_application::{InputError, InputSender, KeyEvent, KeyValue};

pub struct UinputSender {
    device: Mutex<VirtualDevice>,
}

impl UinputSender {
    /// Opens `/dev/uinput` and declares a virtual keyboard able to emit the whole
    /// standard key range. Requires access to `/dev/uinput` (the `input` group or
    /// the udev rule in `packaging/99-uinput.rules`).
    pub fn open() -> Result<Self, InputError> {
        let mut keys = AttributeSet::<KeyCode>::new();
        // Declare the standard keyboard key range (1..=248 covers letters,
        // digits, function, navigation, and numpad).
        for code in 1u16..=248 {
            keys.insert(KeyCode::new(code));
        }

        let device = VirtualDevice::builder()
            .map_err(|e| InputError(format!("could not open /dev/uinput: {e}")))?
            .name("Plinth Virtual Keyboard")
            .with_keys(&keys)
            .map_err(|e| InputError(format!("could not declare the keys: {e}")))?
            .build()
            .map_err(|e| InputError(format!("could not create the virtual device: {e}")))?;

        Ok(Self {
            device: Mutex::new(device),
        })
    }
}

fn to_evdev_event(event: &KeyEvent) -> InputEvent {
    let value = match event.value {
        KeyValue::Down => 1,
        KeyValue::Up => 0,
    };
    EvKeyEvent::new(KeyCode::new(event.code.get()), value).into()
}

impl InputSender for UinputSender {
    fn emit(&self, events: &[KeyEvent]) -> Result<(), InputError> {
        let raw: Vec<InputEvent> = events.iter().map(to_evdev_event).collect();
        let mut device = self
            .device
            .lock()
            .map_err(|_| InputError("device mutex poisoned".into()))?;
        device
            .emit(&raw)
            .map_err(|e| InputError(format!("failed emitting events: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plinth_domain::ScanCode;

    /// Real integration test: creates a `VirtualDevice` and emits `KEY_A`.
    /// Requires `/dev/uinput` to be accessible → `#[ignore]` so it doesn't run in
    /// CI. Run on the dev machine with: `cargo test -p plinth-infra -- --ignored`
    /// and verify with `wev` / `evtest` / `libinput debug-events`.
    #[test]
    #[ignore = "requires /dev/uinput (dev machine, not CI)"]
    fn emits_key_a_to_real_uinput() {
        let sender = UinputSender::open().expect("open uinput");
        let key_a = ScanCode::new(30);
        sender
            .emit(&[KeyEvent::down(key_a), KeyEvent::up(key_a)])
            .expect("emit KEY_A");
    }
}
