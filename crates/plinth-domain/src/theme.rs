//! Design tokens as the domain sees them: a closed catalog of named values,
//! each with its kind and whether a template may change it, and the resolved
//! theme a template carries.
//!
//! Only integers live here (no floats), so `Template` stays `Eq`. Reading the
//! tokens file and formatting CSS belong to other layers.

use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::fmt;

use crate::error::DomainError;

/// The dotted name of a token, like `color.button.background`: 2 to 4
/// segments, each `[a-z][a-z0-9-]*`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TokenPath(String);

impl TokenPath {
    pub const MIN_SEGMENTS: usize = 2;
    pub const MAX_SEGMENTS: usize = 4;

    pub fn new(raw: &str) -> Result<Self, DomainError> {
        let segments: Vec<&str> = raw.split('.').collect();
        let count_ok = (Self::MIN_SEGMENTS..=Self::MAX_SEGMENTS).contains(&segments.len());
        if count_ok && segments.iter().all(|s| is_segment(s)) {
            Ok(Self(raw.to_string()))
        } else {
            Err(DomainError::InvalidTokenPath(raw.to_string()))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for TokenPath {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TokenPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// `[A-Za-z0-9][A-Za-z0-9 -]*`: a font name that cannot carry CSS syntax.
fn is_font_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '-')
}

/// `[a-z][a-z0-9-]*`: the rule for one segment of a token path.
fn is_segment(segment: &str) -> bool {
    let mut chars = segment.chars();
    matches!(chars.next(), Some('a'..='z'))
        && chars.all(|c| matches!(c, 'a'..='z' | '0'..='9' | '-'))
}

/// An sRGB colour with 8 bits per channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Srgb8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// A number with two decimals, stored as hundredths: `120` is `1.2`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Hundredths(pub u16);

/// Without trailing zeros: `1.2`, `1`, `1.35`, `1.05`.
impl fmt::Display for Hundredths {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (whole, frac) = (self.0 / 100, self.0 % 100);
        match frac {
            0 => write!(f, "{whole}"),
            _ if frac % 10 == 0 => write!(f, "{whole}.{}", frac / 10),
            _ => write!(f, "{whole}.{frac:02}"),
        }
    }
}

/// The kind of value a token holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Color,
    Dimension,
    Duration,
    FontWeight,
    Number,
    FontFamily,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            TokenKind::Color => r##"a colour like "#1e1e2e""##,
            TokenKind::Dimension => r#"a dimension in whole pixels, like "64px""#,
            TokenKind::Duration => r#"a duration in whole milliseconds, like "200ms""#,
            TokenKind::FontWeight => r#"a font weight from 1 to 1000 or a name like "bold""#,
            TokenKind::Number => "a number with at most two decimals, like 1.2",
            TokenKind::FontFamily => "a list of font names",
        })
    }
}

/// A token's value, already parsed and typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenValue {
    Color(Srgb8),
    Px(u16),
    Ms(u16),
    FontWeight(u16),
    Number(Hundredths),
    FontFamily(Vec<String>),
}

impl TokenValue {
    pub fn kind(&self) -> TokenKind {
        match self {
            TokenValue::Color(_) => TokenKind::Color,
            TokenValue::Px(_) => TokenKind::Dimension,
            TokenValue::Ms(_) => TokenKind::Duration,
            TokenValue::FontWeight(_) => TokenKind::FontWeight,
            TokenValue::Number(_) => TokenKind::Number,
            TokenValue::FontFamily(_) => TokenKind::FontFamily,
        }
    }

    /// The number a range rule bounds, for the kinds that have one.
    fn magnitude(&self) -> Option<u16> {
        match self {
            TokenValue::Px(n) | TokenValue::Ms(n) | TokenValue::FontWeight(n) => Some(*n),
            TokenValue::Number(h) => Some(h.0),
            TokenValue::Color(_) | TokenValue::FontFamily(_) => None,
        }
    }

    /// `n` written in this value's unit, to report a bound: `44px`, `1.6`.
    fn in_unit(&self, n: u16) -> String {
        match self {
            TokenValue::Px(_) => TokenValue::Px(n).to_string(),
            TokenValue::Ms(_) => TokenValue::Ms(n).to_string(),
            TokenValue::Number(_) => Hundredths(n).to_string(),
            _ => n.to_string(),
        }
    }
}

impl fmt::Display for TokenValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenValue::Color(c) => write!(f, "#{:02x}{:02x}{:02x}", c.r, c.g, c.b),
            TokenValue::Px(n) => write!(f, "{n}px"),
            TokenValue::Ms(n) => write!(f, "{n}ms"),
            TokenValue::FontWeight(n) => write!(f, "{n}"),
            TokenValue::Number(h) => write!(f, "{h}"),
            TokenValue::FontFamily(names) => f.write_str(&names.join(", ")),
        }
    }
}

/// What values a token accepts beyond its kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenRule {
    Any,
    /// Inclusive bounds, in the token's own unit (hundredths for a number).
    Range {
        min: u16,
        max: u16,
    },
}

/// One entry of the catalog: its default and whether a template may change it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenSpec {
    pub path: TokenPath,
    pub default: TokenValue,
    pub themeable: bool,
    pub rule: TokenRule,
}

/// A resolved theme: every token of the catalog with its value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Theme(BTreeMap<TokenPath, TokenValue>);

impl Theme {
    pub fn get(&self, path: &str) -> Option<&TokenValue> {
        self.0.get(path)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&TokenPath, &TokenValue)> {
        self.0.iter()
    }

    /// The token's value in pixels, if it is a dimension.
    pub fn px(&self, path: &str) -> Option<u16> {
        match self.get(path) {
            Some(TokenValue::Px(n)) => Some(*n),
            _ => None,
        }
    }

    /// The token's value in milliseconds, if it is a duration.
    pub fn ms(&self, path: &str) -> Option<u16> {
        match self.get(path) {
            Some(TokenValue::Ms(n)) => Some(*n),
            _ => None,
        }
    }
}

/// The closed set of tokens: a template can only change what is here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeCatalog {
    specs: Vec<TokenSpec>,
}

impl ThemeCatalog {
    pub fn new(specs: Vec<TokenSpec>) -> Result<Self, DomainError> {
        let mut seen = std::collections::BTreeSet::new();
        for spec in &specs {
            if !seen.insert(spec.path.as_str()) {
                return Err(DomainError::DuplicateToken(spec.path.to_string()));
            }
            check_value(spec.path.as_str(), spec.rule, &spec.default)?;
        }
        Ok(Self { specs })
    }

    pub fn spec(&self, path: &str) -> Option<&TokenSpec> {
        self.specs.iter().find(|s| s.path.as_str() == path)
    }

    pub fn specs(&self) -> &[TokenSpec] {
        &self.specs
    }

    /// The theme of a template that changes nothing.
    pub fn defaults(&self) -> Theme {
        Theme(
            self.specs
                .iter()
                .map(|s| (s.path.clone(), s.default.clone()))
                .collect(),
        )
    }

    /// The defaults with a template's overrides applied. Each override must be
    /// the only one for its token, name a token of the catalog that a template
    /// may change, hold a value of its kind and keep to its rule.
    pub fn resolve(&self, overrides: Vec<(String, TokenValue)>) -> Result<Theme, DomainError> {
        let mut theme = self.defaults();
        let mut seen = std::collections::BTreeSet::new();
        for (name, value) in overrides {
            if !seen.insert(name.clone()) {
                return Err(DomainError::DuplicateToken(name));
            }
            let spec = self
                .spec(&name)
                .ok_or_else(|| DomainError::UnknownToken(name.clone()))?;
            if !spec.themeable {
                return Err(DomainError::TokenNotThemeable(name));
            }
            let expected = spec.default.kind();
            if value.kind() != expected {
                return Err(DomainError::TokenKindMismatch {
                    token: name,
                    expected,
                });
            }
            check_value(&name, spec.rule, &value)?;
            theme.0.insert(spec.path.clone(), value);
        }
        Ok(theme)
    }
}

/// A value keeps to its token's rule, and a font family list is plain names.
fn check_value(token: &str, rule: TokenRule, value: &TokenValue) -> Result<(), DomainError> {
    if let TokenValue::FontFamily(names) = value {
        if names.is_empty() || !names.iter().all(|n| is_font_name(n)) {
            return Err(DomainError::InvalidFontFamily(token.to_string()));
        }
    }
    if let TokenRule::Range { min, max } = rule {
        let inside = value.magnitude().is_some_and(|n| (min..=max).contains(&n));
        if !inside {
            return Err(DomainError::TokenOutOfRange {
                token: token.to_string(),
                value: value.to_string(),
                min: value.in_unit(min),
                max: value.in_unit(max),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUTTON_BACKGROUND: &str = "color.button.background";

    fn colour(r: u8, g: u8, b: u8) -> TokenValue {
        TokenValue::Color(Srgb8 { r, g, b })
    }

    fn spec(path: &str, default: TokenValue) -> TokenSpec {
        TokenSpec {
            path: TokenPath::new(path).unwrap(),
            default,
            themeable: true,
            rule: TokenRule::Any,
        }
    }

    fn catalog() -> ThemeCatalog {
        ThemeCatalog::new(vec![spec(BUTTON_BACKGROUND, colour(38, 42, 59))]).unwrap()
    }

    #[test]
    fn token_path_accepts_dotted_kebab_segments() {
        for raw in [
            "color.button",
            "color.button.background",
            "color.button.pressed-background",
            "a.b2.c-3.d",
        ] {
            assert_eq!(TokenPath::new(raw).unwrap().as_str(), raw);
        }
    }

    #[test]
    fn token_path_rejects_bad_segments() {
        for raw in [
            "color",
            "Color.button",
            "color..button",
            "color.button{",
            "a.b.c.d.e",
        ] {
            assert_eq!(
                TokenPath::new(raw).unwrap_err(),
                DomainError::InvalidTokenPath(raw.to_string()),
                "`{raw}` must be rejected"
            );
        }
    }

    #[test]
    fn catalog_rejects_duplicate_paths() {
        let err = ThemeCatalog::new(vec![
            spec(BUTTON_BACKGROUND, colour(1, 2, 3)),
            spec(BUTTON_BACKGROUND, colour(4, 5, 6)),
        ])
        .unwrap_err();
        assert_eq!(err, DomainError::DuplicateToken(BUTTON_BACKGROUND.into()));
    }

    #[test]
    fn resolve_without_overrides_returns_defaults() {
        let theme = catalog().resolve(vec![]).unwrap();
        assert_eq!(theme, catalog().defaults());
        assert_eq!(
            theme.get(BUTTON_BACKGROUND),
            Some(&TokenValue::Color(Srgb8 {
                r: 38,
                g: 42,
                b: 59
            }))
        );
        assert_eq!(theme.iter().count(), 1);
    }

    #[test]
    fn resolve_applies_an_override() {
        let theme = catalog()
            .resolve(vec![(BUTTON_BACKGROUND.into(), colour(30, 30, 46))])
            .unwrap();
        assert_eq!(theme.get(BUTTON_BACKGROUND), Some(&colour(30, 30, 46)));
        assert_eq!(theme.iter().count(), 1);
    }

    #[test]
    fn resolve_rejects_unknown_token() {
        let err = catalog()
            .resolve(vec![("color.button.nope".into(), colour(30, 30, 46))])
            .unwrap_err();
        assert_eq!(err, DomainError::UnknownToken("color.button.nope".into()));
        assert_eq!(err.to_string(), "unknown theme token `color.button.nope`");
    }

    const BUTTON_MIN: &str = "size.button.min";
    const PAGE_TRANSITION: &str = "motion.page-transition";
    const LABEL_WEIGHT: &str = "font.weight.label";
    const LINE_HEIGHT: &str = "font.line-height.label";
    const QUIT_BACKGROUND: &str = "color.quit.background";
    const LABEL_FAMILY: &str = "font.family.label";

    fn ranged(path: &str, default: TokenValue, min: u16, max: u16) -> TokenSpec {
        TokenSpec {
            rule: TokenRule::Range { min, max },
            ..spec(path, default)
        }
    }

    fn fixed(path: &str, default: TokenValue) -> TokenSpec {
        TokenSpec {
            themeable: false,
            ..spec(path, default)
        }
    }

    fn family(names: &[&str]) -> TokenValue {
        TokenValue::FontFamily(names.iter().map(|n| n.to_string()).collect())
    }

    /// One token of every kind, with the bounds of the real catalog.
    fn full_catalog() -> ThemeCatalog {
        ThemeCatalog::new(vec![
            spec(BUTTON_BACKGROUND, colour(38, 42, 59)),
            fixed(QUIT_BACKGROUND, colour(178, 58, 72)),
            ranged(BUTTON_MIN, TokenValue::Px(64), 44, 96),
            ranged(PAGE_TRANSITION, TokenValue::Ms(200), 0, 400),
            ranged(LABEL_WEIGHT, TokenValue::FontWeight(600), 300, 900),
            ranged(LINE_HEIGHT, TokenValue::Number(Hundredths(120)), 100, 160),
            fixed(
                LABEL_FAMILY,
                family(&["system-ui", "Segoe UI", "Roboto", "sans-serif"]),
            ),
        ])
        .unwrap()
    }

    fn resolve_err(overrides: Vec<(&str, TokenValue)>) -> DomainError {
        full_catalog()
            .resolve(
                overrides
                    .into_iter()
                    .map(|(name, value)| (name.to_string(), value))
                    .collect(),
            )
            .unwrap_err()
    }

    #[test]
    fn resolve_rejects_non_themeable_token() {
        let err = resolve_err(vec![(QUIT_BACKGROUND, colour(38, 42, 59))]);
        assert_eq!(err, DomainError::TokenNotThemeable(QUIT_BACKGROUND.into()));
        assert_eq!(
            err.to_string(),
            "theme token `color.quit.background` is fixed and cannot be changed by a template"
        );
        let err = resolve_err(vec![(LABEL_FAMILY, family(&["Arial"]))]);
        assert_eq!(err, DomainError::TokenNotThemeable(LABEL_FAMILY.into()));
    }

    #[test]
    fn resolve_rejects_wrong_kind() {
        let err = resolve_err(vec![(BUTTON_BACKGROUND, TokenValue::Px(64))]);
        assert_eq!(
            err,
            DomainError::TokenKindMismatch {
                token: BUTTON_BACKGROUND.into(),
                expected: TokenKind::Color,
            }
        );
        let err = resolve_err(vec![(BUTTON_MIN, TokenValue::Ms(64))]);
        assert_eq!(
            err.to_string(),
            r#"theme token `size.button.min` expects a dimension in whole pixels, like "64px""#
        );
    }

    #[test]
    fn resolve_rejects_below_min() {
        let err = resolve_err(vec![(BUTTON_MIN, TokenValue::Px(30))]);
        assert_eq!(
            err,
            DomainError::TokenOutOfRange {
                token: BUTTON_MIN.into(),
                value: "30px".into(),
                min: "44px".into(),
                max: "96px".into(),
            }
        );
        assert_eq!(
            err.to_string(),
            "theme token `size.button.min`: 30px is outside the allowed range 44px to 96px"
        );
    }

    #[test]
    fn resolve_rejects_above_max_duration() {
        let err = resolve_err(vec![(PAGE_TRANSITION, TokenValue::Ms(500))]);
        assert_eq!(
            err.to_string(),
            "theme token `motion.page-transition`: 500ms is outside the allowed range 0ms to 400ms"
        );
    }

    #[test]
    fn resolve_rejects_number_out_of_range() {
        let err = resolve_err(vec![(LINE_HEIGHT, TokenValue::Number(Hundredths(170)))]);
        assert_eq!(
            err.to_string(),
            "theme token `font.line-height.label`: 1.7 is outside the allowed range 1 to 1.6"
        );
        let err = resolve_err(vec![(LABEL_WEIGHT, TokenValue::FontWeight(950))]);
        assert_eq!(
            err.to_string(),
            "theme token `font.weight.label`: 950 is outside the allowed range 300 to 900"
        );
    }

    #[test]
    fn resolve_rejects_duplicate_override() {
        let err = resolve_err(vec![
            (BUTTON_MIN, TokenValue::Px(72)),
            (BUTTON_MIN, TokenValue::Px(80)),
        ]);
        assert_eq!(err, DomainError::DuplicateToken(BUTTON_MIN.into()));
    }

    #[test]
    fn resolve_accepts_the_bounds() {
        for (px, ms, weight, number) in [(44, 0, 300, 100), (96, 400, 900, 160)] {
            let theme = full_catalog()
                .resolve(vec![
                    (BUTTON_MIN.into(), TokenValue::Px(px)),
                    (PAGE_TRANSITION.into(), TokenValue::Ms(ms)),
                    (LABEL_WEIGHT.into(), TokenValue::FontWeight(weight)),
                    (LINE_HEIGHT.into(), TokenValue::Number(Hundredths(number))),
                ])
                .unwrap();
            assert_eq!(theme.get(BUTTON_MIN), Some(&TokenValue::Px(px)));
            assert_eq!(theme.get(PAGE_TRANSITION), Some(&TokenValue::Ms(ms)));
            assert_eq!(
                theme.get(LABEL_WEIGHT),
                Some(&TokenValue::FontWeight(weight))
            );
            assert_eq!(
                theme.get(LINE_HEIGHT),
                Some(&TokenValue::Number(Hundredths(number)))
            );
            assert_eq!(theme.iter().count(), full_catalog().specs().len());
        }
    }

    #[test]
    fn catalog_rejects_default_out_of_range() {
        let err =
            ThemeCatalog::new(vec![ranged(BUTTON_MIN, TokenValue::Px(40), 44, 96)]).unwrap_err();
        assert_eq!(
            err,
            DomainError::TokenOutOfRange {
                token: BUTTON_MIN.into(),
                value: "40px".into(),
                min: "44px".into(),
                max: "96px".into(),
            }
        );
    }

    #[test]
    fn catalog_rejects_injected_font_family() {
        for names in [&["Arial;}"][..], &[][..], &["Arial", "Segoe UI", "'x'"][..]] {
            let err = ThemeCatalog::new(vec![fixed(LABEL_FAMILY, family(names))]).unwrap_err();
            assert_eq!(
                err,
                DomainError::InvalidFontFamily(LABEL_FAMILY.into()),
                "{names:?} must be rejected"
            );
        }
        assert_eq!(
            DomainError::InvalidFontFamily(LABEL_FAMILY.into()).to_string(),
            "theme token `font.family.label` needs a non-empty list of plain font names"
        );
    }

    #[test]
    fn number_displays_without_trailing_zeros() {
        for (hundredths, text) in [
            (120, "1.2"),
            (100, "1"),
            (135, "1.35"),
            (105, "1.05"),
            (0, "0"),
        ] {
            assert_eq!(Hundredths(hundredths).to_string(), text);
            assert_eq!(TokenValue::Number(Hundredths(hundredths)).to_string(), text);
        }
        assert_eq!(TokenValue::Px(64).to_string(), "64px");
        assert_eq!(TokenValue::Ms(200).to_string(), "200ms");
        assert_eq!(TokenValue::FontWeight(600).to_string(), "600");
    }

    #[test]
    fn theme_px_reads_dimensions() {
        let theme = full_catalog().defaults();
        assert_eq!(theme.px(BUTTON_MIN), Some(64));
        assert_eq!(theme.px(PAGE_TRANSITION), None);
        assert_eq!(theme.px(BUTTON_BACKGROUND), None);
        assert_eq!(theme.px("size.button.nope"), None);
        assert_eq!(theme.ms(PAGE_TRANSITION), Some(200));
        assert_eq!(theme.ms(BUTTON_MIN), None);
    }
}
