//! System OpenSSH invocation with a deliberately tiny, read-only command set.

use std::{
    io::{self, Read},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use cookbench_core::remote::{RemoteHost, SessionRoot};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SshInvocation {
    pub program: &'static str,
    pub args: Vec<String>,
}

impl SshInvocation {
    /// Builds a fixed remote `find` probe. The host is an existing OpenSSH
    /// alias; no `-F`, identity file, password, or host-key override is passed.
    pub fn discover(host: &RemoteHost, root: &SessionRoot) -> Self {
        let remote_command = format!(
            "if [ -d {} ]; then exec find {} -type f -name '*.jsonl' -mtime -7 -print0; fi",
            shell_quote(root.as_str()),
            shell_quote(root.as_str()),
        );
        Self::readonly(host, remote_command)
    }

    /// Resolves only the conventional roots for the Harnesses compiled into
    /// Cookbench. The command is fixed and emits NUL-delimited absolute paths.
    pub fn resolve_automatic_roots(host: &RemoteHost) -> Self {
        Self::readonly(
            host,
            concat!(
                "for root in ",
                "\"${CODEX_HOME:-$HOME/.codex}/sessions\" ",
                "\"${CLAUDE_CONFIG_DIR:-$HOME/.claude}/projects\" ",
                "\"${PI_SESSION_DIR:-$HOME/.pi/agent/sessions}\"; ",
                "do case \"$root\" in /*) printf '%s\\0' \"$root\";; esac; done"
            )
            .to_owned(),
        )
    }

    /// Reads a bounded suffix after discovery has already validated the path.
    /// The fixed `tail` command keeps remote data access read-only.
    pub fn read_suffix(host: &RemoteHost, path: &str) -> Result<Self, SshError> {
        validate_remote_path(path)?;
        Ok(Self::readonly(
            host,
            format!("exec tail -c 65536 {}", shell_quote(path)),
        ))
    }

    fn readonly(host: &RemoteHost, remote_command: String) -> Self {
        Self {
            program: "ssh",
            args: vec![
                "-o".into(),
                "BatchMode=yes".into(),
                "-o".into(),
                "PasswordAuthentication=no".into(),
                "-o".into(),
                "KbdInteractiveAuthentication=no".into(),
                "-o".into(),
                "StrictHostKeyChecking=yes".into(),
                "-o".into(),
                "ConnectTimeout=10".into(),
                "-o".into(),
                "ServerAliveInterval=5".into(),
                "-o".into(),
                "ServerAliveCountMax=2".into(),
                host.alias().to_owned(),
                remote_command,
            ],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProbeOutput {
    pub status: i32,
    pub stdout: Vec<u8>,
    pub stderr: String,
}

#[derive(Debug)]
pub enum SshError {
    Launch(io::Error),
    Disconnected { detail: String },
    UnsafeOutput,
    Timeout,
}

impl std::fmt::Display for SshError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Launch(error) => write!(formatter, "could not start system ssh: {error}"),
            Self::Disconnected { detail } => write!(formatter, "SSH source unavailable: {detail}"),
            Self::UnsafeOutput => formatter.write_str("SSH discovery returned unsafe output"),
            Self::Timeout => formatter.write_str("SSH source exceeded its liveness deadline"),
        }
    }
}

impl std::error::Error for SshError {}

pub trait SshRunner {
    fn run(&self, invocation: &SshInvocation) -> Result<ProbeOutput, SshError>;
}

/// Runs the user-installed `ssh`, inheriting the normal OpenSSH config and
/// known_hosts lookup. This process never writes remote files or opens a port.
pub struct SystemSshRunner;

impl SshRunner for SystemSshRunner {
    fn run(&self, invocation: &SshInvocation) -> Result<ProbeOutput, SshError> {
        let mut child = Command::new(invocation.program)
            .args(&invocation.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(SshError::Launch)?;
        let stdout = child.stdout.take().ok_or(SshError::UnsafeOutput)?;
        let stderr = child.stderr.take().ok_or(SshError::UnsafeOutput)?;
        let stdout_reader = thread::spawn(move || read_bounded(stdout, 512 * 1024));
        let stderr_reader = thread::spawn(move || read_bounded(stderr, 64 * 1024));
        let status = wait_with_deadline(&mut child, Duration::from_secs(15))?;
        let (stdout, stdout_overflow) = stdout_reader
            .join()
            .map_err(|_| SshError::UnsafeOutput)?
            .map_err(SshError::Launch)?;
        let (stderr, stderr_overflow) = stderr_reader
            .join()
            .map_err(|_| SshError::UnsafeOutput)?
            .map_err(SshError::Launch)?;
        if stdout_overflow || stderr_overflow {
            return Err(SshError::UnsafeOutput);
        }
        let result = ProbeOutput {
            status: status.code().unwrap_or(-1),
            stdout,
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
        };
        if status.success() {
            Ok(result)
        } else {
            Err(SshError::Disconnected {
                detail: result.stderr,
            })
        }
    }
}

fn wait_with_deadline(
    child: &mut std::process::Child,
    timeout: Duration,
) -> Result<std::process::ExitStatus, SshError> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait().map_err(SshError::Launch)? {
            return Ok(status);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(SshError::Timeout);
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn read_bounded(mut reader: impl Read, maximum: usize) -> Result<(Vec<u8>, bool), io::Error> {
    let mut retained = Vec::with_capacity(maximum.min(8 * 1024));
    let mut overflow = false;
    let mut buffer = [0_u8; 8 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let remaining = maximum.saturating_sub(retained.len());
        retained.extend_from_slice(&buffer[..read.min(remaining)]);
        overflow |= read > remaining;
    }
    Ok((retained, overflow))
}

pub fn session_paths(output: &ProbeOutput, root: &SessionRoot) -> Result<Vec<String>, SshError> {
    if output.status != 0 {
        return Err(SshError::UnsafeOutput);
    }

    output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|bytes| {
            let path = std::str::from_utf8(bytes).map_err(|_| SshError::UnsafeOutput)?;
            let belongs_to_root = root.as_str() == "/"
                || path == root.as_str()
                || path
                    .strip_prefix(root.as_str())
                    .is_some_and(|suffix| suffix.starts_with('/'));
            if path.chars().any(char::is_control) || !belongs_to_root {
                Err(SshError::UnsafeOutput)
            } else {
                Ok(path.to_owned())
            }
        })
        .collect()
}

pub fn automatic_session_roots(output: &ProbeOutput) -> Result<Vec<SessionRoot>, SshError> {
    if output.status != 0 {
        return Err(SshError::UnsafeOutput);
    }
    let mut roots = Vec::new();
    for bytes in output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
    {
        if roots.len() >= 16 {
            return Err(SshError::UnsafeOutput);
        }
        let path = std::str::from_utf8(bytes).map_err(|_| SshError::UnsafeOutput)?;
        let root = SessionRoot::new(path.to_owned()).map_err(|_| SshError::UnsafeOutput)?;
        if !roots.contains(&root) {
            roots.push(root);
        }
    }
    Ok(roots)
}

pub fn suffix_bytes(output: &ProbeOutput) -> Result<&[u8], SshError> {
    if output.status != 0 || output.stdout.len() > 65_536 {
        return Err(SshError::UnsafeOutput);
    }
    Ok(&output.stdout)
}

fn validate_remote_path(path: &str) -> Result<(), SshError> {
    if path.is_empty()
        || !path.starts_with('/')
        || path.len() > 4 * 1024
        || path.chars().any(char::is_control)
    {
        return Err(SshError::UnsafeOutput);
    }
    Ok(())
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(all(test, unix))]
mod tests {
    use std::{process::Command, time::Duration};

    use super::{wait_with_deadline, SshError};

    #[test]
    fn stalled_child_is_terminated_at_the_local_deadline() {
        let mut child = Command::new("sh")
            .args(["-c", "sleep 2"])
            .spawn()
            .expect("synthetic stalled child should start");
        assert!(matches!(
            wait_with_deadline(&mut child, Duration::from_millis(50)),
            Err(SshError::Timeout)
        ));
        assert!(child.try_wait().unwrap().is_some());
    }
}
