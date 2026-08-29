use super::{DedupeWindow, NotificationContext};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueueItem {
    pub context: NotificationContext,
    pub message: String,
    pub created_at_ms: u64,
    pub retries: u8,
}

impl QueueItem {
    pub fn new(context: NotificationContext, message: String, created_at_ms: u64) -> Self {
        Self {
            context,
            message,
            created_at_ms,
            retries: 0,
        }
    }

    pub fn priority(&self) -> u8 {
        if self.context.event.is_critical() {
            2
        } else if matches!(self.context.event, super::NotificationEventKind::Cooked) {
            1
        } else {
            0
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueuePushResult {
    Enqueued,
    Deduplicated,
    Coalesced,
    Dropped,
}

impl QueuePushResult {
    pub const fn is_enqueued(self) -> bool {
        matches!(self, Self::Enqueued)
    }

    pub const fn is_deduplicated(self) -> bool {
        matches!(self, Self::Deduplicated)
    }

    pub const fn is_coalesced(self) -> bool {
        matches!(self, Self::Coalesced)
    }
}

/// A bounded, in-memory outbound queue. It performs no I/O and never blocks the
/// stove reducer; callers can discard failed records after bounded retries.
#[derive(Clone, Debug)]
pub struct BoundedQueue {
    items: Vec<QueueItem>,
    capacity: usize,
    max_age_ms: u64,
    max_retries: u8,
    dedupe: DedupeWindow,
}

impl BoundedQueue {
    pub fn new(capacity: usize, max_age_ms: u64, max_retries: u8) -> Self {
        Self {
            items: Vec::new(),
            capacity,
            max_age_ms,
            max_retries,
            dedupe: DedupeWindow::new(max_age_ms),
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn push(&mut self, item: QueueItem) -> QueuePushResult {
        if self.capacity == 0 {
            return QueuePushResult::Dropped;
        }
        if self.dedupe.is_duplicate(&item.context, item.created_at_ms) {
            return QueuePushResult::Deduplicated;
        }
        if let Some(index) = self.items.iter().position(|existing| {
            existing.context.destination == item.context.destination
                && existing.context.stove_id == item.context.stove_id
        }) {
            self.dedupe.record(&item.context, item.created_at_ms);
            self.items[index] = item;
            return QueuePushResult::Coalesced;
        }
        if self.items.len() < self.capacity {
            self.dedupe.record(&item.context, item.created_at_ms);
            self.items.push(item);
            return QueuePushResult::Enqueued;
        }

        let Some((index, lowest)) = self
            .items
            .iter()
            .enumerate()
            .min_by_key(|(_, candidate)| (candidate.priority(), candidate.created_at_ms))
        else {
            return QueuePushResult::Dropped;
        };
        if item.priority() > lowest.priority() {
            self.dedupe.record(&item.context, item.created_at_ms);
            self.items[index] = item;
            QueuePushResult::Enqueued
        } else {
            QueuePushResult::Dropped
        }
    }

    pub fn pop_ready(&mut self, now_ms: u64) -> Option<QueueItem> {
        let max_age_ms = self.max_age_ms;
        self.items
            .retain(|item| now_ms.saturating_sub(item.created_at_ms) <= max_age_ms);
        let index = self
            .items
            .iter()
            .enumerate()
            .max_by_key(|(_, item)| (item.priority(), item.created_at_ms))
            .map(|(index, _)| index)?;
        let item = self.items.swap_remove(index);
        Some(item)
    }

    /// Records a failed outbound attempt. `true` means a retry remains queued.
    pub fn record_retry_failure(&mut self, stove_id: &str, now_ms: u64) -> bool {
        let Some(index) = self
            .items
            .iter()
            .position(|item| item.context.stove_id == stove_id)
        else {
            return false;
        };
        let max_age_ms = self.max_age_ms;
        let item = &mut self.items[index];
        item.retries = item.retries.saturating_add(1);
        if item.retries > self.max_retries || now_ms.saturating_sub(item.created_at_ms) > max_age_ms
        {
            self.items.swap_remove(index);
            return false;
        }
        true
    }
}
