//! Key names a template may use in `keys` (`"m"`, `"shift+1"`), mapped to
//! evdev scancodes.
//!
//! This is the template's own vocabulary, separate from the Star Citizen
//! tokens of `keybinds_sc`. It only lists keys in the range `input_windows`
//! sends unchanged (1-88): arrows, the numpad and the right-hand modifiers are
//! not nameable yet, so a template cannot ask for a key that would fail later.

use plinth_application::TemplateError;
use plinth_domain::{KeyChord, ScanCode};

/// The modifiers a chord may hold around its key (the left-hand ones).
const MODIFIERS: [&str; 3] = ["shift", "ctrl", "alt"];

/// Maps a lowercase key name to its evdev scancode.
pub fn scan_code(name: &str) -> Option<ScanCode> {
    let code = match name {
        "esc" => 1,
        "1" => 2,
        "2" => 3,
        "3" => 4,
        "4" => 5,
        "5" => 6,
        "6" => 7,
        "7" => 8,
        "8" => 9,
        "9" => 10,
        "0" => 11,
        "-" => 12,
        "=" => 13,
        "backspace" => 14,
        "tab" => 15,
        "q" => 16,
        "w" => 17,
        "e" => 18,
        "r" => 19,
        "t" => 20,
        "y" => 21,
        "u" => 22,
        "i" => 23,
        "o" => 24,
        "p" => 25,
        "[" => 26,
        "]" => 27,
        "enter" => 28,
        "ctrl" => 29,
        "a" => 30,
        "s" => 31,
        "d" => 32,
        "f" => 33,
        "g" => 34,
        "h" => 35,
        "j" => 36,
        "k" => 37,
        "l" => 38,
        ";" => 39,
        "'" => 40,
        "`" => 41,
        "shift" => 42,
        "\\" => 43,
        "z" => 44,
        "x" => 45,
        "c" => 46,
        "v" => 47,
        "b" => 48,
        "n" => 49,
        "m" => 50,
        "," => 51,
        "." => 52,
        "/" => 53,
        "alt" => 56,
        "space" => 57,
        "f1" => 59,
        "f2" => 60,
        "f3" => 61,
        "f4" => 62,
        "f5" => 63,
        "f6" => 64,
        "f7" => 65,
        "f8" => 66,
        "f9" => 67,
        "f10" => 68,
        "f11" => 87,
        "f12" => 88,
        _ => return None,
    };
    Some(ScanCode::new(code))
}

/// Parses one step of `keys`: `mod+…+key`, case-insensitive, with spaces
/// around `+` ignored. Every token but the last must be a modifier.
pub fn parse_chord(text: &str) -> Result<KeyChord, TemplateError> {
    let lower = text.to_lowercase();
    let tokens: Vec<&str> = lower.split('+').map(str::trim).collect();
    let (key, modifiers) = tokens
        .split_last()
        .ok_or_else(|| TemplateError(format!("empty key step `{text}`")))?;

    let lookup = |name: &str| {
        scan_code(name)
            .ok_or_else(|| TemplateError(format!("unknown key name `{name}` in `{text}`")))
    };

    let mut codes = Vec::with_capacity(modifiers.len());
    for name in modifiers {
        if !MODIFIERS.contains(name) {
            return Err(TemplateError(format!(
                "`{name}` in `{text}` is not a modifier (shift, ctrl, alt)"
            )));
        }
        codes.push(lookup(name)?);
    }
    let key = lookup(key)?;
    KeyChord::new(codes, key).map_err(|e| TemplateError(format!("`{text}`: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code(name: &str) -> u16 {
        scan_code(name)
            .unwrap_or_else(|| panic!("`{name}` has no code"))
            .get()
    }

    #[test]
    fn scan_code_matches_the_table() {
        assert_eq!(code("a"), 30);
        assert_eq!(code("m"), 50);
        assert_eq!(code("0"), 11);
        assert_eq!(code("1"), 2);
        assert_eq!(code("f1"), 59);
        assert_eq!(code("f10"), 68);
        assert_eq!(code("f11"), 87);
        assert_eq!(code("f12"), 88);
        assert_eq!(code("shift"), 42);
        assert_eq!(code("ctrl"), 29);
        assert_eq!(code("alt"), 56);
        assert_eq!(code("space"), 57);
        assert_eq!(code("esc"), 1);
        assert_eq!(code("`"), 41);
        assert_eq!(code("\\"), 43);
        assert_eq!(scan_code("up"), None);
    }

    #[test]
    fn every_named_key_is_in_the_windows_pass_through_range() {
        let names = [
            "esc",
            "1",
            "0",
            "-",
            "=",
            "backspace",
            "tab",
            "q",
            "p",
            "[",
            "]",
            "enter",
            "ctrl",
            "a",
            "l",
            ";",
            "'",
            "`",
            "shift",
            "\\",
            "z",
            "m",
            ",",
            ".",
            "/",
            "alt",
            "space",
            "f1",
            "f12",
        ];
        for name in names {
            let c = code(name);
            assert!((1..=88).contains(&c), "`{name}` = {c}");
        }
    }

    #[test]
    fn parse_chord_is_case_and_space_insensitive() {
        let chord = parse_chord("Shift + 1").unwrap();
        assert_eq!(chord, parse_chord("shift+1").unwrap());
        assert_eq!(chord.modifiers(), &[ScanCode::new(42)]);
        assert_eq!(chord.key(), ScanCode::new(2));
    }

    #[test]
    fn parse_chord_keeps_the_modifier_order() {
        let chord = parse_chord("ctrl+shift+1").unwrap();
        assert_eq!(chord.modifiers(), &[ScanCode::new(29), ScanCode::new(42)]);
    }

    #[test]
    fn parse_chord_rejects_unknown_name() {
        let err = parse_chord("up").unwrap_err();
        assert!(err.0.contains("`up`"), "{}", err.0);
    }

    #[test]
    fn parse_chord_rejects_non_modifier_prefix() {
        let err = parse_chord("a+b").unwrap_err();
        assert!(err.0.contains("`a`"), "{}", err.0);
    }

    #[test]
    fn parse_chord_rejects_repeated_modifier() {
        assert!(parse_chord("shift+shift+1").is_err());
    }
}
