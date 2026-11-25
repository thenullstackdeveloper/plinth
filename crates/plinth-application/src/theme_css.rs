//! The theme as CSS: custom property names and values, written from the
//! domain's typed values. Nothing a template wrote reaches the UI as text;
//! only what is formatted here.

use plinth_domain::{Theme, TokenPath, TokenValue};

use crate::dto::ThemeVar;

/// `color.button.background` → `--plinth-color-button-background`.
pub fn css_variable(path: &TokenPath) -> String {
    format!("--plinth-{}", path.as_str().replace('.', "-"))
}

/// The one CSS spelling of a value, the only text that reaches the UI:
/// a colour is `#rrggbb` in lowercase, a dimension `{n}px`, a duration
/// `{n}ms`, a weight `{n}`, a number without trailing zeros, and a font
/// family its names joined with `, `, unquoted.
pub fn css_value(value: &TokenValue) -> String {
    match value {
        TokenValue::Color(c) => format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b),
        TokenValue::Px(n) => format!("{n}px"),
        TokenValue::Ms(n) => format!("{n}ms"),
        TokenValue::FontWeight(n) => n.to_string(),
        TokenValue::Number(h) => h.to_string(),
        TokenValue::FontFamily(names) => names.join(", "),
    }
}

/// Every token of the theme as a CSS custom property, in token order.
pub fn theme_vars(theme: &Theme) -> Vec<ThemeVar> {
    theme
        .iter()
        .map(|(path, value)| ThemeVar {
            name: css_variable(path),
            value: css_value(value),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use plinth_domain::{Hundredths, Srgb8};

    #[test]
    fn css_variable_maps_dots_to_dashes() {
        let path = TokenPath::new("color.button.pressed-background").unwrap();
        assert_eq!(
            css_variable(&path),
            "--plinth-color-button-pressed-background"
        );
    }

    #[test]
    fn css_value_formats_colour_lowercase() {
        let value = TokenValue::Color(Srgb8 {
            r: 0xAB,
            g: 0x0C,
            b: 0xEF,
        });
        assert_eq!(css_value(&value), "#ab0cef");
    }

    #[test]
    fn css_value_formats_px() {
        assert_eq!(css_value(&TokenValue::Px(0)), "0px");
        assert_eq!(css_value(&TokenValue::Px(64)), "64px");
    }

    #[test]
    fn css_value_formats_ms() {
        assert_eq!(css_value(&TokenValue::Ms(0)), "0ms");
        assert_eq!(css_value(&TokenValue::Ms(200)), "200ms");
    }

    #[test]
    fn css_value_formats_font_weight() {
        assert_eq!(css_value(&TokenValue::FontWeight(600)), "600");
        assert_eq!(css_value(&TokenValue::FontWeight(1000)), "1000");
    }

    #[test]
    fn css_value_formats_hundredths() {
        let number = |h| css_value(&TokenValue::Number(Hundredths(h)));
        assert_eq!(number(120), "1.2");
        assert_eq!(number(100), "1");
        assert_eq!(number(135), "1.35");
        assert_eq!(number(105), "1.05");
    }

    #[test]
    fn css_value_joins_font_family() {
        let one = TokenValue::FontFamily(vec!["Inter".to_string()]);
        assert_eq!(css_value(&one), "Inter");
        let many = TokenValue::FontFamily(vec![
            "Segoe UI".to_string(),
            "Helvetica Neue".to_string(),
            "sans-serif".to_string(),
        ]);
        assert_eq!(css_value(&many), "Segoe UI, Helvetica Neue, sans-serif");
    }

    #[test]
    fn css_value_never_emits_css_syntax() {
        let values = [
            TokenValue::Color(Srgb8 { r: 0, g: 0, b: 0 }),
            TokenValue::Color(Srgb8 {
                r: 255,
                g: 255,
                b: 255,
            }),
            TokenValue::Px(0),
            TokenValue::Px(u16::MAX),
            TokenValue::Ms(0),
            TokenValue::Ms(u16::MAX),
            TokenValue::FontWeight(1),
            TokenValue::FontWeight(1000),
            TokenValue::Number(Hundredths(0)),
            TokenValue::Number(Hundredths(105)),
            TokenValue::Number(Hundredths(u16::MAX)),
            TokenValue::FontFamily(vec!["Inter".to_string()]),
            TokenValue::FontFamily(vec!["Segoe UI".to_string(), "sans-serif".to_string()]),
        ];
        let forbidden = ['(', ')', ';', '{', '}', '\'', '"', '!', '\\'];
        for value in &values {
            let css = css_value(value);
            assert!(!css.is_empty(), "{value:?} gave an empty value");
            assert!(
                !css.contains(&forbidden[..]),
                "{value:?} gave {css:?}, which holds CSS syntax"
            );
        }
    }
}
