//! Foreground-app tracking. Each OS implements `spawn` in its own module.
//!
//! The callback receives a platform app identifier: on Windows the lowercase exe name.
//! Planned: macOS via `NSWorkspace.didActivateApplicationNotification` (bundle id),
//! Linux/X11 via `_NET_ACTIVE_WINDOW`; Wayland needs a compositor extension.

pub type Callback = Box<dyn Fn(String) + Send + Sync + 'static>;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::{app_id_for_path, spawn, APP_EXTENSIONS};

#[cfg(not(windows))]
pub const APP_EXTENSIONS: &[&str] = &[];

#[cfg(not(windows))]
pub fn spawn(_on_focus: Callback) -> Result<(), String> {
    Err("focus tracking is not implemented on this platform yet".into())
}

#[cfg(not(windows))]
pub fn app_id_for_path(path: &std::path::Path) -> Option<String> {
    path.file_name().map(|n| n.to_string_lossy().into_owned())
}
