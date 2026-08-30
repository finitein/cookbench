use cookbench_core::locator::{HostApplication, SessionLocator, TerminalKind};

#[test]
fn preserves_strong_session_identity_through_serde() {
    let locator = SessionLocator {
        native_session_id: "session-a".to_owned(),
        native_locator: Some("/safe/session-a.jsonl".to_owned()),
        process_id: Some(42),
        parent_process_id: Some(7),
        process_started_at_ms: Some(1_700_000_000_000),
        host_application: Some(HostApplication::MacosTerminal),
        terminal: Some(TerminalKind::MacosTerminal),
        tty: Some("/dev/ttys001".to_owned()),
        terminal_window_id: Some("window-1".to_owned()),
        terminal_session_id: Some("session-1".to_owned()),
        terminal_pane_id: Some("pane-1".to_owned()),
        terminal_control_endpoint: Some("/safe/wezterm.sock".to_owned()),
        tmux_inner_pane: Some("%4".to_owned()),
        tmux_outer_client_tty: Some("/dev/ttys001".to_owned()),
        ide_workspace: Some("/safe/project".to_owned()),
        ..SessionLocator::default()
    };

    let encoded = serde_json::to_string(&locator).unwrap();
    let decoded: SessionLocator = serde_json::from_str(&encoded).unwrap();

    assert_eq!(decoded, locator);
    assert!(decoded.validate().is_ok());
}

#[test]
fn accepts_old_persisted_locator_without_new_identity_fields() {
    let decoded: SessionLocator =
        serde_json::from_str(r#"{"native_session_id":"legacy"}"#).unwrap();

    assert_eq!(decoded.native_session_id, "legacy");
    assert_eq!(decoded.native_locator, None);
    assert_eq!(decoded.process_started_at_ms, None);
    assert_eq!(decoded.terminal_session_id, None);
    assert_eq!(decoded.terminal_control_endpoint, None);
}

#[test]
fn rejects_unsafe_strong_identity_text() {
    let locator = SessionLocator {
        native_session_id: "session-a".to_owned(),
        terminal_session_id: Some("tab\nother".to_owned()),
        ..SessionLocator::default()
    };

    assert!(locator.validate().is_err());
}

#[test]
fn serializes_supported_terminal_family_variants() {
    for terminal in [
        TerminalKind::Ghostty,
        TerminalKind::WezTerm,
        TerminalKind::Zellij,
        TerminalKind::Cmux,
    ] {
        let locator = SessionLocator {
            native_session_id: "session-a".to_owned(),
            terminal: Some(terminal.clone()),
            ..SessionLocator::default()
        };
        assert_eq!(
            serde_json::from_str::<SessionLocator>(&serde_json::to_string(&locator).unwrap())
                .unwrap(),
            locator
        );
    }
}

#[test]
fn rejects_unsafe_terminal_control_endpoint() {
    let locator = SessionLocator {
        native_session_id: "session-a".to_owned(),
        terminal_control_endpoint: Some("/safe/socket\npassword".to_owned()),
        ..SessionLocator::default()
    };

    assert!(locator.validate().is_err());
}
