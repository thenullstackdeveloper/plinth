//! `plinth-infra` — adapters that touch the world. They implement the ports
//! defined in `plinth-application`.

pub mod clock;
pub mod config_fs;
pub mod design_tokens;
pub mod input_log;
#[cfg(target_os = "linux")]
pub mod input_uinput;
#[cfg(target_os = "windows")]
pub mod input_windows;
pub mod key_names;
pub mod keybinds_sc;
pub mod keybinds_static;
pub mod template_dir;
pub mod template_fs;
pub mod theme_scalars;
pub mod window_placement;
