//! Private, bounded ingestion of the optional hook helper's atomic envelopes.
//! Hook data is translated immediately into lifecycle events and then removed;
//! Cookbench never retains hook payloads, transcript text, or commands.

use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use cookbench_core::domain::{
    EventKind, EventMetadata, EventSource, HarnessId, HostIdentity, ProjectIdentity, StoveEvent,
    StoveIdentity,
};
use serde::Deserialize;

pub const MAX_ENVELOPES_PER_POLL: usize = 128;
pub const MAX_ENVELOPE_BYTES: u64 = 16 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HookObservation {
    pub identity: StoveIdentity,
    pub project: ProjectIdentity,
    pub event: StoveEvent,
}

pub struct HookSpool {
    directory: PathBuf,
    host: HostIdentity,
}
impl HookSpool {
    pub fn create(directory: impl Into<PathBuf>, host: HostIdentity) -> io::Result<Self> {
        let directory = directory.into();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            let mut builder = fs::DirBuilder::new();
            builder.recursive(true).mode(0o700).create(&directory)?;
        }
        #[cfg(not(unix))]
        {
            fs::create_dir_all(&directory)?;
        }
        Ok(Self { directory, host })
    }
    pub fn directory(&self) -> &Path {
        &self.directory
    }
    pub fn poll(&self) -> Vec<HookObservation> {
        let Ok(entries) = fs::read_dir(&self.directory) else {
            return Vec::new();
        };
        let mut paths = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| allowed(path))
            .collect::<Vec<_>>();
        paths.sort();
        paths
            .into_iter()
            .take(MAX_ENVELOPES_PER_POLL)
            .filter_map(|path| self.consume(&path))
            .collect()
    }
    fn consume(&self, path: &Path) -> Option<HookObservation> {
        let metadata = fs::symlink_metadata(path).ok()?;
        if metadata.file_type().is_symlink()
            || !metadata.is_file()
            || metadata.len() > MAX_ENVELOPE_BYTES
        {
            let _ = fs::remove_file(path);
            return None;
        }
        let bytes = fs::read(path).ok()?;
        let parsed = serde_json::from_slice::<Envelope>(&bytes)
            .ok()
            .and_then(|envelope| envelope.into_observation(self.host.clone()));
        let _ = fs::remove_file(path);
        parsed
    }
}

/// Owns the optional helper-consumer lifecycle. It polls only the app-private
/// directory and stops synchronously when the desktop process exits.
pub struct HookSpoolHandle {
    cancelled: Arc<AtomicBool>,
    join: Option<thread::JoinHandle<()>>,
}
impl HookSpoolHandle {
    pub fn cancel(mut self) {
        self.cancelled.store(true, Ordering::Release);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}
impl Drop for HookSpoolHandle {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
    }
}

pub fn start(
    spool: HookSpool,
    consume: Arc<dyn Fn(HookObservation) + Send + Sync>,
) -> HookSpoolHandle {
    let cancelled = Arc::new(AtomicBool::new(false));
    let stop = cancelled.clone();
    let join = thread::spawn(move || {
        while !stop.load(Ordering::Acquire) {
            for observation in spool.poll() {
                consume(observation);
            }
            thread::sleep(Duration::from_millis(50));
        }
    });
    HookSpoolHandle {
        cancelled,
        join: Some(join),
    }
}

fn allowed(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("event-") && name.ends_with(".json"))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    schema_version: u8,
    source: String,
    received_at_ms: u64,
    event: EnvelopeEvent,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EnvelopeEvent {
    event_type: String,
    session_id: String,
    harness: String,
    #[serde(default)]
    sequence: Option<u64>,
    #[serde(default)]
    progress: Option<Progress>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Progress {
    completed: u32,
    total: u32,
}

impl Envelope {
    fn into_observation(self, host: HostIdentity) -> Option<HookObservation> {
        if self.schema_version != 1
            || self.source != "hook"
            || self.event.session_id.is_empty()
            || self.event.session_id.len() > 256
            || !self.event.session_id.is_ascii()
        {
            return None;
        }
        let harness = match self.event.harness.as_str() {
            "codex" => HarnessId::Codex,
            "claude_code" => HarnessId::ClaudeCode,
            "pi" => HarnessId::Pi,
            _ => return None,
        };
        let kind = match self.event.event_type.as_str() {
            "session_discovered" => EventKind::SessionDiscovered,
            "user_prompt_submitted" => EventKind::UserPromptSubmitted,
            "tool_started" => EventKind::ToolStarted,
            "tool_completed" => EventKind::ToolCompleted { succeeded: true },
            "question_asked" => EventKind::QuestionAsked,
            "permission_requested" => EventKind::PermissionRequested,
            "turn_completed" => EventKind::TurnCompleted,
            "session_failed" => EventKind::SessionFailed,
            "connection_lost" => EventKind::ConnectionLost,
            "connection_restored" => EventKind::ConnectionRestored,
            "plan_updated" => {
                let progress = self.event.progress?;
                if progress.total == 0 || progress.completed > progress.total {
                    return None;
                }
                EventKind::PlanUpdated {
                    completed: progress.completed,
                    total: progress.total,
                }
            }
            _ => return None,
        };
        let identity = StoveIdentity::new(host.clone(), harness, self.event.session_id);
        Some(HookObservation {
            project: ProjectIdentity::new(host, "(hook project unavailable)"),
            identity,
            event: StoveEvent::new(
                kind,
                EventMetadata::new(
                    EventSource::Hook,
                    100,
                    self.event.sequence.unwrap_or(self.received_at_ms),
                    self.received_at_ms,
                ),
            ),
        })
    }
}
