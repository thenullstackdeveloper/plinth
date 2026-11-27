//! The design tokens: `design/tokens.json`, a DTCG 2025.10 file, embedded in
//! the binary and read into the domain's `ThemeCatalog`.
//!
//! The reader accepts a literal subset of the format and rejects the rest:
//! - six `$type`s, inherited from the nearest group: `color` (`srgb`, opaque,
//!   with a `hex` that matches its `components`), `dimension` (whole `px`),
//!   `duration` (whole `ms`), `fontWeight` (a whole number), `number` (at most
//!   two decimals) and `fontFamily` (a list of names);
//! - group and token names in lowercase kebab case;
//! - no aliases, `$ref`, `$extends`, `$root` or unknown `$` keys;
//! - Plinth's own data in `$extensions["dev.plinth"]`: `themeable`, `min` and
//!   `max`.
//!
//! Every error starts with the path of the group or token it is about.

use plinth_application::TemplateError;
use plinth_domain::{Hundredths, ThemeCatalog, TokenPath, TokenRule, TokenSpec, TokenValue};
use serde_json::{Map, Value};

use crate::theme_scalars::parse_hex_color;

/// The tokens file, embedded so a build always carries the catalog it was
/// tested with.
pub const TOKENS_SRC: &str = include_str!("../../../design/tokens.json");

/// The vendor key under `$extensions` that holds Plinth's own data.
const EXTENSION_KEY: &str = "dev.plinth";

/// The `$` keys a group may carry; everything else starting with `$` is out.
const GROUP_KEYS: [&str; 4] = ["$type", "$description", "$extensions", "$deprecated"];

/// The keys a token may carry.
const TOKEN_KEYS: [&str; 5] = [
    "$value",
    "$type",
    "$description",
    "$extensions",
    "$deprecated",
];

/// The keys `$extensions["dev.plinth"]` may carry.
const PLINTH_KEYS: [&str; 3] = ["themeable", "min", "max"];

/// How far a colour component may sit from `round(byte / 255, 3)`.
const COMPONENT_TOLERANCE: f64 = 0.0005;

const LITERAL_HINT: &str = "write the value itself";
const DIMENSION_RULE: &str =
    r#"a dimension must be whole pixels, like { "value": 64, "unit": "px" }"#;
const DURATION_RULE: &str =
    r#"a duration must be whole milliseconds, like { "value": 200, "unit": "ms" }"#;
const FONT_WEIGHT_RULE: &str = "a font weight must be a whole number from 1 to 1000";
const NUMBER_RULE: &str = "a number needs at most two decimals, from 0 to 655.35";
const FONT_FAMILY_RULE: &str = "a font family must be a list of names";

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
    let here = display_path(path);
    check_dollar_keys(group, &GROUP_KEYS, &here)?;
    let group_type = match group.get("$type") {
        Some(t) => Some(
            t.as_str()
                .ok_or_else(|| invalid(&here, "`$type` must be a string"))?,
        ),
        None => inherited_type,
    };
    for (name, node) in group {
        if name.starts_with('$') {
            continue;
        }
        path.push(name.clone());
        let dotted = path.join(".");
        if !is_kebab(name) {
            return Err(invalid(
                &dotted,
                &format!("`{name}` is not a lowercase kebab-case name like `button-min`"),
            ));
        }
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
    check_dollar_keys(token, &TOKEN_KEYS, dotted)?;
    if let Some(child) = token.keys().find(|k| !k.starts_with('$')) {
        return Err(invalid(
            dotted,
            &format!("a token cannot have children, but it has `{child}`"),
        ));
    }
    let kind = match token.get("$type") {
        Some(t) => t
            .as_str()
            .ok_or_else(|| invalid(dotted, "`$type` must be a string"))?,
        None => group_type.ok_or_else(|| invalid(dotted, "has no $type"))?,
    };
    let value = &token["$value"];
    check_no_references(value, dotted)?;
    let default = match kind {
        "color" => read_color(value, dotted)?,
        "dimension" => TokenValue::Px(
            read_unit_value(value, "px").ok_or_else(|| invalid(dotted, DIMENSION_RULE))?,
        ),
        "duration" => TokenValue::Ms(
            read_unit_value(value, "ms").ok_or_else(|| invalid(dotted, DURATION_RULE))?,
        ),
        "fontWeight" => TokenValue::FontWeight(
            whole(value)
                .filter(|w| (1..=1000).contains(w))
                .ok_or_else(|| invalid(dotted, FONT_WEIGHT_RULE))?,
        ),
        "number" => {
            TokenValue::Number(hundredths(value).ok_or_else(|| invalid(dotted, NUMBER_RULE))?)
        }
        "fontFamily" => TokenValue::FontFamily(
            read_font_family(value).ok_or_else(|| invalid(dotted, FONT_FAMILY_RULE))?,
        ),
        other => return Err(invalid(dotted, &format!("unsupported $type `{other}`"))),
    };
    let (themeable, rule) = read_extensions(token, dotted, &default)?;
    let path = TokenPath::new(dotted).map_err(|e| invalid(dotted, &e.to_string()))?;
    let spec = TokenSpec {
        path,
        default,
        themeable,
        rule,
    };
    // The domain checks the default against its rule; asking it per token
    // keeps the token's path at the front of the message.
    ThemeCatalog::new(vec![spec.clone()]).map_err(|e| invalid(dotted, &e.to_string()))?;
    Ok(spec)
}

/// Rejects the `$` keys outside `allowed`, naming the references DTCG has and
/// Plinth does not support.
fn check_dollar_keys(
    node: &Map<String, Value>,
    allowed: &[&str],
    path: &str,
) -> Result<(), TemplateError> {
    for key in node.keys().filter(|k| k.starts_with('$')) {
        if matches!(key.as_str(), "$ref" | "$extends" | "$root") {
            return Err(invalid(
                path,
                &format!("`{key}` is not supported; {LITERAL_HINT}"),
            ));
        }
        if !allowed.contains(&key.as_str()) {
            return Err(invalid(path, &format!("unknown key `{key}`")));
        }
    }
    Ok(())
}

/// A `$value` holds literals only: no `{group.token}` alias and no `$ref`,
/// at any depth.
fn check_no_references(value: &Value, path: &str) -> Result<(), TemplateError> {
    match value {
        Value::String(s) if s.starts_with('{') && s.ends_with('}') => Err(invalid(
            path,
            &format!("aliases like `{{group.token}}` are not supported; {LITERAL_HINT}"),
        )),
        Value::Array(items) => items.iter().try_for_each(|v| check_no_references(v, path)),
        Value::Object(map) => {
            if map.contains_key("$ref") {
                return Err(invalid(
                    path,
                    &format!("`$ref` is not supported; {LITERAL_HINT}"),
                ));
            }
            map.values().try_for_each(|v| check_no_references(v, path))
        }
        _ => Ok(()),
    }
}

/// An opaque `srgb` colour, read from its `hex`, whose `components` must be
/// `round(byte / 255, 3)` within the tolerance.
fn read_color(value: &Value, path: &str) -> Result<TokenValue, TemplateError> {
    let color = value
        .as_object()
        .ok_or_else(|| invalid(path, "a colour must be an object"))?;
    if let Some(key) = color
        .keys()
        .find(|k| !matches!(k.as_str(), "colorSpace" | "components" | "alpha" | "hex"))
    {
        return Err(invalid(path, &format!("unknown key `{key}` in a colour")));
    }
    if color.get("colorSpace").and_then(Value::as_str) != Some("srgb") {
        return Err(invalid(path, "a colour must be in the `srgb` colour space"));
    }
    if color.get("alpha").is_some_and(|a| a.as_f64() != Some(1.0)) {
        return Err(invalid(
            path,
            "a colour must be opaque, with no `alpha` or `alpha` 1",
        ));
    }
    let hex = color.get("hex").and_then(Value::as_str);
    let rgb = hex
        .and_then(parse_hex_color)
        .ok_or_else(|| invalid(path, r##"a colour needs a `hex` like "#262a3b""##))?;
    let expected =
        [rgb.r, rgb.g, rgb.b].map(|byte| (f64::from(byte) / 255.0 * 1000.0).round() / 1000.0);
    let components: Option<Vec<f64>> = color
        .get("components")
        .and_then(Value::as_array)
        .filter(|c| c.len() == 3)
        .and_then(|c| c.iter().map(Value::as_f64).collect());
    let matches = components.is_some_and(|c| {
        c.iter()
            .zip(expected)
            .all(|(got, want)| (got - want).abs() <= COMPONENT_TOLERANCE)
    });
    if !matches {
        let want: Vec<String> = expected.iter().map(f64::to_string).collect();
        return Err(invalid(
            path,
            &format!(
                "`hex` {} needs `components` [{}]",
                hex.unwrap_or_default(),
                want.join(", ")
            ),
        ));
    }
    Ok(TokenValue::Color(rgb))
}

/// `{ "value": <whole number>, "unit": <unit> }` and nothing else.
fn read_unit_value(value: &Value, unit: &str) -> Option<u16> {
    let map = value.as_object()?;
    if map.len() != 2 || map.get("unit")?.as_str()? != unit {
        return None;
    }
    whole(map.get("value")?)
}

/// A non-empty list of strings; the domain checks each name.
fn read_font_family(value: &Value) -> Option<Vec<String>> {
    value
        .as_array()
        .filter(|names| !names.is_empty())?
        .iter()
        .map(|n| n.as_str().map(str::to_string))
        .collect()
}

/// A whole JSON number that fits a `u16`; `64.0` is not whole here.
fn whole(value: &Value) -> Option<u16> {
    value.as_u64().and_then(|n| u16::try_from(n).ok())
}

/// A non-negative number with at most two decimals, as hundredths.
fn hundredths(value: &Value) -> Option<Hundredths> {
    let n = value.as_f64().filter(|n| n.is_finite() && *n >= 0.0)?;
    let scaled = n * 100.0;
    let rounded = scaled.round();
    if (scaled - rounded).abs() > 1e-6 || rounded > f64::from(u16::MAX) {
        return None;
    }
    // In range and whole, so the cast is exact.
    Some(Hundredths(rounded as u16))
}

/// `themeable`, `min` and `max` from `$extensions["dev.plinth"]`. A token
/// without them is fixed and takes any value of its kind.
fn read_extensions(
    token: &Map<String, Value>,
    path: &str,
    default: &TokenValue,
) -> Result<(bool, TokenRule), TemplateError> {
    let Some(plinth) = token.get("$extensions").and_then(|e| e.get(EXTENSION_KEY)) else {
        return Ok((false, TokenRule::Any));
    };
    let plinth = plinth.as_object().ok_or_else(|| {
        invalid(
            path,
            &format!("`$extensions.{EXTENSION_KEY}` must be an object"),
        )
    })?;
    if let Some(key) = plinth.keys().find(|k| !PLINTH_KEYS.contains(&k.as_str())) {
        return Err(invalid(
            path,
            &format!("unknown key `{key}` in `$extensions.{EXTENSION_KEY}`"),
        ));
    }
    let themeable = match plinth.get("themeable") {
        Some(t) => t
            .as_bool()
            .ok_or_else(|| invalid(path, "`themeable` must be true or false"))?,
        None => false,
    };
    let rule = match (plinth.get("min"), plinth.get("max")) {
        (None, None) => TokenRule::Any,
        (Some(min), Some(max)) => TokenRule::Range {
            min: read_bound(min, "min", path, default)?,
            max: read_bound(max, "max", path, default)?,
        },
        _ => return Err(invalid(path, "`min` and `max` go together")),
    };
    Ok((themeable, rule))
}

/// A bound in the token's own unit: whole for px, ms and weights, hundredths
/// for a number. Colours and font families have no range.
fn read_bound(
    bound: &Value,
    name: &str,
    path: &str,
    default: &TokenValue,
) -> Result<u16, TemplateError> {
    let read = match default {
        TokenValue::Px(_) | TokenValue::Ms(_) | TokenValue::FontWeight(_) => whole(bound),
        TokenValue::Number(_) => hundredths(bound).map(|h| h.0),
        TokenValue::Color(_) | TokenValue::FontFamily(_) => {
            return Err(invalid(
                path,
                "only a number-like token can have `min` and `max`",
            ))
        }
    };
    read.ok_or_else(|| {
        invalid(
            path,
            &format!("`{name}` must be a number in the token's unit"),
        )
    })
}

/// `[a-z][a-z0-9-]*`, the same rule as a segment of a `TokenPath`.
fn is_kebab(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some('a'..='z'))
        && chars.all(|c| matches!(c, 'a'..='z' | '0'..='9' | '-'))
}

/// The dotted path of a group, or `(root)` for the top of the file.
fn display_path(path: &[String]) -> String {
    if path.is_empty() {
        "(root)".to_string()
    } else {
        path.join(".")
    }
}

fn invalid(path: &str, reason: &str) -> TemplateError {
    TemplateError(format!("invalid design tokens at `{path}`: {reason}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use plinth_application::theme_css::css_variable;
    use plinth_domain::{Hundredths, Srgb8, TokenKind};

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

    /// A tokens file whose only token is `group.token`, with the given body.
    fn one_token(body: &str) -> String {
        format!(r#"{{ "group": {{ "token": {body} }} }}"#)
    }

    fn rejection(src: &str) -> String {
        load_catalog(src).unwrap_err().0
    }

    #[test]
    fn parses_every_supported_type() {
        let src = r##"{
          "color": { "$type": "color", "ink": {
            "$value": { "colorSpace": "srgb", "components": [0.902, 0.91, 0.941], "alpha": 1, "hex": "#e6e8f0" },
            "$description": "Label text.",
            "$deprecated": false
          } },
          "size": { "$type": "dimension", "min": { "$value": { "value": 64, "unit": "px" } } },
          "motion": { "$type": "duration", "page": { "$value": { "value": 200, "unit": "ms" } } },
          "font": {
            "weight": { "$type": "fontWeight", "$value": 600 },
            "line-height": { "$type": "number", "$value": 1.2 },
            "family": { "$type": "fontFamily", "$value": ["system-ui", "Segoe UI"] }
          }
        }"##;
        let catalog = load_catalog(src).unwrap();
        let default = |path: &str| catalog.spec(path).unwrap().default.clone();
        assert_eq!(
            default("color.ink"),
            TokenValue::Color(Srgb8 {
                r: 230,
                g: 232,
                b: 240
            })
        );
        assert_eq!(default("size.min"), TokenValue::Px(64));
        assert_eq!(default("motion.page"), TokenValue::Ms(200));
        assert_eq!(default("font.weight"), TokenValue::FontWeight(600));
        assert_eq!(
            default("font.line-height"),
            TokenValue::Number(Hundredths(120))
        );
        assert_eq!(
            default("font.family"),
            TokenValue::FontFamily(vec!["system-ui".into(), "Segoe UI".into()])
        );
        assert_eq!(catalog.specs().len(), 6);
        for (kind, value, reason) in [
            ("fontWeight", "0", FONT_WEIGHT_RULE),
            ("fontWeight", "1001", FONT_WEIGHT_RULE),
            ("fontWeight", "600.5", FONT_WEIGHT_RULE),
            ("fontWeight", r#""bold""#, FONT_WEIGHT_RULE),
            ("number", "1.205", NUMBER_RULE),
            ("number", "-1", NUMBER_RULE),
            ("number", r#""1.2""#, NUMBER_RULE),
            ("fontFamily", r#""system-ui""#, FONT_FAMILY_RULE),
            ("fontFamily", "[]", FONT_FAMILY_RULE),
            ("fontFamily", r#"["Arial", 3]"#, FONT_FAMILY_RULE),
        ] {
            assert_eq!(
                rejection(&one_token(&format!(
                    r#"{{ "$type": "{kind}", "$value": {value} }}"#
                ))),
                format!("invalid design tokens at `group.token`: {reason}"),
                "{kind} {value}"
            );
        }
    }

    #[test]
    fn reads_bounds_and_themeable_from_extensions() {
        let src = r##"{
          "size": { "$type": "dimension",
            "button": { "$value": { "value": 64, "unit": "px" },
              "$extensions": { "dev.plinth": { "themeable": true, "min": 44, "max": 96 }, "org.example": { "note": 1 } } },
            "touch": { "$value": { "value": 44, "unit": "px" } }
          },
          "font": {
            "line-height": { "$type": "number", "$value": 1.2,
              "$extensions": { "dev.plinth": { "themeable": true, "min": 1.0, "max": 1.6 } } },
            "weight": { "$type": "fontWeight", "$value": 600,
              "$extensions": { "dev.plinth": { "min": 300, "max": 900 } } }
          }
        }"##;
        let catalog = load_catalog(src).unwrap();
        let button = catalog.spec("size.button").unwrap();
        assert!(button.themeable);
        assert_eq!(button.rule, TokenRule::Range { min: 44, max: 96 });
        let touch = catalog.spec("size.touch").unwrap();
        assert!(!touch.themeable);
        assert_eq!(touch.rule, TokenRule::Any);
        let line_height = catalog.spec("font.line-height").unwrap();
        assert_eq!(line_height.rule, TokenRule::Range { min: 100, max: 160 });
        let weight = catalog.spec("font.weight").unwrap();
        assert!(!weight.themeable);
        assert_eq!(weight.rule, TokenRule::Range { min: 300, max: 900 });
        assert_eq!(
            rejection(&one_token(
                r#"{ "$type": "dimension", "$value": { "value": 30, "unit": "px" },
                     "$extensions": { "dev.plinth": { "min": 44, "max": 96 } } }"#
            )),
            "invalid design tokens at `group.token`: theme token `group.token`: 30px is outside the allowed range 44px to 96px"
        );
        assert_eq!(
            rejection(&one_token(
                r#"{ "$type": "dimension", "$value": { "value": 64, "unit": "px" },
                     "$extensions": { "dev.plinth": { "themable": true } } }"#
            )),
            "invalid design tokens at `group.token`: unknown key `themable` in `$extensions.dev.plinth`"
        );
        assert_eq!(
            rejection(&one_token(
                r#"{ "$type": "dimension", "$value": { "value": 64, "unit": "px" },
                     "$extensions": { "dev.plinth": { "min": 44 } } }"#
            )),
            "invalid design tokens at `group.token`: `min` and `max` go together"
        );
        assert_eq!(
            rejection(&one_token(
                r#"{ "$type": "dimension", "$value": { "value": 64, "unit": "px" },
                     "$extensions": { "dev.plinth": { "themeable": "yes" } } }"#
            )),
            "invalid design tokens at `group.token`: `themeable` must be true or false"
        );
        assert_eq!(
            rejection(&one_token(
                r#"{ "$type": "number", "$value": 1.2,
                     "$extensions": { "dev.plinth": { "min": 1, "max": 1.605 } } }"#
            )),
            "invalid design tokens at `group.token`: `max` must be a number in the token's unit"
        );
        assert_eq!(
            rejection(&one_token(
                r##"{ "$type": "color", "$value": { "colorSpace": "srgb", "components": [0, 0, 0], "hex": "#000000" },
                     "$extensions": { "dev.plinth": { "min": 0, "max": 1 } } }"##
            )),
            "invalid design tokens at `group.token`: only a number-like token can have `min` and `max`"
        );
    }

    #[test]
    fn rejects_curly_alias() {
        assert_eq!(
            rejection(&one_token(r#"{ "$type": "fontWeight", "$value": "{font.weight.base}" }"#)),
            "invalid design tokens at `group.token`: aliases like `{group.token}` are not supported; write the value itself"
        );
        assert_eq!(
            rejection(&one_token(
                r##"{ "$type": "color", "$value": { "colorSpace": "srgb", "components": ["{a.r}", 0, 0], "hex": "#000000" } }"##
            )),
            "invalid design tokens at `group.token`: aliases like `{group.token}` are not supported; write the value itself"
        );
    }

    #[test]
    fn rejects_json_pointer_ref() {
        assert_eq!(
            rejection(&one_token(
                r##"{ "$type": "color", "$value": { "$ref": "#/color/button/background/$value" } }"##
            )),
            "invalid design tokens at `group.token`: `$ref` is not supported; write the value itself"
        );
        assert_eq!(
            rejection(r##"{ "group": { "$ref": "#/other" } }"##),
            "invalid design tokens at `group`: `$ref` is not supported; write the value itself"
        );
    }

    #[test]
    fn rejects_extends_and_root() {
        assert_eq!(
            rejection(
                r##"{ "group": { "$extends": "{base}", "token": { "$type": "fontWeight", "$value": 600 } } }"##
            ),
            "invalid design tokens at `group`: `$extends` is not supported; write the value itself"
        );
        assert_eq!(
            rejection(r##"{ "group": { "$root": { "$type": "fontWeight", "$value": 600 } } }"##),
            "invalid design tokens at `group`: `$root` is not supported; write the value itself"
        );
    }

    #[test]
    fn rejects_unknown_dollar_key() {
        assert_eq!(
            rejection(&one_token(
                r#"{ "$type": "fontWeight", "$value": 600, "$themeable": true }"#
            )),
            "invalid design tokens at `group.token`: unknown key `$themeable`"
        );
        assert_eq!(
            rejection(
                r#"{ "group": { "$comment": "x", "token": { "$type": "fontWeight", "$value": 600 } } }"#
            ),
            "invalid design tokens at `group`: unknown key `$comment`"
        );
        assert_eq!(
            rejection(r#"{ "$schema": "x" }"#),
            "invalid design tokens at `(root)`: unknown key `$schema`"
        );
    }

    #[test]
    fn rejects_token_with_children() {
        assert_eq!(
            rejection(&one_token(
                r#"{ "$type": "fontWeight", "$value": 600, "bold": { "$value": 700 } }"#
            )),
            "invalid design tokens at `group.token`: a token cannot have children, but it has `bold`"
        );
    }

    #[test]
    fn rejects_unknown_type() {
        for kind in ["shadow", "typography", "border", "colour"] {
            assert_eq!(
                rejection(&one_token(&format!(
                    r#"{{ "$type": "{kind}", "$value": 1 }}"#
                ))),
                format!("invalid design tokens at `group.token`: unsupported $type `{kind}`")
            );
        }
    }

    #[test]
    fn rejects_name_outside_kebab() {
        for (src, path, name) in [
            (
                r#"{ "Group": { "token": { "$type": "fontWeight", "$value": 600 } } }"#,
                "Group",
                "Group",
            ),
            (
                r#"{ "group": { "big_token": { "$type": "fontWeight", "$value": 600 } } }"#,
                "group.big_token",
                "big_token",
            ),
            (
                r#"{ "group": { "2x": { "$type": "fontWeight", "$value": 600 } } }"#,
                "group.2x",
                "2x",
            ),
            (
                r#"{ "group": { "empty_group": {} } }"#,
                "group.empty_group",
                "empty_group",
            ),
        ] {
            assert_eq!(
                rejection(src),
                format!("invalid design tokens at `{path}`: `{name}` is not a lowercase kebab-case name like `button-min`")
            );
        }
    }

    #[test]
    fn rejects_non_srgb_colour() {
        assert_eq!(
            rejection(&one_token(
                r##"{ "$type": "color", "$value": { "colorSpace": "display-p3", "components": [0.149, 0.165, 0.231], "hex": "#262a3b" } }"##
            )),
            "invalid design tokens at `group.token`: a colour must be in the `srgb` colour space"
        );
    }

    #[test]
    fn rejects_alpha_below_one() {
        assert_eq!(
            rejection(&one_token(
                r##"{ "$type": "color", "$value": { "colorSpace": "srgb", "components": [0.149, 0.165, 0.231], "alpha": 0.5, "hex": "#262a3b" } }"##
            )),
            "invalid design tokens at `group.token`: a colour must be opaque, with no `alpha` or `alpha` 1"
        );
    }

    #[test]
    fn colour_requires_hex() {
        assert_eq!(
            rejection(&one_token(
                r##"{ "$type": "color", "$value": { "colorSpace": "srgb", "components": [0, 0, 0], "hex": "#000" } }"##
            )),
            r##"invalid design tokens at `group.token`: a colour needs a `hex` like "#262a3b""##
        );
        assert_eq!(
            rejection(&one_token(
                r#"{ "$type": "color", "$value": { "colorSpace": "srgb", "components": [0.149, 0.165, 0.231] } }"#
            )),
            r##"invalid design tokens at `group.token`: a colour needs a `hex` like "#262a3b""##
        );
    }

    #[test]
    fn colour_hex_must_match_components() {
        assert_eq!(
            rejection(&one_token(
                r##"{ "$type": "color", "$value": { "colorSpace": "srgb", "components": [0.149, 0.165, 0.232], "hex": "#262a3b" } }"##
            )),
            "invalid design tokens at `group.token`: `hex` #262a3b needs `components` [0.149, 0.165, 0.231]"
        );
        assert_eq!(
            rejection(&one_token(
                r##"{ "$type": "color", "$value": { "colorSpace": "srgb", "components": [0.149, 0.165], "hex": "#262a3b" } }"##
            )),
            "invalid design tokens at `group.token`: `hex` #262a3b needs `components` [0.149, 0.165, 0.231]"
        );
        assert_eq!(
            rejection(&one_token(
                r##"{ "$type": "color", "$value": { "colorSpace": "srgb", "components": [0.149, 0.165, 0.231], "hex": "#262a3b", "name": "x" } }"##
            )),
            "invalid design tokens at `group.token`: unknown key `name` in a colour"
        );
        // Within the tolerance of 0.0005 it still matches.
        assert!(load_catalog(&one_token(
            r##"{ "$type": "color", "$value": { "colorSpace": "srgb", "components": [0.1494, 0.165, 0.2306], "hex": "#262a3b" } }"##
        ))
        .is_ok());
    }

    #[test]
    fn rejects_rem_and_fractional_px() {
        for value in [
            r#"{ "value": 4, "unit": "rem" }"#,
            r#"{ "value": 64.5, "unit": "px" }"#,
            r#"{ "value": -1, "unit": "px" }"#,
            r#""64px""#,
        ] {
            assert_eq!(
                rejection(&one_token(&format!(
                    r#"{{ "$type": "dimension", "$value": {value} }}"#
                ))),
                r#"invalid design tokens at `group.token`: a dimension must be whole pixels, like { "value": 64, "unit": "px" }"#,
                "{value}"
            );
        }
    }

    #[test]
    fn rejects_seconds_duration() {
        for value in [
            r#"{ "value": 0.2, "unit": "s" }"#,
            r#"{ "value": 1, "unit": "s" }"#,
            r#"{ "value": 200.5, "unit": "ms" }"#,
        ] {
            assert_eq!(
                rejection(&one_token(&format!(
                    r#"{{ "$type": "duration", "$value": {value} }}"#
                ))),
                r#"invalid design tokens at `group.token`: a duration must be whole milliseconds, like { "value": 200, "unit": "ms" }"#,
                "{value}"
            );
        }
    }

    // --- The embedded catalog: its tokens, safety floors and contrast ---

    /// The four tokens no template may change: `Quit` stays recognisable, a
    /// target never shrinks below a finger, and the font is the system one.
    const FIXED_TOKENS: [&str; 4] = [
        "color.quit.background",
        "color.quit.text",
        "size.touch.min",
        "font.family.label",
    ];

    fn catalog() -> ThemeCatalog {
        embedded_catalog().expect("design/tokens.json loads")
    }

    fn spec<'a>(catalog: &'a ThemeCatalog, path: &str) -> &'a TokenSpec {
        catalog
            .spec(path)
            .unwrap_or_else(|| panic!("`{path}` is not in design/tokens.json"))
    }

    fn px(catalog: &ThemeCatalog, path: &str) -> u16 {
        match spec(catalog, path).default {
            TokenValue::Px(n) => n,
            ref other => panic!("`{path}` is not a dimension: {other:?}"),
        }
    }

    fn range(catalog: &ThemeCatalog, path: &str) -> (u16, u16) {
        match spec(catalog, path).rule {
            TokenRule::Range { min, max } => (min, max),
            TokenRule::Any => panic!("`{path}` has no range"),
        }
    }

    fn color(catalog: &ThemeCatalog, path: &str) -> Srgb8 {
        match spec(catalog, path).default {
            TokenValue::Color(c) => c,
            ref other => panic!("`{path}` is not a colour: {other:?}"),
        }
    }

    /// A catalog entry as the test writes it: path, default as CSS, whether a
    /// template may change it, and its range in the token's own unit.
    type Entry = (String, String, bool, Option<(u16, u16)>);

    #[test]
    fn embedded_catalog_has_the_25_tokens() {
        let expected: [(&str, &str, bool, _); 25] = [
            ("color.panel.background", "#11121a", true, None),
            ("color.sidebar.background", "#191b26", true, None),
            ("color.button.background", "#262a3b", true, None),
            ("color.button.border", "#626a90", true, None),
            ("color.button.text", "#e6e8f0", true, None),
            ("color.button.pressed-background", "#343a55", true, None),
            ("color.button.toggled-background", "#e0b05c", true, None),
            ("color.button.toggled-text", "#15161e", true, None),
            ("color.quit.background", "#b23a48", false, None),
            ("color.quit.text", "#ffffff", false, None),
            ("size.sidebar.width", "128px", true, Some((96, 240))),
            ("size.button.min", "64px", true, Some((44, 96))),
            ("size.button.border-width", "1px", true, Some((0, 4))),
            ("size.touch.min", "44px", false, None),
            ("radius.button", "10px", true, Some((0, 24))),
            ("space.page.padding", "12px", true, Some((0, 24))),
            ("space.page.gap", "8px", true, Some((4, 16))),
            ("space.sidebar.padding", "8px", true, Some((0, 16))),
            ("space.sidebar.gap", "8px", true, Some((4, 16))),
            ("font.size.label", "16px", true, Some((12, 28))),
            ("font.weight.label", "600", true, Some((300, 900))),
            ("font.line-height.label", "1.2", true, Some((100, 160))),
            (
                "font.family.label",
                "system-ui, Segoe UI, Roboto, sans-serif",
                false,
                None,
            ),
            ("motion.page-transition", "200ms", true, Some((0, 400))),
            ("motion.press-feedback", "80ms", true, Some((0, 200))),
        ];
        let catalog = catalog();
        let mut actual: Vec<Entry> = catalog
            .specs()
            .iter()
            .map(|s| {
                let range = match s.rule {
                    TokenRule::Range { min, max } => Some((min, max)),
                    TokenRule::Any => None,
                };
                (
                    s.path.to_string(),
                    s.default.to_string(),
                    s.themeable,
                    range,
                )
            })
            .collect();
        let mut expected: Vec<Entry> = expected
            .iter()
            .map(|(p, v, t, r)| (p.to_string(), v.to_string(), *t, *r))
            .collect();
        actual.sort();
        expected.sort();
        assert_eq!(actual, expected);
    }

    #[test]
    fn fixed_tokens_are_not_themeable() {
        let catalog = catalog();
        for path in FIXED_TOKENS {
            assert!(!spec(&catalog, path).themeable, "`{path}` must be fixed");
        }
        for s in catalog.specs() {
            if !FIXED_TOKENS.contains(&s.path.as_str()) {
                assert!(s.themeable, "`{}` should be themeable", s.path);
            }
        }
    }

    /// The pairs of tokens that would share one CSS custom property.
    fn css_name_collisions(paths: &[TokenPath]) -> Vec<(String, String)> {
        let mut seen: std::collections::BTreeMap<String, &TokenPath> = Default::default();
        let mut collisions = Vec::new();
        for path in paths {
            if let Some(other) = seen.insert(css_variable(path), path) {
                collisions.push((other.to_string(), path.to_string()));
            }
        }
        collisions
    }

    #[test]
    fn css_variable_names_are_unique() {
        let paths: Vec<TokenPath> = catalog().specs().iter().map(|s| s.path.clone()).collect();
        assert_eq!(paths.len(), 25);
        assert_eq!(css_name_collisions(&paths), vec![]);
        // `a.b-c` and `a.b.c` are two tokens but one `--plinth-a-b-c`.
        let clash = [
            TokenPath::new("a.b-c").unwrap(),
            TokenPath::new("a.b.c").unwrap(),
        ];
        assert_eq!(
            css_name_collisions(&clash),
            vec![("a.b-c".to_string(), "a.b.c".to_string())]
        );
    }

    #[test]
    fn button_min_never_goes_below_the_touch_floor() {
        let catalog = catalog();
        let touch = px(&catalog, "size.touch.min");
        assert_eq!(touch, 44, "WCAG 2.5.5 target size");
        let (min, _) = range(&catalog, "size.button.min");
        assert!(min >= touch, "size.button.min can go down to {min}px");
    }

    #[test]
    fn sidebar_width_always_leaves_a_touch_target() {
        let catalog = catalog();
        let (narrowest, _) = range(&catalog, "size.sidebar.width");
        let (_, widest_padding) = range(&catalog, "space.sidebar.padding");
        let touch = px(&catalog, "size.touch.min");
        // 96 - 2 * 16 = 64 >= 44
        assert!(
            narrowest.saturating_sub(2 * widest_padding) >= touch,
            "a {narrowest}px sidebar with {widest_padding}px padding leaves less than {touch}px"
        );
    }

    #[test]
    fn gaps_never_merge_targets() {
        // Two targets closer than this read as one under a finger.
        const MIN_GAP_PX: u16 = 4;
        let catalog = catalog();
        for path in ["space.page.gap", "space.sidebar.gap"] {
            let (min, _) = range(&catalog, path);
            assert!(min >= MIN_GAP_PX, "`{path}` can go down to {min}px");
        }
    }

    #[test]
    fn every_themeable_scalar_token_has_a_range() {
        for s in catalog().specs() {
            let scalar = !matches!(s.default.kind(), TokenKind::Color | TokenKind::FontFamily);
            if s.themeable && scalar {
                assert!(
                    matches!(s.rule, TokenRule::Range { .. }),
                    "`{}` can be themed to any value",
                    s.path
                );
            }
        }
    }

    /// The WCAG 2.x relative luminance of an sRGB colour.
    fn luminance(c: Srgb8) -> f64 {
        let linear = |byte: u8| {
            let v = f64::from(byte) / 255.0;
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(c.r) + 0.7152 * linear(c.g) + 0.0722 * linear(c.b)
    }

    /// The WCAG 2.x contrast ratio, from 1 to 21.
    fn contrast(a: Srgb8, b: Srgb8) -> f64 {
        let (la, lb) = (luminance(a), luminance(b));
        (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }

    fn assert_contrast(catalog: &ThemeCatalog, fg: &str, bg: &str, floor: f64) {
        let ratio = contrast(color(catalog, fg), color(catalog, bg));
        assert!(
            ratio >= floor,
            "`{fg}` on `{bg}` is {ratio:.2}:1, below {floor}:1"
        );
    }

    #[test]
    fn contrast_follows_the_wcag_formula() {
        let black = Srgb8 { r: 0, g: 0, b: 0 };
        let white = Srgb8 {
            r: 255,
            g: 255,
            b: 255,
        };
        assert!((contrast(black, white) - 21.0).abs() < 1e-9);
        assert!((contrast(white, white) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn default_text_pairs_contrast() {
        let catalog = catalog();
        for (fg, bg) in [
            ("color.button.text", "color.button.background"),
            ("color.button.text", "color.button.pressed-background"),
            (
                "color.button.toggled-text",
                "color.button.toggled-background",
            ),
            ("color.quit.text", "color.quit.background"),
        ] {
            assert_contrast(&catalog, fg, bg, 4.5);
        }
    }

    #[test]
    fn default_borders_contrast() {
        let catalog = catalog();
        for bg in ["color.panel.background", "color.sidebar.background"] {
            // WCAG 1.4.11, non-text contrast.
            assert_contrast(&catalog, "color.button.border", bg, 3.0);
        }
    }

    #[test]
    fn layout_tokens_exist_in_tokens_json() {
        let catalog = embedded_catalog().unwrap();
        for path in plinth_domain::LAYOUT_TOKENS {
            let spec = catalog
                .spec(path)
                .unwrap_or_else(|| panic!("`{path}` is missing from tokens.json"));
            assert_eq!(
                spec.default.kind(),
                TokenKind::Dimension,
                "`{path}` must be a dimension"
            );
        }
    }

    // --- The theming guide must not drift from the catalog ---

    /// The guide for template authors, checked against the embedded catalog.
    const THEMING_DOC: &str = include_str!("../../../docs/theming.md");

    /// The header of the guide's token table.
    const TOKEN_TABLE_HEADER: &str = "| token | type | allowed |";

    /// The rows of the guide's token table, as `token -> (type, allowed)`,
    /// with the backticks around the cells dropped.
    fn theming_doc_rows() -> std::collections::BTreeMap<String, (String, String)> {
        let mut lines = THEMING_DOC
            .lines()
            .skip_while(|line| line.trim() != TOKEN_TABLE_HEADER);
        assert!(
            lines.next().is_some(),
            "docs/theming.md has no `{TOKEN_TABLE_HEADER}` table"
        );
        let mut rows = std::collections::BTreeMap::new();
        for line in lines.skip(1).take_while(|line| line.starts_with('|')) {
            let cells: Vec<String> = line
                .trim()
                .trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().trim_matches('`').to_string())
                .collect();
            let [token, kind, allowed] = <[String; 3]>::try_from(cells)
                .unwrap_or_else(|cells| panic!("a row needs three cells: {cells:?}"));
            assert!(
                rows.insert(token.clone(), (kind, allowed)).is_none(),
                "`{token}` is listed twice"
            );
        }
        rows
    }

    /// The token's `$type`, as `tokens.json` spells it.
    fn dtcg_type(kind: TokenKind) -> &'static str {
        match kind {
            TokenKind::Color => "color",
            TokenKind::Dimension => "dimension",
            TokenKind::Duration => "duration",
            TokenKind::FontWeight => "fontWeight",
            TokenKind::Number => "number",
            TokenKind::FontFamily => "fontFamily",
        }
    }

    /// What the guide must say a template may write for `spec`.
    fn allowed(spec: &TokenSpec) -> String {
        if !spec.themeable {
            return "fixed".to_string();
        }
        let in_unit = |n: u16| match spec.default {
            TokenValue::Px(_) => TokenValue::Px(n).to_string(),
            TokenValue::Ms(_) => TokenValue::Ms(n).to_string(),
            TokenValue::Number(_) => Hundredths(n).to_string(),
            _ => n.to_string(),
        };
        match (spec.default.kind(), spec.rule) {
            (TokenKind::Color, _) => "#rrggbb".to_string(),
            (_, TokenRule::Range { min, max }) => format!("{} to {}", in_unit(min), in_unit(max)),
            (_, TokenRule::Any) => panic!("`{}` is themeable without a range", spec.path),
        }
    }

    #[test]
    fn theming_doc_lists_every_themeable_token() {
        let rows = theming_doc_rows();
        let catalog = catalog();
        let themeable: Vec<&TokenSpec> = catalog.specs().iter().filter(|s| s.themeable).collect();
        assert_eq!(themeable.len(), 21);
        for spec in &themeable {
            let expected = (dtcg_type(spec.default.kind()).to_string(), allowed(spec));
            assert_eq!(
                rows.get(spec.path.as_str()),
                Some(&expected),
                "docs/theming.md must list `{}` as {expected:?}",
                spec.path
            );
        }
        for token in rows.keys() {
            assert!(
                catalog.spec(token).is_some(),
                "docs/theming.md lists `{token}`, which is not in design/tokens.json"
            );
        }
    }

    #[test]
    fn theming_doc_does_not_list_fixed_tokens_as_themeable() {
        let rows = theming_doc_rows();
        let catalog = catalog();
        for path in FIXED_TOKENS {
            let kind = dtcg_type(spec(&catalog, path).default.kind()).to_string();
            assert_eq!(
                rows.get(path),
                Some(&(kind, "fixed".to_string())),
                "docs/theming.md must list `{path}` as fixed"
            );
        }
        let fixed_rows: Vec<&String> = rows
            .iter()
            .filter(|(_, (_, allowed))| allowed == "fixed")
            .map(|(token, _)| token)
            .collect();
        assert_eq!(fixed_rows, {
            let mut fixed: Vec<&str> = FIXED_TOKENS.to_vec();
            fixed.sort();
            fixed
        });
    }

    /// The heading of the guide's section on a button's own `style`.
    const STYLE_SECTION: &str = "## Per-button style";

    /// The tokens the guide's `style` section lists, one `- `token`` bullet
    /// each, in the order it lists them.
    fn theming_doc_style_tokens() -> Vec<String> {
        let mut lines = THEMING_DOC
            .lines()
            .skip_while(|line| line.trim() != STYLE_SECTION);
        assert!(
            lines.next().is_some(),
            "docs/theming.md has no `{STYLE_SECTION}` section"
        );
        lines
            .take_while(|line| !line.starts_with("## "))
            .filter_map(|line| line.strip_prefix("- `"))
            .filter_map(|rest| rest.split('`').next())
            .map(str::to_string)
            .collect()
    }

    #[test]
    fn theming_doc_lists_the_four_style_tokens() {
        let listed = theming_doc_style_tokens();
        assert_eq!(listed, plinth_domain::STYLE_TOKENS.to_vec());
        // Each one a colour a template may change, so a style takes it like
        // a theme does.
        let catalog = catalog();
        for path in plinth_domain::STYLE_TOKENS {
            let spec = spec(&catalog, path);
            assert!(spec.themeable, "`{path}` must be themeable");
            assert_eq!(spec.default.kind(), TokenKind::Color, "`{path}`");
        }
    }
}
