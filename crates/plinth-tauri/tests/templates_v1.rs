//! The reference templates of format 1.0, in `tests/templates-v1/`. Every
//! one of them has to keep loading and fitting the panel in every later
//! Plinth: within format 1 a change only adds optional fields. They are
//! never edited; a new minor of the format adds new ones next to them (see
//! `docs/templates.md`).

use std::path::PathBuf;

use plinth_application::GameKeybindProvider;
use plinth_domain::{Component, ImageKind, ImageRef, PageTransition, PressEffect};
use plinth_infra::keybinds_static::StaticKeybindProvider;
use plinth_infra::strict_yaml;
use plinth_infra::template_dir::TemplateDir;
use plinth_infra::template_fs::FsTemplateSource;
use plinth_tauri_lib::panel::PANEL_SIZE;

fn corpus() -> TemplateDir {
    TemplateDir::new(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("templates-v1"),
    )
}

fn source() -> FsTemplateSource {
    FsTemplateSource::new(StaticKeybindProvider::wow_defaults().catalog()).unwrap()
}

#[test]
fn every_v1_reference_template_loads_and_fits() {
    let dir = corpus();
    let names = dir.list().unwrap();
    assert!(
        names.len() >= 4,
        "the corpus has {} templates: {names:?}",
        names.len()
    );
    for expected in [
        "full/",
        "minimal.yaml",
        "wow-default-keys.yaml",
        "wow-demo.yaml",
    ] {
        assert!(
            names.iter().any(|name| name == expected),
            "{expected} is not in {names:?}"
        );
    }
    let source = source();
    for name in &names {
        let package = dir
            .read(name)
            .unwrap_or_else(|e| panic!("{name} cannot be read: {}", e.0));
        let template = source
            .load_package(&package)
            .unwrap_or_else(|e| panic!("{name} no longer loads: {}", e.0));
        template
            .check_fit(PANEL_SIZE)
            .unwrap_or_else(|e| panic!("{name} no longer fits the panel: {e}"));
    }
}

#[test]
fn the_full_reference_template_uses_every_v1_feature() {
    let package = corpus().read("full/").unwrap();
    let template = source().load_package(&package).unwrap();
    let components: Vec<&Component> = template
        .pages()
        .iter()
        .flat_map(|page| page.components())
        .chain(template.sidebar().components.iter())
        .collect();

    let images: Vec<ImageRef> = components.iter().filter_map(|c| c.visual.image).collect();
    let icons: Vec<ImageRef> = components.iter().filter_map(|c| c.visual.icon).collect();
    assert!(!images.is_empty(), "no button has an image");
    assert!(!icons.is_empty(), "no button has an icon");
    assert!(
        images
            .iter()
            .chain(&icons)
            .any(|image| template.images()[image.0].kind == ImageKind::Webp),
        "neither an image nor an icon is a WebP"
    );
    assert!(
        components.iter().any(|c| c.visual.hide_label),
        "no button hides its label"
    );
    assert!(
        components.iter().any(|c| !c.style.is_empty()),
        "no button has a style"
    );
    assert!(
        components
            .iter()
            .any(|c| c.position.row_span > 1 || c.position.col_span > 1),
        "no button spans more than one cell"
    );

    let effects = template.effects();
    assert_ne!(effects.page_transition, PageTransition::Slide);
    assert_ne!(effects.press, PressEffect::Lighten);

    // A default background, a page with its own, and a page that shows the
    // default.
    let doc = strict_yaml::from_str(&package.yaml).unwrap();
    assert!(doc.get("background").is_some(), "no default background");
    let own: Vec<bool> = doc["pages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|page| page.get("background").is_some())
        .collect();
    let with_own = own
        .iter()
        .position(|&has| has)
        .expect("no page has a background of its own");
    let with_default = own
        .iter()
        .position(|&has| !has)
        .expect("every page has its own background, so none shows the default");
    let default = template.page_background(with_default);
    assert!(default.is_some(), "the default background is not shown");
    assert!(template.page_background(with_own).is_some());
    assert_ne!(template.page_background(with_own), default);
}
