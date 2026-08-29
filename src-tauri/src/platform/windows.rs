use tauri::{Runtime, WebviewWindow};

use super::OverlayError;

/// A topmost Cookbench-owned window is supported by ordinary Win32 window APIs
/// and does not require elevation.
pub(super) fn apply_overlay<R: Runtime>(window: &WebviewWindow<R>) -> Result<(), OverlayError> {
    window.set_always_on_top(true)?;
    Ok(())
}
