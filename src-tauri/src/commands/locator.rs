use tauri::State;

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
pub fn activate_stove_locator(
    stove_id: String,
    state: State<'_, AppState>,
) -> LocatorActivationResult {
    let Some(locator) = state.stoves.locator_for(&stove_id) else {
        return unavailable();
    };
    let Some(stove) = state.stoves.core_stove(&stove_id) else {
        return unavailable();
    };
    let locator = correlate_with_running_processes(&stove.identity.harness, locator);

    activate_with(&locator, &mut NativeJumpExecutor)
}

fn unavailable() -> LocatorActivationResult {
    LocatorActivationResult {
        target: LocatorActivationTarget::Unavailable,
        status: LocatorActivationStatus::Unavailable,
        resume_session_id: None,
    }
}
