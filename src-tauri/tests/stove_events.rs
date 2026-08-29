use cookbench_core::domain::{
    EventKind, EventMetadata, EventSource, HarnessId, HostIdentity, ProjectIdentity, StoveEvent,
    StoveIdentity,
};
use cookbench_desktop_lib::{
    app_state::{LocatorCapability, StoveStore},
    events::StoveChange,
};

fn identity() -> StoveIdentity {
    StoveIdentity::new(
        HostIdentity::local("test-host"),
        HarnessId::Codex,
        "session-1",
    )
}

fn project() -> ProjectIdentity {
    ProjectIdentity::new(HostIdentity::local("test-host"), "/synthetic/project")
}

fn event(kind: EventKind, sequence: u64) -> StoveEvent {
    StoveEvent::new(
        kind,
        EventMetadata::new(EventSource::StructuredSession, 100, sequence, sequence),
    )
}

#[test]
fn snapshot_wire_is_sanitized_and_includes_required_presentation_metadata() {
    let store = StoveStore::default();
    store
        .apply(
            identity(),
            project(),
            LocatorCapability::Available,
            event(
                EventKind::PlanUpdated {
                    completed: 2,
                    total: 5,
                },
                1,
            ),
        )
        .unwrap();
    store
        .apply(
            identity(),
            project(),
            LocatorCapability::Available,
            event(EventKind::TurnCompleted, 2),
        )
        .unwrap();

    let snapshot = store.snapshot();
    assert_eq!(snapshot.revision, 2);
    let stove = &snapshot.stoves[0];
    assert_eq!(stove.harness.id, "codex");
    assert_eq!(stove.host.id, "test-host");
    assert_eq!(stove.project_label, "project");
    assert_eq!(stove.project_root, "/synthetic/project");
    assert_eq!(stove.project_root_display, "/synthetic/project");
    assert!(stove.task_title.is_none());
    assert!(stove.current_action.is_none());
    assert!(stove.next_action.is_none());
    assert!(stove.elapsed_ms.is_none());
    assert_eq!(
        stove.state,
        cookbench_desktop_lib::app_state::StoveStateWire::Cooked
    );
    assert_eq!(
        stove.progress.as_ref().unwrap().provenance,
        cookbench_desktop_lib::app_state::ProgressProvenanceWire::StructuredSession
    );
    assert_eq!(stove.locator_capability, LocatorCapability::Available);
    assert!(stove.retained_completion);

    let json = serde_json::to_string(&snapshot).unwrap();
    assert!(json.contains("\"projectLabel\""));
    assert!(json.contains("\"projectRootDisplay\""));
    assert!(json.contains("\"retainedCompletion\":true"));
    assert!(json.contains("\"provenance\":\"structuredSession\""));
    for forbidden in [
        "\"transcript\"",
        "\"prompt\"",
        "\"command\"",
        "\"credential\"",
        "\"locator\"",
    ] {
        assert!(!json.contains(forbidden), "wire payload leaked {forbidden}");
    }
}

#[test]
fn changes_are_monotonic_and_cooked_stoves_remain_until_clear() {
    let store = StoveStore::default();
    let first = store
        .apply(
            identity(),
            project(),
            LocatorCapability::Unavailable,
            event(EventKind::UserPromptSubmitted, 1),
        )
        .unwrap();
    let second = store
        .apply(
            identity(),
            project(),
            LocatorCapability::Unavailable,
            event(EventKind::TurnCompleted, 2),
        )
        .unwrap();
    assert_eq!(first.revision, 1);
    assert_eq!(second.revision, 2);
    assert!(second.stove.unwrap().retained_completion);
    assert_eq!(store.snapshot().stoves.len(), 1);

    let removed = store
        .apply(
            identity(),
            project(),
            LocatorCapability::Unavailable,
            event(EventKind::ClearRequested, 3),
        )
        .unwrap();
    assert_eq!(removed.revision, 3);
    assert!(removed.stove.is_none());
    assert!(removed.removed_stove_id.is_some());
    assert!(store.snapshot().stoves.is_empty());
}

#[test]
fn change_serializes_with_camel_case_fields() {
    let payload = StoveChange::remove(9, "local:test:pi:s1".into());
    let json = serde_json::to_string(&payload).unwrap();
    assert_eq!(
        json,
        r#"{"revision":9,"stove":null,"removedStoveId":"local:test:pi:s1"}"#
    );
}
