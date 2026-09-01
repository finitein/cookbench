use tauri::{Runtime, WebviewWindow};

use super::OverlayError;

/// Tauri maps this to a floating AppKit window. This presentation does not use
/// Automation or Accessibility APIs; those permissions are reserved for a
/// future exact-host-jump feature.
pub(super) fn apply_overlay<R: Runtime>(window: &WebviewWindow<R>) -> Result<(), OverlayError> {
    window.set_always_on_top(true)?;
    window.set_visible_on_all_workspaces(true)?;
    Ok(())
}
