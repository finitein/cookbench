use std::{
    cell::RefCell,
    collections::VecDeque,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use cookbench_core::{
    domain::{EventKind, HarnessId},
    remote::{PollInterval, RemoteHost, SessionRoot},
};
use cookbench_desktop_lib::remote::{
    ssh::{ProbeOutput, SshError, SshInvocation, SshRunner},
    zero_install::{ParsedRemoteSession, RemoteLifecycleParser, ZeroInstallSshSource},
};

struct FakeSsh {
    results: RefCell<VecDeque<Result<ProbeOutput, SshError>>>,
}

impl FakeSsh {
    fn new(results: impl IntoIterator<Item = Result<ProbeOutput, SshError>>) -> Self {
        Self {
            results: RefCell::new(results.into_iter().collect()),
        }
    }
}

impl SshRunner for FakeSsh {
    fn run(&self, _: &SshInvocation) -> Result<ProbeOutput, SshError> {
        self.results
            .borrow_mut()
            .pop_front()
            .expect("a planned SSH result")
    }
}

fn host() -> RemoteHost {
    RemoteHost::new(
        "fixture-host",
        vec![SessionRoot::new("/srv/cookbench/sessions").unwrap()],
    )
    .unwrap()
}

fn successful_probe(path: &str) -> Result<ProbeOutput, SshError> {
    let mut stdout = path.as_bytes().to_vec();
    stdout.push(0);
    Ok(ProbeOutput {
        status: 0,
        stdout,
        stderr: String::new(),
    })
}

fn nul_probe(paths: &[&str]) -> Result<ProbeOutput, SshError> {
    let mut stdout = Vec::new();
    for path in paths {
        stdout.extend_from_slice(path.as_bytes());
        stdout.push(0);
    }
    Ok(ProbeOutput {
        status: 0,
        stdout,
        stderr: String::new(),
    })
}

fn suffix_probe(bytes: &[u8]) -> Result<ProbeOutput, SshError> {
    Ok(ProbeOutput {
        status: 0,
        stdout: bytes.to_vec(),
        stderr: String::new(),
    })
}

struct FixtureParser;

impl RemoteLifecycleParser for FixtureParser {
    fn parse_suffix(&self, _: &str, suffix: &[u8]) -> Option<ParsedRemoteSession> {
        (suffix == b"fixture:tool-started\n").then_some(ParsedRemoteSession {
            harness: HarnessId::Codex,
            native_session_id: "opaque-session-1".to_owned(),
            project_root: Some("/synthetic/project".to_owned()),
            events: vec![EventKind::SessionDiscovered, EventKind::ToolStarted],
        })
    }
}

#[test]
fn discovery_uses_existing_ssh_config_with_strict_host_key_and_no_password_options() {
    let host = host();
    let invocation = SshInvocation::discover(&host, &host.session_roots()[0]);

    assert_eq!(invocation.program, "ssh");
    assert!(invocation
        .args
        .windows(2)
        .any(|pair| pair[0] == "-o" && pair[1] == "StrictHostKeyChecking=yes"));
    assert!(invocation
        .args
        .windows(2)
        .any(|pair| pair[0] == "-o" && pair[1] == "BatchMode=yes"));
    assert!(invocation
        .args
        .windows(2)
        .any(|pair| pair[0] == "-o" && pair[1] == "PasswordAuthentication=no"));
    for option in [
        "ConnectTimeout=10",
        "ServerAliveInterval=5",
        "ServerAliveCountMax=2",
    ] {
        assert!(invocation
            .args
            .windows(2)
            .any(|pair| pair == ["-o", option]));
    }
    assert!(!invocation
        .args
        .iter()
        .any(|arg| arg.contains("UserKnownHostsFile") || arg.contains("StrictHostKeyChecking=no")));
    assert_eq!(invocation.args[invocation.args.len() - 2], "fixture-host");
}

#[test]
fn custom_roots_are_only_read_with_a_fixed_find_probe() {
    let host = RemoteHost::new(
        "fixture-host",
        vec![SessionRoot::new("/custom root/sessions").unwrap()],
    )
    .unwrap();
    let invocation = SshInvocation::discover(&host, &host.session_roots()[0]);
    let remote_probe = invocation.args.last().unwrap();

    assert_eq!(
        remote_probe,
        "if [ -d '/custom root/sessions' ]; then exec find '/custom root/sessions' -type f -name '*.jsonl' -mtime -7 -print0; fi"
    );
    assert!(!remote_probe.contains("rm "));
    assert!(!remote_probe.contains(">"));
    assert!(!remote_probe.contains("curl"));
}

#[test]
fn automatic_roots_use_fixed_remote_home_expressions() {
    let host = RemoteHost::new("fixture-host", vec![]).unwrap();
    let invocation = SshInvocation::resolve_automatic_roots(&host);
    let remote_probe = invocation.args.last().unwrap();

    assert!(remote_probe.contains("CODEX_HOME"));
    assert!(remote_probe.contains("CLAUDE_CONFIG_DIR"));
    assert!(remote_probe.contains("PI_SESSION_DIR"));
    assert!(remote_probe.contains("$HOME/.codex"));
    assert!(!remote_probe.contains("rm "));
    assert!(!remote_probe.contains("curl"));
}

#[test]
fn empty_roots_resolve_and_scan_every_supported_harness() {
    let host = RemoteHost::new("fixture-host", vec![]).unwrap();
    let mut source = ZeroInstallSshSource::new(
        host,
        FakeSsh::new([
            nul_probe(&[
                "/home/test/.codex/sessions",
                "/home/test/.claude/projects",
                "/home/test/.pi/agent/sessions",
            ]),
            successful_probe("/home/test/.codex/sessions/codex.jsonl"),
            successful_probe("/home/test/.claude/projects/claude.jsonl"),
            successful_probe("/home/test/.pi/agent/sessions/pi.jsonl"),
        ]),
    );

    let (paths, _) = source.discover().unwrap();
    assert_eq!(
        paths,
        [
            "/home/test/.codex/sessions/codex.jsonl",
            "/home/test/.claude/projects/claude.jsonl",
            "/home/test/.pi/agent/sessions/pi.jsonl",
        ]
    );
}

#[test]
fn zero_install_discovery_returns_remote_paths_without_remote_writes() {
    let mut source = ZeroInstallSshSource::new(
        host(),
        FakeSsh::new([successful_probe("/srv/cookbench/sessions/run.jsonl")]),
    );

    let (paths, restored) = source.discover().unwrap();
    assert_eq!(paths, ["/srv/cookbench/sessions/run.jsonl"]);
    assert!(!restored);
    assert_eq!(source.poll_interval(), PollInterval::Active);
}

#[test]
fn disconnection_uses_slow_polling_and_a_successful_probe_reports_reconnection() {
    let mut source = ZeroInstallSshSource::new(
        host(),
        FakeSsh::new([
            Err(SshError::Disconnected {
                detail: "host key rejected or offline".to_owned(),
            }),
            successful_probe("/srv/cookbench/sessions/run.jsonl"),
        ]),
    );

    assert!(source.discover().is_err());
    assert_eq!(source.poll_interval(), PollInterval::Disconnected);
    let (_, restored) = source.discover().unwrap();
    assert!(restored);
    assert_eq!(source.poll_interval(), PollInterval::Active);
}

#[test]
fn hosts_and_native_session_ids_are_collision_safe() {
    let root = SessionRoot::new("/srv/cookbench/sessions").unwrap();
    let encoded = RemoteHost::new("node:one", vec![root.clone()]).unwrap();
    let literal = RemoteHost::new("node%3Aone", vec![root]).unwrap();

    assert_ne!(encoded.identity(), literal.identity());
}

#[test]
fn discovery_rejects_a_path_that_only_shares_a_root_prefix() {
    let mut source = ZeroInstallSshSource::new(
        host(),
        FakeSsh::new([successful_probe(
            "/srv/cookbench/sessions-elsewhere/run.jsonl",
        )]),
    );

    assert!(matches!(source.discover(), Err(SshError::UnsafeOutput)));
    assert_eq!(source.poll_interval(), PollInterval::Disconnected);
}

#[test]
fn bounded_suffix_observation_normalizes_lifecycle_events_with_remote_identity() {
    let mut source = ZeroInstallSshSource::new(
        host(),
        FakeSsh::new([
            successful_probe("/srv/cookbench/sessions/run.jsonl"),
            suffix_probe(b"fixture:tool-started\n"),
        ]),
    );

    let poll = source.observe(&FixtureParser);
    assert!(!poll.disconnected);
    assert_eq!(poll.events.len(), 2);
    assert!(matches!(
        poll.events[0].event.kind,
        EventKind::SessionDiscovered
    ));
    assert!(matches!(poll.events[1].event.kind, EventKind::ToolStarted));
    assert_eq!(poll.events[0].stove.host, host().identity());
    assert_eq!(poll.events[0].stove.native_session_id, "opaque-session-1");
}

#[test]
fn disconnect_emits_no_completion_and_recovery_restores_before_new_lifecycle_events() {
    let mut source = ZeroInstallSshSource::new(
        host(),
        FakeSsh::new([
            successful_probe("/srv/cookbench/sessions/run.jsonl"),
            suffix_probe(b"fixture:tool-started\n"),
            Err(SshError::Disconnected {
                detail: "offline".to_owned(),
            }),
            successful_probe("/srv/cookbench/sessions/run.jsonl"),
            suffix_probe(b"fixture:tool-started\n"),
        ]),
    );

    let initial = source.observe(&FixtureParser);
    assert_eq!(initial.events.len(), 2);
    let lost = source.observe(&FixtureParser);
    assert!(lost.disconnected);
    assert!(lost
        .events
        .iter()
        .all(|event| !matches!(event.event.kind, EventKind::TurnCompleted)));
    assert!(lost
        .events
        .iter()
        .all(|event| matches!(event.event.kind, EventKind::ConnectionLost)));

    let restored = source.observe(&FixtureParser);
    assert!(restored.restored);
    assert!(matches!(
        restored.events[0].event.kind,
        EventKind::ConnectionRestored
    ));
    assert_eq!(
        restored.events.len(),
        1,
        "unchanged native history is not replayed"
    );
}

#[test]
fn unchanged_suffixes_do_not_replay_lifecycle_events() {
    let mut source = ZeroInstallSshSource::new(
        host(),
        FakeSsh::new([
            successful_probe("/srv/cookbench/sessions/run.jsonl"),
            suffix_probe(b"fixture:tool-started\n"),
            successful_probe("/srv/cookbench/sessions/run.jsonl"),
            suffix_probe(b"fixture:tool-started\n"),
        ]),
    );

    assert_eq!(source.observe(&FixtureParser).events.len(), 2);
    assert!(source.observe(&FixtureParser).events.is_empty());
}

struct LineParser;

impl RemoteLifecycleParser for LineParser {
    fn parse_suffix(&self, _: &str, suffix: &[u8]) -> Option<ParsedRemoteSession> {
        let lines = suffix
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .count();
        (lines > 0).then_some(ParsedRemoteSession {
            harness: HarnessId::Codex,
            native_session_id: "opaque-session-1".to_owned(),
            project_root: None,
            events: vec![EventKind::ToolStarted; lines],
        })
    }
}

#[test]
fn appended_suffix_emits_only_new_records_instead_of_replaying_the_window() {
    let mut source = ZeroInstallSshSource::new(
        host(),
        FakeSsh::new([
            successful_probe("/srv/cookbench/sessions/run.jsonl"),
            suffix_probe(b"one\n"),
            successful_probe("/srv/cookbench/sessions/run.jsonl"),
            suffix_probe(b"one\ntwo\n"),
        ]),
    );

    assert_eq!(source.observe(&LineParser).events.len(), 1);
    assert_eq!(source.observe(&LineParser).events.len(), 1);
}

#[test]
fn shared_sequence_counter_stays_monotonic_when_a_source_is_reconfigured() {
    let counter = Arc::new(AtomicU64::new(100));
    let mut first = ZeroInstallSshSource::new(
        host(),
        FakeSsh::new([
            successful_probe("/srv/cookbench/sessions/one.jsonl"),
            suffix_probe(b"fixture:tool-started\n"),
        ]),
    )
    .with_sequence_counter(counter.clone());
    let first_poll = first.observe(&FixtureParser);

    let mut restarted = ZeroInstallSshSource::new(
        host(),
        FakeSsh::new([
            successful_probe("/srv/cookbench/sessions/two.jsonl"),
            suffix_probe(b"fixture:tool-started\n"),
        ]),
    )
    .with_sequence_counter(counter.clone());
    let restarted_poll = restarted.observe(&FixtureParser);

    assert!(
        restarted_poll.events[0].event.metadata.sequence
            > first_poll.events.last().unwrap().event.metadata.sequence
    );
    assert_eq!(counter.load(Ordering::Acquire), 104);
}
