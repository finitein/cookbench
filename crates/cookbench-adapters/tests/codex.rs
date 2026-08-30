use std::{fs, path::PathBuf};

use cookbench_adapters::{
    codex::{
        codex_home_from, correlate_processes, default_codex_home, inspect_notify_hook,
        parse_record, sanitize_fixture_record, CodexAdapter, CodexProcess, CodexThreadSource,
        NotifyHookPlan,
    },
    HarnessAdapter, HostSource, SessionLocatorKind,
};
use cookbench_core::{
    domain::{EventKind, HarnessId, HostIdentity},
    locator::HostApplication,
};
use serde_json::Value;
use tokio::sync::mpsc;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/codex")
}

#[tokio::test]
async fn discovers_codex_sessions_and_emits_only_metadata_events() {
    let adapter = CodexAdapter::with_root(fixture_root(), HostIdentity::local("test-host"));
    let sessions = adapter
        .discover(&HostSource::local(HostIdentity::local("test-host")))
        .await
        .unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].harness, HarnessId::Codex);
    assert_eq!(sessions[0].native_session_id, "session-0001");
    assert_eq!(
        sessions[0].project.as_ref().unwrap().canonical_root,
        "/workspace/demo"
    );
    assert_eq!(sessions[0].locator.kind, SessionLocatorKind::LocalPath);
    assert!(sessions[0].title.is_none());

    let (sender, mut receiver) = mpsc::channel(16);
    let handle = adapter.watch(sender.into()).await.unwrap();
    let mut observed = Vec::new();
    for _ in 0..7 {
        observed.push(receiver.recv().await.unwrap());
    }
    assert!(matches!(
        observed[0].event.kind,
        EventKind::SessionDiscovered
    ));
    assert!(observed
        .iter()
        .any(|event| matches!(event.event.kind, EventKind::UserPromptSubmitted)));
    assert!(observed
        .iter()
        .any(|event| matches!(event.event.kind, EventKind::ToolStarted)));
    assert!(observed.iter().any(|event| matches!(
        event.event.kind,
        EventKind::ToolCompleted { succeeded: true }
    )));
    assert!(observed.iter().any(|event| matches!(
        event.event.kind,
        EventKind::PlanUpdated {
            completed: 1,
            total: 3
        }
    )));
    assert!(observed
        .iter()
        .any(|event| matches!(event.event.kind, EventKind::TurnCompleted)));
    handle.cancel();
}

#[test]
fn honors_codex_home_override() {
    assert_eq!(
        codex_home_from(
            Some("/tmp/custom-codex".into()),
            Some("/home/ignored".into())
        ),
        PathBuf::from("/tmp/custom-codex")
    );
    assert!(default_codex_home().ends_with(".codex") || std::env::var_os("CODEX_HOME").is_some());
}

#[test]
fn parses_lifecycle_progress_failure_and_ignores_unknown_records() {
    let event = |line| {
        parse_record(line, 42, 32, 1024)
            .unwrap()
            .event
            .unwrap()
            .kind
    };
    assert!(matches!(
        event(r#"{"type":"user_message","payload":{"text":"secret"}}"#),
        EventKind::UserPromptSubmitted
    ));
    assert!(matches!(
        event(r#"{"type":"tool_started","payload":{}}"#),
        EventKind::ToolStarted
    ));
    assert!(matches!(
        event(r#"{"type":"tool_completed","payload":{"success":false}}"#),
        EventKind::ToolCompleted { succeeded: false }
    ));
    assert!(matches!(
        event(
            r#"{"type":"update_plan","payload":{"plan":[{"status":"done"},{"status":"pending"}]}}"#
        ),
        EventKind::PlanUpdated {
            completed: 1,
            total: 2
        }
    ));
    assert!(matches!(
        event(r#"{"type":"turn_failed","payload":{}}"#),
        EventKind::SessionFailed
    ));
    assert!(parse_record(
        r#"{"type":"event_msg","payload":{"type":"error"}}"#,
        1,
        32,
        1024
    )
    .unwrap()
    .event
    .is_none());
    assert!(parse_record(
        r#"{"type":"future_record","payload":{"text":"secret"}}"#,
        1,
        32,
        1024
    )
    .unwrap()
    .event
    .is_none());
    assert!(matches!(
        event(
            r#"{"type":"response_item","payload":{"type":"message","role":"user","content":"secret"}}"#
        ),
        EventKind::UserPromptSubmitted
    ));
    assert!(matches!(
        event(r#"{"type":"event_msg","payload":{"type":"task_complete"}}"#),
        EventKind::TurnCompleted
    ));
    assert!(matches!(
        event(r#"{"type":"event_msg","payload":{"type":"permission_requested"}}"#),
        EventKind::PermissionRequested
    ));
    assert!(matches!(
        event(
            r#"{"type":"response_item","payload":{"type":"function_call","name":"request_user_input","arguments":"[redacted]"}}"#
        ),
        EventKind::QuestionAsked
    ));
}

#[test]
fn classifies_only_safe_structural_codex_session_metadata() {
    let root = parse_record(
        r#"{"type":"session_meta","payload":{"id":"root-1","cwd":"/synthetic/root","originator":"codex_work_desktop","thread_source":"user"}}"#,
        1,
        32,
        1024,
    )
    .unwrap();
    let metadata = root.session_metadata.unwrap();
    assert_eq!(metadata.thread_source, CodexThreadSource::User);
    assert!(metadata.is_desktop_origin());
    assert!(!metadata.is_subagent());

    let child = parse_record(
        r#"{"type":"session_meta","payload":{"id":"child-1","thread_source":"subagent","source":{"subagent":{"thread_spawn":{"parent_thread_id":"root-1","depth":2}}}}}"#,
        1,
        32,
        1024,
    )
    .unwrap();
    let metadata = child.session_metadata.unwrap();
    assert!(metadata.is_subagent());
    assert_eq!(metadata.parent_session_id.as_deref(), Some("root-1"));
    assert_eq!(metadata.subagent_depth, Some(2));

    let unknown = parse_record(
        r#"{"type":"session_meta","payload":{"thread_source":"future-worker","originator":{"not":"a string"}}}"#,
        1,
        32,
        1024,
    )
    .unwrap();
    let metadata = unknown.session_metadata.unwrap();
    assert_eq!(metadata.thread_source, CodexThreadSource::Unknown);
    assert!(!metadata.is_subagent());
    assert!(!metadata.is_desktop_origin());
}

#[tokio::test]
async fn hides_codex_subagent_sessions_but_keeps_root_sessions() {
    let root = std::env::temp_dir().join(format!(
        "cookbench-codex-root-filter-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("root.jsonl"),
        format!(
            "{}\n",
            r#"{"type":"session_meta","payload":{"id":"root-session","cwd":"/synthetic/root","originator":"codex_work_desktop","thread_source":"user"}}"#
        ),
    )
    .unwrap();
    fs::write(
        root.join("subagent.jsonl"),
        format!(
            "{}\n",
            r#"{"type":"session_meta","payload":{"id":"child-session","cwd":"/synthetic/root","thread_source":"subagent"}}"#
        ),
    )
    .unwrap();

    let adapter = CodexAdapter::with_root(&root, HostIdentity::local("test-host"));
    let sessions = adapter
        .discover(&HostSource::local(HostIdentity::local("test-host")))
        .await
        .unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].native_session_id, "root-session");
    assert_eq!(
        sessions[0].host_application,
        Some(HostApplication::CodexDesktop)
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn fixture_sanitizer_preserves_ids_times_paths_and_plan_status_without_content() {
    let raw = r#"{"timestamp":1725000000000,"type":"update_plan","payload":{"id":"session-0001","cwd":"/workspace/demo","prompt":"private","command":"rm -rf /","plan":[{"step":"secret","status":"completed"}]}}"#;
    let sanitized: Value = serde_json::from_str(&sanitize_fixture_record(raw).unwrap()).unwrap();
    assert_eq!(sanitized["timestamp"], 1725000000000_u64);
    assert_eq!(sanitized["payload"]["id"], "session-0001");
    assert_eq!(sanitized["payload"]["cwd"], "/workspace/demo");
    assert_eq!(sanitized["payload"]["plan"][0]["status"], "completed");
    assert_eq!(sanitized["payload"]["prompt"], "[redacted]");
    assert_eq!(sanitized["payload"]["command"], "[redacted]");
    assert_eq!(sanitized["payload"]["plan"][0]["step"], "[redacted]");
}

#[tokio::test]
async fn correlates_processes_by_session_or_unique_project() {
    let adapter = CodexAdapter::with_root(fixture_root(), HostIdentity::local("test-host"));
    let sessions = adapter
        .discover(&HostSource::local(HostIdentity::local("test-host")))
        .await
        .unwrap();
    let matches = correlate_processes(
        &sessions,
        &[CodexProcess {
            pid: 7,
            session_id: Some("session-0001".into()),
            cwd: None,
        }],
    );
    assert_eq!(matches[0].0, 7);
    assert_eq!(matches[0].1.native_session_id, "session-0001");
}

#[test]
fn existing_notify_is_only_given_a_safe_chaining_plan() {
    let hook = vec!["cookbench-hook".to_owned(), "codex".to_owned()];
    assert_eq!(
        inspect_notify_hook("notify = [\"/usr/local/bin/notify\", \"--json\"]", &hook),
        NotifyHookPlan::Chain {
            existing_command: vec!["/usr/local/bin/notify".into(), "--json".into()],
            cookbench_command: hook.clone()
        }
    );
    assert_eq!(
        inspect_notify_hook("notify = \"notify && surprise\"", &hook),
        NotifyHookPlan::ReadOnlyFallback {
            reason: "notify is not a simple argv array"
        }
    );
}

#[test]
fn committed_fixture_is_sanitized() {
    let fixture = fs::read_to_string(fixture_root().join("session-0001.jsonl")).unwrap();
    assert!(!fixture.contains("secret"));
    assert!(!fixture.contains("rm -rf"));
}
