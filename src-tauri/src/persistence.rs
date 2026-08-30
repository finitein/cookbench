//! Desktop persistence orchestration for Cookbench-owned presentation state.
//!
//! Native harness sessions remain authoritative. This service only stores
//! configuration, retained Cooked locators, and clear cursors in atomic JSON.

use std::path::Path;

use cookbench_core::{
    domain::{EventMetadata, StoveIdentity, StoveState},
    persistence::{
        AtomicJsonFile, ClearCursor, PersistedConfig, PersistedState, PersistenceError,
        RetainedStove, RetainedStovePresentation,
    },
};

const MAX_RETAINED_STOVES: usize = 1_024;
const MAX_CLEAR_CURSORS: usize = 1_024;

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

    /// Incorporates one already-normalized state transition into Cookbench's
    /// retained view. Non-Cooked transitions relight the same native identity.
    pub fn persist_transition(
        &self,
        state: &mut PersistedState,
        locator: StoveIdentity,
        stove_state: StoveState,
        event: &EventMetadata,
    ) -> Result<(), PersistenceError> {
        self.persist_transition_with_presentation(
            state,
            locator,
            stove_state,
            event,
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
        event: &EventMetadata,
        presentation: RetainedStovePresentation,
    ) -> Result<(), PersistenceError> {
        if state.is_hidden(&locator, event) {
            return Ok(());
        }

        let previous = state.retained.clone();
        state
            .retained
            .retain(|retained| retained.locator != locator);
        if stove_state == StoveState::Cooked {
            state.retained.push(RetainedStove::with_presentation(
                locator,
                event.timestamp_ms,
                presentation,
            ));
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

fn error_kind(error: &PersistenceError) -> PersistenceErrorKind {
    match error {
        PersistenceError::Json(_) => PersistenceErrorKind::InvalidJson,
        PersistenceError::UnsupportedVersion { .. } => PersistenceErrorKind::UnsupportedVersion,
        PersistenceError::Io(_) => PersistenceErrorKind::Io,
    }
}
