use std::{
    collections::HashMap,
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
    persistence::{PersistedConfig, PersistedState, RetainedStovePresentation},
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
        }
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
        let loaded = service.load();
        let retained = loaded.state.retained.clone();
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
        summary: Option<StoveSummary>,
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
                    let presentation = RetainedStovePresentation::new(
                        summary.project_label,
                        summary.project_root_display,
                    );
                    let _ = runtime.service.persist_transition_with_presentation(
                        &mut runtime.state,
                        identity_for_persistence,
                        stove.state,
                        &metadata,
                        presentation,
                    );
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
        let Some(notifications) =
            app.try_state::<crate::commands::notifications::NotificationCommandState>()
        else {
            return;
        };
        // Source timestamps describe harness ordering, not local queue age.
        // Using them here could expire a newly observed event.
        let now_ms = current_time_ms();
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
}

impl std::fmt::Display for AppStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Store(error) => error.fmt(f),
            Self::Emit(error) => error.fmt(f),
            Self::Persistence(error) => f.write_str(error),
            Self::UnknownStove => f.write_str("Cookbench does not have that Stove"),
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
            return Ok(StoveChange::remove(revision, id));
        }

        inner
            .source_cursors
            .entry(id.clone())
            .or_default()
            .insert(raw_metadata.source, raw_metadata);

        let valid_locator = locator.filter(|locator| locator.validate().is_ok());
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
        let wire = StoveWire::from_stored(&id, &next, capability, &summary);
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
                    )
                })
                .collect(),
        }
    }
}

impl StoveWire {
    fn from_stored(
        id: &str,
        stove: &Stove,
        locator_capability: LocatorCapability,
        summary: &StoveSummary,
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
        domain::{EventKind, StoveState},
        notifications::NotificationEventKind,
    };

    use super::{crossed_milestone, notification_event};

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
}
