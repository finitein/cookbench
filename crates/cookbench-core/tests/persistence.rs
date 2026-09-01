use std::{
    fs,
    path::PathBuf,
    sync::Arc,
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use cookbench_core::{
    domain::{
        EventMetadata, EventSource, HarnessId, HostIdentity, ProjectIdentity, Stove, StoveIdentity,
        StoveState,
    },
    notifications::NotificationEventKind,
    persistence::{
        AppLocale, ArchiveReason, ArchivedSession, AtomicJsonFile, ClearCursor,
        CookedAttentionCursor, GlobalBarPlacement, PersistedConfig, PersistedState, PinnedSession,
        RetainedStove, RetainedStovePresentation, SessionRecord,
    },
};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "cookbench-persistence-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn locator() -> StoveIdentity {
    StoveIdentity::new(
        HostIdentity::local("test-machine"),
        HarnessId::Codex,
        "native-session-1",
    )
}

fn locator_with(native_session_id: impl Into<String>) -> StoveIdentity {
    StoveIdentity::new(
        HostIdentity::local("test-machine"),
        HarnessId::Codex,
        native_session_id,
    )
}

fn cooked_stove(native_session_id: impl Into<String>, sequence: u64) -> Stove {
    Stove {
        identity: locator_with(native_session_id),
        project: ProjectIdentity::new(HostIdentity::local("test-machine"), "/synthetic/project"),
        state: StoveState::Cooked,
        progress: None,
        state_before_disconnect: None,
        last_event: Some(EventMetadata::new(
            EventSource::Hook,
            100,
            sequence,
            sequence,
        )),
    }
}

fn retained(completed_at_ms: u64) -> RetainedStove {
    RetainedStove::new(locator(), completed_at_ms)
}

fn session_record() -> SessionRecord {
    SessionRecord::new(
        locator(),
        Some("/safe/session.jsonl".to_owned()),
        42,
        RetainedStovePresentation::new("cookbench", "/safe/cookbench"),
        StoveState::Cooking,
    )
    .expect("safe session record")
}

#[test]
fn atomic_replacement_never_exposes_partial_json() {
    let temp = TestDirectory::new();
    let file = Arc::new(AtomicJsonFile::<PersistedState>::new(
        temp.file("state.json"),
    ));
    file.save(&PersistedState::default()).unwrap();

    let writer = {
        let file = Arc::clone(&file);
        thread::spawn(move || {
            for completed_at_ms in 1..=100 {
                file.save(&PersistedState::with_retained(vec![retained(
                    completed_at_ms,
                )]))
                .unwrap();
            }
        })
    };

    while !writer.is_finished() {
        match fs::read(file.path()) {
            Ok(bytes) => {
                let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                assert!(value.is_object());
            }
            Err(error)
                if cfg!(windows)
                    && (error.kind() == std::io::ErrorKind::NotFound
                        || matches!(error.raw_os_error(), Some(5 | 32))) =>
            {
                thread::yield_now();
            }
            Err(error) => panic!("unexpected persistence read failure: {error}"),
        }
    }
    writer.join().unwrap();
    assert_eq!(file.load().unwrap().retained[0].completed_at_ms, 100);
}

#[test]
fn cooked_summaries_survive_restart() {
    let temp = TestDirectory::new();
    let path = temp.file("state.json");
    AtomicJsonFile::new(&path)
        .save(&PersistedState::with_retained(vec![retained(42)]))
        .unwrap();

    let restarted = AtomicJsonFile::<PersistedState>::new(path).load().unwrap();
    assert_eq!(restarted.retained, vec![retained(42)]);
}

#[test]
fn persisted_schema_contains_only_safe_retained_fields() {
    let value = serde_json::to_value(PersistedState::with_retained(vec![retained(42)])).unwrap();
    let encoded = value.to_string();
    for forbidden in [
        "prompt",
        "transcript",
        "command",
        "output",
        "code",
        "token",
        "secret",
    ] {
        assert!(
            !encoded.contains(forbidden),
            "persisted state exposed {forbidden}"
        );
    }

    let retained = value["retained"][0].as_object().unwrap();
    assert_eq!(retained.len(), 5);
    assert!(retained.contains_key("locator"));
    assert!(retained.contains_key("completed_at_ms"));
    assert!(retained.contains_key("presentation"));
    assert!(retained.contains_key("completion_event"));
    assert!(retained.contains_key("completion_source_event"));
}

#[test]
fn retained_completion_source_event_is_metadata_only_and_backwards_compatible() {
    let source_event = EventMetadata::new(EventSource::Hook, 100, 900, 42);
    let presentation_event = EventMetadata::new(EventSource::Hook, 100, 2, 42);
    let state = PersistedState::with_retained(vec![RetainedStove::new(locator(), 42)
        .with_completion_events(source_event.clone(), presentation_event)]);
    let value = serde_json::to_value(&state).unwrap();
    assert_eq!(
        value["retained"][0]["completion_source_event"]["sequence"],
        900
    );
    assert!(!value.to_string().contains("prompt"));

    let mut legacy = value;
    legacy["retained"][0]
        .as_object_mut()
        .unwrap()
        .remove("completion_source_event");
    let decoded: PersistedState = serde_json::from_value(legacy).unwrap();
    assert_eq!(decoded.retained[0].completion_source_event, None);
    assert_eq!(
        decoded.retained[0]
            .completion_event
            .as_ref()
            .unwrap()
            .sequence,
        2
    );
}

#[test]
fn clear_cursor_hides_only_events_at_or_before_clear_point() {
    let cursor = ClearCursor::new(locator(), 10, 10_000);
    assert!(cursor.hides(
        &locator(),
        &EventMetadata::new(EventSource::Hook, 100, 9, 99_999)
    ));
    assert!(cursor.hides(
        &locator(),
        &EventMetadata::new(EventSource::Hook, 100, 10, 10_000)
    ));
    assert!(!cursor.hides(
        &locator(),
        &EventMetadata::new(EventSource::Hook, 100, 11, 1)
    ));
    assert!(!cursor.hides(
        &locator(),
        &EventMetadata::new(EventSource::Hook, 100, 10, 10_001)
    ));
}

#[test]
fn newer_prompt_relights_the_same_cleared_native_session() {
    let cleared = PersistedState {
        version: PersistedState::CURRENT_VERSION,
        retained: Vec::new(),
        clear_cursors: vec![ClearCursor::new(locator(), 10, 10_000)],
        cooked_attention_cursors: Vec::new(),
        pinned: Vec::new(),
        archived: Vec::new(),
        tracked: Vec::new(),
    };

    let newer_prompt = EventMetadata::new(EventSource::StructuredSession, 100, 11, 11_000);
    assert!(!cleared.is_hidden(&locator(), &newer_prompt));
}

#[test]
fn unknown_future_fields_are_ignored() {
    let temp = TestDirectory::new();
    let path = temp.file("state.json");
    fs::write(
        &path,
        r#"{"version":1,"retained":[],"clear_cursors":[],"future_field":{"nested":true}}"#,
    )
    .unwrap();
    let state = AtomicJsonFile::<PersistedState>::new(path).load().unwrap();
    assert_eq!(state.version, 1);
    assert!(state.retained.is_empty());
    assert!(state.clear_cursors.is_empty());

    let config_path = temp.file("config.json");
    fs::write(&config_path, r#"{"version":1,"future_setting":true}"#).unwrap();
    assert_eq!(
        AtomicJsonFile::<PersistedConfig>::new(config_path)
            .load()
            .unwrap(),
        PersistedConfig::default()
    );
}

#[test]
fn retained_stoves_without_a_presentation_migrate_to_an_empty_safe_default() {
    let legacy = r#"{
        "version": 1,
        "retained": [{
            "locator": {
                "host": {"kind":"Local","id":"test-host"},
                "harness":"Codex",
                "native_session_id":"opaque-session"
            },
            "completed_at_ms": 42
        }],
        "clear_cursors": []
    }"#;

    let state: PersistedState = serde_json::from_str(legacy).unwrap();
    assert_eq!(state.retained[0].presentation.project_label, "");
    assert_eq!(state.retained[0].presentation.project_root_display, "");
}

#[test]
fn malformed_json_does_not_replace_the_last_valid_state() {
    let temp = TestDirectory::new();
    let path = temp.file("state.json");
    let file = AtomicJsonFile::new(&path);
    file.save(&PersistedState::with_retained(vec![retained(42)]))
        .unwrap();
    let valid = fs::read(&path).unwrap();
    fs::write(&path, b"{ this is malformed JSON").unwrap();

    assert!(file.load().is_err());
    assert_eq!(fs::read(&path).unwrap(), b"{ this is malformed JSON");
    assert_ne!(fs::read(&path).unwrap(), valid);
}

#[test]
fn config_never_serializes_credential_values() {
    let config = PersistedConfig::default();
    let encoded = serde_json::to_string(&config).unwrap();
    assert!(!encoded.contains("credential_value"));
    assert!(config.layout.global_bar_visible);
    assert!(!config.layout.hover_details_enabled);
    assert_eq!(
        config.layout.global_bar_placement,
        GlobalBarPlacement::TopCenter
    );
    assert!(config.preferences.always_on_top);
    assert_eq!(config.preferences.locale, AppLocale::System);
}

#[test]
fn legacy_config_defaults_local_notifications_to_sound_and_approved_events() {
    let config: PersistedConfig = serde_json::from_str(
        r#"{"version":1,"preferences":{"always_on_top":true,"notifications_enabled":false}}"#,
    )
    .unwrap();

    let local = config.preferences.local_notifications;
    assert_eq!(config.preferences.locale, AppLocale::System);
    assert!(local.sound);
    assert!(!local.system_banner);
    assert!(!local.bar_flash);
    assert!(!local.system_attention);
    assert_eq!(
        local.events,
        vec![
            NotificationEventKind::NeedsHuman,
            NotificationEventKind::Cooked,
            NotificationEventKind::Failed,
            NotificationEventKind::Disconnected,
        ]
    );
}

#[test]
fn interface_locale_round_trips_without_exposing_session_data() {
    let mut config = PersistedConfig::default();
    config.preferences.locale = AppLocale::ZhCn;

    let encoded = serde_json::to_string(&config).unwrap();
    assert!(encoded.contains("\"locale\":\"zh-CN\""));
    let restored: PersistedConfig = serde_json::from_str(&encoded).unwrap();
    assert_eq!(restored.preferences.locale, AppLocale::ZhCn);
}

#[test]
fn local_notification_preferences_round_trip_custom_channels_and_deduplicated_events() {
    let config: PersistedConfig = serde_json::from_str(
        r#"{
            "version": 1,
            "preferences": {
                "local_notifications": {
                    "sound": false,
                    "system_banner": true,
                    "bar_flash": true,
                    "system_attention": true,
                    "events": ["Cooked", "Failed", "Cooked", "SessionAppeared", "Failed"]
                }
            }
        }"#,
    )
    .unwrap();

    assert_eq!(
        config.preferences.local_notifications.events,
        vec![
            NotificationEventKind::SessionAppeared,
            NotificationEventKind::Cooked,
            NotificationEventKind::Failed,
        ]
    );

    let restored: PersistedConfig =
        serde_json::from_value(serde_json::to_value(&config).unwrap()).unwrap();
    assert_eq!(restored, config);
}

#[test]
fn v2_state_defaults_new_session_collections() {
    let v2 = r#"{"version":2,"retained":[],"clear_cursors":[]}"#;
    let state: PersistedState = serde_json::from_str(v2).unwrap();
    assert!(state.pinned.is_empty());
    assert!(state.archived.is_empty());
    assert!(state.tracked.is_empty());
    assert!(state.cooked_attention_cursors.is_empty());
}

#[test]
fn cooked_attention_cursors_are_bounded_and_keep_the_newest_acknowledgements() {
    let mut state = PersistedState::default();
    for sequence in 0..=PersistedState::MAX_COOKED_ATTENTION_CURSORS as u64 {
        assert!(state.acknowledge_cooked(
            &cooked_stove(format!("native-session-{sequence}"), sequence),
            sequence,
        ));
    }

    assert_eq!(
        state.cooked_attention_cursors.len(),
        PersistedState::MAX_COOKED_ATTENTION_CURSORS
    );
    assert!(!state
        .cooked_attention_cursors
        .iter()
        .any(|cursor| cursor.sequence == 0));
    assert!(state
        .cooked_attention_cursors
        .iter()
        .any(|cursor| cursor.sequence == PersistedState::MAX_COOKED_ATTENTION_CURSORS as u64));
}

#[test]
fn deserialized_cooked_attention_cursors_are_bounded_deterministically() {
    let cursors: Vec<_> = (0..=PersistedState::MAX_COOKED_ATTENTION_CURSORS as u64)
        .map(|index| CookedAttentionCursor {
            locator: locator_with(format!("cursor-{index:03}")),
            source: EventSource::Hook,
            confidence: 100,
            sequence: 1,
            timestamp_ms: 1,
            acknowledged_at_ms: 1,
        })
        .collect();
    let first = PersistedState {
        cooked_attention_cursors: cursors.clone(),
        ..PersistedState::default()
    };
    let second = PersistedState {
        cooked_attention_cursors: cursors.into_iter().rev().collect(),
        ..PersistedState::default()
    };

    let first: PersistedState =
        serde_json::from_value(serde_json::to_value(first).unwrap()).unwrap();
    let second: PersistedState =
        serde_json::from_value(serde_json::to_value(second).unwrap()).unwrap();

    assert_eq!(
        first.cooked_attention_cursors.len(),
        PersistedState::MAX_COOKED_ATTENTION_CURSORS
    );
    assert_eq!(
        first.cooked_attention_cursors,
        second.cooked_attention_cursors
    );
    assert!(!first
        .cooked_attention_cursors
        .iter()
        .any(|cursor| cursor.locator.native_session_id == "cursor-000"));
}

#[test]
fn deserialized_cursors_deduplicate_nonadjacent_locator_entries() {
    let cursor = |native_session_id: &str, acknowledged_at_ms| CookedAttentionCursor {
        locator: locator_with(native_session_id),
        source: EventSource::Hook,
        confidence: 100,
        sequence: acknowledged_at_ms,
        timestamp_ms: acknowledged_at_ms,
        acknowledged_at_ms,
    };
    let state = PersistedState {
        cooked_attention_cursors: vec![
            cursor("same-session", 3),
            cursor("another-session", 2),
            cursor("same-session", 1),
        ],
        ..PersistedState::default()
    };

    let state: PersistedState =
        serde_json::from_value(serde_json::to_value(state).unwrap()).unwrap();
    assert_eq!(state.cooked_attention_cursors.len(), 2);
    assert_eq!(state.cooked_attention_cursors[0].acknowledged_at_ms, 3);
}

#[test]
fn cooked_attention_cursor_upsert_replaces_the_same_locator() {
    let mut state = PersistedState::default();
    let first = cooked_stove("native-session-1", 1);
    let replacement = cooked_stove("native-session-1", 2);
    assert!(state.acknowledge_cooked(&first, 1));
    assert!(state.acknowledge_cooked(&replacement, 2));

    assert_eq!(state.cooked_attention_cursors.len(), 1);
    assert!(state.cooked_attention_cursors[0].acknowledges(&replacement));
}

#[test]
fn nonempty_cooked_attention_cursor_schema_is_metadata_only() {
    let mut state = PersistedState::default();
    let cooked = cooked_stove("native-session-1", 7);
    assert!(state.acknowledge_cooked(&cooked, 9));

    let value = serde_json::to_value(state).unwrap();
    let cursor = value["cooked_attention_cursors"][0].as_object().unwrap();
    assert_eq!(cursor.len(), 6);
    for expected in [
        "locator",
        "source",
        "confidence",
        "sequence",
        "timestamp_ms",
        "acknowledged_at_ms",
    ] {
        assert!(cursor.contains_key(expected));
    }
    let encoded = value.to_string();
    for forbidden in [
        "prompt",
        "transcript",
        "command",
        "output",
        "task",
        "activity",
    ] {
        assert!(!encoded.contains(forbidden), "cursor exposed {forbidden}");
    }
}

#[test]
fn acknowledging_an_old_completion_uses_the_confirmation_time_for_eviction() {
    let cursors: Vec<_> = (1..=PersistedState::MAX_COOKED_ATTENTION_CURSORS as u64)
        .map(|index| CookedAttentionCursor {
            locator: locator_with(format!("newer-completion-{index}")),
            source: EventSource::Hook,
            confidence: 100,
            sequence: index,
            timestamp_ms: index,
            acknowledged_at_ms: index,
        })
        .collect();
    let mut state = PersistedState {
        cooked_attention_cursors: cursors,
        ..PersistedState::default()
    };
    let old_completion = cooked_stove("old-completion", 0);

    assert!(state.acknowledge_cooked(&old_completion, 1_000));
    assert_eq!(
        state.cooked_attention_cursors.len(),
        PersistedState::MAX_COOKED_ATTENTION_CURSORS
    );
    assert!(state
        .cooked_attention_cursors
        .iter()
        .any(|cursor| cursor.acknowledges(&old_completion)));
    assert!(!state
        .cooked_attention_cursors
        .iter()
        .any(|cursor| cursor.locator.native_session_id == "newer-completion-1"));
}

#[test]
fn session_records_keep_only_safe_metadata() {
    let record = session_record();
    let state = PersistedState {
        version: PersistedState::CURRENT_VERSION,
        retained: Vec::new(),
        clear_cursors: Vec::new(),
        cooked_attention_cursors: Vec::new(),
        pinned: vec![PinnedSession {
            session: record.clone(),
            pinned_at_ms: 50,
        }],
        archived: vec![ArchivedSession {
            session: record,
            archived_at_ms: 51,
            reason: ArchiveReason::Manual,
        }],
        tracked: Vec::new(),
    };
    let encoded = serde_json::to_string(&state).unwrap();
    for forbidden in [
        "prompt",
        "transcript",
        "command",
        "output",
        "task",
        "activity",
    ] {
        assert!(
            !encoded.contains(forbidden),
            "persisted state exposed {forbidden}"
        );
    }
    assert!(SessionRecord::new(
        locator(),
        Some("bad\nlocator".to_owned()),
        1,
        RetainedStovePresentation::default(),
        StoveState::Starting,
    )
    .is_none());
}

#[test]
fn legacy_layouts_keep_the_global_bar_visible() {
    let config: PersistedConfig = serde_json::from_str(
        r#"{"version":1,"layout":{"detached_stoves":[],"detached_layouts":[]}}"#,
    )
    .unwrap();

    assert!(config.layout.global_bar_visible);
    assert!(!config.layout.hover_details_enabled);
    assert_eq!(
        config.layout.global_bar_placement,
        GlobalBarPlacement::TopCenter
    );
}

#[test]
fn legacy_layouts_leave_global_bar_size_unset() {
    let config: PersistedConfig = serde_json::from_str(
        r#"{"version":1,"layout":{"detached_stoves":[],"detached_layouts":[]}}"#,
    )
    .unwrap();

    assert_eq!(config.layout.global_bar_size, None);
}

#[test]
fn legacy_global_bar_presets_do_not_override_freeform_window_size() {
    let config: PersistedConfig = serde_json::from_str(
        r#"{"version":1,"layout":{"global_bar_visible":false,"global_bar_placement":"bottomRight","global_bar_size":"wide","detached_stoves":[],"detached_layouts":[]}}"#,
    )
    .unwrap();

    assert_eq!(config.layout.global_bar_size, None);
    assert!(!config.layout.global_bar_visible);
    assert_eq!(
        config.layout.global_bar_placement,
        GlobalBarPlacement::BottomRight
    );
}
