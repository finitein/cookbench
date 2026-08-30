use std::{
    fs::{self, File, FileTimes},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use cookbench_core::domain::{HarnessId, HostIdentity, StoveState};
use cookbench_desktop_lib::runtime::{
    archive_inventory::discover_expired_local_sessions, LocalObservationConfig,
};

const AGE: Duration = Duration::from_secs(3 * 24 * 60 * 60);

fn temp_root() -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "cookbench-archive-inventory-{}-{nonce}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

fn write_old(path: &Path, content: &str, now: SystemTime) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
    File::open(path)
        .unwrap()
        .set_times(FileTimes::new().set_modified(now.checked_sub(AGE).unwrap()))
        .unwrap();
}

fn config(root: &Path) -> LocalObservationConfig {
    LocalObservationConfig {
        host: HostIdentity::local("archive-test-host"),
        codex_root: root.join("codex"),
        claude_root: root.join("claude"),
        pi_roots: vec![root.join("pi")],
        startup_min_modified: SystemTime::UNIX_EPOCH,
        startup_candidate_limit: 64,
        pinned_local_paths: Vec::new(),
    }
}

#[test]
fn inventories_old_regular_sessions_across_harnesses_without_storing_content() {
    let root = temp_root();
    let now = SystemTime::now();
    write_old(
        &root.join("codex/root.jsonl"),
        "{\"type\":\"session_meta\",\"payload\":{\"id\":\"codex-old\",\"cwd\":\"/workspace/codex\",\"thread_source\":\"user\"}}\n",
        now,
    );
    write_old(
        &root.join("claude/-workspace-claude/claude-old.jsonl"),
        "{\"type\":\"user\",\"session_name\":\"do not persist this title\"}\n",
        now,
    );
    write_old(
        &root.join("pi/tree/pi-old.jsonl"),
        "{\"type\":\"session_start\",\"sessionId\":\"pi-old\",\"cwd\":\"/workspace/pi\"}\n",
        now,
    );

    let records = discover_expired_local_sessions(&config(&root), now, 64);

    assert_eq!(records.len(), 3);
    assert!(records
        .iter()
        .any(|record| record.locator.harness == HarnessId::Codex));
    assert!(records
        .iter()
        .any(|record| record.locator.harness == HarnessId::ClaudeCode));
    assert!(records
        .iter()
        .any(|record| record.locator.harness == HarnessId::Pi));
    assert!(records
        .iter()
        .all(|record| record.last_state == StoveState::Disconnected));
    let serialized = serde_json::to_string(&records).unwrap();
    assert!(!serialized.contains("do not persist this title"));
    assert!(!serialized.contains("task"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn excludes_recent_files_and_codex_subagents() {
    let root = temp_root();
    let now = SystemTime::now();
    write_old(
        &root.join("codex/subagent.jsonl"),
        "{\"type\":\"session_meta\",\"payload\":{\"id\":\"codex-subagent\",\"cwd\":\"/workspace/codex\",\"thread_source\":\"subagent\"}}\n",
        now,
    );
    let recent = root.join("codex/recent.jsonl");
    fs::create_dir_all(recent.parent().unwrap()).unwrap();
    fs::write(
        &recent,
        "{\"type\":\"session_meta\",\"payload\":{\"id\":\"codex-recent\",\"cwd\":\"/workspace/codex\",\"thread_source\":\"user\"}}\n",
    )
    .unwrap();

    assert!(discover_expired_local_sessions(&config(&root), now, 64).is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn honours_output_limit() {
    let root = temp_root();
    let now = SystemTime::now();
    for index in 0..5 {
        write_old(
            &root.join(format!("codex/{index}.jsonl")),
            &format!(
                "{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"codex-{index}\",\"cwd\":\"/workspace/codex\",\"thread_source\":\"user\"}}}}\n"
            ),
            now,
        );
    }

    assert_eq!(
        discover_expired_local_sessions(&config(&root), now, 2).len(),
        2
    );
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn rejects_symlinked_candidates_and_paths_outside_configured_roots() {
    use std::os::unix::fs::symlink;

    let root = temp_root();
    let outside = temp_root();
    let now = SystemTime::now();
    write_old(
        &outside.join("outside.jsonl"),
        "{\"type\":\"session_meta\",\"payload\":{\"id\":\"outside\",\"cwd\":\"/workspace/outside\",\"thread_source\":\"user\"}}\n",
        now,
    );
    let linked = root.join("codex/link.jsonl");
    fs::create_dir_all(linked.parent().unwrap()).unwrap();
    symlink(outside.join("outside.jsonl"), &linked).unwrap();

    assert!(discover_expired_local_sessions(&config(&root), now, 64).is_empty());
    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(outside).unwrap();
}
