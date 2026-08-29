use tauri::{Runtime, WebviewWindow};

use super::OverlayError;

pub(super) fn apply_overlay<R: Runtime>(window: &WebviewWindow<R>) -> Result<(), OverlayError> {
    if is_wayland_session() {
        // A Wayland compositor can ignore an application's keep-above request.
        // Show the fully functional bar, but never claim overlay success.
        return Err(OverlayError::BestEffortWayland);
    }

    // On X11 Tauri's always-on-top request maps to the window manager's EWMH
    // keep-above hint. The window manager remains the final authority.
    window.set_always_on_top(true)?;
    Ok(())
}

fn is_wayland_session() -> bool {
    std::env::var("XDG_SESSION_TYPE").is_ok_and(|value| value.eq_ignore_ascii_case("wayland"))
        || std::env::var_os("WAYLAND_DISPLAY").is_some()
}
