//! `InputSender` over the Win32 `SendInput` API.
//!
//! The Windows counterpart of `input_uinput`. `SendInput` injects events into
//! the same queue the OS uses for physical keyboards, so the focused window
//! cannot tell them apart. It is the mechanism used by Stream Deck, AutoHotkey
//! and similar tools, and it never touches the target process.
//!
//! **Scancodes:** events are sent with `KEYEVENTF_SCANCODE`, i.e. by physical
//! key position rather than by virtual key. This matches the domain's
//! `ScanCode` model and makes the result independent of the user's keyboard
//! layout, which is what games bind against.
//!
//! **Known limitation:** the domain carries evdev scancodes. For the main block
//! (letters, digits, F1-F12, left-hand modifiers, space, enter, escape) evdev
//! codes and Windows Set-1 scancodes are numerically identical, so they are
//! passed through unchanged. The extended cluster (arrows, navigation keys,
//! right-hand modifiers, numpad enter) uses E0-prefixed scancodes on Windows
//! with different numbering, and is **not** translated yet - emitting one of
//! those returns an error instead of silently sending the wrong key.

use plinth_application::{InputError, InputSender, KeyEvent, KeyValue};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE,
    VIRTUAL_KEY,
};

/// Sends key events through the Win32 `SendInput` API.
#[derive(Default)]
pub struct WindowsInputSender;

impl WindowsInputSender {
    pub fn new() -> Self {
        Self
    }
}

/// evdev codes above this belong to the extended cluster, whose Windows
/// scancodes are E0-prefixed and numbered differently.
const MAX_PASSTHROUGH_CODE: u16 = 88;

fn to_input(event: &KeyEvent) -> Result<INPUT, InputError> {
    let code = event.code.get();
    if code == 0 || code > MAX_PASSTHROUGH_CODE {
        return Err(InputError(format!(
            "scancode {code} is outside the pass-through range (1-{MAX_PASSTHROUGH_CODE}); \
             extended keys are not translated yet"
        )));
    }

    let mut flags = KEYEVENTF_SCANCODE;
    if matches!(event.value, KeyValue::Up) {
        flags |= KEYEVENTF_KEYUP;
    }

    Ok(INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: code,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    })
}

impl InputSender for WindowsInputSender {
    fn emit(&self, events: &[KeyEvent]) -> Result<(), InputError> {
        let inputs: Vec<INPUT> = events
            .iter()
            .map(to_input)
            .collect::<Result<Vec<_>, InputError>>()?;

        let size = std::mem::size_of::<INPUT>() as i32;
        // SAFETY: `inputs` is a valid, correctly sized slice of INPUT structs
        // and `size` is the size the API expects.
        let sent = unsafe { SendInput(&inputs, size) };

        if sent as usize != inputs.len() {
            return Err(InputError(format!(
                "SendInput delivered {sent} of {} events",
                inputs.len()
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plinth_domain::ScanCode;

    #[test]
    fn maps_a_press_to_a_scancode_event() {
        let input = to_input(&KeyEvent::down(ScanCode::new(30))).unwrap();
        // SAFETY: the union is initialised as a keyboard input by `to_input`.
        let ki = unsafe { input.Anonymous.ki };
        assert_eq!(ki.wScan, 30);
        assert_eq!(ki.dwFlags, KEYEVENTF_SCANCODE);
    }

    #[test]
    fn marks_key_up_events() {
        let input = to_input(&KeyEvent::up(ScanCode::new(30))).unwrap();
        // SAFETY: as above.
        let ki = unsafe { input.Anonymous.ki };
        assert_eq!(ki.dwFlags, KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP);
    }

    #[test]
    fn rejects_untranslated_extended_keys() {
        // evdev KEY_UP (103) is an extended key on Windows.
        assert!(to_input(&KeyEvent::down(ScanCode::new(103))).is_err());
    }
}
