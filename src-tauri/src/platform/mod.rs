//! Platform-neutral presentation primitives for Cookbench's floating bars.
//!
//! Cookbench only manages its own windows. It never raises, embeds, or controls
//! a coding harness window.

mod capabilities;
pub mod gnome_bridge;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub(crate) use macos::status_item_image_x;
mod overlay;
#[cfg(target_os = "windows")]
mod windows;

pub use capabilities::{
    capabilities_for, current_desktop_environment, DesktopEnvironment, OverlayCapabilities,
    OverlaySupport,
};
pub use overlay::{OverlayController, OverlayError, TauriOverlayController};

use crate::app_state::StoveSnapshot;

/// Delivers one immutable Stove snapshot to platform-owned presentation
/// surfaces. The snapshot has already been normalized and attention-ranked by
/// `AppState`; platform renderers must not derive their own order.
pub fn publish_presentation_snapshot<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    snapshot: &StoveSnapshot,
) {
    publish_optional_gnome_snapshot(snapshot);
    crate::desktop_shell::runtime::refresh_status_stoves_snapshot(app, snapshot);
}

pub fn publish_optional_gnome_snapshot(snapshot: &StoveSnapshot) {
    #[cfg(target_os = "linux")]
    if let Some(runtime_dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        let payload = gnome_bridge::GnomePresentationSnapshot::from(snapshot);
        let _ = gnome_bridge::write_presentation_file(std::path::Path::new(&runtime_dir), &payload);
    }

    #[cfg(not(target_os = "linux"))]
    let _ = snapshot;
}

pub fn clear_optional_gnome_snapshot() {
    #[cfg(target_os = "linux")]
    if let Some(runtime_dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        let _ = gnome_bridge::remove_presentation_file(std::path::Path::new(&runtime_dir));
    }
}

use tauri::{Runtime, WebviewWindow};

pub(crate) fn apply_platform_overlay<R: Runtime>(
    window: &WebviewWindow<R>,
) -> Result<(), OverlayError> {
    #[cfg(target_os = "macos")]
    return macos::apply_overlay(window);

    #[cfg(target_os = "windows")]
    return windows::apply_overlay(window);

    #[cfg(target_os = "linux")]
    return linux::apply_overlay(window);

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = window;
        Err(OverlayError::UnsupportedPlatform)
    }
}
