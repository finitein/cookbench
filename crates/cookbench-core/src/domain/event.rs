use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum EventSource {
    Inference,
    Process,
    StructuredSession,
    Hook,
}

impl EventSource {
    pub const fn authority(self) -> u8 {
        match self {
            Self::Inference => 0,
            Self::Process => 1,
            Self::StructuredSession => 2,
            Self::Hook => 3,
        }
    }

    pub const fn provides_structured_progress(self) -> bool {
        matches!(self, Self::StructuredSession | Self::Hook)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EventMetadata {
    pub source: EventSource,
    pub confidence: u8,
    pub sequence: u64,
    pub timestamp_ms: u64,
}

impl EventMetadata {
    pub const fn new(
        source: EventSource,
        confidence: u8,
        sequence: u64,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            source,
            confidence,
            sequence,
            timestamp_ms,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EventKind {
    SessionDiscovered,
    UserPromptSubmitted,
    PlanUpdated { completed: u32, total: u32 },
    ToolStarted,
    ToolCompleted { succeeded: bool },
    QuestionAsked,
    PermissionRequested,
    TurnCompleted,
    SessionFailed,
    ProcessExited,
    ConnectionLost,
    ConnectionRestored,
    ClearRequested,
    Tick,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StoveEvent {
    pub kind: EventKind,
    pub metadata: EventMetadata,
}

impl StoveEvent {
    pub const fn new(kind: EventKind, metadata: EventMetadata) -> Self {
        Self { kind, metadata }
    }
}
