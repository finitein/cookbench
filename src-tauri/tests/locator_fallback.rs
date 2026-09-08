use cookbench_core::locator::{HostApplication, SessionLocator, TerminalKind};
use cookbench_desktop_lib::locator::{
    actions_for, activate_with, correlate_terminal_locator, jump_with, JumpAction, JumpExecutor,
    JumpOutcome, LocatorActivationStatus, LocatorActivationTarget, ObservedProcess,
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
        self.outcomes.pop().unwrap_or(JumpOutcome::Unsupported)
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
fn codex_thread_deep_link_precedes_application_and_project_fallbacks() {
    let locator = SessionLocator {
        host_application: Some(HostApplication::CodexDesktop),
        working_directory: Some("/workspace/cookbench".to_owned()),
        native_session_id: "opaque-codex-session".to_owned(),
        ..SessionLocator::default()
    };

    assert!(matches!(
        actions_for(&locator).as_slice(),
        [
            JumpAction::CodexDesktopThread {
                native_session_id
            },
            JumpAction::ApplicationWindow {
                application: "com.openai.codex"
            },
            JumpAction::ProjectDirectory { .. },
            JumpAction::ResumeInstructions { .. },
        ] if native_session_id == "opaque-codex-session"
    ));
}

#[test]
fn accepted_codex_deep_link_reports_the_exact_thread_request_without_claiming_focus() {
    let mut locator = locator();
    locator.host_application = Some(HostApplication::CodexDesktop);
    locator.terminal = None;
    locator.tmux_pane = None;
    let mut executor = RecordingExecutor::with_outcomes([JumpOutcome::VisibleFallback]);

    let result = activate_with(&locator, &mut executor);

    assert_eq!(result.target, LocatorActivationTarget::ExactThread);
    assert_eq!(result.status, LocatorActivationStatus::VisibleFallback);
}

#[test]
fn claude_and_pi_can_correlate_one_running_terminal_without_reading_command_arguments() {
    let processes = vec![
        ObservedProcess::new(
            10,
            1,
            None,
            "/System/Applications/Utilities/Terminal.app/Contents/MacOS/Terminal",
            None,
        ),
        ObservedProcess::new(20, 10, Some("ttys004"), "login", None),
        ObservedProcess::new(
            30,
            20,
            Some("ttys004"),
            "claude",
            Some("/workspace/cookbench"),
        ),
    ];
    let base = SessionLocator {
        working_directory: Some("/workspace/cookbench".to_owned()),
        native_session_id: "opaque-claude-session".to_owned(),
        ..SessionLocator::default()
    };

    let correlated = correlate_terminal_locator(
        &cookbench_core::domain::HarnessId::ClaudeCode,
        base,
        &processes,
    );

    assert_eq!(
        correlated.host_application,
        Some(HostApplication::MacosTerminal)
    );
    assert_eq!(correlated.terminal, Some(TerminalKind::MacosTerminal));
    assert_eq!(correlated.tty.as_deref(), Some("/dev/ttys004"));
    assert!(matches!(
        actions_for(&correlated).first(),
        Some(JumpAction::ExactTerminalTab { .. })
    ));
}

#[test]
fn catalog_terminal_harnesses_use_allowlisted_process_names_for_exact_return() {
    let processes = vec![
        ObservedProcess::new(10, 1, None, "Terminal", None),
        ObservedProcess::new(20, 10, Some("ttys014"), "zsh", None),
        ObservedProcess::new(
            30,
            20,
            Some("ttys014"),
            "qwen",
            Some("/workspace/cookbench"),
        ),
    ];
    let base = SessionLocator {
        working_directory: Some("/workspace/cookbench".to_owned()),
        native_session_id: "qwen-session".to_owned(),
        ..SessionLocator::default()
    };

    let correlated = correlate_terminal_locator(
        &cookbench_core::domain::HarnessId::Other("qwen_code".into()),
        base,
        &processes,
    );
    assert_eq!(correlated.tty.as_deref(), Some("/dev/ttys014"));
    assert_eq!(
        correlated.host_application,
        Some(HostApplication::MacosTerminal)
    );
}

#[test]
fn presence_only_profiles_never_claim_an_exact_terminal() {
    let processes = vec![
        ObservedProcess::new(10, 1, None, "Terminal", None),
        ObservedProcess::new(
            30,
            10,
            Some("ttys015"),
            "workbuddy",
            Some("/workspace/cookbench"),
        ),
    ];
    let base = SessionLocator {
        working_directory: Some("/workspace/cookbench".to_owned()),
        native_session_id: "workbuddy-presence".to_owned(),
        ..SessionLocator::default()
    };
    let correlated = correlate_terminal_locator(
        &cookbench_core::domain::HarnessId::Other("workbuddy".into()),
        base.clone(),
        &processes,
    );
    assert_eq!(correlated, base);
}

#[test]
fn ambiguous_terminal_processes_do_not_claim_an_exact_session() {
    let processes = vec![
        ObservedProcess::new(10, 1, None, "Terminal", None),
        ObservedProcess::new(
            20,
            10,
            Some("ttys004"),
            "claude",
            Some("/workspace/cookbench"),
        ),
        ObservedProcess::new(
            21,
            10,
            Some("ttys005"),
            "claude",
            Some("/workspace/cookbench"),
        ),
    ];
    let base = SessionLocator {
        working_directory: Some("/workspace/cookbench".to_owned()),
        native_session_id: "opaque-claude-session".to_owned(),
        ..SessionLocator::default()
    };

    let correlated = correlate_terminal_locator(
        &cookbench_core::domain::HarnessId::ClaudeCode,
        base,
        &processes,
    );

    assert_eq!(correlated.tty, None);
    assert!(!matches!(
        actions_for(&correlated).first(),
        Some(JumpAction::ExactTerminalTab { .. })
    ));
}

#[test]
fn iterm_is_an_exact_terminal_capability() {
    let locator = SessionLocator {
        host_application: Some(HostApplication::ITerm2),
        terminal: Some(TerminalKind::ITerm2),
        tty: Some("/dev/ttys008".to_owned()),
        native_session_id: "opaque-terminal-session".to_owned(),
        ..SessionLocator::default()
    };

    assert!(matches!(
        actions_for(&locator).first(),
        Some(JumpAction::ExactTerminalTab {
            terminal: TerminalKind::ITerm2,
            tty,
        }) if tty == "/dev/ttys008"
    ));
}

#[test]
fn correlates_an_iterm_host_without_using_window_titles() {
    let processes = vec![
        ObservedProcess::new(
            10,
            1,
            None,
            "/Applications/iTerm.app/Contents/MacOS/iTerm2",
            None,
        ),
        ObservedProcess::new(20, 10, Some("ttys008"), "zsh", None),
        ObservedProcess::new(30, 20, Some("ttys008"), "pi", Some("/workspace/cookbench")),
    ];
    let base = SessionLocator {
        working_directory: Some("/workspace/cookbench".to_owned()),
        native_session_id: "opaque-pi-session".to_owned(),
        ..SessionLocator::default()
    };

    let correlated =
        correlate_terminal_locator(&cookbench_core::domain::HarnessId::Pi, base, &processes);

    assert_eq!(correlated.host_application, Some(HostApplication::ITerm2));
    assert_eq!(correlated.terminal, Some(TerminalKind::ITerm2));
    assert_eq!(correlated.tty.as_deref(), Some("/dev/ttys008"));
}

#[test]
fn pi_project_metadata_selects_the_matching_terminal_from_multiple_pi_sessions() {
    let processes = vec![
        ObservedProcess::new(10, 1, None, "Terminal", None),
        ObservedProcess::new(30, 10, Some("ttys008"), "pi", Some("/workspace/cookbench")),
        ObservedProcess::new(
            31,
            10,
            Some("ttys009"),
            "pi",
            Some("/workspace/another-project"),
        ),
    ];
    let base = SessionLocator {
        working_directory: Some("/workspace/cookbench".to_owned()),
        native_session_id: "opaque-pi-session".to_owned(),
        ..SessionLocator::default()
    };

    let correlated =
        correlate_terminal_locator(&cookbench_core::domain::HarnessId::Pi, base, &processes);

    assert_eq!(correlated.tty.as_deref(), Some("/dev/ttys008"));
}

fn grok_locator(session_id: &str) -> SessionLocator {
    SessionLocator {
        native_locator: Some(format!(
            "/synthetic/.grok/sessions/project/{session_id}/updates.jsonl"
        )),
        native_session_id: session_id.to_owned(),
        working_directory: Some("/workspace/cookbench".to_owned()),
        ..SessionLocator::default()
    }
}

fn grok_process(process_id: u32, tty: &str) -> ObservedProcess {
    ObservedProcess::new(
        process_id,
        10,
        Some(tty),
        "grok",
        Some("/workspace/cookbench"),
    )
}

#[test]
fn grok_file_identity_distinguishes_same_project_processes() {
    let processes = vec![
        ObservedProcess::new(10, 1, None, "Terminal", None),
        grok_process(30, "ttys020").with_grok_session_proof("session-a", true, 1),
        grok_process(31, "ttys021").with_grok_session_proof("session-b", true, 1),
    ];

    let correlated = correlate_terminal_locator(
        &cookbench_core::domain::HarnessId::Other("grok_cli".into()),
        grok_locator("session-a"),
        &processes,
    );

    assert_eq!(correlated.tty.as_deref(), Some("/dev/ttys020"));
    assert_eq!(correlated.process_id, Some(30));
}

#[test]
fn grok_process_with_multiple_open_sessions_only_uses_a_fallback() {
    let base = grok_locator("session-a");
    let processes = vec![
        ObservedProcess::new(10, 1, None, "Terminal", None),
        grok_process(30, "ttys020").with_grok_session_proof("session-a", true, 2),
    ];

    let correlated = correlate_terminal_locator(
        &cookbench_core::domain::HarnessId::Other("grok_cli".into()),
        base.clone(),
        &processes,
    );

    assert_eq!(
        correlated.host_application,
        Some(HostApplication::MacosTerminal)
    );
    assert_eq!(correlated.terminal, None);
    assert_eq!(correlated.tty, None);
    assert!(!matches!(
        actions_for(&correlated).first(),
        Some(JumpAction::ExactTerminalTab { .. })
    ));
    let mut executor = RecordingExecutor::with_outcomes([JumpOutcome::VisibleFallback]);
    let result = activate_with(&correlated, &mut executor);
    assert_eq!(result.target, LocatorActivationTarget::ApplicationWindow);
    assert_eq!(result.status, LocatorActivationStatus::VisibleFallback);
}

#[test]
fn multiple_grok_processes_with_the_same_tty_are_still_ambiguous() {
    let processes = vec![
        ObservedProcess::new(10, 1, None, "Terminal", None),
        grok_process(30, "ttys020").with_grok_session_proof("session-a", true, 1),
        grok_process(31, "ttys020").with_grok_session_proof("session-a", true, 1),
    ];

    let correlated = correlate_terminal_locator(
        &cookbench_core::domain::HarnessId::Other("grok_cli".into()),
        grok_locator("session-a"),
        &processes,
    );

    assert_eq!(
        correlated.host_application,
        Some(HostApplication::MacosTerminal)
    );
    assert_eq!(correlated.terminal, None);
    assert_eq!(correlated.tty, None);
}

#[test]
fn grok_unproven_candidate_blocks_an_otherwise_matching_exact_return() {
    let processes = vec![
        ObservedProcess::new(10, 1, None, "Terminal", None),
        grok_process(30, "ttys020").with_grok_session_proof("session-a", true, 1),
        grok_process(31, "ttys021"),
    ];

    let correlated = correlate_terminal_locator(
        &cookbench_core::domain::HarnessId::Other("grok_cli".into()),
        grok_locator("session-a"),
        &processes,
    );

    assert_eq!(
        correlated.host_application,
        Some(HostApplication::MacosTerminal)
    );
    assert_eq!(correlated.terminal, None);
    assert_eq!(correlated.tty, None);
}

#[test]
fn grok_proof_budget_exhaustion_blocks_exact_return() {
    let mut processes = vec![
        ObservedProcess::new(10, 1, None, "Terminal", None),
        grok_process(30, "ttys020").with_grok_session_proof("session-a", true, 1),
    ];
    processes.extend((31..=46).map(|process_id| grok_process(process_id, "ttys021")));

    let correlated = correlate_terminal_locator(
        &cookbench_core::domain::HarnessId::Other("grok_cli".into()),
        grok_locator("session-a"),
        &processes,
    );

    assert_eq!(
        correlated.host_application,
        Some(HostApplication::MacosTerminal)
    );
    assert_eq!(correlated.terminal, None);
    assert_eq!(correlated.tty, None);
}

#[test]
fn grok_requires_valid_matching_bounded_session_proof() {
    let harness = cookbench_core::domain::HarnessId::Other("grok_cli".into());
    let terminal = ObservedProcess::new(10, 1, None, "Terminal", None);
    let cases = [
        grok_process(30, "ttys020").with_grok_session_proof("session-b", true, 1),
        grok_process(30, "ttys020").with_grok_session_proof("session-a", false, 1),
        grok_process(30, "ttys020").with_grok_session_proof("session-a", true, 0),
        grok_process(30, "ttys020").with_grok_session_proof("session-a", true, 2),
    ];

    for process in cases {
        let base = grok_locator("session-a");
        let correlated =
            correlate_terminal_locator(&harness, base.clone(), &[terminal.clone(), process]);
        assert_eq!(
            correlated.host_application,
            Some(HostApplication::MacosTerminal)
        );
        assert_eq!(correlated.terminal, None);
        assert_eq!(correlated.tty, None);
    }
}

#[test]
fn grok_rejects_unsafe_or_overlong_native_locator_before_exact_return() {
    let harness = cookbench_core::domain::HarnessId::Other("grok_cli".into());
    let terminal = ObservedProcess::new(10, 1, None, "Terminal", None);
    let process = grok_process(30, "ttys020").with_grok_session_proof("session-a", true, 1);
    let mut unsafe_locator = grok_locator("session-a");
    unsafe_locator.native_locator = Some("/synthetic/session-a/updates.jsonl\ninvalid".into());
    let mut overlong_locator = grok_locator("session-a");
    overlong_locator.native_locator = Some(format!("/{}", "a".repeat(4097)));
    let mut outside_grok_root = grok_locator("session-a");
    outside_grok_root.native_locator = Some("/synthetic/not-grok/session-a/updates.jsonl".into());

    for base in [unsafe_locator, overlong_locator, outside_grok_root] {
        let correlated = correlate_terminal_locator(
            &harness,
            base.clone(),
            &[terminal.clone(), process.clone()],
        );
        assert_ne!(correlated.terminal, Some(TerminalKind::MacosTerminal));
        assert_eq!(correlated.tty, None);
    }
}

#[test]
fn grok_fallback_clears_stale_exact_terminal_selectors() {
    let harness = cookbench_core::domain::HarnessId::Other("grok_cli".into());
    let mut base = grok_locator("session-a");
    base.process_id = Some(999);
    base.parent_process_id = Some(998);
    base.process_started_at_ms = Some(123);
    base.terminal = Some(TerminalKind::MacosTerminal);
    base.tty = Some("/dev/ttys099".into());
    base.tmux_pane = Some("%99".into());
    base.terminal_pane_id = Some("99".into());
    let correlated = correlate_terminal_locator(
        &harness,
        base,
        &[
            ObservedProcess::new(10, 1, None, "Terminal", None),
            grok_process(30, "ttys020").with_grok_session_proof("session-a", true, 2),
        ],
    );

    assert_eq!(
        correlated.host_application,
        Some(HostApplication::MacosTerminal)
    );
    assert_eq!(correlated.process_id, None);
    assert_eq!(correlated.parent_process_id, None);
    assert_eq!(correlated.process_started_at_ms, None);
    assert_eq!(correlated.terminal, None);
    assert_eq!(correlated.tty, None);
    assert_eq!(correlated.tmux_pane, None);
    assert_eq!(correlated.terminal_pane_id, None);
    assert!(!matches!(
        actions_for(&correlated).first(),
        Some(JumpAction::ExactTerminalTab { .. } | JumpAction::ExactPane { .. })
    ));
}

#[test]
fn permission_denial_continues_to_project_directory() {
    let mut executor = RecordingExecutor::with_outcomes([
        JumpOutcome::PermissionDenied,
        JumpOutcome::NotFound,
        JumpOutcome::FocusedExact,
    ]);

    let result = jump_with(&locator(), &mut executor);
    assert!(matches!(result.action, JumpAction::ProjectDirectory { .. }));
    assert_eq!(executor.seen.len(), 3);
}

#[test]
fn elevated_target_continues_to_resume_instructions() {
    let mut executor = RecordingExecutor::with_outcomes([
        JumpOutcome::NotFound,
        JumpOutcome::Unsupported,
        JumpOutcome::VerificationFailed,
        JumpOutcome::VisibleFallback,
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
    let mut executor = RecordingExecutor::with_outcomes([JumpOutcome::FocusedExact]);
    let result = activate_with(&locator(), &mut executor);

    assert_eq!(result.target, LocatorActivationTarget::ExactPane);
    assert_eq!(result.status, LocatorActivationStatus::Focused);
    assert_eq!(result.resume_session_id, None);
}

#[test]
fn reports_visible_resume_after_permission_and_elevation_fallbacks() {
    let mut executor = RecordingExecutor::with_outcomes([
        JumpOutcome::PermissionDenied,
        JumpOutcome::Ambiguous,
        JumpOutcome::VerificationFailed,
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

#[test]
fn exact_drivers_only_stop_on_a_verified_exact_focus() {
    let mut executor =
        RecordingExecutor::with_outcomes([JumpOutcome::TimedOut, JumpOutcome::VisibleFallback]);

    let result = jump_with(&locator(), &mut executor);

    assert!(matches!(
        result.action,
        JumpAction::ApplicationWindow { .. }
    ));
    assert_eq!(result.outcome, JumpOutcome::VisibleFallback);
    assert_eq!(executor.seen.len(), 2);
}

#[test]
fn malformed_terminal_selector_cannot_create_an_exact_action() {
    let locator = SessionLocator {
        host_application: Some(HostApplication::ITerm2),
        terminal: Some(TerminalKind::ITerm2),
        tty: Some("/dev/ttys008\nrun-unrelated-command".to_owned()),
        native_session_id: "opaque-terminal-session".to_owned(),
        ..SessionLocator::default()
    };

    assert!(matches!(
        actions_for(&locator).as_slice(),
        [JumpAction::ResumeInstructions { .. }]
    ));
}

#[test]
fn wezterm_requires_a_numeric_pane_selector_and_preserves_its_control_endpoint() {
    let locator = SessionLocator {
        terminal: Some(TerminalKind::WezTerm),
        terminal_pane_id: Some("42".to_owned()),
        terminal_control_endpoint: Some("/tmp/wezterm.sock".to_owned()),
        native_session_id: "opaque-wezterm-session".to_owned(),
        ..SessionLocator::default()
    };

    assert!(matches!(
        actions_for(&locator).as_slice(),
        [
            JumpAction::ExactWezTermPane {
                pane_id: 42,
                control_endpoint: Some(endpoint),
            },
            JumpAction::ResumeInstructions { .. },
        ] if endpoint == "/tmp/wezterm.sock"
    ));

    let invalid = SessionLocator {
        terminal_pane_id: Some("42; unrelated-command".to_owned()),
        ..locator
    };
    assert!(matches!(
        actions_for(&invalid).as_slice(),
        [JumpAction::ResumeInstructions { .. }]
    ));
}

#[test]
fn zellij_and_cmux_need_their_complete_native_selectors() {
    let zellij = SessionLocator {
        terminal: Some(TerminalKind::Zellij),
        terminal_session_id: Some("workspace".to_owned()),
        terminal_pane_id: Some("terminal_3".to_owned()),
        native_session_id: "opaque-zellij-session".to_owned(),
        ..SessionLocator::default()
    };
    assert!(matches!(
        actions_for(&zellij).first(),
        Some(JumpAction::ExactZellijPane { session_name, pane_id })
            if session_name == "workspace" && pane_id == "terminal_3"
    ));

    let cmux = SessionLocator {
        terminal: Some(TerminalKind::Cmux),
        terminal_pane_id: Some("panel-9".to_owned()),
        native_session_id: "opaque-cmux-session".to_owned(),
        ..SessionLocator::default()
    };
    assert!(matches!(
        actions_for(&cmux).first(),
        Some(JumpAction::ExactCmuxPanel { panel_id, control_endpoint: None })
            if panel_id == "panel-9"
    ));

    let incomplete = SessionLocator {
        terminal: Some(TerminalKind::Zellij),
        native_session_id: "opaque-zellij-session".to_owned(),
        ..SessionLocator::default()
    };
    assert!(matches!(
        actions_for(&incomplete).as_slice(),
        [JumpAction::ResumeInstructions { .. }]
    ));
}

#[test]
fn ghostty_uses_a_native_identifier_without_guessing_from_tty() {
    let locator = SessionLocator {
        terminal: Some(TerminalKind::Ghostty),
        terminal_pane_id: Some("terminal-8".to_owned()),
        native_session_id: "opaque-ghostty-session".to_owned(),
        ..SessionLocator::default()
    };
    assert!(matches!(
        actions_for(&locator).first(),
        Some(JumpAction::ExactGhosttyTerminal { terminal_id }) if terminal_id == "terminal-8"
    ));
}
