//! Private, bounded ingestion of the optional hook helper's atomic envelopes.
//! Hook data is translated immediately into lifecycle events and then removed;
//! Cookbench never retains hook payloads, transcript text, or commands.

use std::{
    collections::BTreeMap,
    fs, io,
    io::Write,
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
use cookbench_core::locator::{HostApplication, SessionLocator, TerminalKind};
use serde::{Deserialize, Serialize};

pub const MAX_ENVELOPES_PER_POLL: usize = 128;
pub const MAX_ENVELOPE_BYTES: u64 = 16 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HookObservation {
    pub identity: StoveIdentity,
    pub project: ProjectIdentity,
    pub locator: Option<SessionLocator>,
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
        let observation = serde_json::from_slice::<Envelope>(&bytes)
            .ok()
            .and_then(|envelope| envelope.into_observation(self.host.clone()));
        if let Some(observation) = observation.as_ref() {
            record_hook_health(
                &self.directory,
                &observation.identity.harness,
                observation.event.metadata.timestamp_ms,
            );
        }
        let _ = fs::remove_file(path);
        observation
    }
}

/// Stores only the latest per-harness receipt time in the app-private spool.
/// It is bounded metadata for Settings health; hook payloads remain transient.
fn record_hook_health(directory: &Path, harness: &HarnessId, received_at_ms: u64) {
    let path = directory.join("hook-health.json");
    let mut ledger = fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<HookHealthLedger>(&bytes).ok())
        .unwrap_or_default();
    ledger
        .last_event_ms
        .insert(harness_key(harness), received_at_ms);
    let Ok(bytes) = serde_json::to_vec(&ledger) else {
        return;
    };
    let temporary = directory.join(".hook-health.tmp");
    let Ok(mut file) = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temporary)
    else {
        return;
    };
    if file.write_all(&bytes).is_ok() && file.sync_all().is_ok() {
        let _ = fs::rename(temporary, path);
    }
}

#[derive(Default, Deserialize, Serialize)]
struct HookHealthLedger {
    #[serde(default)]
    last_event_ms: BTreeMap<String, u64>,
}

fn harness_key(harness: &HarnessId) -> String {
    match harness {
        HarnessId::Codex => "codex".into(),
        HarnessId::ClaudeCode => "claudeCode".into(),
        HarnessId::Pi => "pi".into(),
        HarnessId::Other(value) => value.clone(),
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
    #[serde(default)]
    locator: Option<EnvelopeLocator>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Progress {
    completed: u32,
    total: u32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EnvelopeLocator {
    #[serde(default)]
    native_locator: Option<String>,
    #[serde(default)]
    working_directory: Option<String>,
    #[serde(default)]
    process_id: Option<u32>,
    #[serde(default)]
    terminal: Option<String>,
    #[serde(default)]
    terminal_session_id: Option<String>,
    #[serde(default)]
    terminal_pane_id: Option<String>,
    #[serde(default)]
    terminal_control_endpoint: Option<String>,
    #[serde(default)]
    tmux_pane: Option<String>,
}

impl Envelope {
    fn into_observation(self, host: HostIdentity) -> Option<HookObservation> {
        if !(self.schema_version == 1 || self.schema_version == 2)
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
            value
                if cookbench_adapters::harness_profile(value)
                    .is_some_and(|profile| profile.structured_lifecycle) =>
            {
                HarnessId::Other(value.to_owned())
            }
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
        let session_id = self.event.session_id;
        let locator = self
            .event
            .locator
            .and_then(|locator| locator.into_locator(&session_id));
        let identity = StoveIdentity::new(host.clone(), harness, session_id);
        Some(HookObservation {
            project: ProjectIdentity::new(host, "(hook project unavailable)"),
            identity,
            locator,
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

impl EnvelopeLocator {
    fn into_locator(self, native_session_id: &str) -> Option<SessionLocator> {
        let locator = SessionLocator {
            native_locator: self.native_locator,
            process_id: self.process_id,
            working_directory: self.working_directory,
            host_application: self.terminal.as_deref().and_then(host_application),
            terminal: self.terminal.as_deref().and_then(terminal_kind),
            tmux_pane: self.tmux_pane,
            terminal_session_id: self.terminal_session_id,
            terminal_pane_id: self.terminal_pane_id,
            terminal_control_endpoint: self.terminal_control_endpoint,
            native_session_id: native_session_id.to_owned(),
            ..SessionLocator::default()
        };
        locator.validate().ok().map(|_| locator)
    }
}

fn terminal_kind(value: &str) -> Option<TerminalKind> {
    match value {
        "iterm2" => Some(TerminalKind::ITerm2),
        "wezterm" => Some(TerminalKind::WezTerm),
        "ghostty" => Some(TerminalKind::Ghostty),
        "zellij" => Some(TerminalKind::Zellij),
        "cmux" => Some(TerminalKind::Cmux),
        "macos_terminal" => Some(TerminalKind::MacosTerminal),
        "gnome_terminal" => Some(TerminalKind::GnomeTerminal),
        "konsole" => Some(TerminalKind::Konsole),
        "xfce_terminal" => Some(TerminalKind::XfceTerminal),
        "terminal" => Some(TerminalKind::Other("terminal".into())),
        _ => None,
    }
}

fn host_application(value: &str) -> Option<HostApplication> {
    match value {
        "iterm2" => Some(HostApplication::ITerm2),
        "wezterm" => Some(HostApplication::WezTerm),
        "ghostty" => Some(HostApplication::Ghostty),
        "zellij" => Some(HostApplication::Zellij),
        "cmux" => Some(HostApplication::Cmux),
        "macos_terminal" => Some(HostApplication::MacosTerminal),
        "gnome_terminal" => Some(HostApplication::GnomeTerminal),
        "konsole" => Some(HostApplication::Konsole),
        "xfce_terminal" => Some(HostApplication::XfceTerminal),
        "visual_studio_code" => Some(HostApplication::VisualStudioCode),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_v1_envelopes_without_locator() {
        let envelope: Envelope = serde_json::from_str(
            r#"{"schema_version":1,"source":"hook","received_at_ms":7,"event":{"event_type":"turn_completed","session_id":"old-session","harness":"codex"}}"#,
        )
        .unwrap();
        let observation = envelope
            .into_observation(HostIdentity::local("local"))
            .unwrap();
        assert_eq!(observation.locator, None);
        assert_eq!(observation.identity.native_session_id, "old-session");
    }

    #[test]
    fn converts_bounded_hook_locator_to_session_locator() {
        let envelope: Envelope = serde_json::from_str(
            r#"{"schema_version":2,"source":"hook","received_at_ms":7,"event":{"event_type":"turn_completed","session_id":"session-42","harness":"claude_code","locator":{"native_locator":"/tmp/session.jsonl","working_directory":"/tmp/project","process_id":12,"terminal":"wezterm","terminal_pane_id":"4","terminal_control_endpoint":"/tmp/wezterm.sock"}}}"#,
        )
        .unwrap();
        let observation = envelope
            .into_observation(HostIdentity::local("local"))
            .unwrap();
        let locator = observation.locator.unwrap();
        assert_eq!(locator.native_session_id, "session-42");
        assert_eq!(locator.terminal, Some(TerminalKind::WezTerm));
        assert_eq!(locator.terminal_pane_id.as_deref(), Some("4"));
    }

    #[test]
    fn writes_only_bounded_per_harness_health_metadata() {
        let directory =
            std::env::temp_dir().join(format!("cookbench-health-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        record_hook_health(&directory, &HarnessId::ClaudeCode, 42);
        let value: serde_json::Value =
            serde_json::from_slice(&fs::read(directory.join("hook-health.json")).unwrap()).unwrap();
        assert_eq!(value["last_event_ms"]["claudeCode"], 42);
        assert_eq!(value.as_object().unwrap().len(), 1);
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn accepts_allowlisted_catalog_harnesses_as_forward_compatible_ids() {
        let envelope: Envelope = serde_json::from_str(
            r#"{"schema_version":2,"source":"hook","received_at_ms":7,"event":{"event_type":"turn_completed","session_id":"qwen-session","harness":"qwen_code"}}"#,
        )
        .unwrap();
        let observation = envelope
            .into_observation(HostIdentity::local("local"))
            .unwrap();
        assert_eq!(
            observation.identity.harness,
            HarnessId::Other("qwen_code".into())
        );
    }

    #[test]
    fn health_ledger_keeps_each_expanded_harness_separate() {
        let directory =
            std::env::temp_dir().join(format!("cookbench-expanded-health-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        record_hook_health(&directory, &HarnessId::Other("qwen_code".into()), 42);
        record_hook_health(&directory, &HarnessId::Other("kimi_code".into()), 43);
        let value: serde_json::Value =
            serde_json::from_slice(&fs::read(directory.join("hook-health.json")).unwrap()).unwrap();
        assert_eq!(value["last_event_ms"]["qwen_code"], 42);
        assert_eq!(value["last_event_ms"]["kimi_code"], 43);
        let _ = fs::remove_dir_all(directory);
    }
}
