use serde::{Deserialize, Serialize};

use super::EventSource;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StructuredProgress {
    pub completed: u32,
    pub total: u32,
    pub source: EventSource,
}

impl StructuredProgress {
    pub const fn new(completed: u32, total: u32, source: EventSource) -> Option<Self> {
        if !source.provides_structured_progress() || total == 0 || completed > total {
            return None;
        }

        Some(Self {
            completed,
            total,
            source,
        })
    }

    pub const fn fraction(&self) -> (u32, u32) {
        (self.completed, self.total)
    }
}
