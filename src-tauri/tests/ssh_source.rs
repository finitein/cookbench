use std::{cell::RefCell, collections::VecDeque};

use cookbench_core::remote::{PollInterval, RemoteHost, SessionRoot};
use cookbench_desktop_lib::remote::{
    ssh::{ProbeOutput, SshError, SshInvocation, SshRunner},
    zero_install::ZeroInstallSshSource,
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
    assert!(!invocation
        .args
        .iter()
        .any(|arg| arg.contains("UserKnownHostsFile") || arg.contains("StrictHostKeyChecking=no")));
    assert_eq!(invocation.args[10], "fixture-host");
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
        "exec find '/custom root/sessions' -type f -name '*.jsonl' -print0"
    );
    assert!(!remote_probe.contains("rm "));
    assert!(!remote_probe.contains(">"));
    assert!(!remote_probe.contains("curl"));
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
