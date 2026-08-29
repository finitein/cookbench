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

use cookbench_core::locator::{HostApplication, SessionLocator, TerminalKind};

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
