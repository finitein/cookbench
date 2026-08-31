use std::sync::atomic::{AtomicBool, Ordering};

use cookbench_core::locator::HostApplication;
use tauri::Manager;

use crate::{
    app_state::AppState,
    locator::{
        activate_with, correlate_with_running_processes, LocatorActivationResult,
        LocatorActivationStatus, LocatorActivationTarget, NativeJumpExecutor,
    },
};

static CLI_ACTIVATION_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

struct CliActivationGuard;

impl CliActivationGuard {
    fn try_acquire() -> Option<Self> {
        CLI_ACTIVATION_IN_FLIGHT
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .ok()
            .map(|_| Self)
    }
}

impl Drop for CliActivationGuard {
    fn drop(&mut self) {
        CLI_ACTIVATION_IN_FLIGHT.store(false, Ordering::Release);
    }
}

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
    let _cli_guard = if is_codex_desktop(locator.host_application.as_ref()) {
        None
    } else {
        let Some(guard) = CliActivationGuard::try_acquire() else {
            return Ok(unavailable());
        };
        Some(guard)
    };
    let result = tauri::async_runtime::spawn_blocking(move || {
        let locator = correlate_with_running_processes(&harness, locator);
        activate_with(&locator, &mut NativeJumpExecutor)
    })
    .await
    .map_err(|_| "Cookbench could not complete the return attempt.".to_owned())?;
    Ok(result)
}

fn is_codex_desktop(host_application: Option<&HostApplication>) -> bool {
    matches!(host_application, Some(HostApplication::CodexDesktop))
}

fn unavailable() -> LocatorActivationResult {
    LocatorActivationResult {
        target: LocatorActivationTarget::Unavailable,
        status: LocatorActivationStatus::Unavailable,
        resume_session_id: None,
    }
}

#[cfg(test)]
mod tests {
    use cookbench_core::locator::HostApplication;

    use super::{is_codex_desktop, CliActivationGuard};

    #[test]
    fn cli_activation_guard_rejects_a_concurrent_return_attempt_and_releases_afterward() {
        let first = CliActivationGuard::try_acquire().expect("first attempt acquires the guard");
        assert!(CliActivationGuard::try_acquire().is_none());
        drop(first);
        assert!(CliActivationGuard::try_acquire().is_some());
    }

    #[test]
    fn only_codex_desktop_bypasses_the_cli_activation_guard() {
        assert!(is_codex_desktop(Some(&HostApplication::CodexDesktop)));
        assert!(!is_codex_desktop(Some(&HostApplication::MacosTerminal)));
        assert!(!is_codex_desktop(None));
    }
}
