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

/// Evidence from a single native drag gesture. This deliberately never uses a
/// process-wide hook: callers may only wait while they own an active token.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DragReleaseEvidence {
    Released,
    Unavailable,
}

/// Converts a bounded platform probe into completion evidence. A successful
/// first probe that already sees Button1 up represents a quick release.
#[cfg(any(target_os = "windows", target_os = "linux", test))]
pub(crate) fn classify_drag_release(
    initially_down: bool,
    released: bool,
    timed_out: bool,
) -> DragReleaseEvidence {
    if released || !initially_down {
        DragReleaseEvidence::Released
    } else {
        debug_assert!(timed_out);
        DragReleaseEvidence::Unavailable
    }
}

pub fn wait_for_local_drag_release() -> DragReleaseEvidence {
    #[cfg(target_os = "windows")]
    return windows::wait_for_left_release();
    #[cfg(target_os = "linux")]
    return linux::wait_for_left_release();
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    DragReleaseEvidence::Unavailable
}

#[cfg(test)]
mod drag_release_tests {
    use super::*;

    #[test]
    fn quick_release_is_completion_evidence() {
        assert_eq!(
            classify_drag_release(false, false, false),
            DragReleaseEvidence::Released
        );
    }

    #[test]
    fn held_button_timeout_is_unavailable() {
        assert_eq!(
            classify_drag_release(true, false, true),
            DragReleaseEvidence::Unavailable
        );
    }
}

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
    crate::desktop_shell::runtime::queue_status_stoves_refresh(app, snapshot.clone());
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
