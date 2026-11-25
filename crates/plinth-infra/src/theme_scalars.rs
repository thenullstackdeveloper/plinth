//! Parsers for the scalars a template writes in its `theme`. Each one accepts
//! a single strict spelling, so nothing but the value itself reaches the
//! domain.

use plinth_domain::{Hundredths, Srgb8};
use serde_json::Value;

/// `#rrggbb`, in either case. Shorthand (`#fff`), a missing `#` or anything
/// around the value is rejected.
pub fn parse_hex_color(raw: &str) -> Option<Srgb8> {
    let digits = raw.strip_prefix('#')?;
    if digits.len() != 6 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |i: usize| u8::from_str_radix(&digits[i..i + 2], 16).ok();
    Some(Srgb8 {
        r: channel(0)?,
        g: channel(2)?,
        b: channel(4)?,
    })
}

/// 1 to 3 digits and `px`, like `64px`.
pub fn parse_px(raw: &str) -> Option<u16> {
    digits_then(raw, "px", 3)
}

/// 1 to 4 digits and `ms`, like `200ms`.
pub fn parse_ms(raw: &str) -> Option<u16> {
    digits_then(raw, "ms", 4)
}

/// `1..=max_digits` ASCII digits followed by exactly `unit`, nothing else.
fn digits_then(raw: &str, unit: &str, max_digits: usize) -> Option<u16> {
    let digits = raw.strip_suffix(unit)?;
    let len_ok = (1..=max_digits).contains(&digits.len());
    if !len_ok || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// The weight names of DTCG 2025.10, spelled exactly.
const WEIGHT_ALIASES: [(&str, u16); 18] = [
    ("thin", 100),
    ("hairline", 100),
    ("extra-light", 200),
    ("ultra-light", 200),
    ("light", 300),
    ("normal", 400),
    ("regular", 400),
    ("book", 400),
    ("medium", 500),
    ("semi-bold", 600),
    ("demi-bold", 600),
    ("bold", 700),
    ("extra-bold", 800),
    ("ultra-bold", 800),
    ("black", 900),
    ("heavy", 900),
    ("extra-black", 950),
    ("ultra-black", 950),
];

/// A whole number from 1 to 1000, or the exact name of a DTCG weight alias.
pub fn parse_font_weight(raw: &Value) -> Option<u16> {
    match raw {
        Value::String(name) => WEIGHT_ALIASES
            .iter()
            .find(|(alias, _)| alias == name)
            .map(|(_, weight)| *weight),
        Value::Number(n) => n
            .as_u64()
            .filter(|w| (1..=1000).contains(w))
            .and_then(|w| u16::try_from(w).ok()),
        _ => None,
    }
}

/// A finite, non-negative number with at most two decimals whose hundredths
/// fit a `u16`. Text is rejected.
pub fn parse_hundredths(raw: &Value) -> Option<Hundredths> {
    let n = raw.as_f64().filter(|n| n.is_finite() && *n >= 0.0)?;
    let scaled = n * 100.0;
    let rounded = scaled.round();
    if (scaled - rounded).abs() > 1e-6 || rounded > f64::from(u16::MAX) {
        return None;
    }
    // Whole and within `u16`, so the cast is exact.
    Some(Hundredths(rounded as u16))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_color_accepts_six_digits() {
        assert_eq!(
            parse_hex_color("#1E1E2E"),
            Some(Srgb8 {
                r: 30,
                g: 30,
                b: 46
            })
        );
        for bad in ["#fff", "1e1e2e", "#1e1e2g", " #1e1e2e"] {
            assert_eq!(parse_hex_color(bad), None, "`{bad}` must be rejected");
        }
    }

    #[test]
    fn parse_px_is_strict() {
        assert_eq!(parse_px("0px"), Some(0));
        assert_eq!(parse_px("64px"), Some(64));
        assert_eq!(parse_px("240px"), Some(240));
        for bad in [
            "px", "64", "64PX", "4rem", "64.5px", "1000px", "-4px", " 64px", "64px ", "6 4px",
            "+64px", "64pxpx",
        ] {
            assert_eq!(parse_px(bad), None, "`{bad}` must be rejected");
        }
    }

    #[test]
    fn parse_ms_is_strict() {
        assert_eq!(parse_ms("0ms"), Some(0));
        assert_eq!(parse_ms("150ms"), Some(150));
        assert_eq!(parse_ms("9999ms"), Some(9999));
        for bad in [
            "ms", "150", "0.2s", "1s", "150MS", "10000ms", "-1ms", "15.5ms", " 150ms", "150px",
        ] {
            assert_eq!(parse_ms(bad), None, "`{bad}` must be rejected");
        }
    }

    #[test]
    fn parse_font_weight_accepts_aliases() {
        let aliases = [
            ("thin", 100),
            ("hairline", 100),
            ("extra-light", 200),
            ("ultra-light", 200),
            ("light", 300),
            ("normal", 400),
            ("regular", 400),
            ("book", 400),
            ("medium", 500),
            ("semi-bold", 600),
            ("demi-bold", 600),
            ("bold", 700),
            ("extra-bold", 800),
            ("ultra-bold", 800),
            ("black", 900),
            ("heavy", 900),
            ("extra-black", 950),
            ("ultra-black", 950),
        ];
        for (name, weight) in aliases {
            assert_eq!(
                parse_font_weight(&Value::from(name)),
                Some(weight),
                "`{name}`"
            );
        }
        assert_eq!(parse_font_weight(&Value::from(1)), Some(1));
        assert_eq!(parse_font_weight(&Value::from(700)), Some(700));
        assert_eq!(parse_font_weight(&Value::from(1000)), Some(1000));
        for bad in [
            Value::from("Bold"),
            Value::from("semibold"),
            Value::from(" bold"),
            Value::from("700"),
            Value::from(0),
            Value::from(1001),
            Value::from(-700),
            Value::from(700.5),
            Value::Null,
        ] {
            assert_eq!(parse_font_weight(&bad), None, "`{bad}` must be rejected");
        }
    }

    #[test]
    fn parse_hundredths_rejects_three_decimals() {
        assert_eq!(parse_hundredths(&Value::from(1.2)), Some(Hundredths(120)));
        assert_eq!(parse_hundredths(&Value::from(1.35)), Some(Hundredths(135)));
        assert_eq!(parse_hundredths(&Value::from(1)), Some(Hundredths(100)));
        assert_eq!(parse_hundredths(&Value::from(0)), Some(Hundredths(0)));
        assert_eq!(
            parse_hundredths(&Value::from(655.35)),
            Some(Hundredths(u16::MAX))
        );
        for bad in [
            Value::from(1.234),
            Value::from(1.205),
            Value::from(655.36),
            Value::from(1_000_000),
            Value::from(-1.2),
            Value::from("1.2"),
            Value::Null,
        ] {
            assert_eq!(parse_hundredths(&bad), None, "`{bad}` must be rejected");
        }
    }
}
