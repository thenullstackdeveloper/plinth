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
//!
//! An `image`, an `icon` or a page `background` is read from the template's
//! folder with the safe read of `TemplateAssets`, checked by its header
//! (`image_check`) against its extension and the limits of its use, and kept
//! once per file. A template brings at most 64 distinct image files and
//! 16 MiB between them.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use plinth_application::{TemplateError, TemplateSource};
use plinth_domain::{
    Action, ActionCatalog, AppAction, Component, ComponentId, ComponentPosition, DomainError,
    ImageAsset, ImagePath, ImageRef, ImageUse, KeyMacro, Page, SchemaVersion, Sidebar, SidebarSide,
    Template, TemplateMetadata, Theme, ThemeCatalog, TokenKind, TokenValue, VisualProps,
};
use serde_json::Value;

use crate::design_tokens::embedded_catalog;
use crate::image_check::{inspect, ImageError};
use crate::key_names::parse_chord;
use crate::strict_yaml;
use crate::template_dir::{TemplateAssets, TemplatePackage};
use crate::theme_scalars::{
    parse_font_weight, parse_hex_color, parse_hundredths, parse_ms, parse_px,
};

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
    /// sources). It is a single-file template, so it cannot reference images.
    pub fn load_str(&self, yaml: &str) -> Result<Template, TemplateError> {
        self.load_package(&TemplatePackage::single_file(yaml))
    }

    /// Parses + validates + maps a template and reads the images it
    /// references from its folder.
    pub fn load_package(&self, package: &TemplatePackage) -> Result<Template, TemplateError> {
        // A key written twice is refused, wherever it is.
        let doc = strict_yaml::from_str(&package.yaml)?;

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
        let mut images = ImageLoader::new(&package.assets);
        self.map_template(&doc, &mut images)
    }

    fn map_template(
        &self,
        doc: &Value,
        images: &mut ImageLoader<'_>,
    ) -> Result<Template, TemplateError> {
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

        // The default background first, then each page's own: the images
        // are numbered in the order they are read.
        let background = map_background(doc, images)?;
        let pages_node = doc
            .get("pages")
            .and_then(Value::as_array)
            .ok_or_else(|| TemplateError("missing `pages`".into()))?;
        let mut pages = Vec::with_capacity(pages_node.len());
        for page_node in pages_node {
            let own = map_background(page_node, images)?;
            let comps = page_node
                .get("components")
                .and_then(Value::as_array)
                .ok_or_else(|| TemplateError("a page has no `components`".into()))?;
            let components = self.map_components(comps, images)?;
            pages.push(
                Page::new(components)
                    .map_err(domain_err)?
                    .with_background(own),
            );
        }

        let sidebar_node = doc
            .get("sidebar")
            .ok_or_else(|| TemplateError("missing `sidebar`".into()))?;
        let default_side = match str_field(sidebar_node, "default_side")?.as_str() {
            "left" => SidebarSide::Left,
            "right" => SidebarSide::Right,
            other => return Err(TemplateError(format!("invalid sidebar side: {other}"))),
        };
        let sidebar_nodes = sidebar_node
            .get("components")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default();
        // The domain refuses `image` in the sidebar; `style` does not reach
        // it yet, so it is refused here.
        for node in sidebar_nodes {
            if node.get("style").is_some() {
                return Err(domain_err(DomainError::NotAllowedInSidebar {
                    field: "style",
                    component: str_field(node, "id")?,
                }));
            }
        }
        let sidebar_comps = self.map_components(sidebar_nodes, images)?;
        let sidebar = Sidebar {
            default_side,
            components: sidebar_comps,
        };

        let theme = self.map_theme(doc.get("theme"))?;
        Template::new(metadata, pages, sidebar)
            .and_then(|template| {
                template
                    .with_theme(theme)
                    .with_background(background)
                    .with_images(images.finish())
            })
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
        // Per entry, in this order: the token exists, a template may change
        // it, the value parses to its kind; `resolve` then checks the range.
        let mut overrides = Vec::with_capacity(entries.len());
        for (name, raw) in entries {
            let spec = self
                .theme_catalog
                .spec(name)
                .ok_or_else(|| theme_err(DomainError::UnknownToken(name.clone())))?;
            if !spec.themeable {
                return Err(theme_err(DomainError::TokenNotThemeable(name.clone())));
            }
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

    fn map_components(
        &self,
        nodes: &[Value],
        images: &mut ImageLoader<'_>,
    ) -> Result<Vec<Component>, TemplateError> {
        nodes
            .iter()
            .map(|n| self.map_component(n, images))
            .collect()
    }

    fn map_component(
        &self,
        node: &Value,
        images: &mut ImageLoader<'_>,
    ) -> Result<Component, TemplateError> {
        let id = ComponentId::new(str_field(node, "id")?);
        let pos = node
            .get("position")
            .ok_or_else(|| TemplateError("missing `position`".into()))?;
        let position = ComponentPosition {
            row: u16_field(pos, "row")?,
            col: u16_field(pos, "col")?,
        };
        let action = match node.get("keys") {
            Some(keys) => Action::Keys(map_keys(keys)?),
            None => {
                // Check 2: the legality authority is the domain.
                let action_raw = str_field(node, "action")?;
                self.catalog.resolve(&action_raw).map_err(domain_err)?
            }
        };
        // The domain refuses the other looks on the quit button; `style`
        // does not reach it yet, so it is refused here.
        if matches!(action, Action::App(AppAction::Quit)) && node.get("style").is_some() {
            return Err(domain_err(DomainError::NotAllowedOnQuit { field: "style" }));
        }
        let image = opt_str(node, "image")
            .map(|raw| images.load(&raw, ImageUse::ButtonImage))
            .transpose()?;
        let icon = opt_str(node, "icon")
            .map(|raw| images.load(&raw, ImageUse::Icon))
            .transpose()?;
        let visual = VisualProps {
            label: opt_str(node, "label"),
            icon,
            image,
            hide_label: node
                .get("hide_label")
                .and_then(Value::as_bool)
                .unwrap_or(false),
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

/// The most distinct image files a template may bring, and the most bytes
/// between them. `template.yaml` does not count.
const MAX_TEMPLATE_IMAGES: usize = 64;
const MAX_TEMPLATE_IMAGE_BYTES: u64 = 16 * 1024 * 1024;

/// The images of one template, read as its components reference them: each
/// file once, in order of first appearance, so its index is its `ImageRef`.
/// A file is known by its canonical path, so two spellings of it (another
/// case, on a disk that ignores it) are one image, and it counts once
/// towards the template's limits.
struct ImageLoader<'a> {
    assets: &'a TemplateAssets,
    images: Vec<ImageAsset>,
    bytes: u64,
    by_path: HashMap<ImagePath, usize>,
    by_file: HashMap<PathBuf, usize>,
}

impl<'a> ImageLoader<'a> {
    fn new(assets: &'a TemplateAssets) -> Self {
        Self {
            assets,
            images: Vec::new(),
            bytes: 0,
            by_path: HashMap::new(),
            by_file: HashMap::new(),
        }
    }

    /// The image at `raw` for `use_`: read, checked against its extension
    /// and the limits of `use_`, or the one already read for that file,
    /// checked against the limits of `use_` too.
    fn load(&mut self, raw: &str, use_: ImageUse) -> Result<ImageRef, TemplateError> {
        let path = ImagePath::parse(raw).map_err(domain_err)?;
        if let Some(&index) = self.by_path.get(&path) {
            return self.reuse(index, use_);
        }
        // The safe read goes first: a link is refused before its target can
        // be recognised as a file already read.
        let bytes = self.assets.read(&path, use_.limits().max_bytes)?;
        let file = self.canonical(&path)?;
        if let Some(&index) = self.by_file.get(&file) {
            self.by_path.insert(path, index);
            return self.reuse(index, use_);
        }
        let shown = path.as_str();
        let info = inspect(&bytes).map_err(|e| match e {
            // Bytes in no format Plinth reads are not what the extension says.
            ImageError::NotAnImage => format_mismatch(&path),
            ImageError::Malformed(reason) => {
                TemplateError(format!("`{shown}` is not supported: {reason}"))
            }
        })?;
        if info.kind != path.kind() {
            return Err(format_mismatch(&path));
        }
        let asset = ImageAsset {
            path: path.clone(),
            kind: info.kind,
            width: info.width,
            height: info.height,
            bytes: bytes.into(),
        };
        asset.check(use_).map_err(domain_err)?;
        let files = self.images.len() + 1;
        let total = self
            .bytes
            .saturating_add(u64::try_from(asset.bytes.len()).unwrap_or(u64::MAX));
        if files > MAX_TEMPLATE_IMAGES || total > MAX_TEMPLATE_IMAGE_BYTES {
            return Err(domain_err(DomainError::TemplateAssetsTooLarge {
                files,
                bytes: total,
            }));
        }
        let index = self.images.len();
        self.images.push(asset);
        self.bytes = total;
        self.by_path.insert(path, index);
        self.by_file.insert(file, index);
        Ok(ImageRef(index))
    }

    /// An image already read, used again: it has to fit `use_` as well.
    fn reuse(&self, index: usize, use_: ImageUse) -> Result<ImageRef, TemplateError> {
        self.images[index].check(use_).map_err(domain_err)?;
        Ok(ImageRef(index))
    }

    /// The canonical path of an image the safe read has just accepted.
    fn canonical(&self, path: &ImagePath) -> Result<PathBuf, TemplateError> {
        let TemplateAssets::Folder(root) = self.assets else {
            return Err(TemplateError(format!(
                "`{}` has no template folder",
                path.as_str()
            )));
        };
        let mut full = root.clone();
        full.extend(path.parts());
        full.canonicalize()
            .map_err(|e| TemplateError(format!("`{}` could not be read: {e}", path.as_str())))
    }

    fn finish(&mut self) -> Vec<ImageAsset> {
        std::mem::take(&mut self.images)
    }
}

/// The `background` of `node` (the root or a page), read with the limits of
/// a background, or `None` if it has none.
fn map_background(
    node: &Value,
    images: &mut ImageLoader<'_>,
) -> Result<Option<ImageRef>, TemplateError> {
    node.get("background")
        .and_then(|background| background.get("image"))
        .and_then(Value::as_str)
        .map(|raw| images.load(raw, ImageUse::Background))
        .transpose()
}

fn format_mismatch(path: &ImagePath) -> TemplateError {
    TemplateError(format!(
        "`{}` is not a {} file",
        path.as_str(),
        path.kind().name()
    ))
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

/// A theme value written the one way its kind allows. A font family is never
/// written in a template: its only token is fixed.
fn parse_scalar(kind: TokenKind, raw: &Value) -> Option<TokenValue> {
    match kind {
        TokenKind::Color => raw
            .as_str()
            .and_then(parse_hex_color)
            .map(TokenValue::Color),
        TokenKind::Dimension => raw.as_str().and_then(parse_px).map(TokenValue::Px),
        TokenKind::Duration => raw.as_str().and_then(parse_ms).map(TokenValue::Ms),
        TokenKind::FontWeight => parse_font_weight(raw).map(TokenValue::FontWeight),
        TokenKind::Number => parse_hundredths(raw).map(TokenValue::Number),
        TokenKind::FontFamily => None,
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

    // --- Every kind a template can override ---

    #[test]
    fn template_override_of_each_kind_loads() {
        use plinth_domain::Hundredths;

        let src = FsTemplateSource::new(catalog()).unwrap();
        let template = src
            .load_str(&with_theme(&[
                r##"color.button.text: "#FFFFFF""##,
                r#"size.button.min: "72px""#,
                r#"motion.page-transition: "150ms""#,
                "font.weight.label: 700",
                "font.line-height.label: 1.35",
            ]))
            .unwrap();
        let theme = template.theme();
        assert_eq!(theme.get("color.button.text"), Some(&colour(255, 255, 255)));
        assert_eq!(theme.get("size.button.min"), Some(&TokenValue::Px(72)));
        assert_eq!(
            theme.get("motion.page-transition"),
            Some(&TokenValue::Ms(150))
        );
        assert_eq!(
            theme.get("font.weight.label"),
            Some(&TokenValue::FontWeight(700))
        );
        assert_eq!(
            theme.get("font.line-height.label"),
            Some(&TokenValue::Number(Hundredths(135)))
        );

        let alias = src
            .load_str(&with_theme(&[r#"font.weight.label: "semi-bold""#]))
            .unwrap();
        assert_eq!(
            alias.theme().get("font.weight.label"),
            Some(&TokenValue::FontWeight(600))
        );
    }

    fn theme_rejection(entries: &[&str]) -> String {
        let src = FsTemplateSource::new(catalog()).unwrap();
        src.load_str(&with_theme(entries)).unwrap_err().0
    }

    #[test]
    fn rejects_out_of_range_dimension() {
        assert_eq!(
            theme_rejection(&[r#"size.button.min: "40px""#]),
            "invalid theme: theme token `size.button.min`: 40px is outside the allowed range 44px to 96px"
        );
        assert_eq!(
            theme_rejection(&[r#"size.sidebar.width: "241px""#]),
            "invalid theme: theme token `size.sidebar.width`: 241px is outside the allowed range 96px to 240px"
        );
    }

    #[test]
    fn rejects_rem() {
        assert_eq!(
            theme_rejection(&[r#"size.button.min: "4rem""#]),
            r#"invalid theme: theme token `size.button.min` expects a dimension in whole pixels, like "64px""#
        );
    }

    #[test]
    fn rejects_fixed_tokens() {
        assert_eq!(
            theme_rejection(&[r##"color.quit.background: "#000000""##]),
            "invalid theme: theme token `color.quit.background` is fixed and cannot be changed by a template"
        );
        assert_eq!(
            theme_rejection(&[r##"color.quit.text: "#b23a48""##]),
            "invalid theme: theme token `color.quit.text` is fixed and cannot be changed by a template"
        );
        // Fixed comes before kind: no value a template can write is a font
        // list, and the author still learns the token is fixed.
        assert_eq!(
            theme_rejection(&[r#"font.family.label: "Arial""#]),
            "invalid theme: theme token `font.family.label` is fixed and cannot be changed by a template"
        );
    }

    #[test]
    fn rejects_kind_mismatch() {
        assert_eq!(
            theme_rejection(&["size.button.min: 64"]),
            r#"invalid theme: theme token `size.button.min` expects a dimension in whole pixels, like "64px""#
        );
        assert_eq!(
            theme_rejection(&[r#"motion.page-transition: "0.2s""#]),
            r#"invalid theme: theme token `motion.page-transition` expects a duration in whole milliseconds, like "200ms""#
        );
        assert_eq!(
            theme_rejection(&[r#"font.weight.label: "Bold""#]),
            r#"invalid theme: theme token `font.weight.label` expects a font weight from 1 to 1000 or a name like "bold""#
        );
        assert_eq!(
            theme_rejection(&[r#"font.line-height.label: "1.2""#]),
            "invalid theme: theme token `font.line-height.label` expects a number with at most two decimals, like 1.2"
        );
        assert_eq!(
            theme_rejection(&[r#"color.button.text: "72px""#]),
            r##"invalid theme: theme token `color.button.text` expects a colour like "#1e1e2e""##
        );
    }

    #[test]
    fn rejects_spaces_semicolons_and_braces() {
        for value in [
            r#""64px 1px""#,
            r#""64px;color:red""#,
            r#""{64px}""#,
            r#""64px}""#,
        ] {
            let err = theme_rejection(&[&format!("size.button.min: {value}")]);
            assert!(
                err.contains("does not satisfy the schema at `/theme/size.button.min`"),
                "{value}: {err}"
            );
        }
    }

    #[test]
    fn rejects_more_than_32_entries() {
        let entries: Vec<String> = (0..33)
            .map(|i| format!("size.x{i}.min: \"64px\""))
            .collect();
        let refs: Vec<&str> = entries.iter().map(String::as_str).collect();
        let err = theme_rejection(&refs);
        assert!(
            err.contains("does not satisfy the schema at `/theme`"),
            "{err}"
        );
        assert!(err.contains("has more than 32 properties"), "{err}");
    }

    #[test]
    fn rejects_a_huge_number() {
        assert_eq!(
            theme_rejection(&["font.line-height.label: 1000000"]),
            "invalid theme: theme token `font.line-height.label` expects a number with at most two decimals, like 1.2"
        );
    }

    // --- Images from a template folder ---

    use crate::image_check::test_images::png;

    /// A template folder of its own under the temp dir with `files` (path
    /// below it, bytes), unique to the test.
    fn image_folder(test: &str, files: &[(&str, Vec<u8>)]) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("plinth-template-fs-{}-{test}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        for (path, bytes) in files {
            let file = folder.join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, bytes).unwrap();
        }
        folder
    }

    fn folder_package(folder: &Path, yaml: String) -> TemplatePackage {
        TemplatePackage {
            yaml,
            assets: TemplateAssets::Folder(folder.to_path_buf()),
        }
    }

    #[test]
    fn loads_a_folder_template_with_a_button_image() {
        let bytes = png(3, 2);
        let folder = image_folder("button-image", &[("img/fire.png", bytes.clone())]);
        let src = FsTemplateSource::new(catalog()).unwrap();
        let yaml = with_component("keys: \"m\"\n        image: \"img/fire.png\"");

        let template = src.load_package(&folder_package(&folder, yaml)).unwrap();

        let component = &template.pages()[0].components()[0];
        assert_eq!(component.visual.image, Some(ImageRef(0)));
        let [image] = template.images() else {
            panic!("one image, got {:?}", template.images().len());
        };
        assert_eq!(image.path.as_str(), "img/fire.png");
        assert_eq!(image.kind, plinth_domain::ImageKind::Png);
        assert_eq!((image.width, image.height), (3, 2));
        assert_eq!(&image.bytes[..], &bytes[..]);
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn a_png_named_webp_is_a_format_mismatch() {
        let folder = image_folder("mismatch", &[("x.webp", png(3, 2))]);
        let src = FsTemplateSource::new(catalog()).unwrap();
        let yaml = with_component("keys: \"m\"\n        image: \"x.webp\"");

        let err = src
            .load_package(&folder_package(&folder, yaml))
            .unwrap_err();

        assert_eq!(err.0, "`x.webp` is not a webp file");
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn a_png_whose_bytes_are_no_image_is_a_format_mismatch() {
        let folder = image_folder("no-image", &[("x.png", b"GIF89a\x01\x00\x01\x00".to_vec())]);
        let src = FsTemplateSource::new(catalog()).unwrap();
        let yaml = with_component("keys: \"m\"\n        image: \"x.png\"");

        let err = src
            .load_package(&folder_package(&folder, yaml))
            .unwrap_err();

        assert_eq!(err.0, "`x.png` is not a png file");
        let _ = std::fs::remove_dir_all(folder);
    }

    /// A template whose page has one button per path in `images`, ten to a
    /// row: `b0` shows `images[0]`, and so on.
    fn with_images(images: &[String]) -> String {
        let components: String = images
            .iter()
            .enumerate()
            .map(|(i, path)| {
                format!(
                    "      - {{ id: b{i}, position: {{ row: {}, col: {} }}, keys: \"m\", image: \"{path}\" }}\n",
                    i / 10,
                    i % 10
                )
            })
            .collect();
        format!(
            "schema_version: \"1\"\nmetadata: {{ game: WoW, version: \"1\", author: dev }}\npages:\n  - components:\n{components}sidebar: {{ default_side: right, components: [] }}\n"
        )
    }

    const TOO_MANY_IMAGES: &str = "the template's images exceed 64 files or 16 MiB";

    #[test]
    fn a_template_with_65_images_is_rejected() {
        let files: Vec<(String, Vec<u8>)> =
            (0..65).map(|i| (format!("p{i}.png"), png(1, 1))).collect();
        let refs: Vec<(&str, Vec<u8>)> =
            files.iter().map(|(p, b)| (p.as_str(), b.clone())).collect();
        let folder = image_folder("65-images", &refs);
        let src = FsTemplateSource::new(catalog()).unwrap();
        let paths: Vec<String> = files.into_iter().map(|(p, _)| p).collect();

        let sixty_four = src.load_package(&folder_package(&folder, with_images(&paths[..64])));
        assert_eq!(
            sixty_four.map(|t| t.images().len()).map_err(|e| e.0),
            Ok(64)
        );

        let sixty_five = src.load_package(&folder_package(&folder, with_images(&paths)));
        assert_eq!(
            sixty_five.err().map(|e| e.0).as_deref(),
            Some(TOO_MANY_IMAGES)
        );
        let _ = std::fs::remove_dir_all(folder);
    }

    /// A valid 1 × 1 PNG of exactly `len` bytes, padded with a `tEXt` chunk.
    fn png_of_len(len: usize) -> Vec<u8> {
        use crate::image_check::test_images::{chunk, idat, ihdr, png_from};
        let bare = png(1, 1).len();
        let mut text = b"Comment\0".to_vec();
        text.resize(len - bare - 12, b'x');
        png_from(&[
            ihdr(1, 1),
            idat(1, 1),
            chunk(b"tEXt", &text),
            chunk(b"IEND", &[]),
        ])
    }

    #[test]
    fn a_template_over_16_mib_is_rejected() {
        // The largest a button image may be: 16 of them are 16 MiB exactly.
        let one_mib = png_of_len(1024 * 1024);
        assert_eq!(one_mib.len(), 1024 * 1024);
        let files: Vec<(String, Vec<u8>)> = (0..17)
            .map(|i| (format!("big{i}.png"), one_mib.clone()))
            .collect();
        let refs: Vec<(&str, Vec<u8>)> =
            files.iter().map(|(p, b)| (p.as_str(), b.clone())).collect();
        let folder = image_folder("16-mib", &refs);
        let src = FsTemplateSource::new(catalog()).unwrap();
        let paths: Vec<String> = files.into_iter().map(|(p, _)| p).collect();

        let sixteen = src.load_package(&folder_package(&folder, with_images(&paths[..16])));
        assert_eq!(sixteen.map(|t| t.images().len()).map_err(|e| e.0), Ok(16));

        let seventeen = src.load_package(&folder_package(&folder, with_images(&paths)));
        assert_eq!(
            seventeen.err().map(|e| e.0).as_deref(),
            Some(TOO_MANY_IMAGES)
        );
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn the_same_file_used_twice_counts_once() {
        let mut files: Vec<(String, Vec<u8>)> =
            (0..63).map(|i| (format!("p{i}.png"), png(1, 1))).collect();
        files.push((
            "img/dup.webp".into(),
            include_bytes!("fixtures/lossless-3x2.webp").to_vec(),
        ));
        let refs: Vec<(&str, Vec<u8>)> =
            files.iter().map(|(p, b)| (p.as_str(), b.clone())).collect();
        let folder = image_folder("same-file", &refs);
        // On a disk that ignores case (macOS, Windows) another spelling is the
        // same file, and only its canonical path tells; elsewhere the same
        // spelling twice is.
        let again = if folder.join("img/Dup.webp").is_file() {
            "img/Dup.webp"
        } else {
            "img/dup.webp"
        };
        let mut paths: Vec<String> = files.into_iter().map(|(p, _)| p).collect();
        paths.push(again.into());
        let src = FsTemplateSource::new(catalog()).unwrap();

        // 65 buttons, 64 files.
        let template = src
            .load_package(&folder_package(&folder, with_images(&paths)))
            .unwrap();

        assert_eq!(template.images().len(), 64);
        let components = template.pages()[0].components();
        assert_eq!(components[63].visual.image, Some(ImageRef(63)));
        assert_eq!(components[64].visual.image, Some(ImageRef(63)));
        let webp = &template.images()[63];
        assert_eq!(webp.kind, plinth_domain::ImageKind::Webp);
        assert_eq!((webp.width, webp.height), (3, 2));
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn a_single_file_template_cannot_reference_images() {
        let src = FsTemplateSource::new(catalog()).unwrap();
        let yaml = with_component("keys: \"m\"\n        image: \"img/fire.png\"");

        let err = src.load_str(&yaml).unwrap_err();

        assert_eq!(
            err.0,
            "a single-file template cannot reference images; put it in a folder with template.yaml"
        );
    }
    // --- Icons, hidden labels, and where each look is allowed ---

    /// A PNG of `width` × `height` with no pixels: the loader only reads the
    /// header, so it can claim any size and stay a few bytes long.
    fn png_head(width: u32, height: u32) -> Vec<u8> {
        use crate::image_check::test_images::{chunk, ihdr, png_from};
        png_from(&[ihdr(width, height), chunk(b"IEND", &[])])
    }

    fn load_in(folder: &Path, yaml: String) -> Result<Template, String> {
        let src = FsTemplateSource::new(catalog()).unwrap();
        src.load_package(&folder_package(folder, yaml))
            .map_err(|e| e.0)
    }

    /// `with_component` for a button that sends `m`, plus `extra` lines.
    fn keys_button(extra: &[&str]) -> String {
        let lines: String = extra
            .iter()
            .map(|line| format!("\n        {line}"))
            .collect();
        with_component(&format!("keys: \"m\"{lines}"))
    }

    #[test]
    fn icon_and_image_together_are_rejected() {
        let folder = image_folder("icon-with-image", &[("x.png", png_head(3, 2))]);

        let err = load_in(
            &folder,
            keys_button(&[r#"icon: "x.png""#, r#"image: "x.png""#]),
        )
        .expect_err("an icon and an image on one button");
        assert!(
            err.contains("does not satisfy the schema at `/pages/0/components/0`"),
            "{err}"
        );

        // Behind the schema, the domain refuses it too.
        let alone = load_in(&folder, keys_button(&[r#"icon: "x.png""#])).unwrap();
        let mut both = alone.pages()[0].components()[0].clone();
        assert_eq!(both.visual.icon, Some(ImageRef(0)));
        both.visual.image = both.visual.icon;
        let err = Template::new(
            alone.metadata().clone(),
            vec![Page::new(vec![both]).unwrap()],
            alone.sidebar().clone(),
        )
        .expect_err("the domain lets an icon and an image through");
        assert_eq!(
            err.to_string(),
            "`btn` has both an icon and an image; a button shows one or the other"
        );
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn an_icon_over_256_px_is_rejected() {
        let kib = 1024;
        let folder = image_folder(
            "big-icon",
            &[
                ("ok.png", png_head(256, 256)),
                ("wide.png", png_head(257, 1)),
                ("tall.png", png_head(1, 257)),
                ("full.png", png_of_len(256 * kib)),
                ("heavy.png", png_of_len(256 * kib + 1)),
            ],
        );
        let icon = |path: &str| keys_button(&[&format!("icon: \"{path}\"")]);

        for fits in ["ok.png", "full.png"] {
            let template = load_in(&folder, icon(fits)).unwrap_or_else(|e| panic!("{fits}: {e}"));
            assert_eq!(
                template.pages()[0].components()[0].visual.icon,
                Some(ImageRef(0))
            );
        }
        for (path, err) in [
            (
                "wide.png",
                "`wide.png` is too large for an icon: 257x1 > 256x256",
            ),
            (
                "tall.png",
                "`tall.png` is too large for an icon: 1x257 > 256x256",
            ),
            (
                "heavy.png",
                "`heavy.png` is too large: 262145 bytes > 262144 bytes",
            ),
        ] {
            assert_eq!(load_in(&folder, icon(path)).err().as_deref(), Some(err));
        }
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn a_file_that_fits_a_button_image_is_too_large_for_an_icon() {
        let folder = image_folder("two-uses", &[("img/a.png", png_head(300, 300))]);
        let button = |id: &str, col: u16, look: &str| {
            format!("      - {{ id: {id}, position: {{ row: 0, col: {col} }}, keys: \"m\", {look}: \"img/a.png\" }}\n")
        };
        let template = |buttons: String| {
            format!(
                "schema_version: \"1\"\nmetadata: {{ game: WoW, version: \"1\", author: dev }}\npages:\n  - components:\n{buttons}sidebar: {{ default_side: right, components: [] }}\n"
            )
        };

        // As a button image it fits, on one button or two.
        let images = template(button("one", 0, "image") + &button("two", 1, "image"));
        assert_eq!(load_in(&folder, images).map(|t| t.images().len()), Ok(1));

        // Already read for the image, the same file is no icon.
        let mixed = template(button("one", 0, "image") + &button("two", 1, "icon"));
        assert_eq!(
            load_in(&folder, mixed).err().as_deref(),
            Some("`img/a.png` is too large for an icon: 300x300 > 256x256")
        );
        let _ = std::fs::remove_dir_all(folder);
    }

    /// The quit button on a page, with `extra` lines.
    fn quit_button(extra: &str) -> String {
        with_component(&format!("action: \"app.quit\"\n        {extra}"))
    }

    #[test]
    fn quit_rejects_icon_image_style_and_hide_label() {
        let folder = image_folder("quit", &[("x.png", png_head(3, 2))]);
        // A plain quit button loads.
        assert!(load_in(&folder, quit_button(r#"label: "Quit""#)).is_ok());

        for (extra, field) in [
            (r#"icon: "x.png""#, "icon"),
            (r#"image: "x.png""#, "image"),
            (
                r##"style: { color.button.background: "#000000" }"##,
                "style",
            ),
            ("hide_label: true", "hide_label"),
        ] {
            assert_eq!(
                load_in(&folder, quit_button(extra)).err(),
                Some(format!(
                    "the quit button cannot have `{field}`, so it always looks the same"
                )),
                "{extra}"
            );
        }
        let _ = std::fs::remove_dir_all(folder);
    }

    /// `VALID` with the lock button of its sidebar carrying `extra`.
    fn with_sidebar_lock(extra: &str) -> String {
        VALID.replace(
            "      action: \"app.toggle_lock\"\n",
            &format!("      action: \"app.toggle_lock\"\n      {extra}\n"),
        )
    }

    #[test]
    fn the_sidebar_rejects_image() {
        let folder = image_folder("sidebar", &[("x.png", png_head(3, 2))]);

        // An icon alone, with its label hidden, is fine in the sidebar.
        let icon = with_sidebar_lock("icon: \"x.png\"\n      hide_label: true");
        let template = load_in(&folder, icon).unwrap();
        let lock = &template.sidebar().components[0].visual;
        assert_eq!(lock.icon, Some(ImageRef(0)));
        assert!(lock.hide_label);

        let image = with_sidebar_lock("image: \"x.png\"");
        assert_eq!(
            load_in(&folder, image).err().as_deref(),
            Some("`lock` is in the sidebar, which does not allow `image`")
        );
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn the_sidebar_rejects_style() {
        let src = FsTemplateSource::new(catalog()).unwrap();
        // The same look on a page button loads.
        let on_page = keys_button(&[r##"style: { color.button.background: "#000000" }"##]);
        assert!(src.load_str(&on_page).is_ok());

        let in_sidebar = with_sidebar_lock(r##"style: { color.button.background: "#000000" }"##);
        assert_eq!(
            src.load_str(&in_sidebar).err().map(|e| e.0).as_deref(),
            Some("`lock` is in the sidebar, which does not allow `style`")
        );
    }

    // --- Page backgrounds ---

    /// A template with one button per page: `root` is the default background,
    /// `own[i]` the background of page `i`, and `extra` goes on every button.
    fn with_backgrounds(root: Option<&str>, own: &[Option<&str>], extra: &str) -> String {
        let background = |path: &str| format!("background: {{ image: \"{path}\" }}");
        let pages: String = own
            .iter()
            .enumerate()
            .map(|(i, path)| {
                let first = path.map(|p| background(p) + "\n    ").unwrap_or_default();
                format!(
                    "  - {first}components:\n      - {{ id: b{i}, position: {{ row: 0, col: 0 }}, keys: \"m\"{extra} }}\n"
                )
            })
            .collect();
        let root = root.map(|p| background(p) + "\n").unwrap_or_default();
        format!(
            "schema_version: \"1\"\nmetadata: {{ game: WoW, version: \"1\", author: dev }}\n{root}pages:\n{pages}sidebar: {{ default_side: right, components: [] }}\n"
        )
    }

    /// The path of the background page `index` shows, if any.
    fn background_of(template: &Template, index: usize) -> Option<&str> {
        template
            .page_background(index)
            .map(|image| template.images()[image.0].path.as_str())
    }

    #[test]
    fn a_background_over_its_limits_is_rejected() {
        let mib = 1024 * 1024;
        let folder = image_folder(
            "big-background",
            &[
                ("ok.png", png_head(2048, 2048)),
                ("wide.png", png_head(2049, 1)),
                ("tall.png", png_head(1, 2049)),
                ("full.png", png_of_len(4 * mib)),
                ("heavy.png", png_of_len(4 * mib + 1)),
            ],
        );

        // Too large for a button image, but not for a background.
        for fits in ["ok.png", "full.png"] {
            let template = load_in(&folder, with_backgrounds(Some(fits), &[None], ""))
                .unwrap_or_else(|e| panic!("{fits}: {e}"));
            assert_eq!(background_of(&template, 0), Some(fits));
        }
        for (path, err) in [
            (
                "wide.png",
                "`wide.png` is too large for a background: 2049x1 > 2048x2048",
            ),
            (
                "tall.png",
                "`tall.png` is too large for a background: 1x2049 > 2048x2048",
            ),
            (
                "heavy.png",
                "`heavy.png` is too large: 4194305 bytes > 4194304 bytes",
            ),
        ] {
            // At the root and on a page alike.
            let at_root = with_backgrounds(Some(path), &[None], "");
            let on_page = with_backgrounds(None, &[Some(path)], "");
            for yaml in [at_root, on_page] {
                assert_eq!(load_in(&folder, yaml).err().as_deref(), Some(err));
            }
        }
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn a_page_background_wins_over_the_default() {
        let folder = image_folder(
            "own-background",
            &[
                ("root.png", png_head(4, 4)),
                ("img/own.png", png_head(5, 5)),
            ],
        );

        let yaml = with_backgrounds(Some("root.png"), &[None, Some("img/own.png"), None], "");
        let template = load_in(&folder, yaml).unwrap();

        assert_eq!(background_of(&template, 1), Some("img/own.png"));
        // Only for its page: the others keep the default.
        assert_eq!(background_of(&template, 0), Some("root.png"));
        assert_eq!(background_of(&template, 2), Some("root.png"));
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn pages_without_their_own_background_take_the_default() {
        let folder = image_folder(
            "default-background",
            &[("root.png", png_head(4, 4)), ("own.png", png_head(5, 5))],
        );

        let shared = load_in(
            &folder,
            with_backgrounds(Some("root.png"), &[None, None], ""),
        )
        .unwrap();
        assert_eq!(background_of(&shared, 0), Some("root.png"));
        assert_eq!(background_of(&shared, 1), Some("root.png"));
        assert_eq!(shared.images().len(), 1);
        // Past the last page there is no page, and no background.
        assert_eq!(shared.page_background(2), None);

        // With no default, a page without its own has none.
        let no_default = load_in(
            &folder,
            with_backgrounds(None, &[Some("own.png"), None], ""),
        )
        .unwrap();
        assert_eq!(background_of(&no_default, 0), Some("own.png"));
        assert_eq!(no_default.page_background(1), None);
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn the_same_file_meets_the_limits_of_each_use() {
        let folder = image_folder(
            "background-and-image",
            &[("img/big.png", png_head(1500, 1500))],
        );
        const TOO_LARGE: &str =
            "`img/big.png` is too large for a button image: 1500x1500 > 1024x1024";
        let image = r#", image: "img/big.png""#;

        // As a background it fits, at the root and on a page, as one image.
        let backgrounds = with_backgrounds(Some("img/big.png"), &[Some("img/big.png")], "");
        let template = load_in(&folder, backgrounds).unwrap();
        assert_eq!(template.images().len(), 1);
        assert_eq!(template.page_background(0), Some(ImageRef(0)));

        // Already read for the background, the same file is no button image…
        for yaml in [
            with_backgrounds(Some("img/big.png"), &[None], image),
            with_backgrounds(None, &[Some("img/big.png")], image),
        ] {
            assert_eq!(load_in(&folder, yaml).err().as_deref(), Some(TOO_LARGE));
        }
        // …nor alone.
        let alone = with_backgrounds(None, &[None], image);
        assert_eq!(load_in(&folder, alone).err().as_deref(), Some(TOO_LARGE));
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn a_text_icon_no_longer_loads() {
        let src = FsTemplateSource::new(catalog()).unwrap();
        let err = src
            .load_str(&keys_button(&[r#"icon: "texto""#]))
            .unwrap_err();
        assert_eq!(
            err.0,
            "`texto` is not a valid image path: it must end in `.png` or `.webp`, in lower case"
        );
    }
}
