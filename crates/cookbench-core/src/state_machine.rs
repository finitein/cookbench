use std::fmt;

use crate::domain::{EventKind, EventMetadata, Stove, StoveEvent, StoveState, StructuredProgress};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransitionError {
    CannotClear(StoveState),
    InvalidProgress { completed: u32, total: u32 },
}

impl fmt::Display for TransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CannotClear(state) => write!(formatter, "cannot clear a {state:?} stove"),
            Self::InvalidProgress { completed, total } => {
                write!(formatter, "invalid structured progress {completed}/{total}")
            }
        }
    }
}

impl std::error::Error for TransitionError {}

/// Reduces an observed harness event without mutating the prior stove snapshot.
pub fn reduce(previous: &Stove, event: &StoveEvent) -> Result<Stove, TransitionError> {
    if is_superseded(previous.last_event.as_ref(), &event.metadata) {
        return Ok(previous.clone());
    }

    let mut next = previous.clone();

    if previous.state == StoveState::Disconnected
        && !matches!(
            &event.kind,
            EventKind::ConnectionRestored | EventKind::ClearRequested
        )
    {
        next.last_event = Some(event.metadata.clone());
        return Ok(next);
    }

    match &event.kind {
        EventKind::SessionDiscovered => next.state = StoveState::Starting,
        EventKind::UserPromptSubmitted => {
            next.state = StoveState::Cooking;
            next.progress = None;
        }
        EventKind::PlanUpdated { completed, total } => {
            let Some(progress) = StructuredProgress::new(*completed, *total, event.metadata.source)
            else {
                if event.metadata.source.provides_structured_progress() {
                    return Err(TransitionError::InvalidProgress {
                        completed: *completed,
                        total: *total,
                    });
                }
                return Ok(next);
            };
            next.state = StoveState::Planning;
            next.progress = Some(progress);
        }
        EventKind::ToolStarted | EventKind::ToolCompleted { .. } => {
            next.state = StoveState::Cooking
        }
        EventKind::QuestionAsked | EventKind::PermissionRequested => {
            next.state = StoveState::NeedsHuman
        }
        EventKind::TurnCompleted => next.state = StoveState::Cooked,
        EventKind::SessionFailed => next.state = StoveState::Failed,
        EventKind::ProcessExited | EventKind::Tick => {}
        EventKind::ConnectionLost => {
            next.state_before_disconnect = Some(previous.state);
            next.state = StoveState::Disconnected;
        }
        EventKind::ConnectionRestored => {
            if previous.state == StoveState::Disconnected {
                next.state = previous
                    .state_before_disconnect
                    .unwrap_or(StoveState::Starting);
                next.state_before_disconnect = None;
            }
        }
        EventKind::ClearRequested => {
            if previous.state != StoveState::Cooked {
                return Err(TransitionError::CannotClear(previous.state));
            }
            next.state = StoveState::Removed;
            next.progress = None;
        }
    }

    next.last_event = Some(event.metadata.clone());
    Ok(next)
}

fn is_superseded(previous: Option<&EventMetadata>, incoming: &EventMetadata) -> bool {
    let Some(previous) = previous else {
        return false;
    };

    match incoming.sequence.cmp(&previous.sequence) {
        std::cmp::Ordering::Less => true,
        std::cmp::Ordering::Greater => false,
        std::cmp::Ordering::Equal => match incoming.timestamp_ms.cmp(&previous.timestamp_ms) {
            std::cmp::Ordering::Less => true,
            std::cmp::Ordering::Greater => false,
            std::cmp::Ordering::Equal => {
                incoming.source.authority() < previous.source.authority()
                    || (incoming.source == previous.source
                        && incoming.confidence < previous.confidence)
            }
        },
    }
}
