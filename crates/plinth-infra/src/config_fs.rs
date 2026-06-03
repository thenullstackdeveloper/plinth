//! `ConfigStore` sobre el sistema de ficheros (JSON bajo el dir de config XDG).

use std::path::PathBuf;

use plinth_application::{ConfigError, ConfigStore, Prefs};
use plinth_domain::SidebarSide;
use serde::{Deserialize, Serialize};

/// Mirror serializable de `Prefs`. Mantiene `serde` fuera de la capa de
/// aplicación: el mapeo dominio↔disco vive aquí.
#[derive(Serialize, Deserialize)]
struct PrefsDto {
    sidebar_side: String,
    last_template: Option<String>,
}

pub struct FsConfigStore {
    path: PathBuf,
}

impl FsConfigStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Ubicación por defecto: `$XDG_CONFIG_HOME/plinth/prefs.json`.
    pub fn at_default_location() -> Result<Self, ConfigError> {
        let dir = dirs::config_dir()
            .ok_or_else(|| ConfigError("no se pudo determinar el directorio de config".into()))?;
        Ok(Self::new(dir.join("plinth").join("prefs.json")))
    }
}

fn side_to_str(side: SidebarSide) -> &'static str {
    match side {
        SidebarSide::Left => "left",
        SidebarSide::Right => "right",
    }
}

fn side_from_str(s: &str) -> Result<SidebarSide, ConfigError> {
    match s {
        "left" => Ok(SidebarSide::Left),
        "right" => Ok(SidebarSide::Right),
        other => Err(ConfigError(format!("lado de sidebar inválido: {other}"))),
    }
}

impl ConfigStore for FsConfigStore {
    fn load(&self) -> Result<Prefs, ConfigError> {
        let data = std::fs::read_to_string(&self.path).map_err(|e| ConfigError(e.to_string()))?;
        let dto: PrefsDto = serde_json::from_str(&data).map_err(|e| ConfigError(e.to_string()))?;
        Ok(Prefs {
            sidebar_side: side_from_str(&dto.sidebar_side)?,
            last_template: dto.last_template,
        })
    }

    fn save(&self, prefs: &Prefs) -> Result<(), ConfigError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ConfigError(e.to_string()))?;
        }
        let dto = PrefsDto {
            sidebar_side: side_to_str(prefs.sidebar_side).to_string(),
            last_template: prefs.last_template.clone(),
        };
        let data = serde_json::to_string_pretty(&dto).map_err(|e| ConfigError(e.to_string()))?;
        std::fs::write(&self.path, data).map_err(|e| ConfigError(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let mut path = std::env::temp_dir();
        path.push(format!("plinth-test-{}", std::process::id()));
        path.push("prefs.json");
        let store = FsConfigStore::new(path.clone());

        let prefs = Prefs {
            sidebar_side: SidebarSide::Left,
            last_template: Some("sc_combat.yaml".into()),
        };
        store.save(&prefs).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded, prefs);

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
