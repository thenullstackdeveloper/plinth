//! `plinth-infra` — adapters that touch the world. They implement the ports
//! defined in `plinth-application`.

pub mod clock;
pub mod config_fs;
#[cfg(target_os = "linux")]
pub mod input_uinput;
pub mod keybinds_sc;
pub mod template_fs;
pub mod window_placement;
