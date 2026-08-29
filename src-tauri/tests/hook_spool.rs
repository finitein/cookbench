use cookbench_core::domain::{EventKind, HostIdentity};
use cookbench_desktop_lib::hook_spool::HookSpool;
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
fn root() -> PathBuf {
    std::env::temp_dir().join(format!(
        "cookbench-hook-consumer-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}
#[test]
fn consumes_allowlisted_hook_envelopes_and_maps_needs_human_without_payload_retention() {
    let root = root();
    let spool = HookSpool::create(&root, HostIdentity::local("host")).unwrap();
    fs::write(root.join("event-1.json"), br#"{"schema_version":1,"source":"hook","received_at_ms":5,"event":{"event_type":"question_asked","session_id":"s1","harness":"codex","sequence":7}}"#).unwrap();
    fs::write(root.join("other.json"), b"{}").unwrap();
    let events = spool.poll();
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0].event.kind, EventKind::QuestionAsked));
    assert_eq!(
        events[0].event.metadata.source,
        cookbench_core::domain::EventSource::Hook
    );
    assert!(!root.join("event-1.json").exists());
    assert!(root.join("other.json").exists());
    fs::remove_dir_all(root).unwrap();
}
