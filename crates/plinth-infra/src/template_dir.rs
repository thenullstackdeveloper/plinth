//! The folder of user templates: `dirs::config_dir()/plinth/templates`
//! (`%APPDATA%\plinth\templates` on Windows).
//!
//! It is seeded once with the templates bundled in the binary and never
//! written again, so whatever the user edits there stays. A template is a
//! plain `.yaml` file of the folder (`x.yaml`) or a folder of its own with a
//! `template.yaml` inside (`x/`); only a folder can bring images.

use std::ffi::OsStr;
use std::io::Read;
use std::path::{Path, PathBuf};

use plinth_application::TemplateError;
use plinth_domain::ImagePath;

/// The file a template folder holds its YAML in.
pub const FOLDER_TEMPLATE_FILE: &str = "template.yaml";

/// The longest name a template folder may have.
const MAX_FOLDER_NAME: usize = 64;

/// A template as read from the folder: its YAML and where its images are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplatePackage {
    pub yaml: String,
    pub assets: TemplateAssets,
}

impl TemplatePackage {
    /// A template that is a single YAML file, without images.
    pub fn single_file(yaml: impl Into<String>) -> Self {
        Self {
            yaml: yaml.into(),
            assets: TemplateAssets::None,
        }
    }
}

/// Where a template's images are read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateAssets {
    /// A single-file template: it cannot reference images.
    None,
    /// A template folder: its images are files below it.
    Folder(PathBuf),
}

impl TemplateAssets {
    /// Reads the image at `path`, at most `max_bytes` long. Before opening
    /// it: no part of the path below the template's folder is a symbolic
    /// link (or a junction, on Windows), the canonical path stays inside the
    /// canonical folder, and it is a regular file no longer than
    /// `max_bytes`. The read itself stops past `max_bytes`, so a file that
    /// grows after the check is still refused.
    pub fn read(&self, path: &ImagePath, max_bytes: u64) -> Result<Vec<u8>, TemplateError> {
        let TemplateAssets::Folder(root) = self else {
            return Err(TemplateError(
                "a single-file template cannot reference images; put it in a folder with template.yaml"
                    .into(),
            ));
        };
        let shown = path.as_str();
        let leaves = || TemplateError(format!("`{shown}` leaves the template folder"));
        let unreadable =
            |e: std::io::Error| TemplateError(format!("`{shown}` could not be read: {e}"));

        let mut full = root.clone();
        for part in path.parts() {
            full.push(part);
            let meta = std::fs::symlink_metadata(&full).map_err(unreadable)?;
            if meta.file_type().is_symlink() {
                return Err(leaves());
            }
        }
        let canonical_root = root.canonicalize().map_err(unreadable)?;
        let canonical = full.canonicalize().map_err(unreadable)?;
        if !canonical.starts_with(&canonical_root) {
            return Err(leaves());
        }
        let meta = std::fs::metadata(&canonical).map_err(unreadable)?;
        if !meta.is_file() {
            return Err(TemplateError(format!("`{shown}` is not a regular file")));
        }
        if meta.len() > max_bytes {
            return Err(TemplateError(format!(
                "`{shown}` is too large: {} bytes > {max_bytes} bytes",
                meta.len()
            )));
        }
        let file = std::fs::File::open(&canonical).map_err(unreadable)?;
        read_capped(file, max_bytes).map_err(|e| TemplateError(format!("`{shown}` {}", e.0)))
    }
}

/// Reads `reader` to its end, but never more than `max_bytes`: one byte past
/// the limit is an error, whatever the file said its size was.
pub fn read_capped(reader: impl Read, max_bytes: u64) -> Result<Vec<u8>, TemplateError> {
    let mut bytes = Vec::new();
    reader
        .take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|e| TemplateError(format!("could not be read: {e}")))?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > max_bytes {
        return Err(TemplateError(format!("is larger than {max_bytes} bytes")));
    }
    Ok(bytes)
}

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
            let path = self.file_path_of(name)?;
            std::fs::write(&path, yaml).map_err(|e| io_err(&path, e))?;
        }
        Ok(true)
    }

    /// The templates in the folder, sorted: `x.yaml` for a file, `x/` for a
    /// folder. A folder counts only if it is a real directory (not a link)
    /// with a regular `template.yaml` file (not a link) inside. A missing
    /// folder has none.
    pub fn list(&self) -> Result<Vec<String>, TemplateError> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }
        let entries = std::fs::read_dir(&self.root).map_err(|e| io_err(&self.root, e))?;
        let mut names = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| io_err(&self.root, e))?;
            // `DirEntry::file_type` does not follow links.
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let Some(name) = entry.file_name().to_str().map(str::to_string) else {
                continue;
            };
            if kind.is_file() && is_template_name(&name) {
                names.push(name);
            } else if kind.is_dir() && is_folder_name(&name) && holds_a_template(&entry.path()) {
                names.push(format!("{name}/"));
            }
        }
        names.sort();
        Ok(names)
    }

    /// Reads one template by the name `list` gives it. Anything that is not
    /// a plain `.yaml` file name or a template folder name is refused.
    pub fn read(&self, name: &str) -> Result<TemplatePackage, TemplateError> {
        if let Some(folder) = name.strip_suffix('/') {
            if !is_folder_name(folder) {
                return Err(not_a_template(name));
            }
            let dir = self.root.join(folder);
            let is_dir = std::fs::symlink_metadata(&dir)
                .map(|m| m.file_type().is_dir())
                .map_err(|e| io_err(&dir, e))?;
            if !is_dir || !holds_a_template(&dir) {
                return Err(TemplateError(format!(
                    "`{name}` is not a folder with a {FOLDER_TEMPLATE_FILE} file"
                )));
            }
            let file = dir.join(FOLDER_TEMPLATE_FILE);
            let yaml = std::fs::read_to_string(&file).map_err(|e| io_err(&file, e))?;
            return Ok(TemplatePackage {
                yaml,
                assets: TemplateAssets::Folder(dir),
            });
        }
        let path = self.file_path_of(name)?;
        let yaml = std::fs::read_to_string(&path).map_err(|e| io_err(&path, e))?;
        Ok(TemplatePackage::single_file(yaml))
    }

    fn file_path_of(&self, name: &str) -> Result<PathBuf, TemplateError> {
        if !is_template_name(name) {
            return Err(not_a_template(name));
        }
        Ok(self.root.join(name))
    }
}

fn not_a_template(name: &str) -> TemplateError {
    TemplateError(format!("`{name}` is not a template file name"))
}

/// A plain `.yaml` file name: no directory part, no drive (`C:x.yaml` would
/// replace the folder on Windows), no `..`.
fn is_template_name(name: &str) -> bool {
    name.ends_with(".yaml")
        && !name.contains([':', '/', '\\'])
        && !name.contains("..")
        && Path::new(name).file_name() == Some(OsStr::new(name))
}

/// A template folder name: `^[A-Za-z0-9][A-Za-z0-9._ -]{0,63}$`, not ending
/// in `.yaml`, so a folder `x` and a file `x.yaml` never share a name.
fn is_folder_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphanumeric())
        && name.len() <= MAX_FOLDER_NAME
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | ' ' | '-'))
        && !name.ends_with(".yaml")
}

/// Whether `dir` holds a regular `template.yaml` file, not a link.
fn holds_a_template(dir: &Path) -> bool {
    std::fs::symlink_metadata(dir.join(FOLDER_TEMPLATE_FILE))
        .map(|m| m.file_type().is_file())
        .unwrap_or(false)
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
        assert_eq!(dir.read("a.yaml").unwrap().yaml, "edited: true\n");
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
        assert_eq!(
            dir.read("b.yaml").unwrap(),
            TemplatePackage::single_file("b: 1\n")
        );
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

    // --- Template folders and their images ---

    use crate::image_check::test_images::png;
    use plinth_domain::ImagePath;

    fn image(raw: &str) -> ImagePath {
        ImagePath::parse(raw).unwrap()
    }

    /// `root/t/` as a template folder with `files` (path below it, bytes).
    fn template_folder(test: &str, files: &[(&str, &[u8])]) -> (PathBuf, PathBuf) {
        let root = temp_root(test);
        let folder = root.join("t");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join(FOLDER_TEMPLATE_FILE), "x: 1\n").unwrap();
        for (path, bytes) in files {
            let file = folder.join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, bytes).unwrap();
        }
        (root, folder)
    }

    #[test]
    fn lists_yaml_files_and_template_folders_by_name() {
        let root = temp_root("list-folders");
        std::fs::create_dir_all(root.join("b")).unwrap();
        std::fs::write(root.join("a.yaml"), "a: 1\n").unwrap();
        std::fs::write(root.join("b").join(FOLDER_TEMPLATE_FILE), "b: 1\n").unwrap();
        // A folder without template.yaml is not a template.
        std::fs::create_dir_all(root.join("c")).unwrap();
        std::fs::write(root.join("c").join("other.yaml"), "c: 1\n").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(root.join("b"), root.join("d")).unwrap();
        let dir = TemplateDir::new(root.clone());

        assert_eq!(dir.list().unwrap(), vec!["a.yaml", "b/"]);
        let package = dir.read("b/").unwrap();
        assert_eq!(package.yaml, "b: 1\n");
        let TemplateAssets::Folder(folder) = &package.assets else {
            panic!("a folder template keeps its folder");
        };
        assert!(folder.ends_with("b"), "{}", folder.display());
        assert_eq!(dir.read("a.yaml").unwrap().assets, TemplateAssets::None);
        for name in ["c/", "d/"] {
            assert!(dir.read(name).is_err(), "`{name}` must not read");
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn folder_names_follow_the_rule() {
        let root = temp_root("folder-names");
        let names = ["ok name-1.v2_x", "x.yaml", ".hidden", "-dash", "semi;colon"];
        for name in names {
            std::fs::create_dir_all(root.join(name)).unwrap();
            std::fs::write(root.join(name).join(FOLDER_TEMPLATE_FILE), "x: 1\n").unwrap();
        }
        let long = "a".repeat(65);
        std::fs::create_dir_all(root.join(&long)).unwrap();
        std::fs::write(root.join(&long).join(FOLDER_TEMPLATE_FILE), "x: 1\n").unwrap();
        let dir = TemplateDir::new(root.clone());

        assert_eq!(dir.list().unwrap(), vec!["ok name-1.v2_x/"]);
        for name in [
            "x.yaml/",
            ".hidden/",
            "-dash/",
            "semi;colon/",
            "../t/",
            "a/b/",
            "/",
        ] {
            let err = dir.read(name).unwrap_err();
            assert!(
                err.0.contains("is not a template file name"),
                "`{name}`: {}",
                err.0
            );
        }
        assert!(dir.read(&format!("{long}/")).is_err());
        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn refuses_an_image_through_a_symlink_out_of_the_folder() {
        let (root, folder) = template_folder("symlink", &[]);
        let outside = root.join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("x.png"), png(1, 1)).unwrap();
        std::os::unix::fs::symlink(&outside, folder.join("img")).unwrap();
        std::os::unix::fs::symlink(outside.join("x.png"), folder.join("y.png")).unwrap();
        // A link is refused even when it points inside the folder.
        std::fs::write(folder.join("real.png"), png(1, 1)).unwrap();
        std::os::unix::fs::symlink(folder.join("real.png"), folder.join("alias.png")).unwrap();
        let assets = TemplateAssets::Folder(folder);

        assert!(assets.read(&image("real.png"), 1024).is_ok());
        for (raw, expected) in [
            ("img/x.png", "`img/x.png` leaves the template folder"),
            ("y.png", "`y.png` leaves the template folder"),
            ("alias.png", "`alias.png` leaves the template folder"),
        ] {
            assert_eq!(assets.read(&image(raw), 1024).unwrap_err().0, expected);
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(windows)]
    #[test]
    fn refuses_an_image_through_a_junction_out_of_the_folder() {
        let (root, folder) = template_folder("junction", &[]);
        let outside = root.join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("x.png"), png(1, 1)).unwrap();
        // `mklink /J` makes a junction without any privilege.
        let status = std::process::Command::new("cmd")
            .arg("/C")
            .arg("mklink")
            .arg("/J")
            .arg(folder.join("img"))
            .arg(&outside)
            .status()
            .unwrap();
        assert!(status.success(), "mklink /J failed");
        let assets = TemplateAssets::Folder(folder);

        assert_eq!(
            assets.read(&image("img/x.png"), 1024).unwrap_err().0,
            "`img/x.png` leaves the template folder"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn refuses_a_directory_named_like_an_image() {
        let (root, folder) = template_folder("directory", &[]);
        std::fs::create_dir_all(folder.join("x.png")).unwrap();
        let assets = TemplateAssets::Folder(folder);

        assert_eq!(
            assets.read(&image("x.png"), 1024).unwrap_err().0,
            "`x.png` is not a regular file"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn refuses_a_file_over_its_limit_before_reading() {
        let (root, folder) = template_folder("over-limit", &[("x.png", &[7u8; 11])]);
        let assets = TemplateAssets::Folder(folder);

        assert_eq!(
            assets.read(&image("x.png"), 10).unwrap_err().0,
            "`x.png` is too large: 11 bytes > 10 bytes"
        );
        assert_eq!(assets.read(&image("x.png"), 11).unwrap(), vec![7u8; 11]);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn reading_stops_past_the_limit() {
        let ten = [1u8; 10];
        assert_eq!(read_capped(&ten[..], 10).unwrap(), ten.to_vec());
        assert_eq!(
            read_capped(&[1u8; 11][..], 10).unwrap_err().0,
            "is larger than 10 bytes"
        );
        // An endless reader stops one byte past the limit.
        let endless = std::io::repeat(0);
        assert!(read_capped(endless, 1024).is_err());
    }
}
