use cookbench_bridge::protocol::NormalizedEvent;
use cookbench_desktop_lib::remote::bridge::{
    connect_temporary_bridge, BridgeDeploymentSelection, BridgeError, BridgeRemote, BridgeSession,
    Sha256Digest,
};

#[derive(Default)]
struct FakeSession {
    terminated: bool,
}

impl BridgeSession for FakeSession {
    type Error = String;

    fn negotiate(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn poll(&mut self) -> Result<Vec<NormalizedEvent>, String> {
        Ok(Vec::new())
    }

    fn terminate(&mut self) -> Result<(), String> {
        self.terminated = true;
        Ok(())
    }
}

#[derive(Default)]
struct FakeRemote {
    uploaded: bool,
    verified: bool,
    started: bool,
    removed: bool,
    session: FakeSession,
    remote_hash: Sha256Digest,
}

impl BridgeRemote for FakeRemote {
    type Error = String;
    type Session = FakeSession;

    fn upload_temporary(&mut self, _: &BridgeDeploymentSelection) -> Result<(), Self::Error> {
        self.uploaded = true;
        Ok(())
    }

    fn remote_sha256(
        &mut self,
        _: &BridgeDeploymentSelection,
    ) -> Result<Sha256Digest, Self::Error> {
        self.verified = true;
        Ok(self.remote_hash)
    }

    fn remove_temporary(&mut self, _: &BridgeDeploymentSelection) -> Result<(), Self::Error> {
        self.removed = true;
        Ok(())
    }

    fn start_stdio(&mut self, _: &BridgeDeploymentSelection) -> Result<Self::Session, Self::Error> {
        self.started = true;
        Ok(std::mem::take(&mut self.session))
    }
}

fn digest(byte: u8) -> Sha256Digest {
    Sha256Digest([byte; 32])
}

#[test]
fn explicitly_selected_bridge_is_uploaded_verified_and_bound_to_stdio_lifetime() {
    let selection =
        BridgeDeploymentSelection::explicit("example-host", "/tmp/cookbench-bridge", digest(7))
            .unwrap();
    let mut remote = FakeRemote {
        remote_hash: digest(7),
        ..Default::default()
    };

    let connection = connect_temporary_bridge(&mut remote, selection).unwrap();

    assert!(remote.uploaded);
    assert!(remote.verified);
    assert!(remote.started);
    drop(connection);
}

#[test]
fn checksum_mismatch_never_starts_a_bridge() {
    let selection =
        BridgeDeploymentSelection::explicit("example-host", "/tmp/cookbench-bridge", digest(7))
            .unwrap();
    let mut remote = FakeRemote {
        remote_hash: digest(9),
        ..Default::default()
    };

    assert!(matches!(
        connect_temporary_bridge(&mut remote, selection),
        Err(BridgeError::ChecksumMismatch)
    ));
    assert!(remote.uploaded);
    assert!(remote.verified);
    assert!(!remote.started);
    assert!(remote.removed);
}

#[test]
fn launch_shape_is_fixed_ssh_stdio_without_port_forwarding() {
    let selection =
        BridgeDeploymentSelection::explicit("configured-host", "/tmp/cookbench-bridge", digest(7))
            .unwrap();
    let command = selection.ssh_stdio_command();

    assert_eq!(command.program, "ssh");
    assert_eq!(
        command.arguments,
        vec![
            "-o",
            "ConnectTimeout=10",
            "-o",
            "ServerAliveInterval=5",
            "-o",
            "ServerAliveCountMax=2",
            "--",
            "configured-host",
            "/tmp/cookbench-bridge",
            "--stdio"
        ]
    );
    assert!(!command
        .arguments
        .iter()
        .any(|argument| matches!(argument.as_str(), "-L" | "-R" | "-D")));
}

#[test]
fn unsafe_hosts_and_remote_paths_never_reach_system_ssh() {
    assert!(BridgeDeploymentSelection::explicit(
        "-oProxyCommand=bad",
        "/tmp/cookbench-bridge",
        digest(7)
    )
    .is_err());
    assert!(BridgeDeploymentSelection::explicit(
        "configured-host",
        "/tmp/bridge;touch-pwned",
        digest(7)
    )
    .is_err());
}
