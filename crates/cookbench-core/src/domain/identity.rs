use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum HostKind {
    Local,
    Ssh,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct HostIdentity {
    pub kind: HostKind,
    pub id: String,
}

impl HostIdentity {
    pub fn local(id: impl Into<String>) -> Self {
        Self {
            kind: HostKind::Local,
            id: id.into(),
        }
    }

    pub fn ssh(id: impl Into<String>) -> Self {
        Self {
            kind: HostKind::Ssh,
            id: id.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum HarnessId {
    Codex,
    ClaudeCode,
    Pi,
    Other(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct StoveIdentity {
    pub host: HostIdentity,
    pub harness: HarnessId,
    pub native_session_id: String,
}

impl StoveIdentity {
    pub fn new(
        host: HostIdentity,
        harness: HarnessId,
        native_session_id: impl Into<String>,
    ) -> Self {
        Self {
            host,
            harness,
            native_session_id: native_session_id.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ProjectIdentity {
    pub host: HostIdentity,
    pub canonical_root: String,
}

impl ProjectIdentity {
    pub fn new(host: HostIdentity, canonical_root: impl Into<String>) -> Self {
        Self {
            host,
            canonical_root: canonical_root.into(),
        }
    }
}
