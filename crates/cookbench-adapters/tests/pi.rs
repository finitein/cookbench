use std::path::PathBuf;

use cookbench_adapters::{HarnessAdapter, HostSource, ResumeAction, SessionLocatorKind};
use cookbench_core::domain::{EventKind, HostIdentity};

use cookbench_adapters::pi::{parse_session_file, ExtensionEnvelope, ExtensionEvent, PiAdapter};

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
