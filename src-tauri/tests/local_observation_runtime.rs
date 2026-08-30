use std::{
    fs::{self, File, FileTimes},
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use cookbench_core::domain::{EventKind, HostIdentity, ProjectIdentity, StoveEvent, StoveIdentity};
use cookbench_desktop_lib::runtime::{
    LocalObservationConfig, LocalObservationRuntime, ObservationOrigin, ObservationSink,
    ObservationSummary,
};

#[derive(Default)]
struct Sink(Mutex<Vec<(StoveIdentity, ProjectIdentity, StoveEvent)>>);
impl ObservationSink for Sink {
    fn apply(
        &self,
        identity: StoveIdentity,
        project: ProjectIdentity,
        _: String,
        _: Option<cookbench_core::locator::HostApplication>,
        _: Option<String>,
        _: ObservationSummary,
        _: ObservationOrigin,
        event: StoveEvent,
    ) {
        self.0.lock().unwrap().push((identity, project, event));
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
    write(&claude, "{\"type\":\"user\",\"session_name\":\"synthetic claude\"}\n{\"type\":\"event\",\"subtype\":\"stop\"}\n");
    write(&pi, "{\"type\":\"session_start\",\"sessionId\":\"pi-1\",\"cwd\":\"/synthetic/pi\"}\n{\"type\":\"turn_completed\",\"status\":\"success\"}\n");

    let sink = Arc::new(Sink::default());
    let config = LocalObservationConfig {
        host: HostIdentity::local("synthetic-host"),
        codex_root: codex_root.clone(),
        claude_root: claude_root.clone(),
        pi_roots: vec![pi_root.clone()],
        startup_min_modified: SystemTime::UNIX_EPOCH,
        startup_candidate_limit: 16,
    };
    let mut runtime = LocalObservationRuntime::new(config, sink.clone());
    runtime.bootstrap();
    assert_eq!(runtime.session_count(), 3);
    let statuses = runtime.source_status();
    assert_eq!(statuses.sources.len(), 3);
    assert!(statuses
        .sources
        .iter()
        .all(|source| source.discovered_sessions == 1));
    assert!(statuses
        .sources
        .iter()
        .all(|source| source.parser_errors == 0));
    let status_json = serde_json::to_string(&statuses).unwrap();
    assert!(!status_json.contains("turn_completed"));
    assert!(!status_json.contains("user_prompt"));
    let initial = sink.0.lock().unwrap();
    assert_eq!(initial.len(), 9);
    assert_eq!(
        initial
            .iter()
            .filter(|(_, _, event)| matches!(event.kind, EventKind::SessionDiscovered))
            .count(),
        5
    );
    assert_eq!(
        initial
            .iter()
            .filter(|(_, _, event)| matches!(event.kind, EventKind::TurnCompleted))
            .count(),
        3
    );
    drop(initial);

    append(&codex, "{\"type\":\"user_message\"}\n");
    append(&claude, "{\"type\":\"user\"}\n");
    append(&pi, "{\"type\":\"user_prompt\",\"sessionId\":\"pi-1\"}\n");
    runtime.observe_path(&codex);
    runtime.observe_path(&claude);
    runtime.observe_path(&pi);
    let observed = sink.0.lock().unwrap();
    assert_eq!(observed.len(), 12, "observed: {observed:?}");
    assert!(observed[9..]
        .iter()
        .all(|(_, _, event)| matches!(event.kind, EventKind::UserPromptSubmitted)));
    let wire_debug = format!(
        "{:?}",
        observed
            .iter()
            .map(|(_, _, event)| &event.kind)
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
    };
    let mut runtime = LocalObservationRuntime::new(config, sink.clone());
    runtime.bootstrap();

    assert_eq!(runtime.session_count(), 1);
    let events = sink.0.lock().unwrap();
    assert!(events
        .iter()
        .all(|(identity, _, _)| { identity.native_session_id == "root-session" }));
    drop(events);
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
    };
    let mut runtime = LocalObservationRuntime::new(config, sink.clone());
    runtime.bootstrap();

    assert_eq!(runtime.session_count(), 1);
    assert!(sink
        .0
        .lock()
        .unwrap()
        .iter()
        .all(|(identity, _, _)| identity.native_session_id == "root-session"));
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
        .any(|(_, _, event)| matches!(event.kind, EventKind::SessionDiscovered)));
    fs::remove_dir_all(root).unwrap();
}
