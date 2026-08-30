use serde::{Deserialize, Serialize};

use crate::domain::{EventMetadata, StoveIdentity, StoveState};

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

/// A deliberately small reference to a native session. It is sufficient to
/// rediscover a pinned or archived Stove, but intentionally excludes every
/// piece of conversation and task content.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionRecord {
    pub locator: StoveIdentity,
    #[serde(default)]
    pub native_locator: Option<String>,
    pub observed_at_ms: u64,
    #[serde(default)]
    pub presentation: RetainedStovePresentation,
    pub last_state: StoveState,
}

impl SessionRecord {
    pub const MAX_NATIVE_LOCATOR_BYTES: usize = 4_096;

    pub fn new(
        locator: StoveIdentity,
        native_locator: Option<String>,
        observed_at_ms: u64,
        presentation: RetainedStovePresentation,
        last_state: StoveState,
    ) -> Option<Self> {
        if last_state == StoveState::Removed {
            return None;
        }
        let native_locator = sanitize_native_locator(native_locator)?;
        Some(Self {
            locator,
            native_locator,
            observed_at_ms,
            presentation,
            last_state,
        })
    }

    /// Reject control characters and implausibly large locators instead of
    /// persisting data that cannot safely be used as a native file reference.
    pub fn is_valid(&self) -> bool {
        self.last_state != StoveState::Removed
            && sanitize_native_locator(self.native_locator.clone()).is_some()
    }
}

fn sanitize_native_locator(value: Option<String>) -> Option<Option<String>> {
    match value {
        Some(value)
            if value.is_empty()
                || value.len() > SessionRecord::MAX_NATIVE_LOCATOR_BYTES
                || value.chars().any(char::is_control) =>
        {
            None
        }
        Some(value) => Some(Some(value)),
        None => Some(None),
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PinnedSession {
    pub session: SessionRecord,
    pub pinned_at_ms: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ArchiveReason {
    Expired,
    Manual,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ArchivedSession {
    pub session: SessionRecord,
    pub archived_at_ms: u64,
    pub reason: ArchiveReason,
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
    #[serde(default)]
    pub pinned: Vec<PinnedSession>,
    #[serde(default)]
    pub archived: Vec<ArchivedSession>,
    /// Last-known metadata for active non-Cooked sessions. This lets the
    /// desktop archive expired sessions without reopening native transcripts.
    #[serde(default)]
    pub tracked: Vec<SessionRecord>,
}

impl PersistedState {
    pub const CURRENT_VERSION: u32 = 3;

    pub fn with_retained(retained: Vec<RetainedStove>) -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            retained,
            clear_cursors: Vec::new(),
            pinned: Vec::new(),
            archived: Vec::new(),
            tracked: Vec::new(),
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
            pinned: Vec::new(),
            archived: Vec::new(),
            tracked: Vec::new(),
        }
    }
}

impl Versioned for PersistedState {
    const CURRENT_VERSION: u32 = Self::CURRENT_VERSION;

    fn version(&self) -> u32 {
        self.version
    }
}
