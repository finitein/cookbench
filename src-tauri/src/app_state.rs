use std::{collections::HashMap, sync::RwLock};

use cookbench_core::{
    domain::{
        EventSource, HarnessId, HostIdentity, HostKind, ProjectIdentity, Stove, StoveEvent,
        StoveIdentity, StoveState,
    },
    state_machine,
};
use serde::{Deserialize, Serialize};

use crate::events::StoveChange;

#[derive(Default)]
pub struct AppState {
    pub stoves: StoveStore,
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
        let change = self.stoves.apply_with_summary(
            identity,
            project,
            locator_capability,
            summary,
            event,
        )?;
        crate::events::emit_stove_change(app, change).map_err(AppStateError::Emit)?;
        crate::platform::publish_optional_gnome_snapshot(&self.stoves.snapshot());
        Ok(())
    }
}

#[derive(Debug)]
pub enum AppStateError {
    Store(StoreError),
    Emit(tauri::Error),
}

impl std::fmt::Display for AppStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Store(error) => error.fmt(f),
            Self::Emit(error) => error.fmt(f),
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
        let id = stove_id(&identity);
        let mut inner = self.inner.write().expect("stove store lock poisoned");
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
        let next = match state_machine::reduce(&previous, &event) {
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
            return Ok(StoveChange::remove(revision, id));
        }

        let capability = if matches!(locator_capability, LocatorCapability::Available) {
            locator_capability
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
    value.truncate(StoveSummary::MAX_TEXT_BYTES);
    value
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
