use tauri::Manager;

use crate::{
    app_state::AppState,
    locator::{
        activate_with, correlate_with_running_processes, LocatorActivationResult,
        LocatorActivationStatus, LocatorActivationTarget, NativeJumpExecutor,
    },
};

/// Attempts to return the user to an already-observed work surface. The command
/// only focuses a terminal/application or opens a project directory; it never
/// launches, resumes, or controls a coding agent.
#[tauri::command]
pub async fn activate_stove_locator(
    stove_id: String,
    app: tauri::AppHandle,
) -> Result<LocatorActivationResult, String> {
    let (locator, harness) = {
        let state = app.state::<AppState>();
        let Some(locator) = state.stoves.locator_for(&stove_id) else {
            return Ok(unavailable());
        };
        let Some(stove) = state.stoves.core_stove(&stove_id) else {
            return Ok(unavailable());
        };
        (locator, stove.identity.harness.clone())
    };
    let result = tauri::async_runtime::spawn_blocking(move || {
        let locator = correlate_with_running_processes(&harness, locator);
        activate_with(&locator, &mut NativeJumpExecutor)
    })
    .await
    .map_err(|_| "Cookbench could not complete the return attempt.".to_owned())?;
    Ok(result)
}

fn unavailable() -> LocatorActivationResult {
    LocatorActivationResult {
        target: LocatorActivationTarget::Unavailable,
        status: LocatorActivationStatus::Unavailable,
        resume_session_id: None,
    }
}
