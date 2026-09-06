use std::path::PathBuf;

use cookbench_adapters::{
    goose::{discover_sessions, parse_record, session_from_path, GooseAdapter},
    HarnessAdapter, HostSource,
};
use cookbench_core::domain::{EventKind, HostIdentity};

fn fixture_sessions() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/goose/sessions")
}

#[test]
fn discovers_fixture_and_parses_allowlisted_lifecycle_updates() {
    let root = fixture_sessions();
    let source = HostSource::local(HostIdentity::local("fixture-host"));
    let sessions = discover_sessions(&root, &source).expect("discover");
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].native_session_id, "20260906_1");

    let path = root.join("20260906_1.jsonl");
    let from_path = session_from_path(&path, &source)
        .expect("parse")
        .expect("session");
    assert_eq!(from_path.native_session_id, sessions[0].native_session_id);

    let contents = std::fs::read_to_string(&path).expect("read");
    let kinds: Vec<EventKind> = contents
        .lines()
        .enumerate()
        .flat_map(|(index, line)| parse_record(line, index as u64 + 1))
        .map(|event| event.kind)
        .collect();

    assert!(kinds
        .iter()
        .any(|kind| matches!(kind, EventKind::UserPromptSubmitted)));
    assert!(kinds
        .iter()
        .any(|kind| matches!(kind, EventKind::ToolStarted)));
    assert!(kinds
        .iter()
        .any(|kind| matches!(kind, EventKind::ToolCompleted { succeeded: true })));
    assert!(kinds
        .iter()
        .any(|kind| matches!(kind, EventKind::PermissionRequested)));
    assert!(kinds
        .iter()
        .any(|kind| matches!(kind, EventKind::TurnCompleted)));
}

#[tokio::test]
async fn adapter_discover_exposes_goose_identity() {
    let adapter = GooseAdapter::new(fixture_sessions());
    let sessions = adapter
        .discover(&HostSource::local(HostIdentity::local("fixture-host")))
        .await
        .expect("discover");
    assert_eq!(sessions.len(), 1);
    assert_eq!(
        sessions[0].harness,
        cookbench_core::domain::HarnessId::Other("goose".into())
    );
}
