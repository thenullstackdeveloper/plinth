//! Packaging checks: the installer configuration, the app version and the
//! license have to agree across `tauri.conf.json`, Cargo and `ui/package.json`.

use std::path::Path;

use serde_json::{json, Value};

const LICENSE_ID: &str = "PolyForm-Noncommercial-1.0.0";
const REQUIRED_NOTICE: &str =
    "Required Notice: Copyright thenullstackdeveloper (https://github.com/thenullstackdeveloper)";

fn tauri_conf() -> Value {
    serde_json::from_str(include_str!("../tauri.conf.json")).expect("tauri.conf.json is valid JSON")
}

/// Reads a file relative to the repository root, at run time, so a missing
/// file fails an assertion instead of the build.
fn repo_file(path: &str) -> Option<String> {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path),
    )
    .ok()
}

fn ui_package() -> Value {
    let text = repo_file("ui/package.json").expect("ui/package.json exists");
    serde_json::from_str(&text).expect("ui/package.json is valid JSON")
}

#[test]
fn the_bundle_builds_a_current_user_nsis_installer() {
    let bundle = &tauri_conf()["bundle"];
    assert_eq!(bundle["active"], json!(true));
    assert_eq!(bundle["targets"], json!(["nsis"]));
    assert_eq!(
        bundle["windows"]["nsis"]["installMode"],
        json!("currentUser")
    );
}

#[test]
fn the_app_version_comes_from_cargo_and_is_0_4_0() {
    assert_eq!(env!("CARGO_PKG_VERSION"), "0.4.0");
    assert_eq!(ui_package()["version"], json!(env!("CARGO_PKG_VERSION")));
    assert_eq!(
        tauri_conf().get("version"),
        None,
        "tauri.conf.json must not set its own version: Tauri takes it from Cargo"
    );
}

#[test]
fn the_license_is_declared_the_same_everywhere() {
    assert_eq!(env!("CARGO_PKG_LICENSE"), LICENSE_ID);
    assert_eq!(tauri_conf()["bundle"]["license"], json!(LICENSE_ID));
    assert_eq!(ui_package()["license"], json!(LICENSE_ID));
}

#[test]
fn the_installer_shows_the_repository_license() {
    assert_eq!(
        tauri_conf()["bundle"]["licenseFile"],
        json!("../../LICENSE")
    );
    let license = repo_file("LICENSE").expect("LICENSE exists at the repository root");
    assert_eq!(license.lines().next(), Some(REQUIRED_NOTICE));
    assert!(license.contains("# PolyForm Noncommercial License 1.0.0"));
    assert!(license.contains("<https://polyformproject.org/licenses/noncommercial/1.0.0>"));
}

#[test]
fn the_build_compiles_the_ui_first() {
    assert_eq!(
        tauri_conf()["build"]["beforeBuildCommand"],
        json!("pnpm --dir ../ui build")
    );
}
