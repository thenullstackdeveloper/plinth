//! Parsers for the scalars a template writes in its `theme`. Each one accepts
//! a single strict spelling, so nothing but the value itself reaches the
//! domain.

use plinth_domain::Srgb8;

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
}
