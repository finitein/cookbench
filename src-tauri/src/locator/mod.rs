//! Platform-neutral jump fallback orchestration.
//!
//! Actions are generated as bounded program/argument vectors or native focus
//! requests. This module never runs a shell, opens a port, or controls an
//! agent. Platform backends may decline any request and fall through.

pub mod linux;
pub mod macos;
pub mod tmux;
pub mod vscode;
pub mod windows;

use std::process::Command;

use cookbench_core::locator::{HostApplication, SessionLocator, TerminalKind};
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JumpAction {
    ExactPane {
        program: &'static str,
        args: Vec<String>,
    },
    ApplicationWindow {
        application: &'static str,
    },
    ProjectDirectory {
        path: String,
    },
    ResumeInstructions {
        native_session_id: String,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JumpOutcome {
    Focused,
    VisibleFallback,
    Unavailable,
    PermissionDenied,
    ElevatedTarget,
    Failed,
}

pub trait JumpExecutor {
    fn perform(&mut self, action: &JumpAction) -> JumpOutcome;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JumpResult {
    pub action: JumpAction,
    pub outcome: JumpOutcome,
}

/// The only data sent to the UI after an activation attempt. It never exposes
/// command arguments, paths, terminal metadata, or a native locator record.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocatorActivationResult {
    pub target: LocatorActivationTarget,
    pub status: LocatorActivationStatus,
    pub resume_session_id: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LocatorActivationTarget {
    ExactPane,
    ApplicationWindow,
    ProjectDirectory,
    ResumeInstructions,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LocatorActivationStatus {
    Focused,
    VisibleFallback,
    Unavailable,
}

/// Runs the documented order. A permission boundary or an elevated target is
/// not a terminal failure: users still receive a lower-precision jump option.
pub fn jump_with<E: JumpExecutor>(locator: &SessionLocator, executor: &mut E) -> JumpResult {
    for action in actions_for(locator) {
        let outcome = executor.perform(&action);
        if outcome == JumpOutcome::Focused
            || matches!(action, JumpAction::ResumeInstructions { .. })
        {
            return JumpResult { action, outcome };
        }
    }

    unreachable!("a locator always ends with resume instructions")
}

pub fn activate_with<E: JumpExecutor>(
    locator: &SessionLocator,
    executor: &mut E,
) -> LocatorActivationResult {
    let JumpResult { action, outcome } = jump_with(locator, executor);
    let (target, resume_session_id) = match action {
        JumpAction::ExactPane { .. } => (LocatorActivationTarget::ExactPane, None),
        JumpAction::ApplicationWindow { .. } => (LocatorActivationTarget::ApplicationWindow, None),
        JumpAction::ProjectDirectory { .. } => (LocatorActivationTarget::ProjectDirectory, None),
        JumpAction::ResumeInstructions { native_session_id } => (
            LocatorActivationTarget::ResumeInstructions,
            Some(native_session_id),
        ),
    };
    let status = match outcome {
        JumpOutcome::Focused => LocatorActivationStatus::Focused,
        JumpOutcome::VisibleFallback => LocatorActivationStatus::VisibleFallback,
        JumpOutcome::Unavailable
        | JumpOutcome::PermissionDenied
        | JumpOutcome::ElevatedTarget
        | JumpOutcome::Failed => LocatorActivationStatus::Unavailable,
    };
    LocatorActivationResult {
        target,
        status,
        resume_session_id,
    }
}

/// Executes only Cookbench-owned focus attempts. `Command` receives a program
/// and argument vector directly, so locator values are never shell source.
pub struct NativeJumpExecutor;

impl JumpExecutor for NativeJumpExecutor {
    fn perform(&mut self, action: &JumpAction) -> JumpOutcome {
        match action {
            JumpAction::ExactPane { program, args } => run(program, args),
            JumpAction::ApplicationWindow { application } => activate_application(application),
            JumpAction::ProjectDirectory { path } => open_project_directory(path),
            JumpAction::ResumeInstructions { .. } => JumpOutcome::VisibleFallback,
        }
    }
}

fn run(program: &str, args: &[String]) -> JumpOutcome {
    match Command::new(program).args(args).status() {
        Ok(status) if status.success() => JumpOutcome::Focused,
        Ok(_) => JumpOutcome::Unavailable,
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            JumpOutcome::PermissionDenied
        }
        Err(_) => JumpOutcome::Unavailable,
    }
}

#[cfg(target_os = "macos")]
fn activate_application(application: &str) -> JumpOutcome {
    let flag = if application.contains('.') {
        "-b"
    } else {
        "-a"
    };
    run("open", &[flag.to_owned(), application.to_owned()])
}

#[cfg(not(target_os = "macos"))]
fn activate_application(_application: &str) -> JumpOutcome {
    // Windows elevation and Linux compositor focus policies cannot be inferred
    // safely without platform APIs. Continue to the next documented fallback.
    JumpOutcome::Unavailable
}

#[cfg(target_os = "macos")]
fn open_project_directory(path: &str) -> JumpOutcome {
    run("open", &[path.to_owned()])
}

#[cfg(target_os = "windows")]
fn open_project_directory(path: &str) -> JumpOutcome {
    run("explorer.exe", &[path.to_owned()])
}

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
fn open_project_directory(path: &str) -> JumpOutcome {
    run("xdg-open", &[path.to_owned()])
}

/// The truthful fallback sequence for a locator. Unsupported terminal and IDE
/// integrations simply omit an exact target rather than pretending precision.
pub fn actions_for(locator: &SessionLocator) -> Vec<JumpAction> {
    if locator.validate().is_err() {
        return vec![JumpAction::ResumeInstructions {
            native_session_id: locator.native_session_id.clone(),
        }];
    }

    let mut actions = Vec::with_capacity(4);

    if matches!(locator.terminal.as_ref(), Some(TerminalKind::Tmux)) {
        if let Some(action) = tmux::exact_pane_action(locator.tmux_pane.as_deref()) {
            actions.push(action);
        }
    }

    if let Some(application) = application_for(locator.host_application.as_ref()) {
        actions.push(JumpAction::ApplicationWindow { application });
    }

    if let Some(path) = locator
        .ide_workspace
        .as_ref()
        .or(locator.working_directory.as_ref())
    {
        actions.push(JumpAction::ProjectDirectory { path: path.clone() });
    }

    actions.push(JumpAction::ResumeInstructions {
        native_session_id: locator.native_session_id.clone(),
    });
    actions
}

fn application_for(application: Option<&HostApplication>) -> Option<&'static str> {
    match application {
        Some(HostApplication::CodexDesktop) => Some(macos::CODEX_DESKTOP_BUNDLE_ID),
        Some(HostApplication::MacosTerminal) => Some(macos::TERMINAL_APP),
        Some(HostApplication::ITerm2) => Some(macos::ITERM_APP),
        Some(HostApplication::WindowsTerminal) => Some(windows::WINDOWS_TERMINAL_APP),
        Some(HostApplication::VisualStudioCode) => Some(vscode::VSCODE_APP),
        Some(HostApplication::GnomeTerminal) => Some(linux::GNOME_TERMINAL_APP),
        Some(HostApplication::Konsole) => Some(linux::KONSOLE_APP),
        Some(HostApplication::XfceTerminal) => Some(linux::XFCE_TERMINAL_APP),
        Some(HostApplication::Other(_)) | None => None,
    }
}
