#![cfg(unix)]

use std::{
    fs,
    io::{BufReader, Write},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use cookbench_bridge::protocol::{
    read_lf_frame, ConfiguredHarness, ConfiguredRoot, Frame, NormalizedState, ProtocolVersion,
};

#[test]
fn stdio_bridge_negotiates_emits_a_bounded_native_event_batch_and_shuts_down() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "cookbench-bridge-stdio-{}-{nonce}",
        std::process::id()
    ));
    let sessions = root.join("sessions");
    fs::create_dir_all(&sessions).unwrap();
    fs::write(
        sessions.join("safe.jsonl"),
        "{\"type\":\"session_meta\",\"payload\":{\"id\":\"safe\"}}\n",
    )
    .unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_cookbench-bridge"))
        .arg("--stdio")
        .env("CODEX_HOME", &root)
        .env("CLAUDE_CONFIG_DIR", root.join("missing-claude"))
        .env("PI_SESSION_DIR", root.join("missing-pi"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    input
        .write_all(
            &Frame::Hello {
                version: ProtocolVersion::CURRENT,
            }
            .to_jsonl()
            .unwrap(),
        )
        .unwrap();
    input.flush().unwrap();

    let mut saw_hello = false;
    let mut saw_capabilities = false;
    let mut states = Vec::new();
    loop {
        let frame = Frame::from_jsonl(&read_lf_frame(&mut output).unwrap()).unwrap();
        match frame {
            Frame::Hello { .. } => saw_hello = true,
            Frame::Capabilities { .. } => saw_capabilities = true,
            Frame::Event { event } => states.push(event.state),
            Frame::Configure { .. } => panic!("bridge echoed its configuration"),
            Frame::Heartbeat => break,
            Frame::Shutdown => panic!("bridge shut down before completing the batch"),
        }
    }
    assert!(saw_hello && saw_capabilities);
    assert!(states.is_empty());

    input
        .write_all(
            &Frame::Configure {
                roots: vec![ConfiguredRoot::new(
                    ConfiguredHarness::Codex,
                    sessions.to_string_lossy(),
                )
                .unwrap()],
            }
            .to_jsonl()
            .unwrap(),
        )
        .unwrap();
    input.flush().unwrap();
    loop {
        let frame = Frame::from_jsonl(&read_lf_frame(&mut output).unwrap()).unwrap();
        match frame {
            Frame::Event { event } => states.push(event.state),
            Frame::Heartbeat => break,
            other => panic!("unexpected configured bridge frame: {other:?}"),
        }
    }
    assert_eq!(states, [NormalizedState::Starting]);

    input
        .write_all(&Frame::Shutdown.to_jsonl().unwrap())
        .unwrap();
    input.flush().unwrap();
    assert!(matches!(
        Frame::from_jsonl(&read_lf_frame(&mut output).unwrap()).unwrap(),
        Frame::Shutdown
    ));
    assert!(child.wait().unwrap().success());
    fs::remove_dir_all(root).unwrap();
}
