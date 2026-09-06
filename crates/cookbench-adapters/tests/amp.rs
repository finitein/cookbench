use std::path::PathBuf;

use cookbench_adapters::{
    amp::{discover_sessions, parse_thread, session_from_path, AmpAdapter},
    HarnessAdapter, HostSource,
};
use cookbench_core::domain::{EventKind, HostIdentity};

fn fixture_threads() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/amp/threads")
}

#[test]
fn discovers_fixture_and_parses_allowlisted_lifecycle_updates() {
    let root = fixture_threads();
    let source = HostSource::local(HostIdentity::local("fixture-host"));
    let sessions = discover_sessions(&root, &source).expect("discover");
    assert_eq!(sessions.len(), 1);
    assert_eq!(
        sessions[0].native_session_id,
        "T-0199aaaa-bbbb-7ccc-8ddd-eeeeffff0001"
    );

    let path = root.join("T-0199aaaa-bbbb-7ccc-8ddd-eeeeffff0001.json");
    let from_path = session_from_path(&path, &source)
        .expect("parse")
        .expect("session");
    assert_eq!(from_path.native_session_id, sessions[0].native_session_id);
    assert_eq!(from_path.title.as_deref(), Some("Synthetic Amp fixture"));

    let contents = std::fs::read_to_string(&path).expect("read");
    let kinds: Vec<EventKind> = parse_thread(&contents)
        .into_iter()
        .map(|e| e.kind)
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
        .any(|kind| matches!(kind, EventKind::TurnCompleted)));
}

#[tokio::test]
async fn adapter_discover_exposes_amp_identity() {
    let adapter = AmpAdapter::new(fixture_threads());
    let sessions = adapter
        .discover(&HostSource::local(HostIdentity::local("fixture-host")))
        .await
        .expect("discover");
    assert_eq!(sessions.len(), 1);
    assert_eq!(
        sessions[0].harness,
        cookbench_core::domain::HarnessId::Other("amp".into())
    );
}
