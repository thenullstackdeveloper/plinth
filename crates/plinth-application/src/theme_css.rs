//! The theme as CSS: custom property names and values, written from the
//! domain's typed values. Nothing a template wrote reaches the UI as text;
//! only what is formatted here.

use plinth_domain::{Theme, TokenPath, TokenValue};

use crate::dto::ThemeVar;

/// `color.button.background` → `--plinth-color-button-background`.
pub fn css_variable(path: &TokenPath) -> String {
    format!("--plinth-{}", path.as_str().replace('.', "-"))
}

/// The one CSS spelling of a value: a colour is `#rrggbb` in lowercase.
pub fn css_value(value: &TokenValue) -> String {
    match value {
        TokenValue::Color(c) => format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b),
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
    use plinth_domain::Srgb8;

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
}
