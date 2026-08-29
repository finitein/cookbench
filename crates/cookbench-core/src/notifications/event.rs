//! Privacy-safe events for best-effort local completion feedback.
//!
//! The event deliberately contains no native transcript, prompt, command, or
//! credential data. Delivery is handled by the desktop shell and cannot affect
//! the stove state machine.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalNotificationKind {
    Cooked,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalNotificationEvent {
    pub stove_id: String,
    pub kind: LocalNotificationKind,
    pub occurred_at_ms: u64,
}

impl LocalNotificationEvent {
    pub fn cooked(stove_id: impl Into<String>, occurred_at_ms: u64) -> Self {
        Self {
            stove_id: stove_id.into(),
            kind: LocalNotificationKind::Cooked,
            occurred_at_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LocalNotificationEvent, LocalNotificationKind};

    #[test]
    fn cooked_event_carries_only_an_identity_and_timestamp() {
        let event = LocalNotificationEvent::cooked("local:codex:session-42", 42);

        assert_eq!(event.kind, LocalNotificationKind::Cooked);
        assert_eq!(event.stove_id, "local:codex:session-42");
        assert_eq!(event.occurred_at_ms, 42);
    }
}
