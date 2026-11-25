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

/// The kind of value a token holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Color,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Color => f.write_str(r##"a colour like "#1e1e2e""##),
        }
    }
}

/// A token's value, already parsed and typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenValue {
    Color(Srgb8),
}

impl TokenValue {
    pub fn kind(&self) -> TokenKind {
        match self {
            TokenValue::Color(_) => TokenKind::Color,
        }
    }
}

/// One entry of the catalog: its default and whether a template may change it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenSpec {
    pub path: TokenPath,
    pub default: TokenValue,
    pub themeable: bool,
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

    /// The defaults with a template's overrides applied. Every override must
    /// name a token of the catalog and hold a value of its kind.
    pub fn resolve(&self, overrides: Vec<(String, TokenValue)>) -> Result<Theme, DomainError> {
        let mut theme = self.defaults();
        for (name, value) in overrides {
            let spec = self
                .spec(&name)
                .ok_or_else(|| DomainError::UnknownToken(name.clone()))?;
            let expected = spec.default.kind();
            if value.kind() != expected {
                return Err(DomainError::TokenKindMismatch {
                    token: name,
                    expected,
                });
            }
            theme.0.insert(spec.path.clone(), value);
        }
        Ok(theme)
    }
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
}
