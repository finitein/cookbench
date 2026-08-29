use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

const SPOOL_ENV: &str = "COOKBENCH_HOOK_SPOOL_DIR";

fn unique_temp_dir(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("cookbench-hook-{name}-{nonce}"));
    fs::create_dir_all(&path).expect("temp directory should be created");
    path
}

fn run_hook(spool: Option<&Path>, input: &[u8]) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_cookbench-hook"));
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(spool) = spool {
        command.env(SPOOL_ENV, spool);
    } else {
        command.env_remove(SPOOL_ENV);
    }

    let mut child = command.spawn().expect("hook binary should start");
    child
        .stdin
        .take()
        .expect("stdin should be available")
        .write_all(input)
        .expect("test input should be written");
    child.wait_with_output().expect("hook binary should exit")
}

fn valid_event() -> &'static [u8] {
    br#"{"event_type":"tool_started","session_id":"session-42","harness":"codex","sequence":7}"#
}

#[test]
fn writes_a_sanitized_atomic_envelope() {
    let spool = unique_temp_dir("writes");
    let output = run_hook(Some(&spool), valid_event());

    assert!(
        output.status.success(),
        "stderr: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());

    let entries = fs::read_dir(&spool)
        .expect("spool should be readable")
        .map(|entry| entry.expect("entry should be readable").path())
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].extension().and_then(|value| value.to_str()),
        Some("json")
    );
    assert!(!entries[0]
        .file_name()
        .unwrap()
        .to_string_lossy()
        .contains("tmp"));

    let envelope: serde_json::Value =
        serde_json::from_slice(&fs::read(&entries[0]).expect("envelope should be readable"))
            .expect("envelope should be valid JSON");
    assert_eq!(envelope["schema_version"], 1);
    assert_eq!(envelope["source"], "hook");
    assert_eq!(envelope["event"]["event_type"], "tool_started");
    assert_eq!(envelope["event"]["session_id"], "session-42");
    assert!(envelope.get("prompt").is_none());

    fs::remove_dir_all(spool).expect("temp directory should be removed");
}

#[test]
fn rejects_sensitive_payload_fields_without_echoing_them() {
    let spool = unique_temp_dir("sensitive");
    let secret = "not-for-output";
    let input = format!(
        r#"{{"event_type":"tool_started","session_id":"session-42","harness":"codex","prompt":"{secret}"}}"#
    );
    let output = run_hook(Some(&spool), input.as_bytes());

    assert_eq!(output.status.code(), Some(64));
    let diagnostics = String::from_utf8_lossy(&output.stderr);
    assert!(!diagnostics.contains(secret));
    assert!(fs::read_dir(&spool)
        .expect("spool should be readable")
        .next()
        .is_none());

    fs::remove_dir_all(spool).expect("temp directory should be removed");
}

#[test]
fn rejects_malformed_and_unsupported_events() {
    let spool = unique_temp_dir("invalid");
    for input in [
        br#"{"event_type":"tool_started""#.as_slice(),
        br#"{"event_type":"agent_control","session_id":"session-42","harness":"codex"}"#.as_slice(),
    ] {
        let output = run_hook(Some(&spool), input);
        assert_eq!(output.status.code(), Some(64));
    }
    assert!(fs::read_dir(&spool)
        .expect("spool should be readable")
        .next()
        .is_none());

    fs::remove_dir_all(spool).expect("temp directory should be removed");
}

#[test]
fn rejects_oversized_input_before_writing() {
    let spool = unique_temp_dir("oversized");
    let input = vec![b'x'; 16 * 1024 + 1];
    let output = run_hook(Some(&spool), &input);
    assert_eq!(output.status.code(), Some(64));
    assert!(fs::read_dir(&spool)
        .expect("spool should be readable")
        .next()
        .is_none());

    fs::remove_dir_all(spool).expect("temp directory should be removed");
}

#[test]
fn reports_missing_spool_and_full_spool() {
    let missing = std::env::temp_dir().join("cookbench-hook-missing-spool");
    let _ = fs::remove_dir_all(&missing);
    let missing_output = run_hook(Some(&missing), valid_event());
    assert_eq!(missing_output.status.code(), Some(69));

    let spool = unique_temp_dir("full");
    for index in 0..128 {
        fs::write(spool.join(format!("{index}.json")), b"{}")
            .expect("synthetic spool entry should be written");
    }
    let full_output = run_hook(Some(&spool), valid_event());
    assert_eq!(full_output.status.code(), Some(75));
    assert_eq!(
        fs::read_dir(&spool)
            .expect("spool should be readable")
            .count(),
        128
    );

    fs::remove_dir_all(spool).expect("temp directory should be removed");
}

#[test]
fn self_test_reports_its_execution_time() {
    let output = Command::new(env!("CARGO_BIN_EXE_cookbench-hook"))
        .arg("--self-test")
        .output()
        .expect("self-test should start");
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("self-test: passed in "));
}
