//! The folder of user templates: `dirs::config_dir()/plinth/templates`
//! (`%APPDATA%\plinth\templates` on Windows).
//!
//! It is seeded once with the templates bundled in the binary and never
//! written again, so whatever the user edits there stays. Only plain `.yaml`
//! file names inside the folder can be read.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use plinth_application::TemplateError;

pub struct TemplateDir {
    root: PathBuf,
}

impl TemplateDir {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// `dirs::config_dir()/plinth/templates`.
    pub fn at_default_location() -> Result<Self, TemplateError> {
        let dir = dirs::config_dir()
            .ok_or_else(|| TemplateError("could not determine the config directory".into()))?;
        Ok(Self::new(dir.join("plinth").join("templates")))
    }

    /// Writes `bundled` (`(file name, yaml)`) into the folder if it does not
    /// exist yet, and reports whether it did. An existing folder is left alone.
    pub fn seed(&self, bundled: &[(&str, &str)]) -> Result<bool, TemplateError> {
        if self.root.exists() {
            return Ok(false);
        }
        std::fs::create_dir_all(&self.root).map_err(|e| io_err(&self.root, e))?;
        for (name, yaml) in bundled {
            let path = self.path_of(name)?;
            std::fs::write(&path, yaml).map_err(|e| io_err(&path, e))?;
        }
        Ok(true)
    }

    /// The `.yaml` file names in the folder, sorted. A missing folder has none.
    pub fn list(&self) -> Result<Vec<String>, TemplateError> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }
        let entries = std::fs::read_dir(&self.root).map_err(|e| io_err(&self.root, e))?;
        let mut names = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| io_err(&self.root, e))?;
            let is_file = entry.file_type().map(|t| t.is_file()).unwrap_or(false);
            if let Some(name) = entry.file_name().to_str() {
                if is_file && is_template_name(name) {
                    names.push(name.to_string());
                }
            }
        }
        names.sort();
        Ok(names)
    }

    /// Reads one template by its file name. Anything that is not a plain
    /// `.yaml` file name inside the folder is refused.
    pub fn read(&self, name: &str) -> Result<String, TemplateError> {
        let path = self.path_of(name)?;
        std::fs::read_to_string(&path).map_err(|e| io_err(&path, e))
    }

    fn path_of(&self, name: &str) -> Result<PathBuf, TemplateError> {
        if !is_template_name(name) {
            return Err(TemplateError(format!(
                "`{name}` is not a template file name"
            )));
        }
        Ok(self.root.join(name))
    }
}

/// A plain `.yaml` file name: no directory part, no drive (`C:x.yaml` would
/// replace the folder on Windows), no `..`.
fn is_template_name(name: &str) -> bool {
    name.ends_with(".yaml")
        && !name.contains([':', '/', '\\'])
        && !name.contains("..")
        && Path::new(name).file_name() == Some(OsStr::new(name))
}

fn io_err(path: &Path, e: std::io::Error) -> TemplateError {
    TemplateError(format!("`{}`: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh path under the system temp dir, unique to the test.
    fn temp_root(test: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("plinth-template-dir-{}-{test}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        root
    }

    const BUNDLED: &[(&str, &str)] = &[("b.yaml", "b: 1\n"), ("a.yaml", "a: 1\n")];

    #[test]
    fn seed_writes_into_a_missing_dir() {
        let root = temp_root("seed-missing");
        let dir = TemplateDir::new(root.clone());
        assert!(dir.seed(BUNDLED).unwrap());
        assert_eq!(dir.list().unwrap(), vec!["a.yaml", "b.yaml"]);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn seed_leaves_an_existing_dir_alone() {
        let root = temp_root("seed-existing");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.yaml"), "edited: true\n").unwrap();
        let dir = TemplateDir::new(root.clone());
        assert!(!dir.seed(BUNDLED).unwrap());
        assert_eq!(dir.read("a.yaml").unwrap(), "edited: true\n");
        assert_eq!(dir.list().unwrap(), vec!["a.yaml"]);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn list_ignores_non_yaml_files() {
        let root = temp_root("list-non-yaml");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("notes.txt"), "x").unwrap();
        std::fs::write(root.join("wow.yaml"), "x").unwrap();
        std::fs::create_dir_all(root.join("folder.yaml")).unwrap();
        let dir = TemplateDir::new(root.clone());
        assert_eq!(dir.list().unwrap(), vec!["wow.yaml"]);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn list_of_a_missing_dir_is_empty() {
        let dir = TemplateDir::new(temp_root("list-missing"));
        assert!(dir.list().unwrap().is_empty());
    }

    #[test]
    fn read_returns_the_file() {
        let root = temp_root("read");
        let dir = TemplateDir::new(root.clone());
        dir.seed(BUNDLED).unwrap();
        assert_eq!(dir.read("b.yaml").unwrap(), "b: 1\n");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn read_rejects_paths() {
        let root = temp_root("read-paths");
        let dir = TemplateDir::new(root.clone());
        dir.seed(BUNDLED).unwrap();
        for name in [
            "../x.yaml",
            "a/b.yaml",
            "a\\b.yaml",
            "C:x.yaml",
            "x.txt",
            "..",
            "",
        ] {
            let err = dir.read(name).unwrap_err();
            // Refused for its name, not because no such file exists.
            assert!(
                err.0.contains("is not a template file name"),
                "`{name}`: {}",
                err.0
            );
        }
        let _ = std::fs::remove_dir_all(root);
    }
}
