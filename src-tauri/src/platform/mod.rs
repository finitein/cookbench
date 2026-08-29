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
mod overlay;
#[cfg(target_os = "windows")]
mod windows;

pub use capabilities::{
    capabilities_for, current_desktop_environment, DesktopEnvironment, OverlayCapabilities,
    OverlaySupport,
};
pub use overlay::{OverlayController, OverlayError, TauriOverlayController};

use crate::app_state::StoveSnapshot;

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
