use std::time::SystemTime;

use cookbench_core::domain::HostIdentity;
use tauri::{AppHandle, Manager, State};

use crate::app_state::{AppState, ArchivedSessionWire, StoveSnapshot};

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

#[tauri::command]
pub fn set_stove_pinned(
    stove_id: String,
    pinned: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let pinned_path = pinned
        .then(|| {
            state
                .stoves
                .locator_for(&stove_id)
                .and_then(|locator| locator.native_locator)
        })
        .flatten();
    state
        .set_pinned_and_emit(&app, &stove_id, pinned)
        .map_err(|error| error.to_string())?;
    if let (Some(path), Some(runtime)) = (pinned_path, app.try_state::<crate::LocalRuntimeState>())
    {
        runtime.add_pinned_path(path.into());
    }
    Ok(())
}

/// Removes only Cookbench's presentation of a non-Cooked session and records
/// a metadata-only archive entry. The native session is never modified.
#[tauri::command]
pub fn archive_stove(
    stove_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .archive_stove_and_emit(&app, &stove_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_archived_sessions(
    state: State<'_, AppState>,
) -> Result<Vec<ArchivedSessionWire>, String> {
    let config =
        crate::runtime::LocalObservationConfig::from_environment(HostIdentity::local("local"));
    let expired = crate::runtime::archive_inventory::discover_expired_local_sessions(
        &config,
        SystemTime::now(),
        4_096,
    );
    state
        .import_expired_sessions(expired)
        .map_err(|error| error.to_string())?;
    Ok(state.archived_sessions())
}

#[tauri::command]
pub fn restore_archived_session(
    stove_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .restore_archived_and_emit(&app, &stove_id)
        .map_err(|error| error.to_string())?;
    if let (Some(path), Some(runtime)) = (
        state
            .stoves
            .locator_for(&stove_id)
            .and_then(|locator| locator.native_locator),
        app.try_state::<crate::LocalRuntimeState>(),
    ) {
        runtime.refresh_path(path.into());
    }
    Ok(())
}
