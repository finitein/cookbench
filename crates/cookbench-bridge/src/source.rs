//! Bounded, read-only native-session observation for the SSH stdio bridge.

use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use cookbench_adapters::{
    claude, codex,
    io::{JsonlTailer, TailLimits, TailRecord},
    pi,
};
use cookbench_core::domain::{EventKind, StoveEvent};

use crate::protocol::{ConfiguredHarness, ConfiguredRoot, NormalizedEvent};

const MAX_CANDIDATES: usize = 256;
const MAX_SCANNED_ENTRIES: usize = 4_096;
const MAX_DEPTH: usize = 8;
const REPLAY_WINDOW_BYTES: u64 = 1024 * 1024;
const RECENT_AGE: Duration = Duration::from_secs(30 * 24 * 60 * 60);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SourceHarness {
    Auto,
    Codex,
    ClaudeCode,
    Pi,
}

impl SourceHarness {
    fn wire_name(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Codex => "codex",
            Self::ClaudeCode => "claude_code",
            Self::Pi => "pi",
        }
    }
}

#[derive(Clone, Debug)]
pub struct SourceRoot {
    pub harness: SourceHarness,
    pub path: PathBuf,
}

struct ObservedSession {
    harness: SourceHarness,
    stove_key: String,
    tailer: JsonlTailer,
    sequence: u64,
    project_root: Option<String>,
}

/// Polls bounded native JSONL suffixes only when the desktop sends a heartbeat.
/// The bridge retains byte cursors and normalized identity metadata, never
/// transcript records.
pub struct NativeSessionSource {
    roots: Vec<SourceRoot>,
    sessions: BTreeMap<PathBuf, ObservedSession>,
    output_sequence: u64,
    minimum_modified: SystemTime,
}

impl NativeSessionSource {
    pub fn with_roots(roots: Vec<SourceRoot>, minimum_modified: SystemTime) -> Self {
        Self {
            roots,
            sessions: BTreeMap::new(),
            output_sequence: 0,
            minimum_modified,
        }
    }

    pub fn from_configured_roots(roots: Vec<ConfiguredRoot>) -> Self {
        let roots = roots
            .into_iter()
            .map(|root| SourceRoot {
                harness: match root.harness {
                    ConfiguredHarness::Auto => SourceHarness::Auto,
                    ConfiguredHarness::Codex => SourceHarness::Codex,
                    ConfiguredHarness::ClaudeCode => SourceHarness::ClaudeCode,
                    ConfiguredHarness::Pi => SourceHarness::Pi,
                },
                path: PathBuf::from(root.path),
            })
            .collect();
        let minimum_modified = SystemTime::now()
            .checked_sub(RECENT_AGE)
            .unwrap_or(SystemTime::UNIX_EPOCH);
        Self::with_roots(roots, minimum_modified)
    }

    pub fn poll(&mut self) -> Vec<NormalizedEvent> {
        self.discover_recent();
        let paths = self.sessions.keys().cloned().collect::<Vec<_>>();
        let mut normalized = Vec::new();
        for path in paths {
            let Some(session) = self.sessions.get_mut(&path) else {
                continue;
            };
            let records = match session.tailer.poll() {
                Ok(records) => records,
                Err(_) => continue,
            };
            for record in records {
                let TailRecord::Record(line) = record else {
                    continue;
                };
                session.sequence = session.sequence.saturating_add(1);
                let (detected_harness, events, project_root) =
                    parse(session.harness, &line, session.sequence);
                if let Some(detected_harness) = detected_harness {
                    session.harness = detected_harness;
                }
                if project_root.is_some() {
                    session.project_root = project_root;
                }
                for event in events {
                    if let Some(state) = normalized_state(&event) {
                        normalized.push((
                            session.stove_key.clone(),
                            session.harness.wire_name(),
                            state,
                            normalized_progress(&event),
                            session.project_root.clone(),
                        ));
                    }
                }
            }
        }
        normalized
            .into_iter()
            .map(|(key, harness, state, progress, project_root)| {
                self.event(key, harness, state, progress, project_root)
            })
            .collect()
    }

    fn discover_recent(&mut self) {
        let mut candidates = Vec::new();
        let mut scanned = 0;
        for root in &self.roots {
            collect_candidates(root, &root.path, 0, &mut scanned, &mut candidates);
            if scanned >= MAX_SCANNED_ENTRIES {
                break;
            }
        }
        candidates.retain(|candidate| candidate.modified >= self.minimum_modified);
        candidates.sort_by(|left, right| {
            right
                .modified
                .cmp(&left.modified)
                .then_with(|| left.path.cmp(&right.path))
        });
        candidates.truncate(MAX_CANDIDATES);

        for candidate in candidates {
            if self.sessions.contains_key(&candidate.path) {
                continue;
            }
            let Ok(mut tailer) =
                JsonlTailer::open(&candidate.root, &candidate.path, TailLimits::default())
            else {
                continue;
            };
            if tailer.seek_recent_window(REPLAY_WINDOW_BYTES).is_err() {
                continue;
            }
            let stove_key = stable_path_key(candidate.harness, &candidate.path);
            let sequence = tailer.cursor();
            self.sessions.insert(
                candidate.path,
                ObservedSession {
                    harness: candidate.harness,
                    stove_key,
                    tailer,
                    sequence,
                    project_root: None,
                },
            );
        }
    }

    fn event(
        &mut self,
        key: String,
        harness: &str,
        state: &str,
        progress: Option<(u32, u32)>,
        project_root: Option<String>,
    ) -> NormalizedEvent {
        self.output_sequence = self.output_sequence.saturating_add(1);
        let event = NormalizedEvent::state(key, harness, state, self.output_sequence)
            .with_project_root(project_root);
        match progress {
            Some((completed, total)) => event.with_progress(completed, total),
            None => event,
        }
    }
}

impl Default for NativeSessionSource {
    fn default() -> Self {
        let home = env::var_os("HOME")
            .or_else(|| env::var_os("USERPROFILE"))
            .map(PathBuf::from);
        let codex_home = env::var_os("CODEX_HOME")
            .map(PathBuf::from)
            .or_else(|| home.as_ref().map(|path| path.join(".codex")))
            .unwrap_or_else(|| PathBuf::from(".codex"));
        let claude_home = env::var_os("CLAUDE_CONFIG_DIR")
            .map(PathBuf::from)
            .or_else(|| home.as_ref().map(|path| path.join(".claude")))
            .unwrap_or_else(|| PathBuf::from(".claude"));
        let pi_root = env::var_os("PI_SESSION_DIR")
            .map(PathBuf::from)
            .or_else(|| home.map(|path| path.join(".pi/agent/sessions")))
            .unwrap_or_else(|| PathBuf::from(".pi/agent/sessions"));
        let minimum_modified = SystemTime::now()
            .checked_sub(RECENT_AGE)
            .unwrap_or(SystemTime::UNIX_EPOCH);
        Self::with_roots(
            vec![
                SourceRoot {
                    harness: SourceHarness::Codex,
                    path: codex_home.join("sessions"),
                },
                SourceRoot {
                    harness: SourceHarness::ClaudeCode,
                    path: claude_home.join("projects"),
                },
                SourceRoot {
                    harness: SourceHarness::Pi,
                    path: pi_root,
                },
            ],
            minimum_modified,
        )
    }
}

struct Candidate {
    harness: SourceHarness,
    root: PathBuf,
    path: PathBuf,
    modified: SystemTime,
}

fn collect_candidates(
    source: &SourceRoot,
    directory: &Path,
    depth: usize,
    scanned: &mut usize,
    output: &mut Vec<Candidate>,
) {
    if depth > MAX_DEPTH || *scanned >= MAX_SCANNED_ENTRIES {
        return;
    }
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        *scanned += 1;
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
            collect_candidates(source, &path, depth + 1, scanned, output);
        } else if metadata.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("jsonl"))
        {
            output.push(Candidate {
                harness: source.harness,
                root: source.path.clone(),
                path,
                modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            });
        }
    }
}

fn parse(
    harness: SourceHarness,
    line: &str,
    sequence: u64,
) -> (Option<SourceHarness>, Vec<StoveEvent>, Option<String>) {
    let project_root = extract_project_root(line);
    let (detected, events) = match harness {
        SourceHarness::Codex => (
            Some(SourceHarness::Codex),
            codex::parse_record(line, sequence, 64, 64 * 1024)
                .and_then(|record| record.event)
                .into_iter()
                .collect(),
        ),
        SourceHarness::ClaudeCode => claude::parse_record(line, TailLimits::default(), sequence)
            .map(|record| (Some(SourceHarness::ClaudeCode), record.events))
            .unwrap_or((Some(SourceHarness::ClaudeCode), Vec::new())),
        SourceHarness::Pi => (Some(SourceHarness::Pi), pi::parse_record(line, sequence)),
        SourceHarness::Auto => detect(line, sequence),
    };
    (detected, events, project_root)
}

fn detect(line: &str, sequence: u64) -> (Option<SourceHarness>, Vec<StoveEvent>) {
    if let Some(record) = codex::parse_record(line, sequence, 64, 64 * 1024) {
        if record.session_id.is_some() || record.cwd.is_some() || record.event.is_some() {
            return (
                Some(SourceHarness::Codex),
                record.event.into_iter().collect(),
            );
        }
    }
    if let Some(record) = claude::parse_record(line, TailLimits::default(), sequence) {
        return (Some(SourceHarness::ClaudeCode), record.events);
    }
    let events = pi::parse_record(line, sequence);
    if events.is_empty() {
        (None, events)
    } else {
        (Some(SourceHarness::Pi), events)
    }
}

fn extract_project_root(line: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    find_project_root(&value, 0)
}

fn find_project_root(value: &serde_json::Value, depth: usize) -> Option<String> {
    if depth > 8 {
        return None;
    }
    let object = value.as_object()?;
    for key in [
        "cwd",
        "projectPath",
        "project_path",
        "workingDirectory",
        "working_directory",
    ] {
        if let Some(path) = object.get(key).and_then(serde_json::Value::as_str) {
            if path.starts_with('/')
                && path.len() <= 4 * 1024
                && !path.chars().any(char::is_control)
            {
                return Some(path.to_owned());
            }
        }
    }
    object
        .values()
        .filter(|child| child.is_object())
        .find_map(|child| find_project_root(child, depth + 1))
}

fn normalized_state(event: &StoveEvent) -> Option<&'static str> {
    match event.kind {
        EventKind::SessionDiscovered => Some("starting"),
        EventKind::UserPromptSubmitted
        | EventKind::ToolStarted
        | EventKind::ToolCompleted { .. } => Some("cooking"),
        EventKind::PlanUpdated { .. } => Some("planning"),
        EventKind::QuestionAsked | EventKind::PermissionRequested => Some("needs_human"),
        EventKind::TurnCompleted => Some("cooked"),
        EventKind::SessionFailed | EventKind::ProcessExited => Some("failed"),
        EventKind::ConnectionLost => Some("disconnected"),
        EventKind::ConnectionRestored | EventKind::ClearRequested | EventKind::Tick => None,
    }
}

fn normalized_progress(event: &StoveEvent) -> Option<(u32, u32)> {
    match event.kind {
        EventKind::PlanUpdated { completed, total } if total > 0 && completed <= total => {
            Some((completed, total))
        }
        _ => None,
    }
}

fn stable_path_key(harness: SourceHarness, path: &Path) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in path.as_os_str().to_string_lossy().bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{}-{hash:016x}", harness.wire_name())
}
