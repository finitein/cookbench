use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use cookbench_core::{
    domain::{
        EventKind, EventMetadata, EventSource, HarnessId, HostIdentity, ProjectIdentity,
        StoveEvent, StoveIdentity, StoveState,
    },
    persistence::{
        ArchiveReason, ArchivedSession, ClearCursor, PersistedConfig, PersistedState,
        PinnedSession, RetainedStove, RetainedStovePresentation, SessionRecord,
    },
};
use cookbench_desktop_lib::{
    app_state::{AppState, LocatorCapability, StoveStateWire},
    persistence::{DesktopPersistence, LoadIssue, NativeSessionObservation, PersistenceErrorKind},
};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let suffix = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("cookbench-desktop-persistence-{suffix}"));
        fs::create_dir_all(&path).expect("create test directory");
        Self(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn locator() -> StoveIdentity {
    StoveIdentity::new(
        HostIdentity::local("test-host"),
        HarnessId::Codex,
        "session-42",
    )
}

fn event(sequence: u64, timestamp_ms: u64) -> EventMetadata {
    EventMetadata::new(EventSource::StructuredSession, 100, sequence, timestamp_ms)
}

fn session(state: StoveState) -> SessionRecord {
    session_with_id("session-42", state)
}

#[test]
fn production_path_normalizes_hook_completion_before_persisting_acknowledgement() {
    let directory = TestDirectory::new();
    let app = tauri::test::mock_app();
    let state = AppState::default();
    state.initialize_persistence(&directory.0);
    let identity = StoveIdentity::new(HostIdentity::local("test-host"), HarnessId::Codex, "hook");
    let project = ProjectIdentity::new(HostIdentity::local("test-host"), "/safe/hook");

    state
        .apply_and_emit(
            app.handle(),
            identity.clone(),
            project.clone(),
            LocatorCapability::Unavailable,
            None,
            StoveEvent::new(EventKind::ToolStarted, event(41, 1)),
        )
        .unwrap();
    state
        .apply_and_emit(
            app.handle(),
            identity.clone(),
            project,
            LocatorCapability::Unavailable,
            None,
            StoveEvent::new(
                EventKind::TurnCompleted,
                EventMetadata::new(EventSource::Hook, 90, 900, 2),
            ),
        )
        .unwrap();

    let persisted = DesktopPersistence::in_app_data(&directory.0).load().state;
    assert_eq!(
        persisted.retained[0]
            .completion_event
            .as_ref()
            .unwrap()
            .sequence,
        2
    );
    assert_eq!(
        persisted.retained[0]
            .completion_source_event
            .as_ref()
            .unwrap()
            .sequence,
        900
    );
    assert!(state
        .acknowledge_cooked_and_emit(app.handle(), "local:test-host:codex:hook")
        .unwrap());

    let restarted = AppState::default();
    restarted.initialize_persistence(&directory.0);
    restarted
        .stoves
        .apply(
            StoveIdentity::new(HostIdentity::local("test-host"), HarnessId::Codex, "active"),
            ProjectIdentity::new(HostIdentity::local("test-host"), "/safe/active"),
            LocatorCapability::Unavailable,
            StoveEvent::new(EventKind::ToolStarted, event(1, 3)),
        )
        .unwrap();
    assert_eq!(
        restarted.snapshot().attention_order,
        vec![
            "local:test-host:codex:active".to_owned(),
            "local:test-host:codex:hook".to_owned(),
        ]
    );
}

#[test]
fn newer_normalized_completion_with_the_same_timestamp_re_elevates_after_restart() {
    let directory = TestDirectory::new();
    let app = tauri::test::mock_app();
    let state = AppState::default();
    state.initialize_persistence(&directory.0);
    let identity = StoveIdentity::new(
        HostIdentity::local("test-host"),
        HarnessId::Codex,
        "same-time",
    );
    let project = ProjectIdentity::new(HostIdentity::local("test-host"), "/safe/same-time");

    for (kind, raw_sequence) in [
        (EventKind::ToolStarted, 700),
        (EventKind::TurnCompleted, 701),
    ] {
        state
            .apply_and_emit(
                app.handle(),
                identity.clone(),
                project.clone(),
                LocatorCapability::Unavailable,
                None,
                StoveEvent::new(
                    kind,
                    EventMetadata::new(EventSource::Hook, 90, raw_sequence, 0),
                ),
            )
            .unwrap();
    }
    assert!(state
        .acknowledge_cooked_and_emit(app.handle(), "local:test-host:codex:same-time")
        .unwrap());
    for (kind, raw_sequence) in [
        (EventKind::UserPromptSubmitted, 702),
        (EventKind::TurnCompleted, 703),
    ] {
        state
            .apply_and_emit(
                app.handle(),
                identity.clone(),
                project.clone(),
                LocatorCapability::Unavailable,
                None,
                StoveEvent::new(
                    kind,
                    EventMetadata::new(EventSource::Hook, 90, raw_sequence, 0),
                ),
            )
            .unwrap();
    }
    let persisted = DesktopPersistence::in_app_data(&directory.0).load().state;
    assert_eq!(persisted.cooked_attention_cursors[0].sequence, 2);
    assert_eq!(
        persisted.retained[0]
            .completion_event
            .as_ref()
            .unwrap()
            .sequence,
        4
    );
    assert_eq!(
        persisted.retained[0]
            .completion_source_event
            .as_ref()
            .unwrap()
            .sequence,
        703
    );

    let restarted = AppState::default();
    restarted.initialize_persistence(&directory.0);
    restarted
        .stoves
        .apply(
            StoveIdentity::new(HostIdentity::local("test-host"), HarnessId::Codex, "active"),
            ProjectIdentity::new(HostIdentity::local("test-host"), "/safe/active"),
            LocatorCapability::Unavailable,
            StoveEvent::new(EventKind::ToolStarted, event(1, 1)),
        )
        .unwrap();
    assert_eq!(
        restarted.snapshot().attention_order[0],
        "local:test-host:codex:same-time"
    );
}

#[test]
fn replaying_the_same_raw_completion_after_restart_keeps_it_acknowledged() {
    let directory = TestDirectory::new();
    let app = tauri::test::mock_app();
    let identity = StoveIdentity::new(HostIdentity::local("test-host"), HarnessId::Codex, "replay");
    let project = ProjectIdentity::new(HostIdentity::local("test-host"), "/safe/replay");
    let state = AppState::default();
    state.initialize_persistence(&directory.0);
    for (kind, sequence) in [
        (EventKind::ToolStarted, 700),
        (EventKind::TurnCompleted, 701),
    ] {
        state
            .apply_and_emit(
                app.handle(),
                identity.clone(),
                project.clone(),
                LocatorCapability::Unavailable,
                None,
                StoveEvent::new(kind, event(sequence, sequence)),
            )
            .unwrap();
    }
    assert!(state
        .acknowledge_cooked_and_emit(app.handle(), "local:test-host:codex:replay")
        .unwrap());
    let before = DesktopPersistence::in_app_data(&directory.0)
        .load()
        .state
        .retained[0]
        .clone();

    let restarted = AppState::default();
    restarted.initialize_persistence(&directory.0);
    restarted
        .apply_replay_observation_and_emit(
            app.handle(),
            identity.clone(),
            project.clone(),
            LocatorCapability::Unavailable,
            None,
            None,
            StoveEvent::new(EventKind::SessionDiscovered, event(699, 699)),
        )
        .unwrap();
    for (kind, sequence) in [
        (EventKind::ToolStarted, 700),
        (EventKind::TurnCompleted, 701),
    ] {
        restarted
            .apply_replay_observation_and_emit(
                app.handle(),
                identity.clone(),
                project.clone(),
                LocatorCapability::Unavailable,
                None,
                None,
                StoveEvent::new(kind, event(sequence, sequence)),
            )
            .unwrap();
    }
    restarted
        .stoves
        .apply(
            StoveIdentity::new(HostIdentity::local("test-host"), HarnessId::Codex, "active"),
            ProjectIdentity::new(HostIdentity::local("test-host"), "/safe/active"),
            LocatorCapability::Unavailable,
            StoveEvent::new(EventKind::ToolStarted, event(1, 800)),
        )
        .unwrap();
    assert_eq!(
        restarted.snapshot().attention_order[0],
        "local:test-host:codex:active"
    );
    let after = DesktopPersistence::in_app_data(&directory.0)
        .load()
        .state
        .retained[0]
        .clone();
    assert_eq!(after.completion_event, before.completion_event);
    assert_eq!(
        after.completion_source_event,
        before.completion_source_event
    );
}

#[test]
fn clear_then_new_raw_events_restore_the_new_cooked_completion() {
    let directory = TestDirectory::new();
    let app = tauri::test::mock_app();
    let identity = locator();
    let project = ProjectIdentity::new(HostIdentity::local("test-host"), "/safe/cookbench");
    let state = AppState::default();
    state.initialize_persistence(&directory.0);
    for (kind, sequence) in [
        (EventKind::ToolStarted, 899),
        (EventKind::TurnCompleted, 900),
    ] {
        state
            .apply_and_emit(
                app.handle(),
                identity.clone(),
                project.clone(),
                LocatorCapability::Unavailable,
                None,
                StoveEvent::new(kind, event(sequence, sequence)),
            )
            .unwrap();
    }
    state
        .clear_cooked_and_emit(app.handle(), "local:test-host:codex:session-42")
        .unwrap();
    for (kind, sequence) in [
        (EventKind::UserPromptSubmitted, 901),
        (EventKind::TurnCompleted, 902),
    ] {
        state
            .apply_and_emit(
                app.handle(),
                identity.clone(),
                project.clone(),
                LocatorCapability::Unavailable,
                None,
                StoveEvent::new(kind, event(sequence, sequence)),
            )
            .unwrap();
    }
    let persisted = DesktopPersistence::in_app_data(&directory.0).load().state;
    let retained = &persisted.retained[0];
    assert_eq!(
        retained.completion_source_event.as_ref().unwrap().sequence,
        902
    );
    assert_eq!(retained.completion_event.as_ref().unwrap().sequence, 2);

    let restarted = AppState::default();
    restarted.initialize_persistence(&directory.0);
    assert_eq!(restarted.snapshot().stoves[0].state, StoveStateWire::Cooked);
}

#[test]
fn legacy_cursor_without_a_completion_fingerprint_fails_open() {
    let directory = TestDirectory::new();
    let persistence = DesktopPersistence::in_app_data(&directory.0);
    // Released v0.3 had no cooked_attention_cursors, so guessing a missing
    // completion identity could hide a completion the user has never seen.
    fs::write(
        persistence.state_path(),
        r#"{
          "version": 3,
          "retained": [{
            "locator": {"host":{"kind":"Local","id":"test-host"},"harness":"Codex","native_session_id":"legacy"},
            "completed_at_ms": 0,
            "presentation": {"project_label":"legacy","project_root_display":"/safe/legacy"}
          }],
          "cooked_attention_cursors": [{
            "locator": {"host":{"kind":"Local","id":"test-host"},"harness":"Codex","native_session_id":"legacy"},
            "source":"StructuredSession","confidence":100,"sequence":0,"timestamp_ms":0,"acknowledged_at_ms":1
          }]
        }"#,
    )
    .unwrap();

    let state = AppState::default();
    state.initialize_persistence(&directory.0);
    assert!(DesktopPersistence::in_app_data(&directory.0)
        .load()
        .state
        .cooked_attention_cursors
        .is_empty());
    state
        .stoves
        .apply(
            StoveIdentity::new(HostIdentity::local("test-host"), HarnessId::Codex, "active"),
            ProjectIdentity::new(HostIdentity::local("test-host"), "/safe/active"),
            LocatorCapability::Unavailable,
            StoveEvent::new(EventKind::ToolStarted, event(1, 1)),
        )
        .unwrap();
    assert_eq!(
        state.snapshot().attention_order[0],
        "local:test-host:codex:legacy"
    );
}

fn session_with_id(native_session_id: &str, state: StoveState) -> SessionRecord {
    SessionRecord::new(
        StoveIdentity::new(
            HostIdentity::local("test-host"),
            HarnessId::Codex,
            native_session_id,
        ),
        Some(format!("/safe/{native_session_id}.jsonl")),
        800,
        RetainedStovePresentation::new("cookbench", "/safe/cookbench"),
        state,
    )
    .expect("valid session")
}

#[test]
fn pre_release_replay_cache_is_migrated_without_losing_clear_cursors() {
    let directory = TestDirectory::new();
    let persistence = DesktopPersistence::in_app_data(&directory.0);
    let legacy = PersistedState {
        version: 1,
        retained: vec![RetainedStove::new(locator(), 800)],
        clear_cursors: vec![ClearCursor::new(locator(), 7, 700)],
        cooked_attention_cursors: Vec::new(),
        pinned: Vec::new(),
        archived: Vec::new(),
        tracked: Vec::new(),
    };
    fs::write(
        persistence.state_path(),
        serde_json::to_vec(&legacy).expect("serialize legacy state"),
    )
    .expect("write legacy state");

    let loaded = persistence.load();

    assert!(loaded.issues.is_empty());
    assert_eq!(loaded.state.version, PersistedState::CURRENT_VERSION);
    assert!(loaded.state.retained.is_empty());
    assert_eq!(loaded.state.clear_cursors, legacy.clear_cursors);
    let migrated: PersistedState =
        serde_json::from_slice(&fs::read(persistence.state_path()).expect("read migrated state"))
            .expect("parse migrated state");
    assert_eq!(migrated, loaded.state);
}

#[test]
fn retained_cooked_stove_survives_restart_without_copying_native_history() {
    let directory = TestDirectory::new();
    let persistence = DesktopPersistence::in_app_data(&directory.0);
    let mut state = PersistedState::default();
    persistence
        .persist_transition_with_presentation(
            &mut state,
            locator(),
            StoveState::Cooked,
            &event(8, 800),
            &event(8, 800),
            RetainedStovePresentation::new("cookbench", "/safe/cookbench"),
        )
        .expect("persist Cooked");

    let restarted = DesktopPersistence::in_app_data(&directory.0).load();
    assert!(restarted.issues.is_empty());
    let restored = persistence.merge_retained_with_discovery(&restarted.state, &[]);
    assert_eq!(restored.len(), 1);
    assert_eq!(restored[0].retained.presentation.project_label, "cookbench");
    assert_eq!(
        restored[0].retained.presentation.project_root_display,
        "/safe/cookbench"
    );

    let persisted = fs::read_to_string(persistence.state_path()).expect("read state");
    for forbidden in [
        "prompt",
        "transcript",
        "command",
        "output",
        "code",
        "token",
        "secret",
        "task_title",
        "current_action",
        "next_action",
    ] {
        assert!(
            !persisted.contains(forbidden),
            "persisted JSON exposed {forbidden}"
        );
    }
}

#[test]
fn app_state_restores_an_undiscovered_retained_cooked_stove_at_startup() {
    let directory = TestDirectory::new();
    let persistence = DesktopPersistence::in_app_data(&directory.0);
    let mut state = PersistedState::default();
    persistence
        .persist_transition_with_presentation(
            &mut state,
            locator(),
            StoveState::Cooked,
            &event(8, 800),
            &event(8, 800),
            RetainedStovePresentation::new("cookbench", "/safe/cookbench"),
        )
        .unwrap();

    let app_state = AppState::default();
    app_state.initialize_persistence(&directory.0);
    let snapshot = app_state.stoves.snapshot();

    assert_eq!(snapshot.stoves.len(), 1);
    assert_eq!(snapshot.stoves[0].state, StoveStateWire::Cooked);
    assert_eq!(snapshot.stoves[0].project_label, "cookbench");
    assert!(snapshot.stoves[0].retained_completion);
}

#[test]
fn manual_clear_survives_restart_and_stale_replay_stays_hidden() {
    let directory = TestDirectory::new();
    let persistence = DesktopPersistence::in_app_data(&directory.0);
    let mut state = PersistedState::default();
    persistence
        .persist_transition(
            &mut state,
            locator(),
            StoveState::Cooked,
            &event(8, 800),
            &event(8, 800),
        )
        .unwrap();
    persistence
        .pin_session(&mut state, session(StoveState::Cooked), 850)
        .unwrap();
    persistence
        .clear_cooked(&mut state, locator(), &event(9, 900))
        .unwrap();

    let restarted = DesktopPersistence::in_app_data(&directory.0).load();
    assert!(restarted.state.retained.is_empty());
    assert!(restarted.state.pinned.is_empty());
    assert!(restarted.state.is_hidden(&locator(), &event(9, 900)));
    assert!(persistence
        .merge_retained_with_discovery(&restarted.state, &[])
        .is_empty());
}

#[test]
fn newer_prompt_relights_after_manual_clear() {
    let directory = TestDirectory::new();
    let persistence = DesktopPersistence::in_app_data(&directory.0);
    let mut state = PersistedState::default();
    persistence
        .clear_cooked(&mut state, locator(), &event(9, 900))
        .unwrap();

    persistence
        .persist_transition(
            &mut state,
            locator(),
            StoveState::Cooking,
            &event(10, 1_000),
            &event(10, 1_000),
        )
        .unwrap();
    assert!(!state.is_hidden(&locator(), &event(10, 1_000)));
    assert!(state.retained.is_empty());
}

#[test]
fn corrupt_or_future_config_is_isolated_from_state_recovery() {
    let directory = TestDirectory::new();
    let persistence = DesktopPersistence::in_app_data(&directory.0);
    let mut state = PersistedState::default();
    persistence
        .persist_transition(
            &mut state,
            locator(),
            StoveState::Cooked,
            &event(8, 800),
            &event(8, 800),
        )
        .unwrap();
    fs::write(persistence.config_path(), r#"{"version":999}"#).unwrap();

    let loaded = persistence.load();
    assert_eq!(loaded.config, PersistedConfig::default());
    assert_eq!(loaded.state.retained.len(), 1);
    assert_eq!(
        loaded.issues,
        vec![LoadIssue::Config(PersistenceErrorKind::UnsupportedVersion)]
    );

    fs::write(persistence.config_path(), b"not json").unwrap();
    let corrupt = persistence.load();
    assert_eq!(corrupt.state.retained.len(), 1);
    assert_eq!(
        corrupt.issues,
        vec![LoadIssue::Config(PersistenceErrorKind::InvalidJson)]
    );
}

#[test]
fn newer_native_observation_supersedes_retained_completion() {
    let directory = TestDirectory::new();
    let persistence = DesktopPersistence::in_app_data(&directory.0);
    let mut state = PersistedState::default();
    persistence
        .persist_transition(
            &mut state,
            locator(),
            StoveState::Cooked,
            &event(8, 800),
            &event(8, 800),
        )
        .unwrap();

    let merged = persistence.merge_retained_with_discovery(
        &state,
        &[NativeSessionObservation {
            locator: locator(),
            last_event: event(10, 1_000),
        }],
    );
    assert!(merged.is_empty());
}

#[test]
fn v2_state_is_bumped_without_losing_existing_data() {
    let directory = TestDirectory::new();
    let persistence = DesktopPersistence::in_app_data(&directory.0);
    fs::write(
        persistence.state_path(),
        r#"{"version":2,"retained":[],"clear_cursors":[]}"#,
    )
    .unwrap();

    let loaded = persistence.load();
    assert!(loaded.issues.is_empty());
    assert_eq!(loaded.state.version, PersistedState::CURRENT_VERSION);
    assert!(loaded.state.pinned.is_empty());
    assert!(loaded.state.archived.is_empty());
}

#[test]
fn manual_archive_removes_pin_and_restore_can_repin() {
    let directory = TestDirectory::new();
    let persistence = DesktopPersistence::in_app_data(&directory.0);
    let mut state = PersistedState::default();
    assert!(persistence
        .pin_session(&mut state, session(StoveState::Cooking), 900)
        .unwrap());
    assert!(persistence.is_pinned(&state, &locator()));

    assert!(persistence
        .archive_session(
            &mut state,
            session(StoveState::Cooking),
            1_000,
            ArchiveReason::Manual,
        )
        .unwrap());
    assert!(!persistence.is_pinned(&state, &locator()));
    assert!(persistence.is_archived(&state, &locator()));
    assert_eq!(
        persistence.archive_snapshot(&state)[0].reason,
        ArchiveReason::Manual
    );

    let restored = persistence
        .restore_session(&mut state, &locator(), true, 1_100)
        .unwrap()
        .expect("archived session restored");
    assert_eq!(restored.reason, ArchiveReason::Manual);
    assert!(persistence.is_pinned(&state, &locator()));
    assert!(!persistence.is_archived(&state, &locator()));
}

#[test]
fn tracked_session_is_removed_when_archived_and_restored_when_active() {
    let directory = TestDirectory::new();
    let persistence = DesktopPersistence::in_app_data(&directory.0);
    let mut state = PersistedState::default();
    assert!(persistence
        .track_session(&mut state, session(StoveState::NeedsHuman))
        .unwrap());
    assert_eq!(state.tracked.len(), 1);
    assert!(persistence
        .archive_session(
            &mut state,
            session(StoveState::NeedsHuman),
            1_000,
            ArchiveReason::Expired,
        )
        .unwrap());
    assert!(state.tracked.is_empty());
    persistence
        .restore_session(&mut state, &locator(), false, 1_100)
        .unwrap();
    assert_eq!(state.tracked.len(), 1);
}

#[test]
fn pinned_session_is_restored_as_one_pinned_stove_after_restart() {
    let directory = TestDirectory::new();
    let persistence = DesktopPersistence::in_app_data(&directory.0);
    let mut state = PersistedState::default();
    persistence
        .pin_session(&mut state, session(StoveState::Cooking), 900)
        .unwrap();

    let app_state = AppState::default();
    app_state.initialize_persistence(&directory.0);
    let snapshot = app_state.stoves.snapshot();

    assert_eq!(snapshot.stoves.len(), 1);
    assert_eq!(snapshot.stoves[0].state, StoveStateWire::Cooking);
    assert!(snapshot.stoves[0].pinned);
}

#[test]
fn stale_tracked_session_moves_to_archive_during_startup() {
    let directory = TestDirectory::new();
    let persistence = DesktopPersistence::in_app_data(&directory.0);
    let mut state = PersistedState::default();
    persistence
        .track_session(&mut state, session(StoveState::Failed))
        .unwrap();

    let app_state = AppState::default();
    app_state.initialize_persistence(&directory.0);

    assert!(app_state.stoves.snapshot().stoves.is_empty());
    let archived = app_state.archived_sessions();
    assert_eq!(archived.len(), 1);
    assert_eq!(archived[0].last_state, StoveStateWire::Failed);
}

#[test]
fn expired_inventory_is_imported_atomically_without_overriding_pins() {
    let directory = TestDirectory::new();
    let persistence = DesktopPersistence::in_app_data(&directory.0);
    let mut state = PersistedState::default();
    persistence
        .pin_session(&mut state, session(StoveState::Cooking), 900)
        .unwrap();
    let other = SessionRecord::new(
        StoveIdentity::new(
            HostIdentity::local("test-host"),
            HarnessId::ClaudeCode,
            "expired-session",
        ),
        Some("/safe/expired.jsonl".into()),
        100,
        RetainedStovePresentation::new("expired", "/safe/expired"),
        StoveState::Disconnected,
    )
    .unwrap();

    assert_eq!(
        persistence
            .archive_expired_sessions(&mut state, [session(StoveState::Cooking), other], 1_000)
            .unwrap(),
        1
    );
    assert!(persistence.is_pinned(&state, &locator()));
    assert_eq!(state.archived.len(), 1);
    assert_eq!(
        state.archived[0].session.locator.native_session_id,
        "expired-session"
    );
}

#[test]
fn expired_inventory_does_not_resurrect_a_cleared_session() {
    let directory = TestDirectory::new();
    let persistence = DesktopPersistence::in_app_data(&directory.0);
    let mut state = PersistedState::default();
    persistence
        .clear_cooked(&mut state, locator(), &event(9, 900))
        .unwrap();
    let mut expired = session(StoveState::Disconnected);
    expired.observed_at_ms = 800;

    assert_eq!(
        persistence
            .archive_expired_sessions(&mut state, [expired], 1_000)
            .unwrap(),
        0
    );
    assert!(state.archived.is_empty());
}

#[test]
fn full_pin_set_rejects_a_new_pin_without_mutating_other_session_records() {
    let directory = TestDirectory::new();
    let persistence = DesktopPersistence::in_app_data(&directory.0);
    let target = session(StoveState::Cooking);
    let mut state = PersistedState {
        pinned: (0..1_024)
            .map(|index| PinnedSession {
                session: session_with_id(&format!("pinned-{index}"), StoveState::Cooking),
                pinned_at_ms: 900,
            })
            .collect(),
        archived: vec![ArchivedSession {
            session: target.clone(),
            archived_at_ms: 950,
            reason: ArchiveReason::Manual,
        }],
        tracked: vec![target.clone()],
        ..PersistedState::default()
    };
    let before = state.clone();

    assert!(!persistence.pin_session(&mut state, target, 1_000).unwrap());
    assert_eq!(state, before);
}

#[test]
fn full_manual_archive_rejects_a_new_entry_without_dropping_pin_or_tracking() {
    let directory = TestDirectory::new();
    let persistence = DesktopPersistence::in_app_data(&directory.0);
    let target = session(StoveState::Failed);
    let mut state = PersistedState {
        pinned: vec![PinnedSession {
            session: target.clone(),
            pinned_at_ms: 900,
        }],
        archived: (0..4_096)
            .map(|index| ArchivedSession {
                session: session_with_id(&format!("archived-{index}"), StoveState::Disconnected),
                archived_at_ms: index,
                reason: ArchiveReason::Manual,
            })
            .collect(),
        tracked: vec![target.clone()],
        ..PersistedState::default()
    };
    let before = state.clone();

    assert!(!persistence
        .archive_session(&mut state, target, 1_000, ArchiveReason::Manual)
        .unwrap());
    assert_eq!(state, before);
}

#[test]
fn failed_archive_writes_leave_restore_and_inventory_state_unchanged() {
    let directory = TestDirectory::new();
    let blocked_parent = directory.0.join("not-a-directory");
    fs::write(&blocked_parent, b"block atomic state writes").unwrap();
    let persistence = DesktopPersistence::in_app_data(&blocked_parent);

    let target = session(StoveState::Cooking);
    let mut restore_state = PersistedState {
        archived: vec![ArchivedSession {
            session: target.clone(),
            archived_at_ms: 900,
            reason: ArchiveReason::Manual,
        }],
        ..PersistedState::default()
    };
    let restore_before = restore_state.clone();
    assert!(persistence
        .restore_session(&mut restore_state, &target.locator, false, 1_000)
        .is_err());
    assert_eq!(restore_state, restore_before);

    let mut inventory_state = PersistedState::default();
    let inventory_before = inventory_state.clone();
    assert!(persistence
        .archive_expired_sessions(&mut inventory_state, [target], 1_000)
        .is_err());
    assert_eq!(inventory_state, inventory_before);
}
