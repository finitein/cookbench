use tauri::{PhysicalPosition, Position, Runtime, WebviewWindow};

use super::OverlayError;

/// Tauri maps this to a floating AppKit window. This presentation does not use
/// Automation or Accessibility APIs; those permissions are reserved for a
/// future exact-host-jump feature.
pub(super) fn apply_overlay<R: Runtime>(window: &WebviewWindow<R>) -> Result<(), OverlayError> {
    window.set_always_on_top(true)?;
    window.set_visible_on_all_workspaces(true)?;
    Ok(())
}

/// Converts Tauri's status-item event coordinates to image-local physical
/// pixels. Tauri intentionally exposes the item rectangle as a DPI position;
/// the click position is already physical.
pub(crate) fn status_item_image_x(click: PhysicalPosition<f64>, item_position: Position) -> f64 {
    click.x - f64::from(item_position.to_physical::<i32>(1.0).x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_item_coordinates_are_relative_to_the_item_origin() {
        assert_eq!(
            status_item_image_x(
                PhysicalPosition::new(112.0, 8.0),
                Position::Physical((100, 0).into())
            ),
            12.0
        );
    }
}
