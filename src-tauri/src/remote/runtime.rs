//! Managed zero-install SSH observation loops.

use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::Duration,
};

use cookbench_bridge::protocol::{
    ConfiguredHarness, ConfiguredRoot, NormalizedEvent, NormalizedState,
};
use cookbench_core::{
    domain::{
        EventKind, EventMetadata, EventSource, HarnessId, HostIdentity, ProjectIdentity,
        StoveEvent, StoveIdentity,
    },
    locator::SessionLocator,
    persistence::RemoteSourceConfig,
    remote::{RemoteHost, SessionRoot},
};
use sha2::{Digest, Sha256};
use tauri::Manager;

use crate::{
    app_state::{AppState, LocatorCapability, StoveSummary},
    runtime::{current_time_ms, ObservationSummary},
};

use super::{
    bridge::{
        connect_temporary_bridge, BridgeDeploymentSelection, Sha256Digest, SystemBridgeRemote,
    },
    parser::FirstPartyRemoteParser,
    ssh::SystemSshRunner,
    zero_install::{RemoteStoveEvent, ZeroInstallSshSource},
};

enum RemoteControl {
    Forget(StoveIdentity),
    Stop,
}

struct RemoteHandle {
    id: String,
    sender: mpsc::Sender<RemoteControl>,
    cancelled: Arc<AtomicBool>,
    known: Arc<Mutex<Vec<StoveIdentity>>>,
    join: Option<thread::JoinHandle<()>>,
}

impl RemoteHandle {
    fn stop(mut self) {
        self.cancelled.store(true, Ordering::Release);
        let _ = self.sender.send(RemoteControl::Stop);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

impl Drop for RemoteHandle {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
        let _ = self.sender.send(RemoteControl::Stop);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

#[derive(Default)]
pub struct RemoteRuntimeState {
    handles: Mutex<Vec<RemoteHandle>>,
    sequences: Mutex<HashMap<String, Arc<AtomicU64>>>,
    diagnostics: Mutex<Vec<String>>,
}

impl RemoteRuntimeState {
    pub fn reconfigure<R: tauri::Runtime>(
        &self,
        app: tauri::AppHandle<R>,
        configured: &[RemoteSourceConfig],
    ) -> Result<(), String> {
        let mut handles = self
            .handles
            .lock()
            .map_err(|_| "remote source state is unavailable".to_owned())?;
        let enabled_ids = configured
            .iter()
            .filter(|source| source.enabled)
            .map(|source| source.id.as_str())
            .collect::<Vec<_>>();
        for handle in handles.drain(..) {
            if !enabled_ids.iter().any(|id| *id == handle.id) {
                mark_disconnected(&app, &handle.known, &self.sequence_for(&handle.id));
            }
            handle.stop();
        }
        for config in configured.iter().filter(|source| source.enabled).take(16) {
            let host = match validated_host(config) {
                Ok(host) => host,
                Err(_) => {
                    self.record_diagnostic("invalid persisted remote source skipped")?;
                    continue;
                }
            };
            if config.bridge_enabled {
                match start_bridge(app.clone(), config, host, self.sequence_for(&config.id)) {
                    Ok(handle) => handles.push(handle),
                    Err(_) => self.record_diagnostic("temporary bridge source could not start")?,
                }
            } else {
                handles.push(start_host(
                    app.clone(),
                    config.id.clone(),
                    host,
                    config.alias.clone(),
                    self.sequence_for(&config.id),
                ));
            }
        }
        Ok(())
    }

    fn sequence_for(&self, id: &str) -> Arc<AtomicU64> {
        let mut counters = self
            .sequences
            .lock()
            .expect("remote sequence state poisoned");
        counters
            .entry(id.to_owned())
            .or_insert_with(|| Arc::new(AtomicU64::new(current_time_ms().saturating_mul(1_000))))
            .clone()
    }

    fn record_diagnostic(&self, message: &str) -> Result<(), String> {
        let mut diagnostics = self
            .diagnostics
            .lock()
            .map_err(|_| "remote diagnostics unavailable".to_owned())?;
        if diagnostics.len() == 32 {
            diagnostics.remove(0);
        }
        diagnostics.push(message.to_owned());
        Ok(())
    }

    pub fn diagnostics(&self) -> Vec<String> {
        self.diagnostics
            .lock()
            .map(|items| items.clone())
            .unwrap_or_default()
    }

    pub fn forget(&self, stove: StoveIdentity) {
        if let Ok(handles) = self.handles.lock() {
            for handle in handles.iter() {
                let _ = handle.sender.send(RemoteControl::Forget(stove.clone()));
            }
        }
    }
}

fn validated_host(config: &RemoteSourceConfig) -> Result<RemoteHost, String> {
    let roots = config
        .session_roots
        .iter()
        .take(16)
        .map(|root| SessionRoot::new(root.clone()).map_err(|error| error.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    RemoteHost::new(config.alias.clone(), roots).map_err(|error| error.to_string())
}

fn start_host<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    id: String,
    host: RemoteHost,
    alias: String,
    sequence: Arc<AtomicU64>,
) -> RemoteHandle {
    let (sender, receiver) = mpsc::channel();
    let cancelled = Arc::new(AtomicBool::new(false));
    let stop = cancelled.clone();
    let known = Arc::new(Mutex::new(Vec::new()));
    let thread_known = known.clone();
    let join = thread::spawn(move || {
        let mut source =
            ZeroInstallSshSource::new(host, SystemSshRunner).with_sequence_counter(sequence);
        let parser = FirstPartyRemoteParser;
        let mut projects = HashMap::new();
        while !stop.load(Ordering::Acquire) {
            let poll = source.observe(&parser);
            for observed in poll.events {
                remember(&thread_known, &observed.stove);
                apply_remote_event(&app, &alias, &mut projects, observed);
            }
            let wait = source.poll_interval().duration();
            match receiver.recv_timeout(wait) {
                Ok(RemoteControl::Forget(stove)) => {
                    source.forget_stove(&stove);
                    projects.remove(&stove);
                }
                Ok(RemoteControl::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        }
    });
    RemoteHandle {
        id,
        sender,
        cancelled,
        known,
        join: Some(join),
    }
}

fn start_bridge<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    config: &RemoteSourceConfig,
    host: RemoteHost,
    sequence: Arc<AtomicU64>,
) -> Result<RemoteHandle, String> {
    let binary = config
        .bridge_binary_path
        .as_ref()
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(packaged_bridge_binary);
    let digest = digest_file(&binary).map_err(|_| "bridge binary is unavailable".to_owned())?;
    let remote_path = temporary_remote_path(&config.id);
    let roots = host
        .session_roots()
        .iter()
        .map(|root| ConfiguredRoot::new(ConfiguredHarness::Auto, root.as_str().to_owned()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "bridge session roots are invalid".to_owned())?;
    let selection = BridgeDeploymentSelection::explicit(host.alias(), remote_path, digest)
        .and_then(|selection| selection.with_roots(roots))
        .map_err(|_| "bridge selection is invalid".to_owned())?;
    let remote = SystemBridgeRemote::new(binary).map_err(|_| "bridge binary is unavailable")?;
    Ok(spawn_bridge(
        app,
        config.id.clone(),
        config.alias.clone(),
        host.identity(),
        remote,
        selection,
        sequence,
    ))
}

fn spawn_bridge<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    id: String,
    alias: String,
    host: HostIdentity,
    mut remote: SystemBridgeRemote,
    selection: BridgeDeploymentSelection,
    sequence: Arc<AtomicU64>,
) -> RemoteHandle {
    let (sender, receiver) = mpsc::channel();
    let cancelled = Arc::new(AtomicBool::new(false));
    let stop = cancelled.clone();
    let known = Arc::new(Mutex::new(Vec::new()));
    let thread_known = known.clone();
    let thread_sequence = sequence.clone();
    let join = thread::spawn(move || {
        let mut projects = HashMap::new();
        let mut reconnecting = false;
        while !stop.load(Ordering::Acquire) {
            let mut connection = match connect_temporary_bridge(&mut remote, selection.clone()) {
                Ok(connection) => connection,
                Err(_) => {
                    if !reconnecting {
                        mark_disconnected(&app, &thread_known, &thread_sequence);
                    }
                    reconnecting = true;
                    if wait_for_bridge_control(
                        &receiver,
                        &stop,
                        &thread_known,
                        &mut projects,
                        Duration::from_secs(30),
                    ) {
                        break;
                    }
                    continue;
                }
            };
            if reconnecting {
                restore_bridge_stoves(&app, &alias, &thread_known, &mut projects, &thread_sequence);
            }
            loop {
                match connection.poll() {
                    Ok(events) => {
                        for event in events {
                            if let Some(observed) = bridge_event(&host, event, &thread_sequence) {
                                remember(&thread_known, &observed.stove);
                                apply_remote_event(&app, &alias, &mut projects, observed);
                            }
                        }
                    }
                    Err(_) => {
                        mark_disconnected(&app, &thread_known, &thread_sequence);
                        reconnecting = true;
                        break;
                    }
                }
                if wait_for_bridge_control(
                    &receiver,
                    &stop,
                    &thread_known,
                    &mut projects,
                    Duration::from_secs(2),
                ) {
                    let _ = connection.close();
                    return;
                }
            }
            drop(connection);
        }
    });
    RemoteHandle {
        id,
        sender,
        cancelled,
        known,
        join: Some(join),
    }
}

fn wait_for_bridge_control(
    receiver: &mpsc::Receiver<RemoteControl>,
    stop: &AtomicBool,
    known: &Arc<Mutex<Vec<StoveIdentity>>>,
    projects: &mut HashMap<StoveIdentity, String>,
    duration: Duration,
) -> bool {
    match receiver.recv_timeout(duration) {
        Ok(RemoteControl::Forget(stove)) => {
            if let Ok(mut known) = known.lock() {
                known.retain(|candidate| candidate != &stove);
            }
            projects.remove(&stove);
            false
        }
        Ok(RemoteControl::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => true,
        Err(mpsc::RecvTimeoutError::Timeout) => stop.load(Ordering::Acquire),
    }
}

fn bridge_event(
    host: &HostIdentity,
    event: NormalizedEvent,
    sequence: &Arc<AtomicU64>,
) -> Option<RemoteStoveEvent> {
    let harness = match event.harness.as_str() {
        "codex" => HarnessId::Codex,
        "claude" | "claude_code" | "claude-code" => HarnessId::ClaudeCode,
        "pi" => HarnessId::Pi,
        value if !value.is_empty() && value.len() <= 64 => HarnessId::Other(value.to_owned()),
        _ => return None,
    };
    if event.stove_key.is_empty()
        || event.stove_key.len() > 256
        || event.stove_key.chars().any(char::is_control)
    {
        return None;
    }
    let project_root = event.project_root.clone();
    let kind = match event.state {
        NormalizedState::Starting => EventKind::SessionDiscovered,
        NormalizedState::Planning => match event.progress {
            Some(progress) if progress.total > 0 && progress.completed <= progress.total => {
                EventKind::PlanUpdated {
                    completed: progress.completed,
                    total: progress.total,
                }
            }
            _ => EventKind::ToolStarted,
        },
        NormalizedState::Cooking => EventKind::ToolStarted,
        NormalizedState::NeedsHuman => EventKind::QuestionAsked,
        NormalizedState::Cooked => EventKind::TurnCompleted,
        NormalizedState::Failed => EventKind::SessionFailed,
        NormalizedState::Disconnected => EventKind::ConnectionLost,
    };
    let next = sequence.fetch_add(1, Ordering::AcqRel).saturating_add(1);
    Some(RemoteStoveEvent {
        stove: StoveIdentity::new(host.clone(), harness, event.stove_key),
        project_root,
        event: StoveEvent::new(
            kind,
            EventMetadata::new(EventSource::StructuredSession, 95, next, current_time_ms()),
        ),
    })
}

fn restore_bridge_stoves<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    alias: &str,
    known: &Arc<Mutex<Vec<StoveIdentity>>>,
    projects: &mut HashMap<StoveIdentity, String>,
    sequence: &Arc<AtomicU64>,
) {
    for stove in known.lock().map(|items| items.clone()).unwrap_or_default() {
        let next = sequence.fetch_add(1, Ordering::AcqRel).saturating_add(1);
        apply_remote_event(
            app,
            alias,
            projects,
            RemoteStoveEvent {
                stove,
                project_root: None,
                event: StoveEvent::new(
                    EventKind::ConnectionRestored,
                    EventMetadata::new(EventSource::StructuredSession, 95, next, current_time_ms()),
                ),
            },
        );
    }
}

fn remember(known: &Arc<Mutex<Vec<StoveIdentity>>>, stove: &StoveIdentity) {
    if let Ok(mut known) = known.lock() {
        if !known.contains(stove) {
            known.push(stove.clone());
        }
    }
}

fn packaged_bridge_binary() -> PathBuf {
    let name = if cfg!(windows) {
        "cookbench-bridge.exe"
    } else {
        "cookbench-bridge"
    };
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.join(name)))
        .unwrap_or_else(|| PathBuf::from(name))
}

fn digest_file(path: &PathBuf) -> Result<Sha256Digest, std::io::Error> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(Sha256Digest(hasher.finalize().into()))
}

fn temporary_remote_path(source_id: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in source_id.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("/tmp/cookbench-bridge-{hash:016x}")
}

fn apply_remote_event<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    alias: &str,
    projects: &mut HashMap<StoveIdentity, String>,
    observed: RemoteStoveEvent,
) {
    if let Some(root) = observed.project_root.as_ref() {
        projects.insert(observed.stove.clone(), root.clone());
    }
    let project_root = projects
        .get(&observed.stove)
        .cloned()
        .unwrap_or_else(|| format!("ssh:{alias}"));
    let project = ProjectIdentity::new(observed.stove.host.clone(), project_root.clone());
    let label = project_root
        .rsplit('/')
        .find(|part| !part.is_empty())
        .unwrap_or(alias);
    let locator = SessionLocator {
        native_session_id: format!("{alias}:{}", observed.stove.native_session_id),
        ..SessionLocator::default()
    };
    let presentation = ObservationSummary::from_event(None, &observed.event, current_time_ms());
    let summary = StoveSummary::new(
        label,
        format!("{alias}:{project_root}"),
        presentation.task_title,
        presentation.current_action,
        presentation.next_action,
        presentation.elapsed_ms,
    );
    let state = app.state::<AppState>();
    let _ = state.apply_observation_and_emit(
        app,
        observed.stove,
        project,
        LocatorCapability::Available,
        Some(locator),
        Some(summary),
        observed.event,
    );
}

fn mark_disconnected<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    known: &Arc<Mutex<Vec<StoveIdentity>>>,
    sequence: &Arc<AtomicU64>,
) {
    let stoves = known.lock().map(|known| known.clone()).unwrap_or_default();
    let state = app.state::<AppState>();
    for identity in stoves {
        let Some(existing) = state
            .stoves
            .snapshot()
            .stoves
            .into_iter()
            .find(|stove| stove.id == remote_stove_id(&identity))
        else {
            continue;
        };
        let next = sequence.fetch_add(1, Ordering::AcqRel).saturating_add(1);
        let _ = state.apply_observation_and_emit(
            app,
            identity.clone(),
            ProjectIdentity::new(identity.host.clone(), existing.project_root),
            LocatorCapability::Unavailable,
            None,
            None,
            cookbench_core::domain::StoveEvent::new(
                cookbench_core::domain::EventKind::ConnectionLost,
                cookbench_core::domain::EventMetadata::new(
                    cookbench_core::domain::EventSource::StructuredSession,
                    90,
                    next,
                    current_time_ms(),
                ),
            ),
        );
    }
}

fn remote_stove_id(identity: &StoveIdentity) -> String {
    let host = match identity.host.kind {
        cookbench_core::domain::HostKind::Local => "local",
        cookbench_core::domain::HostKind::Ssh => "ssh",
    };
    let harness = match &identity.harness {
        cookbench_core::domain::HarnessId::Codex => "codex",
        cookbench_core::domain::HarnessId::ClaudeCode => "claudeCode",
        cookbench_core::domain::HarnessId::Pi => "pi",
        cookbench_core::domain::HarnessId::Other(value) => value,
    };
    format!(
        "{host}:{}:{harness}:{}",
        identity.host.id, identity.native_session_id
    )
}
