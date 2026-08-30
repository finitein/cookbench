use std::{fs, path::PathBuf};

use cookbench_adapters::{
    claude::{
        decode_project_path, encode_project_path, install_hooks, install_hooks_with_command,
        parse_record, uninstall_all_cookbench_hooks, uninstall_hooks, uninstall_hooks_with_command,
        ClaudeAdapter,
    },
    io::TailLimits,
    HarnessAdapter, HostSource,
};
use cookbench_core::domain::{EventKind, HostIdentity};
use serde_json::json;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/claude")
}

#[tokio::test]
async fn discovers_sanitized_claude_sessions_with_titles_and_project_paths() {
    let adapter = ClaudeAdapter::new(fixture_root());
    let sessions = adapter
        .discover(&HostSource::local(HostIdentity::local("fixture-host")))
        .await
        .unwrap();

    assert_eq!(sessions.len(), 3);
    let session = sessions
        .iter()
        .find(|session| session.native_session_id == "claude-session-001")
        .unwrap();
    assert_eq!(session.title.as_deref(), Some("Synthetic Claude task"));
    assert_eq!(
        session.project.as_ref().unwrap().canonical_root,
        "/workspace/demo"
    );
    assert!(adapter.capabilities().structured_progress);
    assert!(!adapter.capabilities().watch_events);
}

#[test]
fn project_path_encoding_round_trips_and_rejects_traversal() {
    let original = PathBuf::from("/workspace/demo-name");
    let encoded = encode_project_path(&original).unwrap();
    assert_eq!(decode_project_path(&encoded), Some(original));
    assert_eq!(decode_project_path("-workspace-..-secret"), None);
    assert_eq!(decode_project_path("workspace-demo"), None);
}

#[test]
fn parser_normalizes_tasks_lifecycle_attention_completion_failure_and_ignores_unknown() {
    let text = fs::read_to_string(fixture_root().join("-workspace-demo/claude-session-001.jsonl"))
        .unwrap();
    let records = text
        .lines()
        .enumerate()
        .filter_map(|(index, line)| parse_record(line, TailLimits::default(), index as u64 + 1))
        .collect::<Vec<_>>();
    let events = records
        .into_iter()
        .flat_map(|record| record.events)
        .map(|event| event.kind)
        .collect::<Vec<_>>();

    assert!(events.iter().any(|event| matches!(
        event,
        EventKind::PlanUpdated {
            completed: 1,
            total: 3
        }
    )));
    assert!(events
        .iter()
        .any(|event| matches!(event, EventKind::ToolStarted)));
    assert!(events
        .iter()
        .any(|event| matches!(event, EventKind::ToolCompleted { succeeded: true })));
    assert!(events
        .iter()
        .any(|event| matches!(event, EventKind::PermissionRequested)));
    assert!(events
        .iter()
        .any(|event| matches!(event, EventKind::QuestionAsked)));
    assert!(events
        .iter()
        .any(|event| matches!(event, EventKind::TurnCompleted)));
    assert_eq!(
        parse_record(
            r#"{"type":"unknown_future_record"}"#,
            TailLimits::default(),
            99
        ),
        None
    );
    let failed = parse_record(
        r#"{"type":"system","subtype":"error"}"#,
        TailLimits::default(),
        100,
    )
    .unwrap();
    assert!(failed
        .events
        .iter()
        .any(|event| matches!(event.kind, EventKind::SessionFailed)));
}

#[test]
fn parser_uses_structured_turn_duration_for_turn_completion() {
    let completed = parse_record(
        r#"{"type":"system","subtype":"turn_duration","durationMs":4177}"#,
        TailLimits::default(),
        101,
    )
    .expect("Claude's explicit turn terminator should be observable");
    assert!(completed
        .events
        .iter()
        .any(|event| matches!(event.kind, EventKind::TurnCompleted)));

    let tool_use = parse_record(
        r#"{"type":"assistant","message":{"stop_reason":"tool_use","content":[{"type":"tool_use"}]}}"#,
        TailLimits::default(),
        102,
    )
    .expect("tool use should remain observable activity");
    assert!(tool_use
        .events
        .iter()
        .any(|event| matches!(event.kind, EventKind::ToolStarted)));
    assert!(!tool_use
        .events
        .iter()
        .any(|event| matches!(event.kind, EventKind::TurnCompleted)));
}

#[test]
fn hook_transforms_preserve_existing_entries_and_uninstall_deterministically() {
    let original = json!({
        "hooks": {
            "PreToolUse": [{"matcher":"Bash", "hooks":[{"type":"command", "command":"existing-helper"}]}],
            "UserPromptSubmit": [{"matcher":"*", "hooks":[{"type":"command", "command":"other-helper"}]}]
        },
        "unrelated": true
    });
    let installed = install_hooks(&original).unwrap();
    assert!(installed.changed);
    assert!(installed.backup.required);
    assert_eq!(
        installed.configuration["hooks"]["PreToolUse"][0]["hooks"][0]["command"],
        "existing-helper"
    );
    assert_eq!(
        install_hooks(&installed.configuration)
            .unwrap()
            .configuration,
        installed.configuration
    );

    let removed = uninstall_hooks(&installed.configuration).unwrap();
    assert!(removed.changed);
    assert_eq!(removed.configuration, original);
    assert_eq!(
        uninstall_hooks(&removed.configuration)
            .unwrap()
            .configuration,
        original
    );
}

#[test]
fn hook_transform_uses_exec_form_for_an_absolute_packaged_helper() {
    let original = json!({"unrelated": true});
    let helper = "/Applications/Cookbench.app/Contents/MacOS/cookbench-hook";
    let args = ["--harness", "claude-code"];

    let installed = install_hooks_with_command(&original, helper, &args).unwrap();
    let hook = &installed.configuration["hooks"]["Stop"][0]["hooks"][0];
    assert_eq!(hook["command"], helper);
    assert_eq!(hook["args"], json!(args));
    assert_eq!(
        install_hooks_with_command(&installed.configuration, helper, &args)
            .unwrap()
            .configuration,
        installed.configuration
    );

    let removed = uninstall_hooks_with_command(&installed.configuration, helper, &args).unwrap();
    assert_eq!(removed.configuration, original);
}

#[test]
fn repair_removes_prior_absolute_cookbench_helpers_without_touching_other_hooks() {
    let original = json!({
        "hooks": {
            "Stop": [
                {"matcher":"*", "hooks":[{"type":"command", "command":"/old/AppImage/path/cookbench-hook", "args":["--harness", "claude-code"], "timeout":10}], "description":"old Cookbench helper"},
                {"matcher":"*", "hooks":[{"type":"command", "command":"other-helper"}]}
            ]
        }
    });

    let removed = uninstall_all_cookbench_hooks(&original).unwrap();
    assert!(removed.changed);
    assert_eq!(
        removed.configuration["hooks"]["Stop"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        removed.configuration["hooks"]["Stop"][0]["hooks"][0]["command"],
        "other-helper"
    );
}

#[test]
fn unsafe_hook_shapes_are_refused_instead_of_overwritten() {
    assert!(install_hooks(&json!({"hooks": {"Stop": {}}})).is_err());
    assert!(install_hooks(&json!({"hooks": {}})).is_err());
}
