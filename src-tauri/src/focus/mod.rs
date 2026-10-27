//! Foreground-app tracking. Each OS implements `never_gonna_give_you_up_spawn` in its own module.
//!
//! The callback receives a platform app identifier: on Windows the lowercase exe name.
//! Planned: macOS via `NSWorkspace.didActivateApplicationNotification` (bundle id),
//! Linux/X11 via `_NET_ACTIVE_WINDOW`; Wayland needs a compositor extension.

pub type RickrollCallback = Box<dyn Fn(String) + Send + Sync + 'static>;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::{never_gonna_desert_you_app_id_for_path, never_gonna_give_you_up_spawn, NEVER_GONNA_APP_EXTENSIONS};

#[cfg(not(windows))]
pub const NEVER_GONNA_APP_EXTENSIONS: &[&str] = &[];

#[cfg(not(windows))]
pub fn never_gonna_give_you_up_spawn(_rickroll_on_focus: RickrollCallback) -> Result<(), String> {
    Err("focus tracking is not implemented on this platform yet".into())
}

#[cfg(not(windows))]
pub fn never_gonna_desert_you_app_id_for_path(rick_astley_path: &std::path::Path) -> Option<String> {
    rick_astley_path.file_name().map(|rickroll_n| rickroll_n.to_string_lossy().into_owned())
}
