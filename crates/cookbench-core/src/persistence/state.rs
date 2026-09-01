use std::collections::HashSet;

use serde::{Deserialize, Deserializer, Serialize};

use crate::domain::{
    EventMetadata, EventSource, HarnessId, HostKind, Stove, StoveIdentity, StoveState,
};

use super::Versioned;

/// The completion record contains a locator, not a copy of the native session.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RetainedStove {
    pub locator: StoveIdentity,
    pub completed_at_ms: u64,
    #[serde(default)]
    pub completion_event: Option<EventMetadata>,
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
            completion_event: None,
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
            completion_event: None,
            presentation,
        }
    }

    pub fn with_completion_event(mut self, event: EventMetadata) -> Self {
        self.completion_event = Some(event);
        self
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

/// A metadata-only acknowledgement of one observed Cooked completion.
///
/// The completion event identity is retained so a later completion in the
/// same native session is surfaced as new attention again.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CookedAttentionCursor {
    pub locator: StoveIdentity,
    pub source: EventSource,
    pub confidence: u8,
    pub sequence: u64,
    pub timestamp_ms: u64,
    pub acknowledged_at_ms: u64,
}

fn deserialize_cooked_attention_cursors<'de, D>(
    deserializer: D,
) -> Result<Vec<CookedAttentionCursor>, D::Error>
where
    D: Deserializer<'de>,
{
    let mut cursors = Vec::<CookedAttentionCursor>::deserialize(deserializer)?;
    normalize_cooked_attention_cursors(&mut cursors);
    Ok(cursors)
}

impl CookedAttentionCursor {
    pub fn from_stove(stove: &Stove, acknowledged_at_ms: u64) -> Option<Self> {
        if stove.state != StoveState::Cooked {
            return None;
        }

        let event = stove.last_event.as_ref()?;
        Some(Self {
            locator: stove.identity.clone(),
            source: event.source,
            confidence: event.confidence,
            sequence: event.sequence,
            timestamp_ms: event.timestamp_ms,
            acknowledged_at_ms,
        })
    }

    pub fn acknowledges(&self, stove: &Stove) -> bool {
        let Some(event) = stove.last_event.as_ref() else {
            return false;
        };

        stove.state == StoveState::Cooked
            && self.locator == stove.identity
            && self.source == event.source
            && self.confidence == event.confidence
            && self.sequence == event.sequence
            && self.timestamp_ms == event.timestamp_ms
            || (stove.state == StoveState::Cooked
                && self.locator == stove.identity
                && event.source == EventSource::StructuredSession
                && event.confidence == 100
                && event.sequence == 1
                && self.timestamp_ms == event.timestamp_ms)
    }
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
    #[serde(default, deserialize_with = "deserialize_cooked_attention_cursors")]
    pub cooked_attention_cursors: Vec<CookedAttentionCursor>,
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
    pub const MAX_COOKED_ATTENTION_CURSORS: usize = 256;

    pub fn with_retained(retained: Vec<RetainedStove>) -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            retained,
            clear_cursors: Vec::new(),
            cooked_attention_cursors: Vec::new(),
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

    /// Replaces any acknowledgement for the same native session and retains a
    /// deterministic, bounded set of the newest acknowledgements.
    pub fn acknowledge_cooked(&mut self, stove: &Stove, acknowledged_at_ms: u64) -> bool {
        let Some(cursor) = CookedAttentionCursor::from_stove(stove, acknowledged_at_ms) else {
            return false;
        };

        self.cooked_attention_cursors
            .retain(|existing| existing.locator != cursor.locator);
        self.cooked_attention_cursors.push(cursor);
        normalize_cooked_attention_cursors(&mut self.cooked_attention_cursors);
        true
    }
}

fn normalize_cooked_attention_cursors(cursors: &mut Vec<CookedAttentionCursor>) {
    cursors.sort_by(compare_cooked_attention_cursor);
    let mut retained_locators = HashSet::with_capacity(cursors.len());
    cursors.retain(|cursor| retained_locators.insert(cursor.locator.clone()));
    cursors.truncate(PersistedState::MAX_COOKED_ATTENTION_CURSORS);
}

fn compare_cooked_attention_cursor(
    left: &CookedAttentionCursor,
    right: &CookedAttentionCursor,
) -> std::cmp::Ordering {
    right
        .acknowledged_at_ms
        .cmp(&left.acknowledged_at_ms)
        .then_with(|| right.timestamp_ms.cmp(&left.timestamp_ms))
        .then_with(|| right.sequence.cmp(&left.sequence))
        .then_with(|| right.source.cmp(&left.source))
        .then_with(|| right.confidence.cmp(&left.confidence))
        .then_with(|| compare_stove_identity(&right.locator, &left.locator))
}

fn compare_stove_identity(left: &StoveIdentity, right: &StoveIdentity) -> std::cmp::Ordering {
    host_kind_order(&left.host.kind)
        .cmp(&host_kind_order(&right.host.kind))
        .then_with(|| left.host.id.cmp(&right.host.id))
        .then_with(|| compare_harness(&left.harness, &right.harness))
        .then_with(|| left.native_session_id.cmp(&right.native_session_id))
}

fn host_kind_order(kind: &HostKind) -> u8 {
    match kind {
        HostKind::Local => 0,
        HostKind::Ssh => 1,
    }
}

fn compare_harness(left: &HarnessId, right: &HarnessId) -> std::cmp::Ordering {
    harness_order(left)
        .cmp(&harness_order(right))
        .then_with(|| match (left, right) {
            (HarnessId::Other(left), HarnessId::Other(right)) => left.cmp(right),
            _ => std::cmp::Ordering::Equal,
        })
}

fn harness_order(harness: &HarnessId) -> u8 {
    match harness {
        HarnessId::Codex => 0,
        HarnessId::ClaudeCode => 1,
        HarnessId::Pi => 2,
        HarnessId::Other(_) => 3,
    }
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            retained: Vec::new(),
            clear_cursors: Vec::new(),
            cooked_attention_cursors: Vec::new(),
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
