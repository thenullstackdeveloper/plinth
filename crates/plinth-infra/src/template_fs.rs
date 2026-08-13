//! `TemplateSource` on the filesystem: YAML → shape validation (JSON Schema) →
//! mapping to the domain aggregate by consulting the catalog.
//!
//! The **three checks in their three places**:
//! 1. *Shape* → here, against the versioned JSON Schema (clear errors).
//! 2. *Catalog membership* → here, but the **authority is the domain**: every
//!    action string goes through `ActionCatalog::resolve`.
//! 3. *Business invariants* → in the domain constructors (`Template`).

use std::path::Path;

use plinth_application::{TemplateError, TemplateSource};
use plinth_domain::{
    ActionCatalog, Component, ComponentId, ComponentPosition, Page, SchemaVersion, Sidebar,
    SidebarSide, Template, TemplateMetadata, VisualProps,
};
use serde_json::Value;

/// The JSON Schema is embedded in the binary so the adapter is self-contained.
const SCHEMA_SRC: &str = include_str!("../../../schema/plinth.template.schema.json");

pub struct FsTemplateSource {
    validator: jsonschema::Validator,
    catalog: ActionCatalog,
}

impl FsTemplateSource {
    pub fn new(catalog: ActionCatalog) -> Result<Self, TemplateError> {
        let schema: Value = serde_json::from_str(SCHEMA_SRC)
            .map_err(|e| TemplateError(format!("invalid schema: {e}")))?;
        let validator = jsonschema::validator_for(&schema)
            .map_err(|e| TemplateError(format!("could not compile the schema: {e}")))?;
        Ok(Self { validator, catalog })
    }

    /// Parses + validates + maps a YAML string (useful for tests and in-memory
    /// sources).
    pub fn load_str(&self, yaml: &str) -> Result<Template, TemplateError> {
        let doc: Value = serde_yaml_ng::from_str(yaml)
            .map_err(|e| TemplateError(format!("invalid YAML: {e}")))?;

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

        Template::new(metadata, pages, sidebar).map_err(domain_err)
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
        // Check 2: the legality authority is the domain.
        let action_raw = str_field(node, "action")?;
        let action = self.catalog.resolve(&action_raw).map_err(domain_err)?;

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

fn domain_err(e: plinth_domain::DomainError) -> TemplateError {
    TemplateError(e.to_string())
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
    use plinth_domain::ActionId;

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
}
