//! `TemplateSource` on the filesystem: YAML → shape validation (JSON Schema) →
//! mapping to the domain aggregate by consulting the catalog.
//!
//! The **three checks in their three places**:
//! 1. *Shape* → here, against the versioned JSON Schema (clear errors).
//! 2. *Catalog membership* → here, but the **authority is the domain**: every
//!    action string goes through `ActionCatalog::resolve`.
//! 3. *Business invariants* → in the domain constructors (`Template`).
//!
//! The optional `theme` follows the same path: the schema checks its shape,
//! each value is parsed to the kind its token has in the `ThemeCatalog`, and
//! the catalog decides what a template may change.

use std::path::Path;

use plinth_application::{TemplateError, TemplateSource};
use plinth_domain::{
    Action, ActionCatalog, Component, ComponentId, ComponentPosition, DomainError, KeyMacro, Page,
    SchemaVersion, Sidebar, SidebarSide, Template, TemplateMetadata, Theme, ThemeCatalog,
    TokenKind, TokenValue, VisualProps,
};
use serde_json::Value;

use crate::design_tokens::embedded_catalog;
use crate::key_names::parse_chord;
use crate::strict_yaml;
use crate::theme_scalars::parse_hex_color;

/// The JSON Schema is embedded in the binary so the adapter is self-contained.
const SCHEMA_SRC: &str = include_str!("../../../schema/plinth.template.schema.json");

pub struct FsTemplateSource {
    validator: jsonschema::Validator,
    catalog: ActionCatalog,
    theme_catalog: ThemeCatalog,
}

impl FsTemplateSource {
    pub fn new(catalog: ActionCatalog) -> Result<Self, TemplateError> {
        let schema: Value = serde_json::from_str(SCHEMA_SRC)
            .map_err(|e| TemplateError(format!("invalid schema: {e}")))?;
        let validator = jsonschema::validator_for(&schema)
            .map_err(|e| TemplateError(format!("could not compile the schema: {e}")))?;
        let theme_catalog = embedded_catalog()?;
        Ok(Self {
            validator,
            catalog,
            theme_catalog,
        })
    }

    /// Parses + validates + maps a YAML string (useful for tests and in-memory
    /// sources).
    pub fn load_str(&self, yaml: &str) -> Result<Template, TemplateError> {
        // A key written twice is refused, wherever it is.
        let doc = strict_yaml::from_str(yaml)?;

        // Before the shape: an unquoted `#` makes the value a comment, so the
        // author gets a hint instead of a schema error about `null`.
        reject_empty_theme_values(&doc)?;

        // Check 1: shape.
        if let Err(error) = self.validator.validate(&doc) {
            return Err(TemplateError(format!(
                "the template does not satisfy the schema at `{}`: {}",
                error.instance_path(),
                error
            )));
        }

        // Checks 2 and 3: catalog + invariants, via the mapping.
        self.map_template(&doc)
    }

    fn map_template(&self, doc: &Value) -> Result<Template, TemplateError> {
        let schema_version = str_field(doc, "schema_version")?;
        let meta_node = doc
            .get("metadata")
            .ok_or_else(|| TemplateError("missing `metadata`".into()))?;
        let metadata = TemplateMetadata {
            game: str_field(meta_node, "game")?,
            version: str_field(meta_node, "version")?,
            author: str_field(meta_node, "author")?,
            schema_version: SchemaVersion(schema_version),
        };

        let pages_node = doc
            .get("pages")
            .and_then(Value::as_array)
            .ok_or_else(|| TemplateError("missing `pages`".into()))?;
        let mut pages = Vec::with_capacity(pages_node.len());
        for page_node in pages_node {
            let comps = page_node
                .get("components")
                .and_then(Value::as_array)
                .ok_or_else(|| TemplateError("a page has no `components`".into()))?;
            let components = self.map_components(comps)?;
            pages.push(Page::new(components).map_err(domain_err)?);
        }

        let sidebar_node = doc
            .get("sidebar")
            .ok_or_else(|| TemplateError("missing `sidebar`".into()))?;
        let default_side = match str_field(sidebar_node, "default_side")?.as_str() {
            "left" => SidebarSide::Left,
            "right" => SidebarSide::Right,
            other => return Err(TemplateError(format!("invalid sidebar side: {other}"))),
        };
        let sidebar_comps = sidebar_node
            .get("components")
            .and_then(Value::as_array)
            .map(|c| self.map_components(c))
            .transpose()?
            .unwrap_or_default();
        let sidebar = Sidebar {
            default_side,
            components: sidebar_comps,
        };

        let theme = self.map_theme(doc.get("theme"))?;
        Template::new(metadata, pages, sidebar)
            .map(|template| template.with_theme(theme))
            .map_err(domain_err)
    }

    /// The template's `theme` resolved against the catalog; no `theme` means
    /// the defaults. Each value is parsed to its token's kind, and the catalog
    /// has the last word on what may change.
    pub fn map_theme(&self, node: Option<&Value>) -> Result<Theme, TemplateError> {
        let Some(node) = node else {
            return Ok(self.theme_catalog.defaults());
        };
        let entries = node
            .as_object()
            .ok_or_else(|| TemplateError("invalid theme: `theme` must be a map".into()))?;
        let mut overrides = Vec::with_capacity(entries.len());
        for (name, raw) in entries {
            let spec = self
                .theme_catalog
                .spec(name)
                .ok_or_else(|| theme_err(DomainError::UnknownToken(name.clone())))?;
            let expected = spec.default.kind();
            let value = parse_scalar(expected, raw).ok_or_else(|| {
                theme_err(DomainError::TokenKindMismatch {
                    token: name.clone(),
                    expected,
                })
            })?;
            overrides.push((name.clone(), value));
        }
        self.theme_catalog.resolve(overrides).map_err(theme_err)
    }

    fn map_components(&self, nodes: &[Value]) -> Result<Vec<Component>, TemplateError> {
        nodes.iter().map(|n| self.map_component(n)).collect()
    }

    fn map_component(&self, node: &Value) -> Result<Component, TemplateError> {
        let id = ComponentId::new(str_field(node, "id")?);
        let pos = node
            .get("position")
            .ok_or_else(|| TemplateError("missing `position`".into()))?;
        let position = ComponentPosition {
            row: u16_field(pos, "row")?,
            col: u16_field(pos, "col")?,
        };
        let visual = VisualProps {
            label: opt_str(node, "label"),
            icon: opt_str(node, "icon"),
        };
        let action = match node.get("keys") {
            Some(keys) => Action::Keys(map_keys(keys)?),
            None => {
                // Check 2: the legality authority is the domain.
                let action_raw = str_field(node, "action")?;
                self.catalog.resolve(&action_raw).map_err(domain_err)?
            }
        };

        Ok(Component {
            id,
            position,
            visual,
            action,
        })
    }
}

impl TemplateSource for FsTemplateSource {
    fn load(&self, reference: &str) -> Result<Template, TemplateError> {
        let yaml = std::fs::read_to_string(Path::new(reference))
            .map_err(|e| TemplateError(format!("could not read `{reference}`: {e}")))?;
        self.load_str(&yaml)
    }
}

/// `keys` is one step (a text) or a list of steps; the schema has already
/// checked its shape, the domain checks its length.
fn map_keys(node: &Value) -> Result<KeyMacro, TemplateError> {
    let steps = match node {
        Value::String(step) => vec![parse_chord(step)?],
        Value::Array(items) => items
            .iter()
            .map(|item| {
                item.as_str()
                    .ok_or_else(|| TemplateError("a `keys` step must be text".into()))
                    .and_then(parse_chord)
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err(TemplateError("`keys` must be text or a list".into())),
    };
    KeyMacro::new(steps).map_err(domain_err)
}

fn domain_err(e: DomainError) -> TemplateError {
    TemplateError(e.to_string())
}

fn theme_err(e: DomainError) -> TemplateError {
    TemplateError(format!("invalid theme: {e}"))
}

/// A theme value written the one way its kind allows.
fn parse_scalar(kind: TokenKind, raw: &Value) -> Option<TokenValue> {
    match kind {
        TokenKind::Color => raw
            .as_str()
            .and_then(parse_hex_color)
            .map(TokenValue::Color),
    }
}

/// YAML reads `key: #1e1e2e` as `key:` and a comment, so the value is `null`.
fn reject_empty_theme_values(doc: &Value) -> Result<(), TemplateError> {
    let Some(entries) = doc.get("theme").and_then(Value::as_object) else {
        return Ok(());
    };
    match entries.iter().find(|(_, value)| value.is_null()) {
        Some((name, _)) => Err(TemplateError(format!(
            r##"invalid theme: `{name}` has no value; a colour must be quoted, like "#1e1e2e", because an unquoted # starts a YAML comment"##
        ))),
        None => Ok(()),
    }
}

fn str_field(node: &Value, key: &str) -> Result<String, TemplateError> {
    node.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| TemplateError(format!("missing text field `{key}`")))
}

fn opt_str(node: &Value, key: &str) -> Option<String> {
    node.get(key).and_then(Value::as_str).map(str::to_string)
}

fn u16_field(node: &Value, key: &str) -> Result<u16, TemplateError> {
    node.get(key)
        .and_then(Value::as_u64)
        .and_then(|n| u16::try_from(n).ok())
        .ok_or_else(|| TemplateError(format!("invalid numeric field `{key}`")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use plinth_domain::{ActionId, ScanCode};

    fn catalog() -> ActionCatalog {
        ActionCatalog::from_actions([ActionId::new("v_eject")])
    }

    const VALID: &str = r#"
schema_version: "1"
metadata:
  game: "Star Citizen"
  version: "4.0"
  author: "dev"
pages:
  - components:
      - id: eject
        position: { row: 0, col: 0 }
        action: "v_eject"
        label: "Eject"
sidebar:
  default_side: right
  components:
    - id: lock
      position: { row: 0, col: 0 }
      action: "app.toggle_lock"
"#;

    #[test]
    fn loads_valid_template() {
        let src = FsTemplateSource::new(catalog()).unwrap();
        let template = src.load_str(VALID).unwrap();
        assert_eq!(template.page_count(), 1);
        assert_eq!(template.metadata().game, "Star Citizen");
    }

    #[test]
    fn rejects_unknown_action_via_catalog() {
        let bad = VALID.replace("v_eject", "rm_rf_slash");
        let src = FsTemplateSource::new(catalog()).unwrap();
        let err = src.load_str(&bad).unwrap_err();
        assert!(
            err.0.contains("unknown action"),
            "expected UnknownAction: {}",
            err.0
        );
    }

    #[test]
    fn rejects_schema_violation() {
        // Missing `position` in a component.
        let bad = r#"
schema_version: "1"
metadata: { game: SC, version: "4.0", author: dev }
pages:
  - components:
      - id: eject
        action: "v_eject"
sidebar: { default_side: right, components: [] }
"#;
        let src = FsTemplateSource::new(catalog()).unwrap();
        assert!(src.load_str(bad).is_err());
    }
    /// A one-page template whose only component carries `component_body`
    /// (everything but `id` and `position`).
    fn with_component(component_body: &str) -> String {
        format!(
            r#"
schema_version: "1"
metadata: {{ game: WoW, version: "1", author: dev }}
pages:
  - components:
      - id: btn
        position: {{ row: 0, col: 0 }}
        {component_body}
sidebar: {{ default_side: right, components: [] }}
"#
        )
    }

    fn keys_of(template: &Template) -> KeyMacro {
        match &template.pages()[0].components()[0].action {
            Action::Keys(keys) => keys.clone(),
            other => panic!("expected Action::Keys, got {other:?}"),
        }
    }

    #[test]
    fn loads_keys_text_and_list() {
        let src = FsTemplateSource::new(catalog()).unwrap();

        let text = src.load_str(&with_component(r#"keys: "shift+1""#)).unwrap();
        let keys = keys_of(&text);
        let chord = &keys.steps()[0];
        assert_eq!(chord.modifiers(), &[ScanCode::new(42)]);
        assert_eq!(chord.key(), ScanCode::new(2));

        let list = src
            .load_str(&with_component(r#"keys: ["c", "b"]"#))
            .unwrap();
        let steps: Vec<_> = keys_of(&list).steps().iter().map(|c| c.key()).collect();
        assert_eq!(steps, vec![ScanCode::new(46), ScanCode::new(48)]);
    }

    #[test]
    fn rejects_action_and_keys_together() {
        let src = FsTemplateSource::new(catalog()).unwrap();
        let body = "action: \"v_eject\"\n        keys: \"m\"";
        let err = src.load_str(&with_component(body)).unwrap_err();
        assert!(err.0.contains("does not satisfy the schema"), "{}", err.0);
    }

    #[test]
    fn rejects_component_without_action_or_keys() {
        let src = FsTemplateSource::new(catalog()).unwrap();
        let err = src
            .load_str(&with_component(r#"label: "Nothing""#))
            .unwrap_err();
        assert!(err.0.contains("does not satisfy the schema"), "{}", err.0);
    }

    #[test]
    fn rejects_empty_keys_list() {
        let src = FsTemplateSource::new(catalog()).unwrap();
        let err = src.load_str(&with_component("keys: []")).unwrap_err();
        assert!(err.0.contains("does not satisfy the schema"), "{}", err.0);
    }

    #[test]
    fn rejects_empty_keys_text() {
        let src = FsTemplateSource::new(catalog()).unwrap();
        let err = src.load_str(&with_component(r#"keys: """#)).unwrap_err();
        assert!(err.0.contains("does not satisfy the schema"), "{}", err.0);
    }

    #[test]
    fn rejects_seventeen_keys() {
        let src = FsTemplateSource::new(catalog()).unwrap();
        let sixteen = format!("keys: [{}]", vec!["\"m\""; 16].join(", "));
        assert!(src.load_str(&with_component(&sixteen)).is_ok());
        let seventeen = format!("keys: [{}]", vec!["\"m\""; 17].join(", "));
        // The schema refuses it first; the domain limit backs it up.
        let err = src.load_str(&with_component(&seventeen)).unwrap_err();
        assert!(err.0.contains("does not satisfy the schema"), "{}", err.0);
    }

    #[test]
    fn rejects_unknown_key_name_in_a_template() {
        let src = FsTemplateSource::new(catalog()).unwrap();
        let err = src.load_str(&with_component(r#"keys: "up""#)).unwrap_err();
        assert!(err.0.contains("`up`"), "{}", err.0);
    }

    #[test]
    fn rejects_repeated_modifier_in_a_template() {
        let src = FsTemplateSource::new(catalog()).unwrap();
        let err = src
            .load_str(&with_component(r#"keys: "shift+shift+1""#))
            .unwrap_err();
        assert!(err.0.contains("twice"), "{}", err.0);
    }

    // --- Keys written twice ---

    #[test]
    fn rejects_a_duplicate_theme_key() {
        let src = FsTemplateSource::new(catalog()).unwrap();
        let err = src
            .load_str(&with_theme(&[
                r##"color.button.background: "#1e1e2e""##,
                r##"color.button.background: "#262a3b""##,
            ]))
            .unwrap_err();
        assert!(
            err.0
                .contains("theme: duplicate key `color.button.background`"),
            "{}",
            err.0
        );
    }

    #[test]
    fn rejects_a_duplicate_component_key() {
        let src = FsTemplateSource::new(catalog()).unwrap();
        let err = src
            .load_str(&with_component("id: again\n        action: \"v_eject\""))
            .unwrap_err();
        assert!(
            err.0.contains("pages[0].components[0]: duplicate key `id`"),
            "{}",
            err.0
        );
    }

    #[test]
    fn rejects_a_duplicate_root_key() {
        let src = FsTemplateSource::new(catalog()).unwrap();
        let twice = format!("{VALID}sidebar: {{ default_side: left, components: [] }}\n");
        let err = src.load_str(&twice).unwrap_err();
        assert!(err.0.starts_with("invalid YAML: "), "{}", err.0);
        assert!(err.0.contains("duplicate key `sidebar`"), "{}", err.0);
    }

    #[test]
    fn yaml_infinity_becomes_null() {
        let infinite = VALID.replacen("{ row: 0, col: 0 }", "{ row: .inf, col: 0 }", 1);
        let doc = strict_yaml::from_str(&infinite).unwrap();
        assert_eq!(
            doc["pages"][0]["components"][0]["position"]["row"],
            Value::Null
        );

        let src = FsTemplateSource::new(catalog()).unwrap();
        let err = src.load_str(&infinite).unwrap_err();
        assert!(
            err.0
                .contains("does not satisfy the schema at `/pages/0/components/0/position/row`"),
            "{}",
            err.0
        );
    }

    // --- The template's theme ---

    const BUTTON_BACKGROUND: &str = "color.button.background";

    /// `VALID` with a `theme` block of `entries` (one `key: value` per line).
    fn with_theme(entries: &[&str]) -> String {
        let block: String = entries.iter().map(|e| format!("  {e}\n")).collect();
        format!("{VALID}theme:\n{block}")
    }

    fn colour(r: u8, g: u8, b: u8) -> TokenValue {
        TokenValue::Color(plinth_domain::Srgb8 { r, g, b })
    }

    #[test]
    fn theme_override_reaches_the_template() {
        let src = FsTemplateSource::new(catalog()).unwrap();
        let template = src
            .load_str(&with_theme(&[r##"color.button.background: "#1e1e2e""##]))
            .unwrap();
        assert_eq!(
            template.theme().get(BUTTON_BACKGROUND),
            Some(&colour(30, 30, 46))
        );
    }

    #[test]
    fn template_without_theme_gets_the_defaults() {
        let src = FsTemplateSource::new(catalog()).unwrap();
        let template = src.load_str(VALID).unwrap();
        assert_eq!(
            template.theme().get(BUTTON_BACKGROUND),
            Some(&colour(38, 42, 59))
        );
        assert_eq!(template.theme(), &embedded_catalog().unwrap().defaults());
    }

    #[test]
    fn rejects_unknown_theme_token() {
        let src = FsTemplateSource::new(catalog()).unwrap();
        let err = src
            .load_str(&with_theme(&[r##"color.button.nope: "#1e1e2e""##]))
            .unwrap_err();
        assert_eq!(
            err.0,
            "invalid theme: unknown theme token `color.button.nope`"
        );
    }

    #[test]
    fn rejects_css_injection_in_a_colour() {
        let src = FsTemplateSource::new(catalog()).unwrap();
        let err = src
            .load_str(&with_theme(&[r#"color.button.background: "url(x)""#]))
            .unwrap_err();
        assert!(
            err.0
                .contains("does not satisfy the schema at `/theme/color.button.background`"),
            "{}",
            err.0
        );
    }

    #[test]
    fn unquoted_hex_gets_a_quoting_hint() {
        let src = FsTemplateSource::new(catalog()).unwrap();
        let err = src
            .load_str(&with_theme(&["color.button.background: #1e1e2e"]))
            .unwrap_err();
        assert_eq!(
            err.0,
            r##"invalid theme: `color.button.background` has no value; a colour must be quoted, like "#1e1e2e", because an unquoted # starts a YAML comment"##
        );
    }

    #[test]
    fn rejects_a_three_digit_hex() {
        let src = FsTemplateSource::new(catalog()).unwrap();
        let err = src
            .load_str(&with_theme(&[r##"color.button.background: "#fff""##]))
            .unwrap_err();
        assert_eq!(
            err.0,
            r##"invalid theme: theme token `color.button.background` expects a colour like "#1e1e2e""##
        );
    }
}
