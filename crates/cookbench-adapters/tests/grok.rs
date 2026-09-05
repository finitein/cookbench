use std::path::PathBuf;

use cookbench_adapters::{
    grok::{discover_sessions, parse_record, session_from_path, GrokAdapter},
    HarnessAdapter, HostSource,
};
use cookbench_core::domain::{EventKind, HostIdentity};

fn fixture_sessions() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/grok_build/sessions")
}

#[test]
fn discovers_fixture_and_parses_allowlisted_lifecycle_updates() {
    let root = fixture_sessions();
    let source = HostSource::local(HostIdentity::local("fixture-host"));
    let sessions = discover_sessions(&root, &source).expect("discover");
    assert_eq!(sessions.len(), 1);
    assert_eq!(
        sessions[0].native_session_id,
        "01999999-aaaa-7bbb-8ccc-ddddeeeeffff"
    );

    let updates = root
        .join("%2Fsynthetic%2Fproject")
        .join("01999999-aaaa-7bbb-8ccc-ddddeeeeffff")
        .join("updates.jsonl");
    let from_path = session_from_path(&updates, &source)
        .expect("parse")
        .expect("session");
    assert_eq!(from_path.native_session_id, sessions[0].native_session_id);

    let contents = std::fs::read_to_string(&updates).expect("read updates");
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
    assert!(kinds.iter().any(|kind| matches!(
        kind,
        EventKind::ToolCompleted { succeeded: true }
    )));
    assert!(kinds.iter().any(|kind| matches!(
        kind,
        EventKind::PlanUpdated {
            completed: 1,
            total: 2
        }
    )));
    assert!(kinds
        .iter()
        .any(|kind| matches!(kind, EventKind::PermissionRequested)));
    assert!(kinds
        .iter()
        .any(|kind| matches!(kind, EventKind::TurnCompleted)));
    assert!(!kinds
        .iter()
        .any(|kind| matches!(kind, EventKind::SessionFailed)));
}

#[tokio::test]
async fn adapter_discover_exposes_grok_build_identity() {
    let adapter = GrokAdapter::new(fixture_sessions());
    let sessions = adapter
        .discover(&HostSource::local(HostIdentity::local("fixture-host")))
        .await
        .expect("discover");
    assert_eq!(sessions.len(), 1);
    assert_eq!(
        sessions[0].harness,
        cookbench_core::domain::HarnessId::Other("grok_cli".into())
    );
}
