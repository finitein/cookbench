use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use cookbench_bridge::protocol::{ConfiguredHarness, ConfiguredRoot};
use cookbench_bridge::{
    protocol::NormalizedState,
    source::{NativeSessionSource, SourceHarness, SourceRoot},
};

fn temp_root() -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "cookbench-bridge-source-{}-{nonce}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

fn write(path: &PathBuf, body: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

#[test]
fn observes_native_lifecycle_without_exposing_record_content() {
    let root = temp_root();
    let codex_root = root.join("codex");
    let claude_root = root.join("claude");
    let pi_root = root.join("pi");
    write(
        &codex_root.join("session.jsonl"),
        "{\"type\":\"session_meta\",\"payload\":{\"id\":\"safe-id\",\"cwd\":\"/fixture/project\"}}\n{\"type\":\"user_message\",\"content\":\"private fixture marker\"}\n{\"type\":\"turn_completed\"}\n",
    );
    write(
        &claude_root.join("project/session.jsonl"),
        "{\"type\":\"user\",\"content\":\"private fixture marker\"}\n{\"type\":\"event\",\"subtype\":\"permission_request\"}\n",
    );
    write(
        &pi_root.join("session.jsonl"),
        "{\"type\":\"session_start\",\"sessionId\":\"pi-safe\"}\n{\"type\":\"turn_completed\",\"status\":\"success\"}\n",
    );
    let mut source = NativeSessionSource::with_roots(
        vec![
            SourceRoot {
                harness: SourceHarness::Codex,
                path: codex_root,
            },
            SourceRoot {
                harness: SourceHarness::ClaudeCode,
                path: claude_root,
            },
            SourceRoot {
                harness: SourceHarness::Pi,
                path: pi_root,
            },
        ],
        UNIX_EPOCH,
    );

    let events = source.poll();
    let states = events.iter().map(|event| &event.state).collect::<Vec<_>>();
    assert!(states.contains(&&NormalizedState::Cooking));
    assert!(states.contains(&&NormalizedState::NeedsHuman));
    assert!(states.contains(&&NormalizedState::Cooked));
    assert!(states.contains(&&NormalizedState::Starting));
    assert!(events.iter().any(|event| event.harness == "codex"));
    assert!(events.iter().any(|event| event.harness == "claude_code"));
    assert!(events.iter().any(|event| event.harness == "pi"));
    assert!(events.iter().any(|event| {
        event.harness == "codex" && event.project_root.as_deref() == Some("/fixture/project")
    }));
    assert!(!format!("{events:?}").contains("private fixture marker"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn emits_only_new_complete_appends_after_the_initial_bounded_replay() {
    let root = temp_root();
    let codex_root = root.join("codex");
    let session = codex_root.join("session.jsonl");
    write(
        &session,
        "{\"type\":\"session_meta\",\"payload\":{\"id\":\"safe-id\"}}\n",
    );
    let mut source = NativeSessionSource::with_roots(
        vec![SourceRoot {
            harness: SourceHarness::Codex,
            path: codex_root,
        }],
        UNIX_EPOCH,
    );
    assert_eq!(source.poll().len(), 1);
    assert!(source.poll().is_empty());

    use std::io::Write;
    fs::OpenOptions::new()
        .append(true)
        .open(&session)
        .unwrap()
        .write_all(b"{\"type\":\"turn_completed\"}\n")
        .unwrap();
    let appended = source.poll();
    assert_eq!(appended.len(), 1);
    assert_eq!(appended[0].state, NormalizedState::Cooked);

    fs::remove_dir_all(root).unwrap();
}

#[test]
#[cfg(unix)]
fn configured_custom_root_auto_detects_harness_and_project_metadata() {
    let root = temp_root().join("arbitrary-storage");
    write(
        &root.join("nested/session.jsonl"),
        "{\"type\":\"session_meta\",\"payload\":{\"id\":\"custom-safe\",\"cwd\":\"/work/custom-project\"}}\n{\"type\":\"tool_started\"}\n",
    );
    let mut source = NativeSessionSource::from_configured_roots(vec![ConfiguredRoot::new(
        ConfiguredHarness::Auto,
        root.to_string_lossy(),
    )
    .unwrap()]);

    let events = source.poll();
    assert!(events.iter().any(|event| {
        event.harness == "codex"
            && event.project_root.as_deref() == Some("/work/custom-project")
            && event.state == NormalizedState::Cooking
    }));

    fs::remove_dir_all(root.parent().unwrap()).unwrap();
}
