use std::collections::BTreeMap;

use super::rules::NotificationContext;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct DedupeKey {
    destination: String,
    stove_id: String,
    event: super::rules::NotificationEventKind,
    milestone: Option<u8>,
}

impl From<&NotificationContext> for DedupeKey {
    fn from(context: &NotificationContext) -> Self {
        Self {
            destination: context.destination.as_str().to_owned(),
            stove_id: context.stove_id.clone(),
            event: context.event,
            milestone: context.milestone,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct DedupeWindow {
    sent_at_ms: BTreeMap<DedupeKey, u64>,
    window_ms: u64,
}

impl DedupeWindow {
    pub fn new(window_ms: u64) -> Self {
        Self {
            sent_at_ms: BTreeMap::new(),
            window_ms,
        }
    }

    pub fn is_duplicate(&self, context: &NotificationContext, now_ms: u64) -> bool {
        self.sent_at_ms
            .get(&DedupeKey::from(context))
            .is_some_and(|sent_at| now_ms.saturating_sub(*sent_at) <= self.window_ms)
    }

    pub fn record(&mut self, context: &NotificationContext, now_ms: u64) {
        self.sent_at_ms.insert(DedupeKey::from(context), now_ms);
    }
}
