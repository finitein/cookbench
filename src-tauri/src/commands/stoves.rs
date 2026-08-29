use tauri::{AppHandle, Manager, State};

use crate::app_state::{AppState, StoveSnapshot};

/// Returns the complete, revisioned stove view used at startup and after an
/// incremental-event gap. It contains Cookbench presentation metadata only.
#[tauri::command]
pub fn get_stoves_snapshot(state: State<'_, AppState>) -> StoveSnapshot {
    state.stoves.snapshot()
}

/// Clears Cookbench's retained presentation only. Native session files and
/// harness processes are never modified or deleted.
#[tauri::command]
pub fn clear_cooked_stove(
    stove_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
    windows: State<'_, super::windows::TauriWindowCommandService>,
) -> Result<(), String> {
    let identity = state
        .stoves
        .core_stove(&stove_id)
        .map(|stove| stove.identity);
    state
        .clear_cooked_and_emit(&app, &stove_id)
        .map_err(|error| error.to_string())?;
    if let (Some(identity), Some(remote)) = (
        identity,
        app.try_state::<crate::remote::runtime::RemoteRuntimeState>(),
    ) {
        remote.forget(identity);
    }
    windows
        .clear_stove(&stove_id)
        .map_err(|error| error.to_string())?;
    super::windows::persist_layouts(&state, &windows)
}
