use serde::{Deserialize, Serialize};

use crate::domain::{EventMetadata, StoveIdentity};

use super::Versioned;

/// The completion record contains a locator, not a copy of the native session.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RetainedStove {
    pub locator: StoveIdentity,
    pub completed_at_ms: u64,
    /// Display-only metadata required to reconstruct a retained Stove before
    /// its native session is rediscovered. It never contains task text.
    #[serde(default)]
    pub presentation: RetainedStovePresentation,
}

impl RetainedStove {
    pub fn new(locator: StoveIdentity, completed_at_ms: u64) -> Self {
        Self {
            locator,
            completed_at_ms,
            presentation: RetainedStovePresentation::default(),
        }
    }

    pub fn with_presentation(
        locator: StoveIdentity,
        completed_at_ms: u64,
        presentation: RetainedStovePresentation,
    ) -> Self {
        Self {
            locator,
            completed_at_ms,
            presentation,
        }
    }
}

/// Deliberately minimal retained rendering data. Titles, activities, prompts,
/// commands, tool output, and native session text are not persisted.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RetainedStovePresentation {
    #[serde(default)]
    pub project_label: String,
    #[serde(default)]
    pub project_root_display: String,
}

impl RetainedStovePresentation {
    pub const MAX_TEXT_BYTES: usize = 512;

    pub fn new(project_label: impl Into<String>, project_root_display: impl Into<String>) -> Self {
        Self {
            project_label: sanitize_display_text(project_label.into()),
            project_root_display: sanitize_display_text(project_root_display.into()),
        }
    }
}

fn sanitize_display_text(mut value: String) -> String {
    value.retain(|character| !character.is_control());
    if value.len() > RetainedStovePresentation::MAX_TEXT_BYTES {
        let mut boundary = RetainedStovePresentation::MAX_TEXT_BYTES;
        while !value.is_char_boundary(boundary) {
            boundary -= 1;
        }
        value.truncate(boundary);
    }
    value
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ClearCursor {
    pub locator: StoveIdentity,
    pub sequence: u64,
    pub timestamp_ms: u64,
}

impl ClearCursor {
    pub fn new(locator: StoveIdentity, sequence: u64, timestamp_ms: u64) -> Self {
        Self {
            locator,
            sequence,
            timestamp_ms,
        }
    }

    /// Sequence is the primary event order; timestamp disambiguates a sequence tie.
    pub fn hides(&self, locator: &StoveIdentity, event: &EventMetadata) -> bool {
        if &self.locator != locator {
            return false;
        }

        event.sequence < self.sequence
            || (event.sequence == self.sequence && event.timestamp_ms <= self.timestamp_ms)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PersistedState {
    pub version: u32,
    #[serde(default)]
    pub retained: Vec<RetainedStove>,
    #[serde(default)]
    pub clear_cursors: Vec<ClearCursor>,
}

impl PersistedState {
    pub const CURRENT_VERSION: u32 = 1;

    pub fn with_retained(retained: Vec<RetainedStove>) -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            retained,
            clear_cursors: Vec::new(),
        }
    }

    pub fn is_hidden(&self, locator: &StoveIdentity, event: &EventMetadata) -> bool {
        self.clear_cursors
            .iter()
            .any(|cursor| cursor.hides(locator, event))
    }
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            retained: Vec::new(),
            clear_cursors: Vec::new(),
        }
    }
}

impl Versioned for PersistedState {
    const CURRENT_VERSION: u32 = Self::CURRENT_VERSION;

    fn version(&self) -> u32 {
        self.version
    }
}
