//! Read-only local session observation.
//!
//! This runtime tails only native JSONL session files and forwards normalized,
//! content-free lifecycle events. It never launches, controls, or configures a
//! harness, and it retains neither transcript records nor a session database.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant, SystemTime},
};

use cookbench_adapters::{
    claude::{self, ClaudeAdapter},
    codex::{self, CodexAdapter},
    io::{DirectoryWatch, JsonlTailer, TailLimits, TailRecord},
    pi::{self, PiAdapter},
    HostSource, NativeSession,
};
use cookbench_core::domain::{
    EventKind, EventMetadata, EventSource, HostIdentity, ProjectIdentity, StoveEvent, StoveIdentity,
};

const MAX_PRESENTATION_TEXT_BYTES: usize = 160;
const MIN_EPOCH_TIMESTAMP_MS: u64 = 1_000_000_000_000;

/// A bounded, content-free projection for hover and notification surfaces.
/// Values are derived from structured session metadata and normalized event
/// kinds only; source transcript fields are never accepted here.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ObservationSummary {
    pub task_title: Option<String>,
    pub current_action: Option<String>,
    pub next_action: Option<String>,
    pub elapsed_ms: Option<u64>,
}

impl ObservationSummary {
    pub fn from_event(title: Option<&str>, event: &StoveEvent, now_ms: u64) -> Self {
        let (current_action, next_action) = activity_for(&event.kind);
        Self {
            task_title: title.and_then(sanitize_title),
            current_action: Some(current_action.to_owned()),
            next_action: Some(next_action.to_owned()),
            elapsed_ms: elapsed_from_event(event.metadata.timestamp_ms, now_ms),
        }
    }
}

fn activity_for(event: &EventKind) -> (&'static str, &'static str) {
    match event {
        EventKind::SessionDiscovered => ("Session discovered", "Waiting for native activity"),
        EventKind::UserPromptSubmitted => ("Working on a new turn", "Watching for activity"),
        EventKind::PlanUpdated { .. } => ("Updating the structured plan", "Continuing the plan"),
        EventKind::ToolStarted => ("Running a tool", "Waiting for its result"),
        EventKind::ToolCompleted { .. } => ("Processing a tool result", "Continuing the task"),
        EventKind::QuestionAsked => ("Needs human input", "Waiting for a response"),
        EventKind::PermissionRequested => ("Needs permission", "Waiting for approval"),
        EventKind::TurnCompleted => ("Task completed", "Waiting for the next prompt"),
        EventKind::SessionFailed => ("Task failed", "Waiting for the next action"),
        EventKind::ProcessExited => ("Host process exited", "Waiting for source activity"),
        EventKind::ConnectionLost => ("Source disconnected", "Waiting to reconnect"),
        EventKind::ConnectionRestored => ("Source reconnected", "Resuming observation"),
        EventKind::ClearRequested => ("Clearing retained state", "No next action reported"),
        EventKind::Tick => ("Observing session state", "Waiting for an update"),
    }
}

fn sanitize_title(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.len() > MAX_PRESENTATION_TEXT_BYTES
        || trimmed.chars().any(char::is_control)
        || ["```", "{", "}", "$(", ";", "--"]
            .iter()
            .any(|marker| trimmed.contains(marker))
    {
        return None;
    }
    Some(trimmed.to_owned())
}

fn elapsed_from_event(timestamp_ms: u64, now_ms: u64) -> Option<u64> {
    (timestamp_ms >= MIN_EPOCH_TIMESTAMP_MS && now_ms >= timestamp_ms)
        .then_some(now_ms.saturating_sub(timestamp_ms))
}

#[derive(Clone, Debug)]
pub struct LocalObservationConfig {
    pub host: HostIdentity,
    pub codex_root: PathBuf,
    pub claude_root: PathBuf,
    pub pi_roots: Vec<PathBuf>,
    pub startup_min_modified: SystemTime,
    pub startup_candidate_limit: usize,
}

impl LocalObservationConfig {
    pub fn from_environment(host: HostIdentity) -> Self {
        let codex = CodexAdapter::new(host.clone());
        let claude = ClaudeAdapter::from_environment()
            .unwrap_or_else(|_| ClaudeAdapter::new(PathBuf::from(".claude/projects")));
        let pi = PiAdapter::new();
        Self {
            host,
            codex_root: codex.root().to_owned(),
            claude_root: claude.projects_root().to_owned(),
            pi_roots: pi.roots().to_vec(),
            startup_min_modified: SystemTime::now()
                .checked_sub(Duration::from_secs(24 * 60 * 60))
                .unwrap_or(SystemTime::UNIX_EPOCH),
            startup_candidate_limit: 64,
        }
    }
}

pub trait ObservationSink: Send + Sync + 'static {
    #[allow(clippy::too_many_arguments)]
    fn apply(
        &self,
        identity: StoveIdentity,
        project: ProjectIdentity,
        locator: String,
        title: Option<String>,
        summary: ObservationSummary,
        origin: ObservationOrigin,
        event: StoveEvent,
    );
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservationOrigin {
    Replay,
    Live,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ParserKind {
    Codex,
    Claude,
    Pi,
}

struct WatchedSession {
    identity: StoveIdentity,
    project: ProjectIdentity,
    title: Option<String>,
    parser: ParserKind,
    tailer: JsonlTailer,
    sequence: u64,
}

const REPLAY_WINDOW_BYTES: u64 = 1024 * 1024;
const MAX_REPLAY_POLLS: usize = 2;
const RESCAN_INTERVAL: Duration = Duration::from_secs(30);
const MAX_SCANNED_ENTRIES: usize = 4_096;
const MAX_DISCOVERY_DEPTH: usize = 8;

/// Owns a bounded observer set. Call [`bootstrap`] once, then [`tick`] from a
/// worker thread or event loop; each tick only processes filesystem-notified
/// paths, never scans arbitrary disk state.
pub struct LocalObservationRuntime<S: ObservationSink> {
    config: LocalObservationConfig,
    sink: Arc<S>,
    watches: Vec<(ParserKind, PathBuf, DirectoryWatch)>,
    sessions: BTreeMap<PathBuf, WatchedSession>,
    last_rescan: Instant,
}

impl<S: ObservationSink> LocalObservationRuntime<S> {
    pub fn new(config: LocalObservationConfig, sink: Arc<S>) -> Self {
        let mut runtime = Self {
            config,
            sink,
            watches: Vec::new(),
            sessions: BTreeMap::new(),
            last_rescan: Instant::now(),
        };
        runtime.ensure_watches();
        runtime
    }

    /// Discovers bounded recent candidates and reconstructs their authoritative
    /// state from a bounded suffix. Stale files are filtered by metadata before
    /// any adapter opens their bodies.
    pub fn bootstrap(&mut self) {
        self.refresh_all();
    }

    pub fn tick(&mut self) {
        self.ensure_watches();
        let mut changed = Vec::new();
        for (kind, _, watch) in &self.watches {
            while let Ok(Some(path)) = watch.try_recv() {
                changed.push((*kind, path));
            }
        }
        for (kind, path) in changed {
            if !self.sessions.contains_key(&path) {
                self.register_path(kind, &path);
            }
            self.observe_path(&path);
        }
        if self.last_rescan.elapsed() >= RESCAN_INTERVAL {
            self.rescan();
        }
    }

    /// Bounded recovery pass for newly created roots and coalesced filesystem
    /// notifications. Existing tailers read only bytes appended since their
    /// cursor, so this is not transcript polling or full-file rereading.
    pub fn rescan(&mut self) {
        self.ensure_watches();
        self.refresh_all();
        let paths = self.sessions.keys().cloned().collect::<Vec<_>>();
        for path in paths {
            self.observe_path(&path);
        }
        self.last_rescan = Instant::now();
    }

    /// Testable one-path equivalent of a filesystem notification.
    pub fn observe_path(&mut self, path: &Path) {
        self.observe_path_once(path, ObservationOrigin::Live);
    }

    fn observe_path_once(&mut self, path: &Path, origin: ObservationOrigin) -> bool {
        let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_owned());
        let lookup = if self.sessions.contains_key(&canonical) {
            canonical
        } else {
            path.to_owned()
        };
        let Some(session) = self.sessions.get_mut(&lookup) else {
            return false;
        };
        let records = match session.tailer.poll() {
            Ok(records) => records,
            Err(_) => return false,
        };
        let observed_bytes = !records.is_empty();
        for record in records {
            let TailRecord::Record(line) = record else {
                continue;
            };
            session.sequence = session.sequence.saturating_add(1);
            let events = parse(session.parser, &line, session.sequence);
            for event in events {
                self.sink.apply(
                    session.identity.clone(),
                    session.project.clone(),
                    session.tailer.path().to_string_lossy().into_owned(),
                    session.title.clone(),
                    ObservationSummary::from_event(
                        session.title.as_deref(),
                        &event,
                        current_time_ms(),
                    ),
                    origin,
                    event,
                );
            }
        }
        observed_bytes
    }

    fn replay_recent(&mut self, path: &Path) {
        for _ in 0..MAX_REPLAY_POLLS {
            if !self.observe_path_once(path, ObservationOrigin::Replay) {
                break;
            }
        }
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    fn refresh_all(&mut self) {
        for kind in [ParserKind::Codex, ParserKind::Claude, ParserKind::Pi] {
            self.refresh_kind(kind);
        }
    }

    fn ensure_watches(&mut self) {
        let missing = roots(&self.config)
            .into_iter()
            .filter(|(kind, root)| {
                !self.watches.iter().any(|(watched_kind, watched_root, _)| {
                    watched_kind == kind && watched_root == root
                })
            })
            .map(|(kind, root)| (kind, root.to_owned()))
            .collect::<Vec<_>>();
        for (kind, root) in missing {
            if let Ok(watch) = DirectoryWatch::open(&root, 256) {
                self.watches.push((kind, root, watch));
            }
        }
    }

    fn refresh_kind(&mut self, kind: ParserKind) {
        for session in discover(kind, &self.config) {
            self.register_session(kind, session);
        }
    }

    fn register_path(&mut self, kind: ParserKind, path: &Path) {
        if !is_recent_path(path, self.config.startup_min_modified) {
            return;
        }
        if let Some(session) = session_from_path(kind, &self.config, path) {
            self.register_session(kind, session);
        }
    }

    fn register_session(&mut self, kind: ParserKind, session: NativeSession) {
        let path = PathBuf::from(&session.locator.value);
        if self.sessions.contains_key(&path) {
            return;
        }
        let root = root_for(kind, &self.config, &path);
        let Ok(mut tailer) = JsonlTailer::open(root, &path, TailLimits::default()) else {
            return;
        };
        if tailer.seek_recent_window(REPLAY_WINDOW_BYTES).is_err() {
            return;
        }
        let sequence = tailer.cursor();
        let identity = StoveIdentity::new(
            session.host.clone(),
            session.harness.clone(),
            session.native_session_id.clone(),
        );
        let project = session
            .project
            .clone()
            .unwrap_or_else(|| ProjectIdentity::new(session.host.clone(), "(unknown project)"));
        let discovered = StoveEvent::new(
            EventKind::SessionDiscovered,
            EventMetadata::new(EventSource::StructuredSession, 100, 0, 0),
        );
        self.sink.apply(
            identity.clone(),
            project.clone(),
            session.locator.value.clone(),
            session.title.clone(),
            ObservationSummary::from_event(
                session.title.as_deref(),
                &discovered,
                current_time_ms(),
            ),
            ObservationOrigin::Replay,
            discovered,
        );
        self.sessions.insert(
            path.clone(),
            WatchedSession {
                identity,
                project,
                title: session.title,
                parser: kind,
                tailer,
                sequence,
            },
        );
        self.replay_recent(&path);
    }
}

pub struct RuntimeHandle {
    cancelled: Arc<AtomicBool>,
    join: Option<thread::JoinHandle<()>>,
}
impl RuntimeHandle {
    pub fn cancel(mut self) {
        self.cancelled.store(true, Ordering::Release);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}
impl Drop for RuntimeHandle {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

pub fn start<S: ObservationSink>(config: LocalObservationConfig, sink: Arc<S>) -> RuntimeHandle {
    let cancelled = Arc::new(AtomicBool::new(false));
    let stop = cancelled.clone();
    let join = thread::spawn(move || {
        let mut runtime = LocalObservationRuntime::new(config, sink);
        runtime.bootstrap();
        while !stop.load(Ordering::Acquire) {
            runtime.tick();
            thread::sleep(Duration::from_millis(50));
        }
    });
    RuntimeHandle {
        cancelled,
        join: Some(join),
    }
}

fn roots(config: &LocalObservationConfig) -> Vec<(ParserKind, &Path)> {
    let mut values = vec![
        (ParserKind::Codex, config.codex_root.as_path()),
        (ParserKind::Claude, config.claude_root.as_path()),
    ];
    values.extend(
        config
            .pi_roots
            .iter()
            .map(|root| (ParserKind::Pi, root.as_path())),
    );
    values
}
fn root_for(kind: ParserKind, config: &LocalObservationConfig, path: &Path) -> PathBuf {
    match kind {
        ParserKind::Codex => config.codex_root.clone(),
        ParserKind::Claude => config.claude_root.clone(),
        ParserKind::Pi => config
            .pi_roots
            .iter()
            .find(|root| path.starts_with(root))
            .cloned()
            .unwrap_or_else(|| path.parent().unwrap_or(Path::new(".")).to_owned()),
    }
}
fn discover(kind: ParserKind, config: &LocalObservationConfig) -> Vec<NativeSession> {
    let source = HostSource::local(config.host.clone());
    let mut scanned = 0;
    let mut paths = Vec::new();
    for root in roots_for_kind(kind, config) {
        collect_recent_jsonl(
            root,
            0,
            &mut scanned,
            config.startup_min_modified,
            &mut paths,
        );
        if scanned >= MAX_SCANNED_ENTRIES {
            break;
        }
    }
    paths.sort_by(|left, right| {
        modified_at_path(right)
            .cmp(&modified_at_path(left))
            .then_with(|| left.cmp(right))
    });
    paths.truncate(config.startup_candidate_limit);

    paths
        .into_iter()
        .filter_map(|path| session_from_path_with_source(kind, config, &source, &path))
        .collect()
}

fn session_from_path(
    kind: ParserKind,
    config: &LocalObservationConfig,
    path: &Path,
) -> Option<NativeSession> {
    session_from_path_with_source(kind, config, &HostSource::local(config.host.clone()), path)
}

fn session_from_path_with_source(
    kind: ParserKind,
    config: &LocalObservationConfig,
    source: &HostSource,
    path: &Path,
) -> Option<NativeSession> {
    match kind {
        ParserKind::Codex => {
            let adapter = CodexAdapter::with_root(config.codex_root.clone(), config.host.clone());
            adapter.session_from_path(source, path).ok().flatten()
        }
        ParserKind::Claude => {
            let root = fs::canonicalize(&config.claude_root)
                .unwrap_or_else(|_| config.claude_root.clone());
            let path = fs::canonicalize(path).unwrap_or_else(|_| path.to_owned());
            claude::discover_session(&root, &path, source)
                .ok()
                .flatten()
        }
        ParserKind::Pi => PiAdapter::with_roots(config.pi_roots.clone())
            .session_metadata_from_path(source, path.to_owned())
            .ok(),
    }
}

fn collect_recent_jsonl(
    directory: &Path,
    depth: usize,
    scanned: &mut usize,
    minimum_modified: SystemTime,
    output: &mut Vec<PathBuf>,
) {
    if depth > MAX_DISCOVERY_DEPTH || *scanned >= MAX_SCANNED_ENTRIES {
        return;
    }
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        *scanned = scanned.saturating_add(1);
        if *scanned > MAX_SCANNED_ENTRIES {
            return;
        }
        let path = entry.path();
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            collect_recent_jsonl(&path, depth + 1, scanned, minimum_modified, output);
        } else if metadata.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("jsonl"))
            && metadata
                .modified()
                .is_ok_and(|modified| modified >= minimum_modified)
        {
            output.push(path);
        }
    }
}

fn roots_for_kind(kind: ParserKind, config: &LocalObservationConfig) -> Vec<&Path> {
    match kind {
        ParserKind::Codex => vec![config.codex_root.as_path()],
        ParserKind::Claude => vec![config.claude_root.as_path()],
        ParserKind::Pi => config.pi_roots.iter().map(PathBuf::as_path).collect(),
    }
}
fn parse(kind: ParserKind, line: &str, sequence: u64) -> Vec<StoveEvent> {
    match kind {
        ParserKind::Codex => codex::parse_record(
            line,
            sequence,
            TailLimits::default().max_json_nesting,
            TailLimits::default().max_json_field_bytes,
        )
        .and_then(|record| record.event)
        .into_iter()
        .collect(),
        ParserKind::Claude => claude::parse_record(line, TailLimits::default(), sequence)
            .map(|record| record.events)
            .unwrap_or_default(),
        ParserKind::Pi => pi::parse_record(line, sequence),
    }
}

fn modified_at_path(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

fn is_recent_path(path: &Path, minimum: SystemTime) -> bool {
    modified_at_path(path).is_some_and(|modified| modified >= minimum)
}

pub fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod presentation_tests {
    use cookbench_core::domain::{EventKind, EventMetadata, EventSource, StoveEvent};

    use super::ObservationSummary;

    #[test]
    fn structured_metadata_becomes_safe_hover_activity_with_known_elapsed_time() {
        let event = StoveEvent::new(
            EventKind::ToolStarted,
            EventMetadata::new(EventSource::StructuredSession, 90, 7, 1_700_000_000_000),
        );
        let summary = ObservationSummary::from_event(
            Some("Verify the public boundary"),
            &event,
            1_700_000_005_000,
        );

        assert_eq!(
            summary.task_title.as_deref(),
            Some("Verify the public boundary")
        );
        assert_eq!(summary.current_action.as_deref(), Some("Running a tool"));
        assert_eq!(
            summary.next_action.as_deref(),
            Some("Waiting for its result")
        );
        assert_eq!(summary.elapsed_ms, Some(5_000));
    }

    #[test]
    fn unsafe_title_and_non_epoch_sequence_do_not_enter_presentation_state() {
        let event = StoveEvent::new(
            EventKind::UserPromptSubmitted,
            EventMetadata::new(EventSource::StructuredSession, 90, 3, 3),
        );
        let summary =
            ObservationSummary::from_event(Some("untrusted {metadata}"), &event, 1_700_000_005_000);

        assert_eq!(summary.task_title, None);
        assert_eq!(summary.elapsed_ms, None);
        assert_eq!(
            summary.current_action.as_deref(),
            Some("Working on a new turn")
        );
    }
}
