use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use cookbench_core::{
    domain::{EventMetadata, EventSource, HarnessId, HostIdentity, StoveIdentity, StoveState},
    persistence::{
        ClearCursor, PersistedConfig, PersistedState, RetainedStove, RetainedStovePresentation,
    },
};
use cookbench_desktop_lib::{
    app_state::{AppState, StoveStateWire},
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

#[test]
fn pre_release_replay_cache_is_migrated_without_losing_clear_cursors() {
    let directory = TestDirectory::new();
    let persistence = DesktopPersistence::in_app_data(&directory.0);
    let legacy = PersistedState {
        version: 1,
        retained: vec![RetainedStove::new(locator(), 800)],
        clear_cursors: vec![ClearCursor::new(locator(), 7, 700)],
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
        .persist_transition(&mut state, locator(), StoveState::Cooked, &event(8, 800))
        .unwrap();
    persistence
        .clear_cooked(&mut state, locator(), &event(9, 900))
        .unwrap();

    let restarted = DesktopPersistence::in_app_data(&directory.0).load();
    assert!(restarted.state.retained.is_empty());
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
        .persist_transition(&mut state, locator(), StoveState::Cooked, &event(8, 800))
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
        .persist_transition(&mut state, locator(), StoveState::Cooked, &event(8, 800))
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
