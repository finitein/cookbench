use std::{fmt, sync::Arc};

use async_trait::async_trait;
use cookbench_core::{
    domain::{HarnessId, HostIdentity, ProjectIdentity, StoveEvent, StoveIdentity},
    locator::HostApplication,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::AdapterCapabilities;

/// A discovery source describes where native session files belong. It carries
/// identity only: adapters own no SSH transport, process lifetime, or agent
/// control capabilities.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HostSource {
    Local(HostIdentity),
    Ssh(HostIdentity),
}

impl HostSource {
    pub fn local(host: HostIdentity) -> Self {
        Self::Local(host)
    }

    pub fn ssh(host: HostIdentity) -> Self {
        Self::Ssh(host)
    }

    pub fn host(&self) -> &HostIdentity {
        match self {
            Self::Local(host) | Self::Ssh(host) => host,
        }
    }
}

/// A bounded, opaque reference to native session metadata, not its content.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionLocator {
    pub kind: SessionLocatorKind,
    pub value: String,
}

impl SessionLocator {
    pub const MAX_VALUE_BYTES: usize = 4 * 1024;

    pub fn new(kind: SessionLocatorKind, value: impl Into<String>) -> Result<Self, AdapterError> {
        let value = value.into();
        if value.is_empty() || value.len() > Self::MAX_VALUE_BYTES {
            return Err(AdapterError::invalid_session_metadata(
                "session locator must be non-empty and bounded",
            ));
        }
        Ok(Self { kind, value })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionLocatorKind {
    LocalPath,
    RemotePath,
    Opaque,
}

/// Minimal native-session metadata. Transcript turns, prompt text, source
/// code, commands, secrets, and credentials intentionally have no place here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeSession {
    pub host: HostIdentity,
    pub harness: HarnessId,
    pub native_session_id: String,
    pub project: Option<ProjectIdentity>,
    pub title: Option<String>,
    pub locator: SessionLocator,
    /// Application-level focus metadata observed from a trusted harness field.
    /// It is optional because a session file often cannot identify its host.
    pub host_application: Option<HostApplication>,
}

impl NativeSession {
    pub const MAX_ID_BYTES: usize = 512;
    pub const MAX_TITLE_BYTES: usize = 512;

    pub fn new(
        host: HostIdentity,
        harness: HarnessId,
        native_session_id: impl Into<String>,
        project: Option<ProjectIdentity>,
        title: Option<String>,
        locator: SessionLocator,
    ) -> Result<Self, AdapterError> {
        let native_session_id = native_session_id.into();
        if native_session_id.is_empty() || native_session_id.len() > Self::MAX_ID_BYTES {
            return Err(AdapterError::invalid_session_metadata(
                "native session ID must be non-empty and bounded",
            ));
        }
        if title
            .as_ref()
            .is_some_and(|title| title.len() > Self::MAX_TITLE_BYTES)
        {
            return Err(AdapterError::invalid_session_metadata(
                "session title must be bounded",
            ));
        }
        Ok(Self {
            host,
            harness,
            native_session_id,
            project,
            title,
            locator,
            host_application: None,
        })
    }

    pub fn with_host_application(mut self, host_application: HostApplication) -> Self {
        self.host_application = Some(host_application);
        self
    }
}

/// A user-mediated suggestion for continuing a session. Returning one is not
/// permission to execute it; the desktop layer decides how to present it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResumeAction {
    OpenSessionLocation(SessionLocator),
    SuggestedCommand { program: String, args: Vec<String> },
}

/// The normalized event channel consumed by Cookbench's state machine.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterEvent {
    pub stove: StoveIdentity,
    pub event: StoveEvent,
}

impl AdapterEvent {
    pub fn new(stove: StoveIdentity, event: StoveEvent) -> Self {
        Self { stove, event }
    }
}

/// The normalized event channel consumed by Cookbench's state machine.
#[derive(Clone, Debug)]
pub struct EventSink {
    sender: mpsc::Sender<AdapterEvent>,
}

impl EventSink {
    pub fn new(sender: mpsc::Sender<AdapterEvent>) -> Self {
        Self { sender }
    }

    pub async fn emit(&self, event: AdapterEvent) -> Result<(), AdapterError> {
        self.sender
            .send(event)
            .await
            .map_err(|_| AdapterError::SinkClosed)
    }
}

impl From<mpsc::Sender<AdapterEvent>> for EventSink {
    fn from(sender: mpsc::Sender<AdapterEvent>) -> Self {
        Self::new(sender)
    }
}

/// Owns a watch lifetime. Calling `cancel`, or dropping the handle, signals
/// all work holding its cancellation token to stop. The handle itself never
/// starts an agent or opens a transport connection.
#[derive(Debug)]
pub struct WatchHandle {
    cancellation: CancellationToken,
    _private: Arc<()>,
}

impl WatchHandle {
    pub fn new() -> Self {
        Self {
            cancellation: CancellationToken::new(),
            _private: Arc::new(()),
        }
    }

    pub fn cancel(&self) {
        self.cancellation.cancel();
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancellation.is_cancelled()
    }

    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation.clone()
    }
}

impl Default for WatchHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for WatchHandle {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdapterError {
    UnsupportedCapability(&'static str),
    InvalidSessionMetadata(String),
    SinkClosed,
    Message(String),
}

impl AdapterError {
    pub fn invalid_session_metadata(message: impl Into<String>) -> Self {
        Self::InvalidSessionMetadata(message.into())
    }
}

impl fmt::Display for AdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedCapability(capability) => {
                write!(formatter, "adapter does not support {capability}")
            }
            Self::InvalidSessionMetadata(message) | Self::Message(message) => {
                formatter.write_str(message)
            }
            Self::SinkClosed => formatter.write_str("Cookbench event sink closed"),
        }
    }
}

impl std::error::Error for AdapterError {}

#[async_trait]
pub trait HarnessAdapter: Send + Sync {
    fn id(&self) -> HarnessId;
    fn capabilities(&self) -> AdapterCapabilities;
    async fn discover(&self, source: &HostSource) -> Result<Vec<NativeSession>, AdapterError>;
    async fn watch(&self, sink: EventSink) -> Result<WatchHandle, AdapterError>;
    fn locate(&self, session: &NativeSession) -> Option<SessionLocator>;
    fn resume(&self, session: &NativeSession) -> Vec<ResumeAction>;
}
