//! Optional, explicitly selected temporary SSH bridge lifecycle.
//!
//! This layer does not discover sessions itself and cannot send a prompt,
//! approve a tool, control an agent, or request a remote port. Its transport
//! boundary permits one temporary upload, a remote digest read, and an SSH
//! standard-I/O process whose lifetime is tied to `BridgeConnection`.

use std::{
    fmt,
    path::PathBuf,
    process::{Child, Command, Stdio},
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Sha256Digest(pub [u8; 32]);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BridgeDeploymentSelection {
    host: String,
    remote_binary_path: String,
    expected_sha256: Sha256Digest,
}

impl BridgeDeploymentSelection {
    /// Call only after an explicit user choice of host and temporary bridge.
    /// There is intentionally no implicit or background constructor.
    pub fn explicit(
        host: impl Into<String>,
        remote_binary_path: impl Into<String>,
        expected_sha256: Sha256Digest,
    ) -> Result<Self, BridgeError> {
        let selection = Self {
            host: host.into(),
            remote_binary_path: remote_binary_path.into(),
            expected_sha256,
        };
        if !safe_host(&selection.host) || !safe_remote_path(&selection.remote_binary_path) {
            return Err(BridgeError::UnsafeSelection);
        }
        Ok(selection)
    }

    pub fn ssh_stdio_command(&self) -> SshStdioCommand {
        SshStdioCommand {
            program: "ssh".to_owned(),
            arguments: vec![
                "--".to_owned(),
                self.host.clone(),
                self.remote_binary_path.clone(),
                "--stdio".to_owned(),
            ],
        }
    }
}

fn safe_host(host: &str) -> bool {
    !host.is_empty()
        && !host.starts_with('-')
        && host.len() <= 255
        && host.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'@' | b':' | b'%')
        })
}

fn safe_remote_path(path: &str) -> bool {
    path.starts_with('/')
        && path.len() <= 1024
        && !path.split('/').any(|segment| segment == "..")
        && path
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'.' | b'_' | b'-'))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SshStdioCommand {
    pub program: String,
    pub arguments: Vec<String>,
}

pub trait BridgeSession {
    type Error: fmt::Display;

    /// Terminates the SSH stdio child and therefore the temporary bridge.
    fn terminate(&mut self) -> Result<(), Self::Error>;
}

/// Restricted transport boundary. It deliberately has no generic command
/// runner, write method after upload, port forwarding, or agent control.
pub trait BridgeRemote {
    type Error: fmt::Display;
    type Session: BridgeSession<Error = Self::Error>;

    /// The sole authorized remote write, after explicit selection.
    fn upload_temporary(
        &mut self,
        selection: &BridgeDeploymentSelection,
    ) -> Result<(), Self::Error>;
    fn remote_sha256(
        &mut self,
        selection: &BridgeDeploymentSelection,
    ) -> Result<Sha256Digest, Self::Error>;
    /// Starts only the fixed `ssh -- host bridge --stdio` shape.
    fn start_stdio(
        &mut self,
        selection: &BridgeDeploymentSelection,
    ) -> Result<Self::Session, Self::Error>;
}

/// Concrete system OpenSSH deployment. It exposes only the three operations in
/// `BridgeRemote`; there is no general-purpose remote command API.
pub struct SystemBridgeRemote {
    local_binary_path: PathBuf,
}

impl SystemBridgeRemote {
    pub fn new(local_binary_path: impl Into<PathBuf>) -> Result<Self, BridgeError> {
        let local_binary_path = local_binary_path.into();
        if !local_binary_path.is_file() {
            return Err(BridgeError::MissingLocalBinary);
        }
        Ok(Self { local_binary_path })
    }
}

pub struct SystemBridgeSession {
    child: Child,
    host: String,
    remote_binary_path: String,
}

#[derive(Debug)]
pub enum SystemBridgeError {
    Io(std::io::Error),
    CommandFailed(&'static str),
    InvalidDigest,
}

impl fmt::Display for SystemBridgeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "system OpenSSH failed: {error}"),
            Self::CommandFailed(action) => write!(formatter, "system OpenSSH {action} failed"),
            Self::InvalidDigest => formatter.write_str("remote bridge digest was invalid"),
        }
    }
}

impl std::error::Error for SystemBridgeError {}

impl BridgeSession for SystemBridgeSession {
    type Error = SystemBridgeError;

    fn terminate(&mut self) -> Result<(), Self::Error> {
        if self
            .child
            .try_wait()
            .map_err(SystemBridgeError::Io)?
            .is_none()
        {
            self.child.kill().map_err(SystemBridgeError::Io)?;
            self.child.wait().map_err(SystemBridgeError::Io)?;
        }
        let status = base_ssh(&self.host)
            .arg(format!("rm -f -- '{}'", self.remote_binary_path))
            .status()
            .map_err(SystemBridgeError::Io)?;
        if status.success() {
            Ok(())
        } else {
            Err(SystemBridgeError::CommandFailed("cleanup"))
        }
    }
}

impl BridgeRemote for SystemBridgeRemote {
    type Error = SystemBridgeError;
    type Session = SystemBridgeSession;

    fn upload_temporary(
        &mut self,
        selection: &BridgeDeploymentSelection,
    ) -> Result<(), Self::Error> {
        let destination = format!("{}:{}", selection.host, selection.remote_binary_path);
        let status = base_scp()
            .arg(&self.local_binary_path)
            .arg(destination)
            .status()
            .map_err(SystemBridgeError::Io)?;
        if status.success() {
            Ok(())
        } else {
            Err(SystemBridgeError::CommandFailed("upload"))
        }
    }

    fn remote_sha256(
        &mut self,
        selection: &BridgeDeploymentSelection,
    ) -> Result<Sha256Digest, Self::Error> {
        let command = format!(
            "sha256sum '{0}' 2>/dev/null || shasum -a 256 '{0}'",
            selection.remote_binary_path
        );
        let output = base_ssh(&selection.host)
            .arg(command)
            .output()
            .map_err(SystemBridgeError::Io)?;
        if !output.status.success() {
            return Err(SystemBridgeError::CommandFailed("checksum"));
        }
        parse_sha256(&output.stdout).ok_or(SystemBridgeError::InvalidDigest)
    }

    fn start_stdio(
        &mut self,
        selection: &BridgeDeploymentSelection,
    ) -> Result<Self::Session, Self::Error> {
        let child = base_ssh(&selection.host)
            .arg(format!("'{}' --stdio", selection.remote_binary_path))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(SystemBridgeError::Io)?;
        Ok(SystemBridgeSession {
            child,
            host: selection.host.clone(),
            remote_binary_path: selection.remote_binary_path.clone(),
        })
    }
}

fn base_ssh(host: &str) -> Command {
    let mut command = Command::new("ssh");
    command.args([
        "-T",
        "-o",
        "BatchMode=yes",
        "-o",
        "PasswordAuthentication=no",
        "-o",
        "KbdInteractiveAuthentication=no",
        "-o",
        "StrictHostKeyChecking=yes",
        "-o",
        "ClearAllForwardings=yes",
        "--",
        host,
    ]);
    command
}

fn base_scp() -> Command {
    let mut command = Command::new("scp");
    command.args([
        "-o",
        "BatchMode=yes",
        "-o",
        "PasswordAuthentication=no",
        "-o",
        "KbdInteractiveAuthentication=no",
        "-o",
        "StrictHostKeyChecking=yes",
        "--",
    ]);
    command
}

fn parse_sha256(output: &[u8]) -> Option<Sha256Digest> {
    let token = std::str::from_utf8(output)
        .ok()?
        .split_whitespace()
        .next()?;
    if token.len() != 64 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let mut digest = [0_u8; 32];
    let (pairs, remainder) = token.as_bytes().as_chunks::<2>();
    if !remainder.is_empty() {
        return None;
    }
    for (index, pair) in pairs.iter().enumerate() {
        digest[index] = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
    }
    Some(Sha256Digest(digest))
}

pub struct BridgeConnection<S: BridgeSession> {
    session: Option<S>,
}

impl<S: BridgeSession> BridgeConnection<S> {
    pub fn close(mut self) -> Result<(), S::Error> {
        if let Some(mut session) = self.session.take() {
            session.terminate()?;
        }
        Ok(())
    }
}

impl<S: BridgeSession> Drop for BridgeConnection<S> {
    fn drop(&mut self) {
        if let Some(session) = self.session.as_mut() {
            let _ = session.terminate();
        }
    }
}

pub fn connect_temporary_bridge<R: BridgeRemote>(
    remote: &mut R,
    selection: BridgeDeploymentSelection,
) -> Result<BridgeConnection<R::Session>, BridgeError> {
    remote
        .upload_temporary(&selection)
        .map_err(|error| BridgeError::Transport(error.to_string()))?;
    let actual = remote
        .remote_sha256(&selection)
        .map_err(|error| BridgeError::Transport(error.to_string()))?;
    if actual != selection.expected_sha256 {
        return Err(BridgeError::ChecksumMismatch);
    }
    let session = remote
        .start_stdio(&selection)
        .map_err(|error| BridgeError::Transport(error.to_string()))?;
    Ok(BridgeConnection {
        session: Some(session),
    })
}

#[derive(Debug, Eq, PartialEq)]
pub enum BridgeError {
    Transport(String),
    ChecksumMismatch,
    MissingLocalBinary,
    UnsafeSelection,
}

impl fmt::Display for BridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(error) => write!(f, "temporary bridge transport failed: {error}"),
            Self::ChecksumMismatch => write!(f, "temporary bridge checksum verification failed"),
            Self::MissingLocalBinary => write!(f, "temporary bridge binary is missing"),
            Self::UnsafeSelection => write!(f, "temporary bridge selection is unsafe"),
        }
    }
}

impl std::error::Error for BridgeError {}
