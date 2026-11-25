//! Which template the panel opens with, and the templates the binary brings
//! to seed the user's folder.

use plinth_application::GameKeybindProvider;
use plinth_infra::keybinds_static::StaticKeybindProvider;
use plinth_infra::template_dir::TemplateDir;
use plinth_infra::template_fs::FsTemplateSource;

use crate::panel::{WOW_DEFAULT_KEYS_TEMPLATE, WOW_DEMO_TEMPLATE};

/// Written into the templates folder the first time it is created.
pub const BUNDLED_TEMPLATES: &[(&str, &str)] = &[
    ("wow-default-keys.yaml", WOW_DEFAULT_KEYS_TEMPLATE),
    ("wow-demo.yaml", WOW_DEMO_TEMPLATE),
];

/// The folder template to open when the user has not chosen one.
pub const DEFAULT_TEMPLATE_NAME: &str = "wow-default-keys.yaml";

/// The template to open: its file name in the folder (`None` for the
/// embedded fallback) and its YAML.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupTemplate {
    pub name: Option<String>,
    pub yaml: String,
}

/// Whether `yaml` loads as a template against the WoW catalog.
pub fn loads_with_wow_catalog(yaml: &str) -> bool {
    FsTemplateSource::new(StaticKeybindProvider::wow_defaults().catalog())
        .map(|source| source.load_str(yaml).is_ok())
        .unwrap_or(false)
}

/// Tries the last chosen template, then the default one, from the folder; the
/// first one that reads and loads wins. If none does, the embedded default
/// opens, so the panel always starts.
pub fn pick_startup_template(
    dir: &TemplateDir,
    last: Option<&str>,
    yaml_loads: impl Fn(&str) -> bool,
) -> StartupTemplate {
    let candidates = last
        .into_iter()
        .chain(std::iter::once(DEFAULT_TEMPLATE_NAME));
    for name in candidates {
        if let Ok(yaml) = dir.read(name) {
            if yaml_loads(&yaml) {
                return StartupTemplate {
                    name: Some(name.to_string()),
                    yaml,
                };
            }
        }
    }
    StartupTemplate {
        name: None,
        yaml: WOW_DEFAULT_KEYS_TEMPLATE.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::panel::WOW_PHASE1_TEMPLATE;
    use std::path::PathBuf;

    fn temp_root(test: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("plinth-startup-{}-{test}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        root
    }

    /// A seeded folder plus extra files.
    fn folder(test: &str, extra: &[(&str, &str)]) -> (PathBuf, TemplateDir) {
        let root = temp_root(test);
        let dir = TemplateDir::new(root.clone());
        dir.seed(BUNDLED_TEMPLATES).unwrap();
        for (name, yaml) in extra {
            std::fs::write(root.join(name), yaml).unwrap();
        }
        (root, dir)
    }

    #[test]
    fn last_template_wins_when_it_loads() {
        let (root, dir) = folder("last-wins", &[]);
        let picked = pick_startup_template(&dir, Some("wow-demo.yaml"), loads_with_wow_catalog);
        assert_eq!(picked.name.as_deref(), Some("wow-demo.yaml"));
        assert_eq!(picked.yaml, WOW_DEMO_TEMPLATE);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn falls_back_to_default_when_last_is_missing() {
        let (root, dir) = folder("last-missing", &[]);
        let picked = pick_startup_template(&dir, Some("gone.yaml"), loads_with_wow_catalog);
        assert_eq!(picked.name.as_deref(), Some(DEFAULT_TEMPLATE_NAME));
        assert_eq!(picked.yaml, WOW_DEFAULT_KEYS_TEMPLATE);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn falls_back_to_default_when_last_is_invalid() {
        let (root, dir) = folder("last-invalid", &[("broken.yaml", "pages: nope\n")]);
        let picked = pick_startup_template(&dir, Some("broken.yaml"), loads_with_wow_catalog);
        assert_eq!(picked.name.as_deref(), Some(DEFAULT_TEMPLATE_NAME));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn falls_back_to_default_without_a_last_template() {
        let (root, dir) = folder("no-last", &[]);
        let picked = pick_startup_template(&dir, None, loads_with_wow_catalog);
        assert_eq!(picked.name.as_deref(), Some(DEFAULT_TEMPLATE_NAME));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn falls_back_to_the_embedded_default() {
        let root = temp_root("embedded");
        std::fs::create_dir_all(&root).unwrap();
        let dir = TemplateDir::new(root.clone());
        let picked = pick_startup_template(&dir, Some("wow-demo.yaml"), loads_with_wow_catalog);
        assert_eq!(picked.name, None);
        assert_eq!(picked.yaml, WOW_DEFAULT_KEYS_TEMPLATE);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn bundled_templates_still_load() {
        let source =
            FsTemplateSource::new(StaticKeybindProvider::wow_defaults().catalog()).unwrap();
        for (name, yaml) in [
            ("wow-default-keys.yaml", WOW_DEFAULT_KEYS_TEMPLATE),
            ("wow-demo.yaml", WOW_DEMO_TEMPLATE),
            ("wow-phase1.yaml", WOW_PHASE1_TEMPLATE),
        ] {
            if let Err(error) = source.load_str(yaml) {
                panic!("`{name}` must load with the strict reader: {}", error.0);
            }
        }
    }

    #[test]
    fn bundled_templates_all_load() {
        for (name, yaml) in BUNDLED_TEMPLATES {
            assert!(loads_with_wow_catalog(yaml), "`{name}` must load");
        }
        assert!(!loads_with_wow_catalog("pages: nope\n"));
    }
}
