use cookbench_core::domain::EventKind;
use serde::{Deserialize, Serialize};

/// The only extension payload accepted by Cookbench. It is intentionally
/// content-free and bounded so Pi prompt/code/command data cannot enter state.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionEnvelope {
    pub version: u8,
    pub session_id: String,
    pub event: ExtensionEvent,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExtensionEvent {
    PromptSubmitted,
    ToolStarted,
    ToolCompleted { succeeded: bool },
    QuestionAsked,
    PermissionRequested,
    TurnCompleted,
    SessionFailed,
    TodoProgress { completed: u32, total: u32 },
}

impl ExtensionEnvelope {
    pub const MAX_SESSION_ID_BYTES: usize = 512;

    pub fn new(session_id: impl Into<String>, event: ExtensionEvent) -> Option<Self> {
        let session_id = session_id.into();
        (!session_id.is_empty() && session_id.len() <= Self::MAX_SESSION_ID_BYTES).then_some(Self {
            version: 1,
            session_id,
            event,
        })
    }

    pub fn event_kind(&self) -> Option<EventKind> {
        match self.event {
            ExtensionEvent::PromptSubmitted => Some(EventKind::UserPromptSubmitted),
            ExtensionEvent::ToolStarted => Some(EventKind::ToolStarted),
            ExtensionEvent::ToolCompleted { succeeded } => {
                Some(EventKind::ToolCompleted { succeeded })
            }
            ExtensionEvent::QuestionAsked => Some(EventKind::QuestionAsked),
            ExtensionEvent::PermissionRequested => Some(EventKind::PermissionRequested),
            ExtensionEvent::TurnCompleted => Some(EventKind::TurnCompleted),
            ExtensionEvent::SessionFailed => Some(EventKind::SessionFailed),
            ExtensionEvent::TodoProgress { completed, total }
                if total > 0 && completed <= total =>
            {
                Some(EventKind::PlanUpdated { completed, total })
            }
            ExtensionEvent::TodoProgress { .. } => None,
        }
    }
}
