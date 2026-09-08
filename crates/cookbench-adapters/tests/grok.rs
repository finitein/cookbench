use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

use cookbench_adapters::{
    grok::{discover_sessions, parse_record, session_from_path, GrokAdapter},
    HarnessAdapter, HostSource,
};
use cookbench_core::domain::{EventKind, HostIdentity};

fn fixture_sessions() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/grok_build/sessions")
}

fn native_fixture_sessions() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/grok_build/native-sessions")
}

fn temporary_summary(name: &str, summary: &str) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let directory = std::env::temp_dir().join(format!(
        "cookbench-grok-summary-{name}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&directory).expect("create temporary fixture directory");
    let path = directory.join("summary.json");
    fs::write(&path, summary).expect("write temporary summary fixture");
    path
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
    assert!(kinds
        .iter()
        .any(|kind| matches!(kind, EventKind::ToolCompleted { succeeded: true })));
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

#[test]
fn discovers_native_parent_summary_and_ignores_subagents() {
    let source = HostSource::local(HostIdentity::local("fixture-host"));
    let sessions = discover_sessions(&native_fixture_sessions(), &source).expect("discover");

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].native_session_id, "native-parent-session");
    assert_eq!(
        sessions[0]
            .project
            .as_ref()
            .map(|project| project.canonical_root.as_str()),
        Some("/synthetic/native-project")
    );
    assert!(sessions[0].locator.value.ends_with("updates.jsonl"));
}

#[test]
fn parses_acp_params_envelope_without_retaining_content() {
    let events = parse_record(
        r#"{"timestamp":1700000000000,"method":"_x.ai/session/update","params":{"sessionId":"synthetic-session","update":{"sessionUpdate":"tool_call_update","status":"completed","content":"must-not-be-read"}}}"#,
        8,
    );

    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0].kind,
        EventKind::ToolCompleted { succeeded: true }
    ));
}

#[test]
fn ignores_non_acp_envelopes_and_subagent_events() {
    for line in [
        r#"{"method":"other/update","params":{"update":{"sessionUpdate":"tool_call","status":"pending"}}}"#,
        r#"{"method":"session/update","params":{"update":{"sessionUpdate":"subagent_spawned"}}}"#,
        r#"{"method":"session/update","params":{"update":{"sessionUpdate":"subagent_finished"}}}"#,
    ] {
        assert!(
            parse_record(line, 1).is_empty(),
            "unexpected event for {line}"
        );
    }
}

#[test]
fn rejects_conflicting_summary_ids_and_oversized_metadata() {
    let source = HostSource::local(HostIdentity::local("fixture-host"));
    let conflicting = temporary_summary(
        "conflicting-ids",
        r#"{"info":{"id":"native-id","session_id":"legacy-id","cwd":"/synthetic/project"}}"#,
    );
    assert!(session_from_path(&conflicting, &source).is_err());
    fs::remove_dir_all(conflicting.parent().expect("fixture parent")).expect("remove fixture");

    let oversized = temporary_summary(
        "oversized",
        &format!(
            r#"{{"info":{{"id":"native-id","cwd":"/synthetic/project"}},"title":"{}"}}"#,
            "x".repeat(64 * 1024)
        ),
    );
    assert!(session_from_path(&oversized, &source).is_err());
    fs::remove_dir_all(oversized.parent().expect("fixture parent")).expect("remove fixture");
}
