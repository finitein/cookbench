//! System OpenSSH invocation with a deliberately tiny, read-only command set.

use std::{io, process::Command};

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
            "exec find {} -type f -name '*.jsonl' -print0",
            shell_quote(root.as_str())
        );
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
}

impl std::fmt::Display for SshError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Launch(error) => write!(formatter, "could not start system ssh: {error}"),
            Self::Disconnected { detail } => write!(formatter, "SSH source unavailable: {detail}"),
            Self::UnsafeOutput => formatter.write_str("SSH discovery returned unsafe output"),
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
        let output = Command::new(invocation.program)
            .args(&invocation.args)
            .output()
            .map_err(SshError::Launch)?;
        let result = ProbeOutput {
            status: output.status.code().unwrap_or(-1),
            stdout: output.stdout,
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        };
        if output.status.success() {
            Ok(result)
        } else {
            Err(SshError::Disconnected {
                detail: result.stderr,
            })
        }
    }
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

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
