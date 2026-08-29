use cookbench_core::locator::{HostApplication, SessionLocator, TerminalKind};
use cookbench_desktop_lib::locator::{
    actions_for, activate_with, jump_with, JumpAction, JumpExecutor, JumpOutcome,
    LocatorActivationStatus, LocatorActivationTarget,
};

#[derive(Default)]
struct RecordingExecutor {
    outcomes: Vec<JumpOutcome>,
    seen: Vec<JumpAction>,
}

impl RecordingExecutor {
    fn with_outcomes(outcomes: impl IntoIterator<Item = JumpOutcome>) -> Self {
        let mut outcomes: Vec<_> = outcomes.into_iter().collect();
        outcomes.reverse();
        Self {
            outcomes,
            seen: Vec::new(),
        }
    }
}

impl JumpExecutor for RecordingExecutor {
    fn perform(&mut self, action: &JumpAction) -> JumpOutcome {
        self.seen.push(action.clone());
        self.outcomes.pop().unwrap_or(JumpOutcome::Unavailable)
    }
}

fn locator() -> SessionLocator {
    SessionLocator {
        terminal: Some(TerminalKind::Tmux),
        tmux_pane: Some("%42".to_owned()),
        host_application: Some(HostApplication::MacosTerminal),
        working_directory: Some("/workspace/cookbench".to_owned()),
        native_session_id: "opaque-session-id".to_owned(),
        ..SessionLocator::default()
    }
}

#[test]
fn orders_exact_pane_then_application_then_project_then_resume() {
    let actions = actions_for(&locator());
    assert!(matches!(actions[0], JumpAction::ExactPane { .. }));
    assert!(matches!(actions[1], JumpAction::ApplicationWindow { .. }));
    assert!(matches!(actions[2], JumpAction::ProjectDirectory { .. }));
    assert!(matches!(actions[3], JumpAction::ResumeInstructions { .. }));
}

#[test]
fn permission_denial_continues_to_project_directory() {
    let mut executor = RecordingExecutor::with_outcomes([
        JumpOutcome::PermissionDenied,
        JumpOutcome::PermissionDenied,
        JumpOutcome::Focused,
    ]);

    let result = jump_with(&locator(), &mut executor);
    assert!(matches!(result.action, JumpAction::ProjectDirectory { .. }));
    assert_eq!(executor.seen.len(), 3);
}

#[test]
fn elevated_target_continues_to_resume_instructions() {
    let mut executor = RecordingExecutor::with_outcomes([
        JumpOutcome::Unavailable,
        JumpOutcome::ElevatedTarget,
        JumpOutcome::Unavailable,
        JumpOutcome::Focused,
    ]);

    let result = jump_with(&locator(), &mut executor);
    assert!(matches!(
        result.action,
        JumpAction::ResumeInstructions { .. }
    ));
    assert_eq!(executor.seen.len(), 4);
}

#[test]
fn unsupported_terminals_do_not_claim_exact_tab_focus() {
    let mut locator = locator();
    locator.terminal = Some(TerminalKind::MacosTerminal);
    assert!(!matches!(
        actions_for(&locator).first(),
        Some(JumpAction::ExactPane { .. })
    ));
}

#[test]
fn tmux_target_is_an_argument_not_shell_source() {
    let action = actions_for(&locator()).remove(0);
    assert_eq!(
        action,
        JumpAction::ExactPane {
            program: "tmux",
            args: vec!["select-pane".to_owned(), "-t".to_owned(), "%42".to_owned()],
        }
    );
}

#[test]
fn unsafe_tmux_target_is_not_used_for_a_command() {
    let mut locator = locator();
    locator.tmux_pane = Some("%42\nrun-unrelated-command".to_owned());

    assert!(matches!(
        actions_for(&locator).as_slice(),
        [JumpAction::ResumeInstructions { .. }]
    ));
}

#[test]
fn reports_available_when_a_precise_target_is_focused() {
    let mut executor = RecordingExecutor::with_outcomes([JumpOutcome::Focused]);
    let result = activate_with(&locator(), &mut executor);

    assert_eq!(result.target, LocatorActivationTarget::ExactPane);
    assert_eq!(result.status, LocatorActivationStatus::Focused);
    assert_eq!(result.resume_session_id, None);
}

#[test]
fn reports_visible_resume_after_permission_and_elevation_fallbacks() {
    let mut executor = RecordingExecutor::with_outcomes([
        JumpOutcome::PermissionDenied,
        JumpOutcome::ElevatedTarget,
        JumpOutcome::Unavailable,
        JumpOutcome::VisibleFallback,
    ]);
    let result = activate_with(&locator(), &mut executor);

    assert_eq!(result.target, LocatorActivationTarget::ResumeInstructions);
    assert_eq!(result.status, LocatorActivationStatus::VisibleFallback);
    assert_eq!(
        result.resume_session_id.as_deref(),
        Some("opaque-session-id")
    );
}
