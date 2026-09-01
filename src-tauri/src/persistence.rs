//! Desktop persistence orchestration for Cookbench-owned presentation state.
//!
//! Native harness sessions remain authoritative. This service only stores
//! configuration, retained Cooked locators, and clear cursors in atomic JSON.

use std::path::Path;

use cookbench_core::{
    domain::{EventMetadata, StoveIdentity, StoveState},
    persistence::{
        ArchiveReason, ArchivedSession, AtomicJsonFile, ClearCursor, PersistedConfig,
        PersistedState, PersistenceError, PinnedSession, RetainedStove, RetainedStovePresentation,
        SessionRecord,
    },
};

const MAX_RETAINED_STOVES: usize = 1_024;
const MAX_CLEAR_CURSORS: usize = 1_024;
const MAX_PINNED_SESSIONS: usize = 1_024;
const MAX_ARCHIVED_SESSIONS: usize = 4_096;
const MAX_TRACKED_SESSIONS: usize = 4_096;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoadIssue {
    Config(PersistenceErrorKind),
    State(PersistenceErrorKind),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PersistenceErrorKind {
    InvalidJson,
    UnsupportedVersion,
    Io,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadedDesktopPersistence {
    pub config: PersistedConfig,
    pub state: PersistedState,
    pub issues: Vec<LoadIssue>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeSessionObservation {
    pub locator: StoveIdentity,
    pub last_event: EventMetadata,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoredCookedStove {
    pub retained: RetainedStove,
}

/// Owns only paths to Cookbench's atomic JSON files. It never opens a native
/// harness session path and it cannot delete source history.
pub struct DesktopPersistence {
    config: AtomicJsonFile<PersistedConfig>,
    state: AtomicJsonFile<PersistedState>,
}

impl DesktopPersistence {
    pub fn in_app_data(app_data_directory: impl AsRef<Path>) -> Self {
        let app_data_directory = app_data_directory.as_ref();
        Self {
            config: AtomicJsonFile::new(app_data_directory.join("config.json")),
            state: AtomicJsonFile::new(app_data_directory.join("state.json")),
        }
    }

    pub fn config_path(&self) -> &Path {
        self.config.path()
    }

    pub fn state_path(&self) -> &Path {
        self.state.path()
    }

    /// Isolates either file failure so one corrupt or future-version file never
    /// prevents the desktop app from starting with the other file's data.
    pub fn load(&self) -> LoadedDesktopPersistence {
        let mut issues = Vec::new();
        let config = match self.config.load() {
            Ok(config) => config,
            Err(error) => {
                issues.push(LoadIssue::Config(error_kind(&error)));
                PersistedConfig::default()
            }
        };
        let mut state = match self.state.load() {
            Ok(state) => state,
            Err(error) => {
                issues.push(LoadIssue::State(error_kind(&error)));
                PersistedState::default()
            }
        };
        if state.version == 1 {
            // Pre-release builds treated bootstrap transcript replay as live
            // completion and filled this Cookbench-owned cache with history.
            // Native session files remain untouched; clear cursors still
            // protect explicit user clears.
            state.retained.clear();
            state.version = PersistedState::CURRENT_VERSION;
            if let Err(error) = self.state.save(&state) {
                issues.push(LoadIssue::State(error_kind(&error)));
            }
        } else if state.version < PersistedState::CURRENT_VERSION {
            state.version = PersistedState::CURRENT_VERSION;
            if let Err(error) = self.state.save(&state) {
                issues.push(LoadIssue::State(error_kind(&error)));
            }
        }

        LoadedDesktopPersistence {
            config,
            state,
            issues,
        }
    }

    pub fn save_config(&self, config: &PersistedConfig) -> Result<(), PersistenceError> {
        self.config.save(config)
    }

    pub fn save_state(&self, state: &PersistedState) -> Result<(), PersistenceError> {
        self.state.save(state)
    }

    /// Records only safe session metadata for the expiry sweep. Cooked
    /// sessions are retained separately, so callers should not track them.
    pub fn track_session(
        &self,
        state: &mut PersistedState,
        session: SessionRecord,
    ) -> Result<bool, PersistenceError> {
        if !session.is_valid() {
            return Ok(false);
        }
        if let Some(existing) = state
            .tracked
            .iter()
            .find(|tracked| tracked.locator == session.locator)
        {
            let same_metadata = existing.native_locator == session.native_locator
                && existing.presentation == session.presentation
                && existing.last_state == session.last_state;
            if same_metadata
                && session
                    .observed_at_ms
                    .saturating_sub(existing.observed_at_ms)
                    < 60_000
            {
                return Ok(false);
            }
        }
        state
            .tracked
            .retain(|tracked| tracked.locator != session.locator);
        if state.tracked.len() >= MAX_TRACKED_SESSIONS {
            return Ok(false);
        }
        state.tracked.push(session);
        cap_tracked(state);
        self.save_state(state)?;
        Ok(true)
    }

    pub fn remove_tracked(
        &self,
        state: &mut PersistedState,
        locator: &StoveIdentity,
    ) -> Result<bool, PersistenceError> {
        let before = state.tracked.len();
        state.tracked.retain(|tracked| &tracked.locator != locator);
        if before == state.tracked.len() {
            return Ok(false);
        }
        self.save_state(state)?;
        Ok(true)
    }

    /// Pins a minimal native-session reference. Invalid locators and a full
    /// pin set are rejected without changing persisted state.
    pub fn pin_session(
        &self,
        state: &mut PersistedState,
        session: SessionRecord,
        pinned_at_ms: u64,
    ) -> Result<bool, PersistenceError> {
        if !session.is_valid() {
            return Ok(false);
        }
        let mut next = state.clone();
        next.archived
            .retain(|archived| archived.session.locator != session.locator);
        next.tracked
            .retain(|tracked| tracked.locator != session.locator);
        next.pinned
            .retain(|pinned| pinned.session.locator != session.locator);
        if next.pinned.len() >= MAX_PINNED_SESSIONS {
            return Ok(false);
        }
        next.pinned.push(PinnedSession {
            session,
            pinned_at_ms,
        });
        self.save_state(&next)?;
        *state = next;
        Ok(true)
    }

    pub fn unpin_session(
        &self,
        state: &mut PersistedState,
        locator: &StoveIdentity,
    ) -> Result<bool, PersistenceError> {
        let before = state.pinned.len();
        state
            .pinned
            .retain(|pinned| &pinned.session.locator != locator);
        if state.pinned.len() == before {
            return Ok(false);
        }
        self.save_state(state)?;
        Ok(true)
    }

    /// Archives Cookbench metadata only. The native session file is never
    /// opened or changed. Archive always removes a matching pin so manual
    /// deletion wins over permanence.
    pub fn archive_session(
        &self,
        state: &mut PersistedState,
        session: SessionRecord,
        archived_at_ms: u64,
        reason: ArchiveReason,
    ) -> Result<bool, PersistenceError> {
        if !session.is_valid() {
            return Ok(false);
        }
        let mut next = state.clone();
        next.pinned
            .retain(|pinned| pinned.session.locator != session.locator);
        next.archived
            .retain(|archived| archived.session.locator != session.locator);
        next.tracked
            .retain(|tracked| tracked.locator != session.locator);
        if next.archived.len() >= MAX_ARCHIVED_SESSIONS {
            if reason == ArchiveReason::Manual {
                if let Some(index) = next
                    .archived
                    .iter()
                    .enumerate()
                    .filter(|(_, archived)| archived.reason == ArchiveReason::Expired)
                    .min_by_key(|(_, archived)| archived.archived_at_ms)
                    .map(|(index, _)| index)
                {
                    next.archived.remove(index);
                } else {
                    return Ok(false);
                }
            } else {
                return Ok(false);
            }
        }
        next.archived.push(ArchivedSession {
            session,
            archived_at_ms,
            reason,
        });
        cap_archived(&mut next);
        self.save_state(&next)?;
        *state = next;
        Ok(true)
    }

    /// Restores the metadata reference to the caller. Restoring does not
    /// touch native history; callers choose whether it should also be pinned.
    pub fn restore_session(
        &self,
        state: &mut PersistedState,
        locator: &StoveIdentity,
        pin: bool,
        restored_at_ms: u64,
    ) -> Result<Option<ArchivedSession>, PersistenceError> {
        let mut next = state.clone();
        let Some(index) = next
            .archived
            .iter()
            .position(|archived| &archived.session.locator == locator)
        else {
            return Ok(None);
        };
        let archived = next.archived.remove(index);
        next.tracked
            .retain(|tracked| tracked.locator != archived.session.locator);
        if archived.session.last_state != StoveState::Cooked {
            next.tracked.push(archived.session.clone());
            cap_tracked(&mut next);
        }
        if pin {
            next.pinned
                .retain(|pinned| pinned.session.locator != *locator);
            if next.pinned.len() >= MAX_PINNED_SESSIONS {
                return Ok(None);
            }
            next.pinned.push(PinnedSession {
                session: archived.session.clone(),
                pinned_at_ms: restored_at_ms,
            });
        }
        self.save_state(&next)?;
        *state = next;
        Ok(Some(archived))
    }

    pub fn is_pinned(&self, state: &PersistedState, locator: &StoveIdentity) -> bool {
        state
            .pinned
            .iter()
            .any(|pinned| &pinned.session.locator == locator)
    }

    pub fn is_archived(&self, state: &PersistedState, locator: &StoveIdentity) -> bool {
        state
            .archived
            .iter()
            .any(|archived| &archived.session.locator == locator)
    }

    pub fn archive_snapshot(&self, state: &PersistedState) -> Vec<ArchivedSession> {
        let mut archived = state.archived.clone();
        archived.sort_by_key(|entry| std::cmp::Reverse(entry.archived_at_ms));
        archived
    }

    /// Imports an explicit, bounded metadata inventory in one atomic write.
    /// Existing manual tombstones, pins, and retained Cooked sessions win.
    pub fn archive_expired_sessions(
        &self,
        state: &mut PersistedState,
        sessions: impl IntoIterator<Item = SessionRecord>,
        archived_at_ms: u64,
    ) -> Result<usize, PersistenceError> {
        let mut next = state.clone();
        let mut added = 0usize;
        for session in sessions {
            if !session.is_valid()
                || session.last_state == StoveState::Cooked
                || next
                    .retained
                    .iter()
                    .any(|retained| retained.locator == session.locator)
                || next
                    .pinned
                    .iter()
                    .any(|pinned| pinned.session.locator == session.locator)
                || next
                    .archived
                    .iter()
                    .any(|archived| archived.session.locator == session.locator)
                || next.clear_cursors.iter().any(|cursor| {
                    cursor.locator == session.locator
                        && cursor.timestamp_ms >= session.observed_at_ms
                })
                || next.archived.len() >= MAX_ARCHIVED_SESSIONS
            {
                continue;
            }
            next.tracked
                .retain(|tracked| tracked.locator != session.locator);
            next.archived.push(ArchivedSession {
                session,
                archived_at_ms,
                reason: ArchiveReason::Expired,
            });
            added = added.saturating_add(1);
        }
        if added == 0 {
            return Ok(0);
        }
        cap_archived(&mut next);
        self.save_state(&next)?;
        *state = next;
        Ok(added)
    }

    /// Incorporates one already-normalized state transition into Cookbench's
    /// retained view. Non-Cooked transitions relight the same native identity.
    pub fn persist_transition(
        &self,
        state: &mut PersistedState,
        locator: StoveIdentity,
        stove_state: StoveState,
        source_event: &EventMetadata,
        presentation_event: &EventMetadata,
    ) -> Result<(), PersistenceError> {
        self.persist_transition_with_presentation(
            state,
            locator,
            stove_state,
            source_event,
            presentation_event,
            RetainedStovePresentation::default(),
        )
    }

    /// Persists only the bounded project rendering data needed for an
    /// undiscovered retained Stove. Callers must not pass task/activity text.
    pub fn persist_transition_with_presentation(
        &self,
        state: &mut PersistedState,
        locator: StoveIdentity,
        stove_state: StoveState,
        source_event: &EventMetadata,
        presentation_event: &EventMetadata,
        presentation: RetainedStovePresentation,
    ) -> Result<(), PersistenceError> {
        if state.is_hidden(&locator, source_event) {
            return Ok(());
        }

        let previous = state.retained.clone();
        state
            .retained
            .retain(|retained| retained.locator != locator);
        if stove_state == StoveState::Cooked {
            state.retained.push(
                RetainedStove::with_presentation(
                    locator,
                    presentation_event.timestamp_ms,
                    presentation,
                )
                .with_completion_events(source_event.clone(), presentation_event.clone()),
            );
            cap_retained(state);
        }
        if state.retained == previous {
            Ok(())
        } else {
            self.save_state(state)
        }
    }

    /// Clears only Cookbench's retained presentation. The cursor prevents stale
    /// native replay from resurrecting completion, while a newer prompt relights.
    pub fn clear_cooked(
        &self,
        state: &mut PersistedState,
        locator: StoveIdentity,
        clear_event: &EventMetadata,
    ) -> Result<(), PersistenceError> {
        state
            .retained
            .retain(|retained| retained.locator != locator);
        state
            .pinned
            .retain(|pinned| pinned.session.locator != locator);
        state.tracked.retain(|tracked| tracked.locator != locator);
        state
            .archived
            .retain(|archived| archived.session.locator != locator);
        state
            .clear_cursors
            .retain(|cursor| cursor.locator != locator);
        state.clear_cursors.push(ClearCursor::new(
            locator,
            clear_event.sequence,
            clear_event.timestamp_ms,
        ));
        cap_clear_cursors(state);
        self.save_state(state)
    }

    /// Retained Cooked locators are restored only when no newly observed native
    /// event supersedes them. The result contains no transcript or task content.
    pub fn merge_retained_with_discovery(
        &self,
        state: &PersistedState,
        discovered: &[NativeSessionObservation],
    ) -> Vec<RestoredCookedStove> {
        state
            .retained
            .iter()
            .filter(|retained| {
                discovered.iter().all(|native| {
                    native.locator != retained.locator
                        || native.last_event.timestamp_ms <= retained.completed_at_ms
                })
            })
            .cloned()
            .map(|retained| RestoredCookedStove { retained })
            .collect()
    }
}

fn cap_retained(state: &mut PersistedState) {
    state
        .retained
        .sort_by_key(|retained| std::cmp::Reverse(retained.completed_at_ms));
    state.retained.truncate(MAX_RETAINED_STOVES);
}

fn cap_clear_cursors(state: &mut PersistedState) {
    state
        .clear_cursors
        .sort_by_key(|cursor| std::cmp::Reverse((cursor.sequence, cursor.timestamp_ms)));
    state.clear_cursors.truncate(MAX_CLEAR_CURSORS);
}

fn cap_archived(state: &mut PersistedState) {
    state.archived.sort_by_key(|archived| {
        (
            archived.reason != ArchiveReason::Manual,
            std::cmp::Reverse(archived.archived_at_ms),
        )
    });
    state.archived.truncate(MAX_ARCHIVED_SESSIONS);
}

fn cap_tracked(state: &mut PersistedState) {
    state
        .tracked
        .sort_by_key(|tracked| std::cmp::Reverse(tracked.observed_at_ms));
    state.tracked.truncate(MAX_TRACKED_SESSIONS);
}

fn error_kind(error: &PersistenceError) -> PersistenceErrorKind {
    match error {
        PersistenceError::Json(_) => PersistenceErrorKind::InvalidJson,
        PersistenceError::UnsupportedVersion { .. } => PersistenceErrorKind::UnsupportedVersion,
        PersistenceError::Io(_) => PersistenceErrorKind::Io,
    }
}
