use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::{Mutex, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use cookbench_core::{
    domain::{
        EventKind, EventMetadata, EventSource, HarnessId, HostIdentity, HostKind, ProjectIdentity,
        Stove, StoveEvent, StoveIdentity, StoveState,
    },
    locator::SessionLocator,
    notifications::{DestinationId, NotificationContext, NotificationEventKind},
    persistence::{
        ArchiveReason, ArchivedSession, PersistedConfig, PersistedState, RetainedStovePresentation,
        SessionRecord,
    },
    state_machine,
};
use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::{events::StoveChange, persistence::DesktopPersistence};

pub struct AppState {
    pub stoves: StoveStore,
    persistence: Mutex<Option<PersistenceRuntime>>,
    apply_lock: Mutex<()>,
}

struct PersistenceRuntime {
    service: DesktopPersistence,
    config: PersistedConfig,
    state: PersistedState,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            stoves: StoveStore::default(),
            persistence: Mutex::new(None),
            apply_lock: Mutex::new(()),
        }
    }
}

/// Thread-safe, insertion-ordered application view over the pure core reducer.
/// Native files and adapters remain the source of all incoming observations.
#[derive(Default)]
pub struct StoveStore {
    inner: RwLock<StoreInner>,
}

#[derive(Default)]
struct StoreInner {
    revision: u64,
    next_order: u64,
    entries: HashMap<String, StoredStove>,
    locators: HashMap<String, SessionLocator>,
    source_cursors: HashMap<String, HashMap<EventSource, EventMetadata>>,
    pinned: HashSet<String>,
}

struct StoredStove {
    stove: Stove,
    locator_capability: LocatorCapability,
    summary: StoveSummary,
    order: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LocatorCapability {
    Available,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoveSnapshot {
    pub revision: u64,
    pub stoves: Vec<StoveWire>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchivedSessionWire {
    pub id: String,
    pub harness: HarnessWire,
    pub host: HostWire,
    pub project_label: String,
    pub project_root_display: String,
    pub session_identity: String,
    pub last_state: StoveStateWire,
    pub reason: ArchiveReasonWire,
    pub archived_at_ms: u64,
    pub source_available: bool,
    pub pinned: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArchiveReasonWire {
    Expired,
    Manual,
}

/// Bounded local presentation summaries. These fields are deliberately not a
/// transcript cache: adapters may only provide concise session metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoveSummary {
    pub project_label: String,
    pub project_root_display: String,
    pub task_title: Option<String>,
    pub current_action: Option<String>,
    pub next_action: Option<String>,
    pub elapsed_ms: Option<u64>,
    /// Native session file metadata used only for the 48-hour visibility rule.
    /// It is never serialized to the frontend.
    pub source_modified_at_ms: Option<u64>,
    /// Local receipt time for sources without filesystem metadata, including
    /// SSH. It is kept out of the frontend wire and used only for expiry.
    pub last_observed_at_ms: Option<u64>,
}

impl StoveSummary {
    const MAX_TEXT_BYTES: usize = 512;

    pub fn new(
        project_label: impl Into<String>,
        project_root_display: impl Into<String>,
        task_title: Option<String>,
        current_action: Option<String>,
        next_action: Option<String>,
        elapsed_ms: Option<u64>,
    ) -> Self {
        Self {
            project_label: bounded(project_label.into()),
            project_root_display: bounded(project_root_display.into()),
            task_title: task_title.map(bounded),
            current_action: current_action.map(bounded),
            next_action: next_action.map(bounded),
            elapsed_ms,
            source_modified_at_ms: None,
            last_observed_at_ms: None,
        }
    }

    pub fn with_source_modified_at_ms(mut self, modified_at_ms: Option<u64>) -> Self {
        self.source_modified_at_ms = modified_at_ms;
        self
    }

    pub fn with_last_observed_at_ms(mut self, observed_at_ms: Option<u64>) -> Self {
        self.last_observed_at_ms = observed_at_ms;
        self
    }

    fn for_project(project: &ProjectIdentity) -> Self {
        let display = bounded(project.canonical_root.clone());
        let label = display
            .rsplit(['/', '\\'])
            .find(|part| !part.is_empty())
            .unwrap_or("Project")
            .to_owned();
        Self::new(label, display, None, None, None, None)
    }
}

/// Sanitized frontend wire model. It intentionally contains no transcript,
/// prompt, code, command, credential, or raw native session locator.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoveWire {
    pub id: String,
    pub harness: HarnessWire,
    pub host: HostWire,
    /// Compatibility display alias used by the first bar implementation.
    pub project_root: String,
    pub project_label: String,
    pub project_root_display: String,
    pub task_title: Option<String>,
    pub current_action: Option<String>,
    pub next_action: Option<String>,
    pub elapsed_ms: Option<u64>,
    pub state: StoveStateWire,
    pub progress: Option<ProgressWire>,
    pub locator_capability: LocatorCapability,
    pub retained_completion: bool,
    pub pinned: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HarnessWire {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostWire {
    pub kind: HostKindWire,
    pub id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HostKindWire {
    Local,
    Ssh,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StoveStateWire {
    Starting,
    Planning,
    Cooking,
    NeedsHuman,
    Cooked,
    Failed,
    Disconnected,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressWire {
    pub completed: u32,
    pub total: u32,
    pub provenance: ProgressProvenanceWire,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProgressProvenanceWire {
    StructuredSession,
    Hook,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StoreError {
    Transition(state_machine::TransitionError),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transition(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for StoreError {}

impl AppState {
    pub fn initialize_persistence(&self, app_data_directory: &Path) {
        let service = DesktopPersistence::in_app_data(app_data_directory);
        let mut loaded = service.load();
        let cutoff = current_time_ms().saturating_sub(SESSION_VISIBILITY_MS);
        let pinned_identities = loaded
            .state
            .pinned
            .iter()
            .map(|pinned| pinned.session.locator.clone())
            .collect::<HashSet<_>>();
        for tracked in loaded.state.tracked.clone() {
            if tracked.last_state != StoveState::Cooked
                && tracked.observed_at_ms < cutoff
                && !pinned_identities.contains(&tracked.locator)
            {
                let _ = service.archive_session(
                    &mut loaded.state,
                    tracked,
                    current_time_ms(),
                    ArchiveReason::Expired,
                );
            }
        }
        let retained = loaded.state.retained.clone();
        let pinned = loaded.state.pinned.clone();
        *self
            .persistence
            .lock()
            .expect("desktop persistence lock poisoned") = Some(PersistenceRuntime {
            service,
            config: loaded.config,
            state: loaded.state,
        });
        for completion in retained {
            let project_root = if completion.presentation.project_root_display.is_empty() {
                "(retained Cookbench completion)".to_owned()
            } else {
                completion.presentation.project_root_display.clone()
            };
            let project_label = if completion.presentation.project_label.is_empty() {
                "Retained project".to_owned()
            } else {
                completion.presentation.project_label
            };
            let project =
                ProjectIdentity::new(completion.locator.host.clone(), project_root.clone());
            let _ = self.stoves.apply_observation(
                completion.locator,
                project,
                LocatorCapability::Unavailable,
                None,
                Some(StoveSummary::new(
                    project_label,
                    project_root,
                    None,
                    None,
                    None,
                    None,
                )),
                StoveEvent::new(
                    EventKind::TurnCompleted,
                    EventMetadata::new(
                        EventSource::StructuredSession,
                        100,
                        0,
                        completion.completed_at_ms,
                    ),
                ),
            );
        }
        for pinned_session in pinned {
            if pinned_session.session.is_valid() {
                self.restore_session_record(&pinned_session.session, true);
            }
        }
    }

    pub fn pinned_local_paths(&self) -> Vec<std::path::PathBuf> {
        self.persistence
            .lock()
            .expect("desktop persistence lock poisoned")
            .as_ref()
            .map(|runtime| {
                runtime
                    .state
                    .pinned
                    .iter()
                    .filter(|entry| entry.session.locator.host.kind == HostKind::Local)
                    .filter_map(|entry| entry.session.native_locator.as_ref())
                    .map(std::path::PathBuf::from)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn persisted_config(&self) -> PersistedConfig {
        self.persistence
            .lock()
            .expect("desktop persistence lock poisoned")
            .as_ref()
            .map(|runtime| runtime.config.clone())
            .unwrap_or_default()
    }

    pub fn update_persisted_config(
        &self,
        update: impl FnOnce(&mut PersistedConfig),
    ) -> Result<(), String> {
        let mut guard = self
            .persistence
            .lock()
            .map_err(|_| "desktop persistence is unavailable".to_owned())?;
        let Some(runtime) = guard.as_mut() else {
            return Err("desktop persistence is not initialized".to_owned());
        };
        update(&mut runtime.config);
        runtime
            .service
            .save_config(&runtime.config)
            .map_err(|error| error.to_string())
    }

    pub fn set_pinned_and_emit<R: tauri::Runtime>(
        &self,
        app: &tauri::AppHandle<R>,
        requested_stove_id: &str,
        pinned: bool,
    ) -> Result<(), AppStateError> {
        let _serial = self.apply_lock.lock().expect("stove apply lock poisoned");
        let record = self.session_record_for(requested_stove_id)?;
        {
            let mut persistence = self
                .persistence
                .lock()
                .expect("desktop persistence lock poisoned");
            let runtime = persistence
                .as_mut()
                .ok_or_else(|| AppStateError::Persistence("persistence is unavailable".into()))?;
            if pinned {
                if !runtime
                    .service
                    .pin_session(&mut runtime.state, record.clone(), current_time_ms())
                    .map_err(|error| AppStateError::Persistence(error.to_string()))?
                {
                    return Err(AppStateError::Persistence(
                        "the pinned session list is full".into(),
                    ));
                }
            } else {
                runtime
                    .service
                    .unpin_session(&mut runtime.state, &record.locator)
                    .map_err(|error| AppStateError::Persistence(error.to_string()))?;
                if record.last_state != StoveState::Cooked {
                    runtime
                        .service
                        .track_session(&mut runtime.state, record.clone())
                        .map_err(|error| AppStateError::Persistence(error.to_string()))?;
                }
            }
        }
        let change = self
            .stoves
            .set_pinned(requested_stove_id, pinned)
            .ok_or(AppStateError::UnknownStove)?;
        crate::events::emit_stove_change(app, change).map_err(AppStateError::Emit)?;
        crate::platform::publish_optional_gnome_snapshot(&self.stoves.snapshot());
        drop(_serial);
        if !pinned {
            self.reconcile_expired_and_emit(app)?;
        }
        Ok(())
    }

    pub fn archive_stove_and_emit<R: tauri::Runtime>(
        &self,
        app: &tauri::AppHandle<R>,
        stove_id: &str,
    ) -> Result<(), AppStateError> {
        self.archive_stove_with_reason(app, stove_id, ArchiveReason::Manual)
    }

    pub fn archived_sessions(&self) -> Vec<ArchivedSessionWire> {
        let persistence = self
            .persistence
            .lock()
            .expect("desktop persistence lock poisoned");
        let Some(runtime) = persistence.as_ref() else {
            return Vec::new();
        };
        runtime
            .service
            .archive_snapshot(&runtime.state)
            .iter()
            .filter(|archived| archived.session.is_valid())
            .map(ArchivedSessionWire::from_archived)
            .collect()
    }

    pub fn import_expired_sessions(
        &self,
        sessions: Vec<SessionRecord>,
    ) -> Result<usize, AppStateError> {
        let _serial = self.apply_lock.lock().expect("stove apply lock poisoned");
        let mut persistence = self
            .persistence
            .lock()
            .expect("desktop persistence lock poisoned");
        let runtime = persistence
            .as_mut()
            .ok_or_else(|| AppStateError::Persistence("persistence is unavailable".into()))?;
        runtime
            .service
            .archive_expired_sessions(&mut runtime.state, sessions, current_time_ms())
            .map_err(|error| AppStateError::Persistence(error.to_string()))
    }

    pub fn restore_archived_and_emit<R: tauri::Runtime>(
        &self,
        app: &tauri::AppHandle<R>,
        requested_stove_id: &str,
    ) -> Result<(), AppStateError> {
        let _serial = self.apply_lock.lock().expect("stove apply lock poisoned");
        let archived = {
            let persistence = self
                .persistence
                .lock()
                .expect("desktop persistence lock poisoned");
            let runtime = persistence
                .as_ref()
                .ok_or_else(|| AppStateError::Persistence("persistence is unavailable".into()))?;
            runtime
                .service
                .archive_snapshot(&runtime.state)
                .into_iter()
                .find(|entry| stove_id(&entry.session.locator) == requested_stove_id)
                .ok_or(AppStateError::UnknownStove)?
        };
        if !source_available(&archived) {
            return Err(AppStateError::Persistence(
                "the native session source is unavailable".into(),
            ));
        }
        let should_pin = archived.session.observed_at_ms
            < current_time_ms().saturating_sub(SESSION_VISIBILITY_MS);
        {
            let mut persistence = self
                .persistence
                .lock()
                .expect("desktop persistence lock poisoned");
            let runtime = persistence
                .as_mut()
                .ok_or_else(|| AppStateError::Persistence("persistence is unavailable".into()))?;
            runtime
                .service
                .restore_session(
                    &mut runtime.state,
                    &archived.session.locator,
                    should_pin,
                    current_time_ms(),
                )
                .map_err(|error| AppStateError::Persistence(error.to_string()))?
                .ok_or(AppStateError::UnknownStove)?;
        }
        let change = self.restore_session_record(&archived.session, should_pin);
        if let Some(change) = change {
            crate::events::emit_stove_change(app, change).map_err(AppStateError::Emit)?;
        }
        crate::platform::publish_optional_gnome_snapshot(&self.stoves.snapshot());
        Ok(())
    }

    pub fn reconcile_expired_and_emit<R: tauri::Runtime>(
        &self,
        app: &tauri::AppHandle<R>,
    ) -> Result<(), AppStateError> {
        let cutoff = current_time_ms().saturating_sub(SESSION_VISIBILITY_MS);
        for stove_id in self.stoves.expiration_candidates(cutoff) {
            self.archive_stove_with_reason(app, &stove_id, ArchiveReason::Expired)?;
        }
        Ok(())
    }

    fn archive_stove_with_reason<R: tauri::Runtime>(
        &self,
        app: &tauri::AppHandle<R>,
        stove_id: &str,
        reason: ArchiveReason,
    ) -> Result<(), AppStateError> {
        let _serial = self.apply_lock.lock().expect("stove apply lock poisoned");
        let stove = self
            .stoves
            .core_stove(stove_id)
            .ok_or(AppStateError::UnknownStove)?;
        if reason == ArchiveReason::Expired && self.stoves.is_pinned(stove_id) {
            return Ok(());
        }
        if stove.state == StoveState::Cooked {
            return Err(AppStateError::CannotArchiveCooked);
        }
        let record = self.session_record_for(stove_id)?;
        {
            let mut persistence = self
                .persistence
                .lock()
                .expect("desktop persistence lock poisoned");
            let runtime = persistence
                .as_mut()
                .ok_or_else(|| AppStateError::Persistence("persistence is unavailable".into()))?;
            if !runtime
                .service
                .archive_session(&mut runtime.state, record, current_time_ms(), reason)
                .map_err(|error| AppStateError::Persistence(error.to_string()))?
            {
                return Err(AppStateError::Persistence(
                    "the session archive is full".into(),
                ));
            }
        }
        if let Some(change) = self.stoves.remove_presentation(stove_id) {
            crate::events::emit_stove_change(app, change).map_err(AppStateError::Emit)?;
        }
        if let Some(remote) = app.try_state::<crate::remote::runtime::RemoteRuntimeState>() {
            remote.forget(stove.identity);
        }
        if let Some(windows) =
            app.try_state::<crate::commands::windows::TauriWindowCommandService>()
        {
            let _ = windows.clear_stove(stove_id);
            let _ = crate::commands::windows::persist_layouts(self, &windows);
        }
        crate::platform::publish_optional_gnome_snapshot(&self.stoves.snapshot());
        Ok(())
    }

    fn session_record_for(&self, stove_id: &str) -> Result<SessionRecord, AppStateError> {
        let stove = self
            .stoves
            .core_stove(stove_id)
            .ok_or(AppStateError::UnknownStove)?;
        let summary = self
            .stoves
            .summary_for_identity(&stove.identity)
            .unwrap_or_else(|| StoveSummary::for_project(&stove.project));
        let native_locator = self
            .stoves
            .locator_for(stove_id)
            .and_then(|locator| locator.native_locator);
        SessionRecord::new(
            stove.identity,
            native_locator,
            latest_observed_at(&summary)
                .or_else(|| {
                    stove
                        .last_event
                        .as_ref()
                        .and_then(|event| comparable_epoch(event.timestamp_ms))
                })
                .unwrap_or_else(current_time_ms),
            RetainedStovePresentation::new(summary.project_label, summary.project_root_display),
            stove.state,
        )
        .ok_or_else(|| AppStateError::Persistence("the native session locator is unsafe".into()))
    }

    fn restore_session_record(&self, record: &SessionRecord, pinned: bool) -> Option<StoveChange> {
        let project_root = if record.presentation.project_root_display.is_empty() {
            "(restored session)".to_owned()
        } else {
            record.presentation.project_root_display.clone()
        };
        let project_label = if record.presentation.project_label.is_empty() {
            "Restored session".to_owned()
        } else {
            record.presentation.project_label.clone()
        };
        let project = ProjectIdentity::new(record.locator.host.clone(), project_root.clone());
        let locator = SessionLocator {
            native_locator: record.native_locator.clone(),
            working_directory: Some(project_root.clone()),
            native_session_id: record.locator.native_session_id.clone(),
            ..SessionLocator::default()
        };
        let mut change = self
            .stoves
            .apply_observation(
                record.locator.clone(),
                project,
                if record.native_locator.is_some() {
                    LocatorCapability::Available
                } else {
                    LocatorCapability::Unavailable
                },
                Some(locator),
                Some(
                    StoveSummary::new(project_label, project_root, None, None, None, None)
                        .with_source_modified_at_ms(Some(record.observed_at_ms))
                        .with_last_observed_at_ms(Some(record.observed_at_ms)),
                ),
                StoveEvent::new(
                    event_kind_for_state(record.last_state),
                    EventMetadata::new(EventSource::Inference, 100, 0, record.observed_at_ms),
                ),
            )
            .ok();
        if pinned {
            change = self
                .stoves
                .set_pinned(&stove_id(&record.locator), true)
                .or(change);
        }
        change
    }

    /// Records one adapter observation, then broadcasts its revisioned result.
    /// This is notification-only: it cannot start, stop, or control a harness.
    pub fn apply_and_emit<R: tauri::Runtime>(
        &self,
        app: &tauri::AppHandle<R>,
        identity: StoveIdentity,
        project: ProjectIdentity,
        locator_capability: LocatorCapability,
        summary: Option<StoveSummary>,
        event: StoveEvent,
    ) -> Result<(), AppStateError> {
        self.apply_observation_and_emit(
            app,
            identity,
            project,
            locator_capability,
            None,
            summary,
            event,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn apply_observation_and_emit<R: tauri::Runtime>(
        &self,
        app: &tauri::AppHandle<R>,
        identity: StoveIdentity,
        project: ProjectIdentity,
        locator_capability: LocatorCapability,
        locator: Option<SessionLocator>,
        summary: Option<StoveSummary>,
        event: StoveEvent,
    ) -> Result<(), AppStateError> {
        self.apply_observation_and_emit_inner(
            app,
            identity,
            project,
            locator_capability,
            locator,
            summary,
            event,
            true,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn apply_replay_observation_and_emit<R: tauri::Runtime>(
        &self,
        app: &tauri::AppHandle<R>,
        identity: StoveIdentity,
        project: ProjectIdentity,
        locator_capability: LocatorCapability,
        locator: Option<SessionLocator>,
        summary: Option<StoveSummary>,
        event: StoveEvent,
    ) -> Result<(), AppStateError> {
        self.apply_observation_and_emit_inner(
            app,
            identity,
            project,
            locator_capability,
            locator,
            summary,
            event,
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_observation_and_emit_inner<R: tauri::Runtime>(
        &self,
        app: &tauri::AppHandle<R>,
        identity: StoveIdentity,
        project: ProjectIdentity,
        locator_capability: LocatorCapability,
        locator: Option<SessionLocator>,
        mut summary: Option<StoveSummary>,
        mut event: StoveEvent,
        side_effects: bool,
    ) -> Result<(), AppStateError> {
        let _serial = self.apply_lock.lock().expect("stove apply lock poisoned");
        {
            let persistence = self
                .persistence
                .lock()
                .expect("desktop persistence lock poisoned");
            if let Some(runtime) = persistence.as_ref() {
                if runtime.service.is_archived(&runtime.state, &identity) {
                    return Ok(());
                }
                if !matches!(event.kind, EventKind::ClearRequested)
                    && runtime.state.is_hidden(&identity, &event.metadata)
                {
                    return Ok(());
                }
                if matches!(event.kind, EventKind::SessionDiscovered) {
                    if let Some(retained) = runtime
                        .state
                        .retained
                        .iter()
                        .find(|retained| retained.locator == identity)
                    {
                        event = StoveEvent::new(
                            EventKind::TurnCompleted,
                            EventMetadata::new(
                                EventSource::StructuredSession,
                                100,
                                event.metadata.sequence,
                                retained.completed_at_ms,
                            ),
                        );
                    }
                }
            }
        }

        // A superseded observation is strictly inert. In particular, it must
        // not wake persistence, notification, or frontend event consumers.
        if self.stoves.is_superseded(&identity, &event.metadata) {
            return Ok(());
        }

        let identity_for_persistence = identity.clone();
        let identity_for_notification = identity.clone();
        let previous_progress = self
            .stoves
            .core_stove_for_identity(&identity_for_notification)
            .and_then(|stove| progress_percent(stove.progress.as_ref()));
        let observed_kind = event.kind.clone();
        let metadata = event.metadata.clone();
        if summary.is_none() {
            summary = self.stoves.summary_for_identity(&identity_for_persistence);
        }
        if let Some(summary) = summary.as_mut() {
            summary.last_observed_at_ms = Some(current_time_ms());
        }
        let change = self.stoves.apply_observation(
            identity,
            project,
            locator_capability,
            locator,
            summary,
            event,
        )?;
        if side_effects {
            if let Some(stove) = self
                .stoves
                .core_stove_for_identity(&identity_for_persistence)
            {
                if let Some(runtime) = self
                    .persistence
                    .lock()
                    .expect("desktop persistence lock poisoned")
                    .as_mut()
                {
                    let summary = self
                        .stoves
                        .summary_for_identity(&identity_for_persistence)
                        .unwrap_or_else(|| StoveSummary::for_project(&stove.project));
                    let observed_at_ms =
                        latest_observed_at(&summary).unwrap_or_else(current_time_ms);
                    let presentation = RetainedStovePresentation::new(
                        summary.project_label,
                        summary.project_root_display,
                    );
                    let _ = runtime.service.persist_transition_with_presentation(
                        &mut runtime.state,
                        identity_for_persistence.clone(),
                        stove.state,
                        &metadata,
                        presentation.clone(),
                    );
                    if stove.state == StoveState::Cooked {
                        let _ = runtime
                            .service
                            .remove_tracked(&mut runtime.state, &identity_for_persistence);
                    } else if let Some(record) = SessionRecord::new(
                        identity_for_persistence.clone(),
                        self.stoves
                            .locator_for(&stove_id(&identity_for_persistence))
                            .and_then(|locator| locator.native_locator),
                        observed_at_ms,
                        presentation,
                        stove.state,
                    ) {
                        if runtime
                            .service
                            .is_pinned(&runtime.state, &identity_for_persistence)
                        {
                            if let Some(pinned) =
                                runtime.state.pinned.iter_mut().find(|pinned| {
                                    pinned.session.locator == identity_for_persistence
                                })
                            {
                                let same_metadata = pinned.session.native_locator
                                    == record.native_locator
                                    && pinned.session.presentation == record.presentation
                                    && pinned.session.last_state == record.last_state;
                                if !same_metadata
                                    || record
                                        .observed_at_ms
                                        .saturating_sub(pinned.session.observed_at_ms)
                                        >= 60_000
                                {
                                    pinned.session = record;
                                    let _ = runtime.service.save_state(&runtime.state);
                                }
                            }
                        } else {
                            let _ = runtime.service.track_session(&mut runtime.state, record);
                        }
                    }
                }
            }
        }
        crate::events::emit_stove_change(app, change).map_err(AppStateError::Emit)?;
        crate::platform::publish_optional_gnome_snapshot(&self.stoves.snapshot());
        if side_effects {
            if let Some(stove) = self
                .stoves
                .core_stove_for_identity(&identity_for_notification)
            {
                let Some(kind) = notification_event(&observed_kind, stove.state) else {
                    return Ok(());
                };
                let summary = self
                    .stoves
                    .summary_for_identity(&identity_for_notification)
                    .unwrap_or_else(|| StoveSummary::for_project(&stove.project));
                self.enqueue_notification(app, &stove, summary.clone(), kind, None);
                if matches!(observed_kind, EventKind::PlanUpdated { .. }) {
                    let current_progress = progress_percent(stove.progress.as_ref());
                    if let Some(milestone) = crossed_milestone(previous_progress, current_progress)
                    {
                        self.enqueue_notification(
                            app,
                            &stove,
                            summary,
                            NotificationEventKind::ProgressMilestone,
                            Some(milestone),
                        );
                    }
                }
            }
        }
        Ok(())
    }

    pub fn clear_cooked_and_emit<R: tauri::Runtime>(
        &self,
        app: &tauri::AppHandle<R>,
        stove_id: &str,
    ) -> Result<(), AppStateError> {
        let Some(stove) = self.stoves.core_stove(stove_id) else {
            return Err(AppStateError::UnknownStove);
        };
        if stove.state != StoveState::Cooked {
            return Err(
                StoreError::Transition(state_machine::TransitionError::CannotClear(stove.state))
                    .into(),
            );
        }
        let previous = self
            .stoves
            .source_cursor(&stove.identity, EventSource::StructuredSession)
            .or_else(|| {
                self.stoves
                    .source_cursor(&stove.identity, EventSource::Hook)
            })
            .or_else(|| stove.last_event.clone())
            .unwrap_or_else(|| EventMetadata::new(EventSource::Inference, 0, 0, current_time_ms()));
        let clear = EventMetadata::new(
            EventSource::Inference,
            100,
            previous.sequence.saturating_add(1),
            current_time_ms().max(previous.timestamp_ms.saturating_add(1)),
        );
        {
            let mut persistence = self
                .persistence
                .lock()
                .expect("desktop persistence lock poisoned");
            if let Some(runtime) = persistence.as_mut() {
                runtime
                    .service
                    .clear_cooked(&mut runtime.state, stove.identity.clone(), &previous)
                    .map_err(|error| AppStateError::Persistence(error.to_string()))?;
            }
        }
        let notification_stove = stove.clone();
        let notification_summary = self
            .stoves
            .summary_for_identity(&stove.identity)
            .unwrap_or_else(|| StoveSummary::for_project(&stove.project));
        self.apply_observation_and_emit(
            app,
            stove.identity,
            stove.project,
            LocatorCapability::Unavailable,
            None,
            None,
            StoveEvent::new(EventKind::ClearRequested, clear),
        )?;
        self.enqueue_notification(
            app,
            &notification_stove,
            notification_summary,
            NotificationEventKind::StoveCleared,
            None,
        );
        Ok(())
    }

    fn enqueue_notification<R: tauri::Runtime>(
        &self,
        app: &tauri::AppHandle<R>,
        stove: &Stove,
        summary: StoveSummary,
        event: NotificationEventKind,
        milestone: Option<u8>,
    ) {
        // Source timestamps describe harness ordering, not local queue age.
        // Using them here could expire a newly observed event.
        let now_ms = current_time_ms();
        if let Some(local) =
            app.try_state::<crate::commands::notifications::LocalAlertCommandState>()
        {
            let config = self.persisted_config();
            let preferences = config.preferences.local_notifications;
            let locale = app
                .try_state::<crate::i18n::NativeLocaleState>()
                .map(|state| state.current())
                .unwrap_or_else(|| crate::i18n::resolve_locale(config.preferences.locale));
            let payload = crate::notifications::local::LocalAlertPayload::new(
                stove_id(&stove.identity),
                &summary.project_label,
                event,
            )
            .with_locale(locale);
            let effects = crate::notifications::local::TauriLocalAlertEffects::new(app);
            let _ = local.0.dispatch(&preferences, &payload, now_ms, &effects);
        }

        let Some(notifications) =
            app.try_state::<crate::commands::notifications::NotificationCommandState>()
        else {
            return;
        };
        notifications.0.enqueue_transition(
            &NotificationContext {
                stove_id: stove_id(&stove.identity),
                project: summary.project_label,
                host: stove.identity.host.clone(),
                harness: stove.identity.harness.clone(),
                destination: DestinationId::new("pending"),
                state: stove.state,
                event,
                progress_percent: progress_percent(stove.progress.as_ref()),
                milestone,
                task: summary.task_title,
                agent: None,
                activity: summary.current_action,
                duration: summary
                    .elapsed_ms
                    .map(|elapsed| format!("{}s", elapsed / 1_000)),
                completed_at: None,
            },
            now_ms,
        );
        notifications.0.request_flush(now_ms);
    }
}

#[derive(Debug)]
pub enum AppStateError {
    Store(StoreError),
    Emit(tauri::Error),
    Persistence(String),
    UnknownStove,
    CannotArchiveCooked,
}

impl std::fmt::Display for AppStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Store(error) => error.fmt(f),
            Self::Emit(error) => error.fmt(f),
            Self::Persistence(error) => f.write_str(error),
            Self::UnknownStove => f.write_str("Cookbench does not have that Stove"),
            Self::CannotArchiveCooked => f.write_str("Cooked Stoves use Clear instead of Archive"),
        }
    }
}
impl std::error::Error for AppStateError {}

impl From<StoreError> for AppStateError {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}

impl StoveStore {
    pub fn apply(
        &self,
        identity: StoveIdentity,
        project: ProjectIdentity,
        locator_capability: LocatorCapability,
        event: StoveEvent,
    ) -> Result<StoveChange, StoreError> {
        self.apply_with_summary(identity, project, locator_capability, None, event)
    }

    pub fn apply_with_summary(
        &self,
        identity: StoveIdentity,
        project: ProjectIdentity,
        locator_capability: LocatorCapability,
        summary: Option<StoveSummary>,
        event: StoveEvent,
    ) -> Result<StoveChange, StoreError> {
        self.apply_observation(identity, project, locator_capability, None, summary, event)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn apply_observation(
        &self,
        identity: StoveIdentity,
        project: ProjectIdentity,
        locator_capability: LocatorCapability,
        locator: Option<SessionLocator>,
        summary: Option<StoveSummary>,
        event: StoveEvent,
    ) -> Result<StoveChange, StoreError> {
        let id = stove_id(&identity);
        let mut inner = self.inner.write().expect("stove store lock poisoned");
        let source_cursor = inner
            .source_cursors
            .get(&id)
            .and_then(|cursors| cursors.get(&event.metadata.source));
        if is_superseded(source_cursor, &event.metadata)
            || inner.entries.get(&id).is_some_and(|entry| {
                cross_source_event_is_older(entry.stove.last_event.as_ref(), &event.metadata)
            })
        {
            if let Some(entry) = inner.entries.get(&id) {
                return Ok(StoveChange::upsert(
                    inner.revision,
                    StoveWire::from_stored(
                        &id,
                        &entry.stove,
                        entry.locator_capability,
                        &entry.summary,
                        inner.pinned.contains(&id),
                    ),
                ));
            }
        }
        let raw_metadata = event.metadata.clone();
        let existing = inner.entries.remove(&id);
        let (previous, order, existing_locator, existing_summary) = match existing.as_ref() {
            Some(entry) => (
                entry.stove.clone(),
                entry.order,
                entry.locator_capability,
                entry.summary.clone(),
            ),
            None => {
                let order = inner.next_order;
                inner.next_order += 1;
                (
                    Stove::new(identity.clone(), project.clone()),
                    order,
                    locator_capability,
                    StoveSummary::for_project(&project),
                )
            }
        };
        let mut normalized_event = event;
        normalized_event.metadata.sequence = previous
            .last_event
            .as_ref()
            .map_or(1, |metadata| metadata.sequence.saturating_add(1));
        let next = match state_machine::reduce(&previous, &normalized_event) {
            Ok(next) => next,
            Err(error) => {
                if let Some(entry) = existing {
                    inner.entries.insert(id, entry);
                }
                return Err(StoreError::Transition(error));
            }
        };
        inner.revision += 1;
        let revision = inner.revision;

        if next.state == StoveState::Removed {
            inner.locators.remove(&id);
            inner.source_cursors.remove(&id);
            inner.pinned.remove(&id);
            return Ok(StoveChange::remove(revision, id));
        }

        inner
            .source_cursors
            .entry(id.clone())
            .or_default()
            .insert(raw_metadata.source, raw_metadata);

        let valid_locator = locator
            .filter(|locator| locator.validate().is_ok())
            .map(|locator| merge_locator(inner.locators.get(&id), locator));
        if let Some(locator) = valid_locator {
            inner.locators.insert(id.clone(), locator);
        }
        let capability = if inner.locators.contains_key(&id)
            || matches!(locator_capability, LocatorCapability::Available)
        {
            LocatorCapability::Available
        } else {
            existing_locator
        };
        let summary = summary.unwrap_or(existing_summary);
        let wire =
            StoveWire::from_stored(&id, &next, capability, &summary, inner.pinned.contains(&id));
        inner.entries.insert(
            id,
            StoredStove {
                stove: next,
                locator_capability: capability,
                summary,
                order,
            },
        );
        Ok(StoveChange::upsert(revision, wire))
    }

    pub fn locator_for(&self, stove_id: &str) -> Option<SessionLocator> {
        self.inner
            .read()
            .expect("stove store lock poisoned")
            .locators
            .get(stove_id)
            .cloned()
    }

    pub fn set_pinned(&self, stove_id: &str, pinned: bool) -> Option<StoveChange> {
        let mut inner = self.inner.write().expect("stove store lock poisoned");
        if !inner.entries.contains_key(stove_id) {
            return None;
        }
        if pinned {
            inner.pinned.insert(stove_id.to_owned());
        } else {
            inner.pinned.remove(stove_id);
        }
        inner.revision = inner.revision.saturating_add(1);
        let revision = inner.revision;
        let entry = inner.entries.get(stove_id).expect("entry checked above");
        Some(StoveChange::upsert(
            revision,
            StoveWire::from_stored(
                stove_id,
                &entry.stove,
                entry.locator_capability,
                &entry.summary,
                pinned,
            ),
        ))
    }

    fn is_pinned(&self, stove_id: &str) -> bool {
        self.inner
            .read()
            .expect("stove store lock poisoned")
            .pinned
            .contains(stove_id)
    }

    pub fn remove_presentation(&self, stove_id: &str) -> Option<StoveChange> {
        let mut inner = self.inner.write().expect("stove store lock poisoned");
        inner.entries.remove(stove_id)?;
        inner.locators.remove(stove_id);
        inner.source_cursors.remove(stove_id);
        inner.pinned.remove(stove_id);
        inner.revision = inner.revision.saturating_add(1);
        Some(StoveChange::remove(inner.revision, stove_id.to_owned()))
    }

    fn expiration_candidates(&self, cutoff_ms: u64) -> Vec<String> {
        let inner = self.inner.read().expect("stove store lock poisoned");
        inner
            .entries
            .iter()
            .filter(|(id, entry)| {
                entry.stove.state != StoveState::Cooked
                    && !inner.pinned.contains(*id)
                    && latest_observed_at(&entry.summary)
                        .is_some_and(|observed| observed < cutoff_ms)
            })
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn contains_identity(&self, identity: &StoveIdentity) -> bool {
        self.inner
            .read()
            .expect("stove store lock poisoned")
            .entries
            .contains_key(&stove_id(identity))
    }

    pub fn core_stove(&self, stove_id: &str) -> Option<Stove> {
        self.inner
            .read()
            .expect("stove store lock poisoned")
            .entries
            .get(stove_id)
            .map(|stored| stored.stove.clone())
    }

    fn core_stove_for_identity(&self, identity: &StoveIdentity) -> Option<Stove> {
        self.core_stove(&stove_id(identity))
    }

    fn summary_for_identity(&self, identity: &StoveIdentity) -> Option<StoveSummary> {
        self.inner
            .read()
            .expect("stove store lock poisoned")
            .entries
            .get(&stove_id(identity))
            .map(|stored| stored.summary.clone())
    }

    fn is_superseded(&self, identity: &StoveIdentity, incoming: &EventMetadata) -> bool {
        let inner = self.inner.read().expect("stove store lock poisoned");
        let id = stove_id(identity);
        is_superseded(
            inner
                .source_cursors
                .get(&id)
                .and_then(|cursors| cursors.get(&incoming.source)),
            incoming,
        ) || inner.entries.get(&id).is_some_and(|entry| {
            cross_source_event_is_older(entry.stove.last_event.as_ref(), incoming)
        })
    }

    fn source_cursor(
        &self,
        identity: &StoveIdentity,
        source: EventSource,
    ) -> Option<EventMetadata> {
        self.inner
            .read()
            .expect("stove store lock poisoned")
            .source_cursors
            .get(&stove_id(identity))
            .and_then(|cursors| cursors.get(&source))
            .cloned()
    }

    pub fn snapshot(&self) -> StoveSnapshot {
        let inner = self.inner.read().expect("stove store lock poisoned");
        let mut entries = inner.entries.iter().collect::<Vec<_>>();
        entries.sort_by_key(|(_, entry)| entry.order);
        StoveSnapshot {
            revision: inner.revision,
            stoves: entries
                .into_iter()
                .map(|(id, entry)| {
                    StoveWire::from_stored(
                        id,
                        &entry.stove,
                        entry.locator_capability,
                        &entry.summary,
                        inner.pinned.contains(id),
                    )
                })
                .collect(),
        }
    }
}

fn merge_locator(existing: Option<&SessionLocator>, incoming: SessionLocator) -> SessionLocator {
    let Some(existing) = existing else {
        return incoming;
    };
    SessionLocator {
        native_locator: incoming
            .native_locator
            .or_else(|| existing.native_locator.clone()),
        process_id: incoming.process_id.or(existing.process_id),
        parent_process_id: incoming.parent_process_id.or(existing.parent_process_id),
        process_started_at_ms: incoming
            .process_started_at_ms
            .or(existing.process_started_at_ms),
        working_directory: incoming
            .working_directory
            .or_else(|| existing.working_directory.clone()),
        host_application: incoming
            .host_application
            .or_else(|| existing.host_application.clone()),
        terminal: incoming.terminal.or_else(|| existing.terminal.clone()),
        tty: incoming.tty.or_else(|| existing.tty.clone()),
        tmux_pane: incoming.tmux_pane.or_else(|| existing.tmux_pane.clone()),
        tmux_inner_pane: incoming
            .tmux_inner_pane
            .or_else(|| existing.tmux_inner_pane.clone()),
        tmux_outer_client_tty: incoming
            .tmux_outer_client_tty
            .or_else(|| existing.tmux_outer_client_tty.clone()),
        terminal_window_id: incoming
            .terminal_window_id
            .or_else(|| existing.terminal_window_id.clone()),
        terminal_session_id: incoming
            .terminal_session_id
            .or_else(|| existing.terminal_session_id.clone()),
        terminal_pane_id: incoming
            .terminal_pane_id
            .or_else(|| existing.terminal_pane_id.clone()),
        terminal_control_endpoint: incoming
            .terminal_control_endpoint
            .or_else(|| existing.terminal_control_endpoint.clone()),
        ide_workspace: incoming
            .ide_workspace
            .or_else(|| existing.ide_workspace.clone()),
        native_session_id: incoming.native_session_id,
    }
}

impl StoveWire {
    fn from_stored(
        id: &str,
        stove: &Stove,
        locator_capability: LocatorCapability,
        summary: &StoveSummary,
        pinned: bool,
    ) -> Self {
        let progress = stove.progress.as_ref().and_then(|progress| {
            let provenance = match progress.source {
                EventSource::StructuredSession => ProgressProvenanceWire::StructuredSession,
                EventSource::Hook => ProgressProvenanceWire::Hook,
                EventSource::Inference | EventSource::Process => return None,
            };
            Some(ProgressWire {
                completed: progress.completed,
                total: progress.total,
                provenance,
            })
        });
        Self {
            id: id.to_owned(),
            harness: harness_wire(&stove.identity.harness),
            host: host_wire(&stove.identity.host),
            project_root: summary.project_root_display.clone(),
            project_label: summary.project_label.clone(),
            project_root_display: summary.project_root_display.clone(),
            task_title: summary.task_title.clone(),
            current_action: summary.current_action.clone(),
            next_action: summary.next_action.clone(),
            elapsed_ms: summary.elapsed_ms,
            state: state_wire(stove.state),
            progress,
            locator_capability,
            retained_completion: stove.state == StoveState::Cooked,
            pinned,
        }
    }
}

impl ArchivedSessionWire {
    fn from_archived(archived: &ArchivedSession) -> Self {
        Self {
            id: stove_id(&archived.session.locator),
            harness: harness_wire(&archived.session.locator.harness),
            host: host_wire(&archived.session.locator.host),
            project_label: archived.session.presentation.project_label.clone(),
            project_root_display: archived.session.presentation.project_root_display.clone(),
            session_identity: compact_session_identity(&archived.session.locator.native_session_id),
            last_state: state_wire(archived.session.last_state),
            reason: match archived.reason {
                ArchiveReason::Expired => ArchiveReasonWire::Expired,
                ArchiveReason::Manual => ArchiveReasonWire::Manual,
            },
            archived_at_ms: archived.archived_at_ms,
            source_available: source_available(archived),
            pinned: false,
        }
    }
}

fn bounded(mut value: String) -> String {
    let mut end = value.len().min(StoveSummary::MAX_TEXT_BYTES);
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    value.truncate(end);
    value
}

fn is_superseded(previous: Option<&EventMetadata>, incoming: &EventMetadata) -> bool {
    let Some(previous) = previous else {
        return false;
    };
    if incoming == previous {
        return true;
    }
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

const MIN_COMPARABLE_EPOCH_MS: u64 = 1_000_000_000_000;

fn cross_source_event_is_older(previous: Option<&EventMetadata>, incoming: &EventMetadata) -> bool {
    let Some(previous) = previous.filter(|previous| previous.source != incoming.source) else {
        return false;
    };
    if previous.timestamp_ms >= MIN_COMPARABLE_EPOCH_MS
        && incoming.timestamp_ms >= MIN_COMPARABLE_EPOCH_MS
    {
        return incoming.timestamp_ms < previous.timestamp_ms
            || (incoming.timestamp_ms == previous.timestamp_ms
                && incoming.source.authority() < previous.source.authority());
    }
    false
}

fn stove_id(identity: &StoveIdentity) -> String {
    let host_kind = match identity.host.kind {
        HostKind::Local => "local",
        HostKind::Ssh => "ssh",
    };
    format!(
        "{host_kind}:{}:{}:{}",
        identity.host.id,
        harness_wire(&identity.harness).id,
        identity.native_session_id
    )
}

fn harness_wire(harness: &HarnessId) -> HarnessWire {
    match harness {
        HarnessId::Codex => HarnessWire {
            id: "codex".into(),
            label: "Codex".into(),
        },
        HarnessId::ClaudeCode => HarnessWire {
            id: "claudeCode".into(),
            label: "Claude Code".into(),
        },
        HarnessId::Pi => HarnessWire {
            id: "pi".into(),
            label: "Pi".into(),
        },
        HarnessId::Other(id) => HarnessWire {
            id: id.clone(),
            label: id.clone(),
        },
    }
}
fn host_wire(host: &HostIdentity) -> HostWire {
    HostWire {
        kind: match host.kind {
            HostKind::Local => HostKindWire::Local,
            HostKind::Ssh => HostKindWire::Ssh,
        },
        id: host.id.clone(),
    }
}
fn state_wire(state: StoveState) -> StoveStateWire {
    match state {
        StoveState::Starting => StoveStateWire::Starting,
        StoveState::Planning => StoveStateWire::Planning,
        StoveState::Cooking => StoveStateWire::Cooking,
        StoveState::NeedsHuman => StoveStateWire::NeedsHuman,
        StoveState::Cooked => StoveStateWire::Cooked,
        StoveState::Failed => StoveStateWire::Failed,
        StoveState::Disconnected => StoveStateWire::Disconnected,
        StoveState::Removed => unreachable!("removed stoves are never serialized"),
    }
}

fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

const SESSION_VISIBILITY_MS: u64 = 2 * 24 * 60 * 60 * 1_000;

fn latest_observed_at(summary: &StoveSummary) -> Option<u64> {
    summary
        .source_modified_at_ms
        .into_iter()
        .chain(summary.last_observed_at_ms)
        .max()
}

fn comparable_epoch(timestamp_ms: u64) -> Option<u64> {
    (timestamp_ms >= MIN_COMPARABLE_EPOCH_MS).then_some(timestamp_ms)
}

fn event_kind_for_state(state: StoveState) -> EventKind {
    match state {
        StoveState::Starting | StoveState::Removed => EventKind::SessionDiscovered,
        StoveState::Planning => EventKind::PlanUpdated {
            completed: 0,
            total: 1,
        },
        StoveState::Cooking => EventKind::ToolStarted,
        StoveState::NeedsHuman => EventKind::QuestionAsked,
        StoveState::Cooked => EventKind::TurnCompleted,
        StoveState::Failed => EventKind::SessionFailed,
        StoveState::Disconnected => EventKind::ConnectionLost,
    }
}

fn source_available(archived: &ArchivedSession) -> bool {
    match archived.session.locator.host.kind {
        HostKind::Ssh => true,
        HostKind::Local => archived
            .session
            .native_locator
            .as_ref()
            .is_some_and(|path| Path::new(path).is_file()),
    }
}

fn compact_session_identity(native_session_id: &str) -> String {
    if native_session_id.len() >= 4
        && native_session_id.len() <= SessionLocator::MAX_TEXT_BYTES
        && native_session_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        let start = native_session_id.len().saturating_sub(8);
        format!("#{}", &native_session_id[start..])
    } else {
        "#session".to_owned()
    }
}

fn notification_event(
    kind: &EventKind,
    effective_state: StoveState,
) -> Option<NotificationEventKind> {
    match kind {
        EventKind::SessionDiscovered if effective_state == StoveState::Starting => {
            Some(NotificationEventKind::SessionAppeared)
        }
        EventKind::UserPromptSubmitted if effective_state == StoveState::Cooking => {
            Some(NotificationEventKind::CookingStarted)
        }
        EventKind::PlanUpdated { .. } if effective_state == StoveState::Planning => {
            Some(NotificationEventKind::PhaseChanged)
        }
        EventKind::ToolStarted | EventKind::ToolCompleted { .. }
            if effective_state == StoveState::Cooking =>
        {
            Some(NotificationEventKind::PhaseChanged)
        }
        EventKind::QuestionAsked | EventKind::PermissionRequested
            if effective_state == StoveState::NeedsHuman =>
        {
            Some(NotificationEventKind::NeedsHuman)
        }
        EventKind::TurnCompleted if effective_state == StoveState::Cooked => {
            Some(NotificationEventKind::Cooked)
        }
        EventKind::SessionFailed if effective_state == StoveState::Failed => {
            Some(NotificationEventKind::Failed)
        }
        EventKind::ConnectionLost if effective_state == StoveState::Disconnected => {
            Some(NotificationEventKind::Disconnected)
        }
        EventKind::ConnectionRestored if effective_state != StoveState::Disconnected => {
            Some(NotificationEventKind::ConnectionRestored)
        }
        EventKind::ClearRequested => Some(NotificationEventKind::StoveCleared),
        _ => None,
    }
}

fn progress_percent(progress: Option<&cookbench_core::domain::StructuredProgress>) -> Option<u8> {
    progress
        .map(|progress| ((u64::from(progress.completed) * 100) / u64::from(progress.total)) as u8)
}

fn crossed_milestone(previous: Option<u8>, current: Option<u8>) -> Option<u8> {
    let previous = previous.unwrap_or(0);
    let current = current?;
    [25, 50, 75, 100]
        .into_iter()
        .rfind(|milestone| previous < *milestone && current >= *milestone)
}

#[cfg(test)]
mod notification_tests {
    use cookbench_core::{
        domain::{
            EventKind, EventMetadata, EventSource, HarnessId, HostIdentity, ProjectIdentity,
            StoveEvent, StoveIdentity, StoveState,
        },
        notifications::NotificationEventKind,
    };

    use super::{
        crossed_milestone, is_superseded, notification_event, LocatorCapability, StoveStore,
        StoveSummary,
    };

    #[test]
    fn notification_projection_includes_milestones_and_manual_clear() {
        assert_eq!(crossed_milestone(Some(24), Some(76)), Some(75));
        assert_eq!(crossed_milestone(Some(75), Some(76)), None);
        assert_eq!(
            notification_event(&EventKind::ClearRequested, StoveState::Cooked),
            Some(NotificationEventKind::StoveCleared)
        );
    }

    #[test]
    fn ineffective_source_events_cannot_send_terminal_notifications() {
        assert_eq!(
            notification_event(&EventKind::TurnCompleted, StoveState::Disconnected),
            None
        );
        assert_eq!(
            notification_event(&EventKind::PermissionRequested, StoveState::Cooking),
            None
        );
    }

    #[test]
    fn identical_terminal_replay_is_inert_after_alert_dedupe_window() {
        let terminal =
            EventMetadata::new(EventSource::StructuredSession, 100, 42, 1_700_000_000_000);

        // Wall-clock time is intentionally not part of source ordering. Even if
        // the same file record is replayed after the one-second alert window,
        // it remains the same observation and cannot trigger side effects again.
        assert!(is_superseded(Some(&terminal), &terminal));
    }

    #[test]
    fn expiry_uses_local_receipt_time_instead_of_a_remote_sequence_timestamp() {
        let host = HostIdentity::ssh("remote-host");
        let identity = StoveIdentity::new(host.clone(), HarnessId::Codex, "remote-session");
        let project = ProjectIdentity::new(host, "/remote/project");
        let store = StoveStore::default();
        store
            .apply_with_summary(
                identity,
                project.clone(),
                LocatorCapability::Unavailable,
                Some(StoveSummary::for_project(&project).with_last_observed_at_ms(Some(100))),
                StoveEvent::new(
                    EventKind::ToolStarted,
                    EventMetadata::new(
                        EventSource::StructuredSession,
                        100,
                        1,
                        1_700_000_000_000_000,
                    ),
                ),
            )
            .unwrap();

        assert_eq!(store.expiration_candidates(200).len(), 1);

        let recent_store = StoveStore::default();
        recent_store
            .apply_with_summary(
                StoveIdentity::new(
                    HostIdentity::ssh("remote-host"),
                    HarnessId::ClaudeCode,
                    "recent-remote-session",
                ),
                project.clone(),
                LocatorCapability::Unavailable,
                Some(
                    StoveSummary::for_project(&project)
                        .with_source_modified_at_ms(Some(50))
                        .with_last_observed_at_ms(Some(300)),
                ),
                StoveEvent::new(
                    EventKind::ToolStarted,
                    EventMetadata::new(
                        EventSource::StructuredSession,
                        100,
                        1,
                        1_700_000_000_000_000,
                    ),
                ),
            )
            .unwrap();
        assert!(recent_store.expiration_candidates(200).is_empty());
    }
}
