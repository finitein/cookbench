use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::domain::{HarnessId, HostIdentity, StoveIdentity};

const MAX_REMOTE_TEXT_BYTES: usize = 4 * 1024;

/// A configured remote session directory. It is a discovery input, never a
/// remote write target.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionRoot(String);

impl SessionRoot {
    pub fn new(path: impl Into<String>) -> Result<Self, HostValidationError> {
        let path = path.into();
        if path.is_empty() || !path.starts_with('/') {
            return Err(HostValidationError::RootMustBeAbsolute);
        }
        validate_text(&path)?;
        Ok(Self(path))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Uses an existing OpenSSH host alias and the user's normal config lookup.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RemoteHost {
    alias: String,
    session_roots: Vec<SessionRoot>,
}

impl RemoteHost {
    pub fn new(
        alias: impl Into<String>,
        session_roots: Vec<SessionRoot>,
    ) -> Result<Self, HostValidationError> {
        let alias = alias.into();
        if alias.is_empty() || alias.starts_with('-') || alias.chars().any(char::is_whitespace) {
            return Err(HostValidationError::UnsafeAlias);
        }
        validate_text(&alias)?;
        Ok(Self {
            alias,
            session_roots,
        })
    }

    pub fn alias(&self) -> &str {
        &self.alias
    }

    pub fn session_roots(&self) -> &[SessionRoot] {
        &self.session_roots
    }

    /// Empty configured roots mean the transport should resolve every
    /// first-party Harness root supported by this Cookbench build.
    pub fn uses_automatic_roots(&self) -> bool {
        self.session_roots.is_empty()
    }

    /// Percent-encoding makes host aliases unambiguous alongside local hosts.
    pub fn identity(&self) -> HostIdentity {
        HostIdentity::ssh(format!("ssh:{}", encode_component(&self.alias)))
    }
}

/// Collision-safe remote session identity. Native IDs remain opaque and are
/// never interpreted as paths or shell input.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct RemoteSessionIdentity {
    pub host: HostIdentity,
    pub harness: HarnessId,
    pub native_session_id: String,
}

impl RemoteSessionIdentity {
    pub fn new(
        host: &RemoteHost,
        harness: HarnessId,
        native_session_id: impl Into<String>,
    ) -> Result<Self, HostValidationError> {
        let native_session_id = native_session_id.into();
        if native_session_id.is_empty() {
            return Err(HostValidationError::EmptySessionId);
        }
        validate_text(&native_session_id)?;
        Ok(Self {
            host: host.identity(),
            harness,
            native_session_id,
        })
    }

    pub fn stove_identity(&self) -> StoveIdentity {
        StoveIdentity::new(
            self.host.clone(),
            self.harness.clone(),
            self.native_session_id.clone(),
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PollInterval {
    Active,
    Idle,
    Disconnected,
}

impl PollInterval {
    pub fn duration(self) -> Duration {
        match self {
            Self::Active => Duration::from_secs(2),
            Self::Idle => Duration::from_secs(15),
            Self::Disconnected => Duration::from_secs(30),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostValidationError {
    UnsafeAlias,
    RootMustBeAbsolute,
    EmptySessionId,
    UnsafeText,
}

impl std::fmt::Display for HostValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::UnsafeAlias => {
                "SSH host aliases must be nonempty, option-free, and whitespace-free"
            }
            Self::RootMustBeAbsolute => "remote session roots must be absolute paths",
            Self::EmptySessionId => "remote native session IDs must be nonempty",
            Self::UnsafeText => "remote source values must be bounded and control-free",
        })
    }
}

impl std::error::Error for HostValidationError {}

fn validate_text(value: &str) -> Result<(), HostValidationError> {
    if value.len() > MAX_REMOTE_TEXT_BYTES || value.chars().any(char::is_control) {
        return Err(HostValidationError::UnsafeText);
    }
    Ok(())
}

fn encode_component(value: &str) -> String {
    value.bytes().fold(String::new(), |mut encoded, byte| {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-') {
            encoded.push(byte as char);
        } else {
            use std::fmt::Write;
            let _ = write!(encoded, "%{byte:02X}");
        }
        encoded
    })
}

#[cfg(test)]
mod tests {
    use crate::domain::HarnessId;

    use super::{PollInterval, RemoteHost, RemoteSessionIdentity, SessionRoot};

    #[test]
    fn aliases_and_native_ids_cannot_collide_with_delimiters() {
        let a =
            RemoteHost::new("ci:one", vec![SessionRoot::new("/safe/sessions").unwrap()]).unwrap();
        let b = RemoteHost::new(
            "ci%3Aone",
            vec![SessionRoot::new("/safe/sessions").unwrap()],
        )
        .unwrap();
        assert_ne!(a.identity(), b.identity());
        assert_ne!(
            RemoteSessionIdentity::new(&a, HarnessId::Codex, "session:one")
                .unwrap()
                .stove_identity(),
            RemoteSessionIdentity::new(&b, HarnessId::Codex, "session:one")
                .unwrap()
                .stove_identity(),
        );
    }

    #[test]
    fn polling_is_fast_only_when_a_source_is_active() {
        assert!(PollInterval::Active.duration() < PollInterval::Idle.duration());
        assert!(PollInterval::Idle.duration() < PollInterval::Disconnected.duration());
    }

    #[test]
    fn empty_roots_select_automatic_supported_harness_discovery() {
        let host = RemoteHost::new("automatic-host", vec![]).expect("automatic roots");

        assert!(host.uses_automatic_roots());
        assert!(host.session_roots().is_empty());
    }
}
