//! The design tokens: `design/tokens.json`, a DTCG 2025.10 file, embedded in
//! the binary and read into the domain's `ThemeCatalog`.
//!
//! This reader understands the part of the format the catalog uses today: a
//! `$type` inherited from the nearest group, a colour read from its `hex`, and
//! `$extensions["dev.plinth"].themeable`.

use plinth_application::TemplateError;
use plinth_domain::{ThemeCatalog, TokenPath, TokenRule, TokenSpec, TokenValue};
use serde_json::{Map, Value};

use crate::theme_scalars::parse_hex_color;

/// The tokens file, embedded so a build always carries the catalog it was
/// tested with.
pub const TOKENS_SRC: &str = include_str!("../../../design/tokens.json");

/// The vendor key under `$extensions` that holds Plinth's own data.
const EXTENSION_KEY: &str = "dev.plinth";

/// The catalog of the embedded tokens file.
pub fn embedded_catalog() -> Result<ThemeCatalog, TemplateError> {
    load_catalog(TOKENS_SRC)
}

/// Reads a tokens file into a catalog.
pub fn load_catalog(src: &str) -> Result<ThemeCatalog, TemplateError> {
    let doc: Value = serde_json::from_str(src)
        .map_err(|e| TemplateError(format!("invalid design tokens: {e}")))?;
    let root = doc
        .as_object()
        .ok_or_else(|| TemplateError("invalid design tokens: the root is not an object".into()))?;
    let mut specs = Vec::new();
    read_group(root, &mut Vec::new(), None, &mut specs)?;
    ThemeCatalog::new(specs).map_err(|e| TemplateError(format!("invalid design tokens: {e}")))
}

/// Walks a group: a child with `$value` is a token, any other child a group.
fn read_group(
    group: &Map<String, Value>,
    path: &mut Vec<String>,
    inherited_type: Option<&str>,
    specs: &mut Vec<TokenSpec>,
) -> Result<(), TemplateError> {
    let group_type = group
        .get("$type")
        .and_then(Value::as_str)
        .or(inherited_type);
    for (name, node) in group {
        if name.starts_with('$') {
            continue;
        }
        path.push(name.clone());
        let dotted = path.join(".");
        let child = node
            .as_object()
            .ok_or_else(|| invalid(&dotted, "is not an object"))?;
        if child.contains_key("$value") {
            specs.push(read_token(child, &dotted, group_type)?);
        } else {
            read_group(child, path, group_type, specs)?;
        }
        path.pop();
    }
    Ok(())
}

fn read_token(
    token: &Map<String, Value>,
    dotted: &str,
    group_type: Option<&str>,
) -> Result<TokenSpec, TemplateError> {
    let kind = token
        .get("$type")
        .and_then(Value::as_str)
        .or(group_type)
        .ok_or_else(|| invalid(dotted, "has no $type"))?;
    let default = match kind {
        "color" => token["$value"]
            .get("hex")
            .and_then(Value::as_str)
            .and_then(parse_hex_color)
            .map(TokenValue::Color)
            .ok_or_else(|| invalid(dotted, r##"a colour needs a `hex` like "#262a3b""##))?,
        other => return Err(invalid(dotted, &format!("unsupported $type `{other}`"))),
    };
    let themeable = token
        .get("$extensions")
        .and_then(|e| e.get(EXTENSION_KEY))
        .and_then(|p| p.get("themeable"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let path = TokenPath::new(dotted).map_err(|e| invalid(dotted, &e.to_string()))?;
    Ok(TokenSpec {
        path,
        default,
        themeable,
        rule: TokenRule::Any,
    })
}

fn invalid(path: &str, reason: &str) -> TemplateError {
    TemplateError(format!("invalid design tokens at `{path}`: {reason}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use plinth_domain::{Srgb8, TokenKind};

    #[test]
    fn embedded_tokens_parse() {
        let catalog = embedded_catalog().unwrap();
        let spec = catalog.spec("color.button.background").unwrap();
        assert_eq!(
            spec.default,
            TokenValue::Color(Srgb8 {
                r: 38,
                g: 42,
                b: 59
            })
        );
        assert!(spec.themeable);
    }

    #[test]
    fn group_type_is_inherited() {
        let src = r##"{
          "color": {
            "$type": "color",
            "panel": {
              "background": {
                "$value": { "colorSpace": "srgb", "components": [0.067, 0.071, 0.102], "hex": "#11121a" }
              }
            }
          }
        }"##;
        let catalog = load_catalog(src).unwrap();
        let spec = catalog.spec("color.panel.background").unwrap();
        assert_eq!(spec.default.kind(), TokenKind::Color);
        assert!(!spec.themeable);
    }

    #[test]
    fn token_without_type_is_rejected() {
        let src = r##"{
          "color": {
            "panel": {
              "background": {
                "$value": { "colorSpace": "srgb", "components": [0.067, 0.071, 0.102], "hex": "#11121a" }
              }
            }
          }
        }"##;
        let err = load_catalog(src).unwrap_err();
        assert_eq!(
            err.0,
            "invalid design tokens at `color.panel.background`: has no $type"
        );
    }
}
