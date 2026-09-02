use std::{
    fs::{self, File, FileTimes},
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use cookbench_core::domain::{EventKind, HostIdentity, ProjectIdentity, StoveEvent, StoveIdentity};
use cookbench_desktop_lib::runtime::{
    start, LocalObservationConfig, LocalObservationRuntime, LocalSourceObservation,
    LocalSourceStatusState, ObservationOrigin, ObservationSink, ObservationSummary,
};

#[derive(Default)]
struct Sink(
    Mutex<
        Vec<(
            StoveIdentity,
            ProjectIdentity,
            cookbench_core::locator::SessionLocator,
            StoveEvent,
        )>,
    >,
);
impl ObservationSink for Sink {
    fn apply(
        &self,
        identity: StoveIdentity,
        project: ProjectIdentity,
        locator: cookbench_core::locator::SessionLocator,
        _: Option<String>,
        _: ObservationSummary,
        _: ObservationOrigin,
        event: StoveEvent,
    ) {
        self.0
            .lock()
            .unwrap()
            .push((identity, project, locator, event));
    }
}

#[derive(Default)]
struct SummarySink(Mutex<Vec<ObservationSummary>>);

impl ObservationSink for SummarySink {
    fn apply(
        &self,
        _: StoveIdentity,
        _: ProjectIdentity,
        _: cookbench_core::locator::SessionLocator,
        _: Option<String>,
        summary: ObservationSummary,
        _: ObservationOrigin,
        _: StoveEvent,
    ) {
        self.0.lock().unwrap().push(summary);
    }
}

fn temp_root() -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "cookbench-observation-{}-{nonce}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}
fn write(path: &PathBuf, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}
fn append(path: &PathBuf, content: &str) {
    use std::io::Write;
    fs::OpenOptions::new()
        .append(true)
        .open(path)
        .unwrap()
        .write_all(content.as_bytes())
        .unwrap();
}

#[test]
fn default_discovery_window_is_bounded_to_two_days() {
    let now = SystemTime::now();
    let config = LocalObservationConfig::from_environment(HostIdentity::local("synthetic-host"));
    let age = now.duration_since(config.startup_min_modified).unwrap();

    assert!(age >= Duration::from_secs(47 * 60 * 60));
    assert!(age <= Duration::from_secs(49 * 60 * 60));
}

#[test]
fn reconstructs_three_native_harnesses_then_observes_appended_lifecycle_records() {
    let root = temp_root();
    let codex_root = root.join("codex");
    let claude_root = root.join("claude");
    let pi_root = root.join("pi");
    let codex = codex_root.join("session.jsonl");
    let claude = claude_root
        .join("-synthetic-claude")
        .join("claude-session.jsonl");
    let pi = pi_root.join("pi-session.jsonl");
    write(&codex, "{\"type\":\"session_meta\",\"payload\":{\"id\":\"codex-1\",\"cwd\":\"/synthetic/codex\"}}\n{\"type\":\"turn_completed\"}\n");
    write(&claude, "{\"type\":\"user\",\"session_name\":\"synthetic claude\"}\n{\"type\":\"system\",\"subtype\":\"turn_duration\",\"durationMs\":4177}\n");
    write(
        &pi,
        "{\"type\":\"session\",\"id\":\"pi-1\",\"cwd\":\"/synthetic/pi\"}\n{\"type\":\"message\",\"message\":{\"role\":\"user\",\"content\":[]}}\n{\"type\":\"message\",\"message\":{\"role\":\"assistant\",\"stopReason\":\"stop\",\"content\":[]}}\n",
    );

    let sink = Arc::new(Sink::default());
    let config = LocalObservationConfig {
        host: HostIdentity::local("synthetic-host"),
        codex_root: codex_root.clone(),
        claude_root: claude_root.clone(),
        pi_roots: vec![pi_root.clone()],
        startup_min_modified: SystemTime::UNIX_EPOCH,
        startup_candidate_limit: 16,
        pinned_local_paths: Vec::new(),
    };
    let mut runtime = LocalObservationRuntime::new(config, sink.clone());
    runtime.bootstrap();
    assert_eq!(runtime.session_count(), 3);
    let statuses = runtime.source_status();
    assert_eq!(statuses.sources.len(), cookbench_adapters::catalog().len());
    assert!(statuses
        .sources
        .iter()
        .filter(|source| matches!(source.harness.as_str(), "codex" | "claudeCode" | "pi"))
        .all(|source| source.discovered_sessions == 1));
    assert!(statuses
        .sources
        .iter()
        .all(|source| source.parser_errors == 0));
    let workbuddy = statuses
        .sources
        .iter()
        .find(|source| source.harness == "workbuddy")
        .unwrap();
    assert_eq!(workbuddy.observation, LocalSourceObservation::PresenceOnly);
    let status_json = serde_json::to_string(&statuses).unwrap();
    assert!(!status_json.contains("turn_completed"));
    assert!(!status_json.contains("user_prompt"));
    let initial = sink.0.lock().unwrap();
    assert_eq!(initial.len(), 10);
    assert_eq!(
        initial
            .iter()
            .filter(|(_, _, _, event)| matches!(event.kind, EventKind::SessionDiscovered))
            .count(),
        5
    );
    assert_eq!(
        initial
            .iter()
            .filter(|(_, _, _, event)| matches!(event.kind, EventKind::TurnCompleted))
            .count(),
        3
    );
    assert!(initial
        .iter()
        .filter(|(identity, _, _, _)| identity.harness == cookbench_core::domain::HarnessId::Pi)
        .all(|(_, project, _, _)| project.canonical_root == "/synthetic/pi"));
    drop(initial);

    append(&codex, "{\"type\":\"user_message\"}\n");
    append(&claude, "{\"type\":\"user\"}\n");
    append(&pi, "{\"type\":\"user_prompt\",\"sessionId\":\"pi-1\"}\n");
    runtime.observe_path(&codex);
    runtime.observe_path(&claude);
    runtime.observe_path(&pi);
    let observed = sink.0.lock().unwrap();
    assert_eq!(observed.len(), 13, "observed: {observed:?}");
    assert!(observed[10..]
        .iter()
        .all(|(_, _, _, event)| matches!(event.kind, EventKind::UserPromptSubmitted)));
    let wire_debug = format!(
        "{:?}",
        observed
            .iter()
            .map(|(_, _, _, event)| &event.kind)
            .collect::<Vec<_>>()
    );
    assert!(!wire_debug.contains("transcript"));
    assert!(!wire_debug.contains("command"));
    drop(observed);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn runtime_does_not_register_codex_subagent_session_files() {
    let root = temp_root();
    let codex_root = root.join("codex");
    let root_session = codex_root.join("root.jsonl");
    let subagent_session = codex_root.join("subagent.jsonl");
    write(
        &root_session,
        &format!(
            "{}\n",
            r#"{"type":"session_meta","payload":{"id":"root-session","cwd":"/synthetic/root","thread_source":"user"}}"#
        ),
    );
    write(
        &subagent_session,
        &format!(
            "{}\n",
            r#"{"type":"session_meta","payload":{"id":"child-session","cwd":"/synthetic/root","thread_source":"subagent"}}"#
        ),
    );

    let sink = Arc::new(Sink::default());
    let config = LocalObservationConfig {
        host: HostIdentity::local("synthetic-host"),
        codex_root,
        claude_root: root.join("claude"),
        pi_roots: vec![root.join("pi")],
        startup_min_modified: SystemTime::UNIX_EPOCH,
        startup_candidate_limit: 16,
        pinned_local_paths: vec![subagent_session],
    };
    let mut runtime = LocalObservationRuntime::new(config, sink.clone());
    runtime.bootstrap();

    assert_eq!(runtime.session_count(), 1);
    let events = sink.0.lock().unwrap();
    assert!(events
        .iter()
        .all(|(identity, _, _, _)| { identity.native_session_id == "root-session" }));
    drop(events);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn preserves_distinct_native_locators_for_sessions_in_the_same_project() {
    let root = temp_root();
    let claude_root = root.join("claude");
    let first = claude_root.join("-synthetic-project").join("first.jsonl");
    let second = claude_root.join("-synthetic-project").join("second.jsonl");
    write(
        &first,
        "{\"type\":\"user\",\"session_name\":\"first\"}\n{\"type\":\"system\",\"subtype\":\"turn_duration\"}\n",
    );
    write(
        &second,
        "{\"type\":\"user\",\"session_name\":\"second\"}\n{\"type\":\"system\",\"subtype\":\"turn_duration\"}\n",
    );

    let sink = Arc::new(Sink::default());
    let config = LocalObservationConfig {
        host: HostIdentity::local("synthetic-host"),
        codex_root: root.join("codex"),
        claude_root,
        pi_roots: vec![root.join("pi")],
        startup_min_modified: SystemTime::UNIX_EPOCH,
        startup_candidate_limit: 16,
        pinned_local_paths: Vec::new(),
    };
    let mut runtime = LocalObservationRuntime::new(config, sink.clone());
    runtime.bootstrap();

    let locators = sink
        .0
        .lock()
        .unwrap()
        .iter()
        .filter(|(identity, _, _, _)| {
            identity.harness == cookbench_core::domain::HarnessId::ClaudeCode
        })
        .map(|(_, _, locator, _)| locator.native_locator.clone().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(locators.len(), 2);
    assert!(locators
        .iter()
        .any(|locator| locator.ends_with("first.jsonl")));
    assert!(locators
        .iter()
        .any(|locator| locator.ends_with("second.jsonl")));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn newer_subagents_do_not_consume_the_root_session_candidate_limit() {
    let root = temp_root();
    let codex_root = root.join("codex");
    let root_session = codex_root.join("root.jsonl");
    write(
        &root_session,
        r#"{"type":"session_meta","payload":{"id":"root-session","cwd":"/synthetic/root","thread_source":"user"}}
"#,
    );
    let base_time = UNIX_EPOCH + Duration::from_secs(10);
    File::options()
        .write(true)
        .open(&root_session)
        .unwrap()
        .set_times(FileTimes::new().set_modified(base_time))
        .unwrap();
    for index in 0..12 {
        let path = codex_root.join(format!("subagent-{index}.jsonl"));
        write(
            &path,
            &format!(
                "{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"child-{index}\",\"cwd\":\"/synthetic/root\",\"thread_source\":\"subagent\"}}}}\n"
            ),
        );
        File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_times(FileTimes::new().set_modified(base_time + Duration::from_secs(index + 1)))
            .unwrap();
    }

    let sink = Arc::new(Sink::default());
    let config = LocalObservationConfig {
        host: HostIdentity::local("synthetic-host"),
        codex_root,
        claude_root: root.join("claude"),
        pi_roots: vec![root.join("pi")],
        startup_min_modified: SystemTime::UNIX_EPOCH,
        startup_candidate_limit: 2,
        pinned_local_paths: Vec::new(),
    };
    let mut runtime = LocalObservationRuntime::new(config, sink.clone());
    runtime.bootstrap();

    assert_eq!(runtime.session_count(), 1);
    assert!(sink
        .0
        .lock()
        .unwrap()
        .iter()
        .all(|(identity, _, _, _)| identity.native_session_id == "root-session"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn filters_one_thousand_stale_paths_before_adapter_body_parsing() {
    let root = temp_root();
    let codex_root = root.join("codex");
    let claude_root = root.join("claude");
    let pi_root = root.join("pi");
    fs::create_dir_all(&codex_root).unwrap();
    fs::create_dir_all(&claude_root).unwrap();
    fs::create_dir_all(&pi_root).unwrap();
    let stale_time = UNIX_EPOCH + Duration::from_secs(1);
    for index in 0..1_000 {
        let path = codex_root.join(format!("stale-{index}.jsonl"));
        fs::write(&path, "body must not become a startup candidate").unwrap();
        File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_times(FileTimes::new().set_modified(stale_time))
            .unwrap();
    }
    let recent = codex_root.join("recent.jsonl");
    write(
        &recent,
        "{\"type\":\"session_meta\",\"payload\":{\"id\":\"recent-session\",\"cwd\":\"/synthetic/recent\"}}\n",
    );

    let sink = Arc::new(Sink::default());
    let config = LocalObservationConfig {
        host: HostIdentity::local("synthetic-host"),
        codex_root,
        claude_root,
        pi_roots: vec![pi_root],
        startup_min_modified: SystemTime::now() - Duration::from_secs(60),
        startup_candidate_limit: 16,
        pinned_local_paths: Vec::new(),
    };
    let mut runtime = LocalObservationRuntime::new(config, sink);
    runtime.bootstrap();

    assert_eq!(runtime.session_count(), 1);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rescan_discovers_a_harness_root_created_after_cookbench_started() {
    let root = temp_root();
    let codex_root = root.join("codex-created-later");
    let sink = Arc::new(Sink::default());
    let config = LocalObservationConfig {
        host: HostIdentity::local("synthetic-host"),
        codex_root: codex_root.clone(),
        claude_root: root.join("claude-missing"),
        pi_roots: vec![root.join("pi-missing")],
        startup_min_modified: SystemTime::UNIX_EPOCH,
        startup_candidate_limit: 16,
        pinned_local_paths: Vec::new(),
    };
    let mut runtime = LocalObservationRuntime::new(config, sink.clone());
    runtime.bootstrap();
    assert_eq!(runtime.session_count(), 0);

    let session = codex_root.join("created-after-start.jsonl");
    write(
        &session,
        "{\"type\":\"session_meta\",\"payload\":{\"id\":\"late-session\",\"cwd\":\"/synthetic/late\"}}\n",
    );
    runtime.rescan();

    assert_eq!(runtime.session_count(), 1);
    assert!(sink
        .0
        .lock()
        .unwrap()
        .iter()
        .any(|(_, _, _, event)| matches!(event.kind, EventKind::SessionDiscovered)));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn pinned_old_session_is_discovered_without_widening_normal_discovery() {
    let root = temp_root();
    let codex_root = root.join("codex");
    let pinned = codex_root.join("pinned.jsonl");
    let ordinary = codex_root.join("ordinary.jsonl");
    write(
        &pinned,
        "{\"type\":\"session_meta\",\"payload\":{\"id\":\"pinned-session\",\"cwd\":\"/synthetic/pinned\"}}\n",
    );
    write(
        &ordinary,
        "{\"type\":\"session_meta\",\"payload\":{\"id\":\"ordinary-session\",\"cwd\":\"/synthetic/ordinary\"}}\n",
    );
    let stale = UNIX_EPOCH + Duration::from_secs(10);
    for path in [&pinned, &ordinary] {
        File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_times(FileTimes::new().set_modified(stale))
            .unwrap();
    }

    let sink = Arc::new(Sink::default());
    let config = LocalObservationConfig {
        host: HostIdentity::local("synthetic-host"),
        codex_root: codex_root.clone(),
        claude_root: root.join("claude"),
        pi_roots: vec![root.join("pi")],
        startup_min_modified: SystemTime::now() - Duration::from_secs(60),
        startup_candidate_limit: 16,
        pinned_local_paths: vec![pinned],
    };
    let mut runtime = LocalObservationRuntime::new(config, sink.clone());
    runtime.bootstrap();

    assert_eq!(runtime.session_count(), 1);
    let identities = sink
        .0
        .lock()
        .unwrap()
        .iter()
        .map(|(identity, _, _, _)| identity.native_session_id.clone())
        .collect::<Vec<_>>();
    assert!(
        identities
            .iter()
            .all(|identity| identity == "pinned-session"),
        "unexpected identities: {identities:?}"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_restored_old_session_can_join_the_running_observer() {
    let root = temp_root();
    let codex_root = root.join("codex");
    let pinned = codex_root.join("restored.jsonl");
    write(
        &pinned,
        "{\"type\":\"session_meta\",\"payload\":{\"id\":\"restored-session\",\"cwd\":\"/synthetic/restored\"}}\n",
    );
    File::options()
        .write(true)
        .open(&pinned)
        .unwrap()
        .set_times(FileTimes::new().set_modified(UNIX_EPOCH + Duration::from_secs(10)))
        .unwrap();

    let sink = Arc::new(Sink::default());
    let config = LocalObservationConfig {
        host: HostIdentity::local("synthetic-host"),
        codex_root,
        claude_root: root.join("claude"),
        pi_roots: vec![root.join("pi")],
        startup_min_modified: SystemTime::now() - Duration::from_secs(60),
        startup_candidate_limit: 16,
        pinned_local_paths: Vec::new(),
    };
    let handle = start(config, sink.clone(), LocalSourceStatusState::default());
    assert!(handle.add_pinned_path(pinned));
    // The observer thread can share a saturated CI runner with the rest of the
    // Rust suite. Keep the assertion event-driven, but allow startup headroom.
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline
        && !sink
            .0
            .lock()
            .unwrap()
            .iter()
            .any(|(identity, _, _, _)| identity.native_session_id == "restored-session")
    {
        std::thread::sleep(Duration::from_millis(20));
    }
    handle.cancel();

    assert!(
        sink.0
            .lock()
            .unwrap()
            .iter()
            .any(|(identity, _, _, _)| { identity.native_session_id == "restored-session" }),
        "restored session was not observed before the startup deadline"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn restoring_an_already_watched_session_replays_events_seen_while_archived() {
    let root = temp_root();
    let codex_root = root.join("codex");
    let session = codex_root.join("restored-watched.jsonl");
    write(
        &session,
        "{\"type\":\"session_meta\",\"payload\":{\"id\":\"restored-watched\",\"cwd\":\"/synthetic/restored\"}}\n{\"type\":\"user_message\"}\n",
    );

    let sink = Arc::new(Sink::default());
    let config = LocalObservationConfig {
        host: HostIdentity::local("synthetic-host"),
        codex_root,
        claude_root: root.join("claude"),
        pi_roots: vec![root.join("pi")],
        startup_min_modified: SystemTime::UNIX_EPOCH,
        startup_candidate_limit: 16,
        pinned_local_paths: Vec::new(),
    };
    let handle = start(config, sink.clone(), LocalSourceStatusState::default());
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline
        && !sink
            .0
            .lock()
            .unwrap()
            .iter()
            .any(|(_, _, _, event)| matches!(event.kind, EventKind::UserPromptSubmitted))
    {
        std::thread::sleep(Duration::from_millis(20));
    }

    append(&session, "{\"type\":\"turn_completed\"}\n");
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline
        && !sink
            .0
            .lock()
            .unwrap()
            .iter()
            .any(|(_, _, _, event)| matches!(event.kind, EventKind::TurnCompleted))
    {
        std::thread::sleep(Duration::from_millis(20));
    }
    sink.0.lock().unwrap().clear();

    assert!(handle.refresh_path(session));
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline
        && !sink
            .0
            .lock()
            .unwrap()
            .iter()
            .any(|(_, _, _, event)| matches!(event.kind, EventKind::TurnCompleted))
    {
        std::thread::sleep(Duration::from_millis(20));
    }
    handle.cancel();

    assert!(sink
        .0
        .lock()
        .unwrap()
        .iter()
        .any(|(_, _, _, event)| matches!(event.kind, EventKind::TurnCompleted)));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_pinned_paths_are_ignored_without_expanding_file_access() {
    let root = temp_root();
    let codex_root = root.join("codex");
    let outside = root.join("outside.jsonl");
    let valid = codex_root.join("valid.jsonl");
    write(
        &outside,
        r#"{"type":"session_meta","payload":{"id":"outside","cwd":"/synthetic/outside"}}"#,
    );
    write(
        &valid,
        r#"{"type":"session_meta","payload":{"id":"valid","cwd":"/synthetic/valid"}}"#,
    );
    let stale = UNIX_EPOCH + Duration::from_secs(10);
    for path in [&outside, &valid] {
        File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_times(FileTimes::new().set_modified(stale))
            .unwrap();
    }
    let sink = Arc::new(Sink::default());
    let config = LocalObservationConfig {
        host: HostIdentity::local("synthetic-host"),
        codex_root,
        claude_root: root.join("claude"),
        pi_roots: vec![root.join("pi")],
        startup_min_modified: SystemTime::now() - Duration::from_secs(60),
        startup_candidate_limit: 16,
        pinned_local_paths: vec![outside, root.join("missing.jsonl"), root.join("bad.txt")],
    };
    let mut runtime = LocalObservationRuntime::new(config, sink);
    runtime.bootstrap();

    assert_eq!(runtime.session_count(), 0);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn observation_summary_includes_source_mtime_without_reading_extra_records() {
    let root = temp_root();
    let codex_root = root.join("codex");
    let session = codex_root.join("session.jsonl");
    write(
        &session,
        r#"{"type":"session_meta","payload":{"id":"mtime-session","cwd":"/synthetic/mtime"}}"#,
    );
    let modified = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    File::options()
        .write(true)
        .open(&session)
        .unwrap()
        .set_times(FileTimes::new().set_modified(modified))
        .unwrap();
    let sink = Arc::new(SummarySink::default());
    let config = LocalObservationConfig {
        host: HostIdentity::local("synthetic-host"),
        codex_root,
        claude_root: root.join("claude"),
        pi_roots: vec![root.join("pi")],
        startup_min_modified: SystemTime::UNIX_EPOCH,
        startup_candidate_limit: 16,
        pinned_local_paths: Vec::new(),
    };
    let mut runtime = LocalObservationRuntime::new(config, sink.clone());
    runtime.bootstrap();

    assert!(sink
        .0
        .lock()
        .unwrap()
        .iter()
        .any(|summary| { summary.source_modified_at_ms == Some(1_700_000_000_000) }));
    fs::remove_dir_all(root).unwrap();
}
