use std::path::PathBuf;

use cookbench_adapters::{HarnessAdapter, HostSource, ResumeAction, SessionLocatorKind};
use cookbench_core::domain::{EventKind, HostIdentity};

use cookbench_adapters::pi::{
    parse_record, parse_session_file, ExtensionEnvelope, ExtensionEvent, PiAdapter,
};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/pi")
        .join(name)
}

#[test]
fn defaults_to_the_conventional_pi_session_root() {
    let adapter = PiAdapter::new();
    assert!(adapter.roots()[0].ends_with(".pi/agent/sessions"));
}

#[tokio::test]
async fn discovers_versioned_session_trees_from_an_overridden_root() {
    let root = fixture("versioned");
    let adapter = PiAdapter::with_roots([root]);
    let host = HostIdentity::local("pi-test-host");
    let sessions = adapter
        .discover(&HostSource::local(host.clone()))
        .await
        .unwrap();

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].harness, cookbench_core::domain::HarnessId::Pi);
    assert_eq!(sessions[0].native_session_id, "pi-synthetic-001");
    assert_eq!(sessions[0].title.as_deref(), Some("Synthetic Pi task"));
    assert_eq!(
        sessions[0].project.as_ref().unwrap().canonical_root,
        "/synthetic/pi-project"
    );
    assert_eq!(sessions[0].locator.kind, SessionLocatorKind::LocalPath);
    assert_eq!(sessions[0].host, host);
}

#[test]
fn parses_lifecycle_and_todo_records_without_retaining_transcript_content() {
    let parsed = parse_session_file(&fixture("versioned/v2/tree/pi-synthetic-001.jsonl")).unwrap();
    let title = parsed.title.clone();
    let kinds = parsed
        .events
        .into_iter()
        .map(|event| event.kind)
        .collect::<Vec<_>>();

    assert!(kinds
        .iter()
        .any(|kind| matches!(kind, EventKind::SessionDiscovered)));
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
        .any(|kind| matches!(kind, EventKind::TurnCompleted)));
    assert_eq!(kinds.len(), 6, "unknown records must be ignored safely");
    assert_eq!(title.as_deref(), Some("Synthetic Pi task"));
}

#[test]
fn parses_current_nested_message_records_once_and_preserves_native_identity() {
    let parsed = parse_session_file(&fixture("current-schema/messages.jsonl")).unwrap();
    let kinds = parsed
        .events
        .iter()
        .map(|event| &event.kind)
        .collect::<Vec<_>>();

    assert_eq!(parsed.native_session_id, "pi-current-schema-001");
    assert_eq!(parsed.title.as_deref(), Some("Synthetic current Pi task"));
    assert_eq!(
        kinds,
        vec![
            &EventKind::SessionDiscovered,
            &EventKind::UserPromptSubmitted,
            &EventKind::ToolStarted,
            &EventKind::ToolCompleted { succeeded: true },
            &EventKind::TurnCompleted,
        ],
        "nested content must not be recursively double-emitted"
    );
}

#[test]
fn parses_current_nested_message_failures_without_treating_length_as_completion() {
    let failed = parse_record(
        r#"{"type":"message","message":{"role":"assistant","stopReason":"error","content":[]}}"#,
        8,
    );
    let aborted = parse_record(
        r#"{"type":"message","message":{"role":"assistant","stopReason":"aborted","content":[]}}"#,
        9,
    );
    let tool_failure = parse_record(
        r#"{"type":"message","message":{"role":"toolResult","isError":true,"content":[]}}"#,
        10,
    );
    let length = parse_record(
        r#"{"type":"message","message":{"role":"assistant","stopReason":"length","content":[]}}"#,
        11,
    );

    assert!(matches!(failed.as_slice(), [event] if matches!(event.kind, EventKind::SessionFailed)));
    assert!(
        matches!(aborted.as_slice(), [event] if matches!(event.kind, EventKind::SessionFailed))
    );
    assert!(
        matches!(tool_failure.as_slice(), [event] if matches!(event.kind, EventKind::ToolCompleted { succeeded: false }))
    );
    assert!(
        matches!(length.as_slice(), [event] if matches!(event.kind, EventKind::SessionFailed)),
        "a terminal truncated response must not leave the Stove active"
    );
}

#[test]
fn ignores_session_info_shaped_user_content_when_deriving_metadata() {
    let parsed =
        parse_session_file(&fixture("current-schema/nested-metadata-shape.jsonl")).unwrap();

    assert_eq!(parsed.native_session_id, "pi-safe-metadata-001");
    assert_eq!(parsed.title, None);
}

#[test]
fn metadata_only_registration_keeps_the_structured_project_locator() {
    let path = fixture("versioned/v2/tree/pi-synthetic-001.jsonl");
    let adapter = PiAdapter::with_roots([fixture("versioned")]);
    let session = adapter
        .session_metadata_from_path(
            &HostSource::local(HostIdentity::local("pi-test-host")),
            path,
        )
        .unwrap();

    assert_eq!(session.native_session_id, "pi-synthetic-001");
    assert_eq!(session.title.as_deref(), Some("Synthetic Pi task"));
    assert_eq!(
        session
            .project
            .as_ref()
            .map(|project| project.canonical_root.as_str()),
        Some("/synthetic/pi-project")
    );
}

#[test]
fn preserves_a_fork_as_its_own_native_session_identity() {
    let parsed = parse_session_file(&fixture("forked/opaque-file-name.jsonl")).unwrap();

    assert_eq!(parsed.native_session_id, "pi-fork-002");
}

#[tokio::test]
async fn offers_user_mediated_resume_for_the_same_pi_session() {
    let adapter = PiAdapter::with_roots([fixture("versioned")]);
    let sessions = adapter
        .discover(&HostSource::local(HostIdentity::local("pi-test-host")))
        .await
        .unwrap();
    let actions = adapter.resume(&sessions[0]);

    assert!(actions
        .iter()
        .any(|action| matches!(action, ResumeAction::OpenSessionLocation(_))));
    assert!(actions.iter().any(|action| matches!(action, ResumeAction::SuggestedCommand { program, args } if program == "pi" && args.as_slice() == ["--resume", "pi-synthetic-001"])));
}

#[test]
fn extension_envelopes_are_bounded_and_content_free() {
    let envelope = ExtensionEnvelope::new(
        "pi-synthetic-001",
        ExtensionEvent::TodoProgress {
            completed: 1,
            total: 2,
        },
    )
    .unwrap();
    assert!(matches!(
        envelope.event_kind(),
        Some(EventKind::PlanUpdated {
            completed: 1,
            total: 2
        })
    ));
    assert_eq!(
        serde_json::to_value(&envelope).unwrap(),
        serde_json::json!({
            "version": 1,
            "sessionId": "pi-synthetic-001",
            "event": { "type": "todo_progress", "completed": 1, "total": 2 }
        })
    );
    assert!(ExtensionEnvelope::new("x".repeat(513), ExtensionEvent::PromptSubmitted).is_none());
    assert!(ExtensionEnvelope::new(
        "pi-synthetic-001",
        ExtensionEvent::TodoProgress {
            completed: 2,
            total: 1
        }
    )
    .unwrap()
    .event_kind()
    .is_none());
}
