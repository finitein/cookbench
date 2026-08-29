use tauri::State;

use crate::app_state::{AppState, StoveSnapshot};

/// Returns the complete, revisioned stove view used at startup and after an
/// incremental-event gap. It contains Cookbench presentation metadata only.
#[tauri::command]
pub fn get_stoves_snapshot(state: State<'_, AppState>) -> StoveSnapshot {
    state.stoves.snapshot()
}
