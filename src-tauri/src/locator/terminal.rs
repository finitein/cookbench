use std::{collections::HashMap, path::Path};

#[cfg(target_os = "macos")]
use std::time::{Duration, Instant};

use cookbench_adapters::{harness_profile, ReturnSurface};
use cookbench_core::{
    domain::HarnessId,
    locator::{HostApplication, SessionLocator, TerminalKind},
};

#[cfg(target_os = "macos")]
use super::run_bounded_for;

const MAX_ANCESTORS: usize = 64;
const MAX_GROK_SESSION_ID_BYTES: usize = 256;
#[cfg(target_os = "macos")]
const MAX_PROCESSES: usize = 4_096;
#[cfg(target_os = "macos")]
const MAX_HARNESS_PROCESSES: usize = 16;
#[cfg(target_os = "macos")]
const PROCESS_DISCOVERY_DEADLINE: Duration = Duration::from_millis(1_500);

/// Content-free process metadata used to correlate a running harness with its
/// terminal. Command arguments are deliberately not collected or retained.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservedProcess {
    pub process_id: u32,
    pub parent_process_id: u32,
    pub tty: Option<String>,
    pub executable: String,
    pub working_directory: Option<String>,
    grok_session_proof: Option<GrokSessionProof>,
}

/// Bounded, content-free proof that a Grok process has an open native session
/// file. This never retains an observed file path.
#[derive(Clone, Debug, Eq, PartialEq)]
struct GrokSessionProof {
    native_session_id: String,
    matching_session_file_open: bool,
    open_session_count: u8,
}

impl ObservedProcess {
    pub fn new(
        process_id: u32,
        parent_process_id: u32,
        tty: Option<&str>,
        executable: &str,
        working_directory: Option<&str>,
    ) -> Self {
        Self {
            process_id,
            parent_process_id,
            tty: tty.map(str::to_owned),
            executable: executable.to_owned(),
            working_directory: working_directory.map(str::to_owned),
            grok_session_proof: None,
        }
    }

    /// Attaches synthetic or live content-free Grok file-descriptor evidence.
    /// A process is eligible for exact return only when this proves one open
    /// session, with the same opaque native session ID as the locator.
    pub fn with_grok_session_proof(
        mut self,
        native_session_id: &str,
        matching_session_file_open: bool,
        open_session_count: u8,
    ) -> Self {
        self.grok_session_proof =
            valid_grok_session_id(native_session_id).then(|| GrokSessionProof {
                native_session_id: native_session_id.to_owned(),
                matching_session_file_open,
                open_session_count,
            });
        self
    }
}

/// Adds an exact terminal target only when one running harness process is an
/// unambiguous match. Ambiguity preserves the existing lower-precision locator.
pub fn correlate_terminal_locator(
    harness: &HarnessId,
    mut locator: SessionLocator,
    processes: &[ObservedProcess],
) -> SessionLocator {
    if !terminal_return_supported(harness) {
        return locator;
    }

    let is_grok = is_grok(harness);
    let grok_target_valid = is_grok && valid_grok_session_locator(&locator);

    let by_id = processes
        .iter()
        .map(|process| (process.process_id, process))
        .collect::<HashMap<_, _>>();
    let expected_directory = locator.working_directory.as_deref().map(normalize_path);
    let mut matches = processes
        .iter()
        .filter(|process| harness_process(harness, &process.executable))
        .filter(|process| process.tty.as_deref().is_some_and(valid_tty))
        .filter(|process| {
            is_grok
                || match expected_directory {
                    Some(expected) => process
                        .working_directory
                        .as_deref()
                        .is_some_and(|actual| normalize_path(actual) == expected),
                    None => true,
                }
        })
        .filter_map(|process| {
            let application = ancestor_application(process, &by_id)?;
            let terminal = match application {
                HostApplication::MacosTerminal => TerminalKind::MacosTerminal,
                HostApplication::ITerm2 => TerminalKind::ITerm2,
                _ => return None,
            };
            Some((process, application, terminal))
        })
        .collect::<Vec<_>>();

    if is_grok {
        let fallback_application = unique_application(&matches);
        let has_unproven_candidate = processes.iter().any(|process| {
            harness_process(harness, &process.executable)
                && process.tty.as_deref().is_some_and(valid_tty)
                && process.grok_session_proof.is_none()
        });
        matches.retain(|(process, _, _)| {
            grok_target_valid && process_has_grok_session_proof(process, &locator)
        });
        matches.sort_by_key(|(process, _, _)| process.process_id);
        if has_unproven_candidate || matches.len() != 1 {
            clear_grok_exact_target(&mut locator);
            if let Some(application) = fallback_application {
                locator.host_application = Some(application);
            }
            return locator;
        }
    }
    matches.sort_by_key(|(process, _, _)| process.process_id);
    if !is_grok {
        matches.dedup_by(|left, right| left.0.tty == right.0.tty);
    }

    let [(process, application, terminal)] = matches.as_slice() else {
        return locator;
    };
    let Some(tty) = process.tty.as_deref().and_then(normalize_tty) else {
        return locator;
    };
    locator.process_id = Some(process.process_id);
    locator.parent_process_id = Some(process.parent_process_id);
    locator.host_application = Some(application.clone());
    locator.terminal = Some(terminal.clone());
    locator.tty = Some(tty);
    locator
}

pub fn correlate_with_running_processes(
    harness: &HarnessId,
    locator: SessionLocator,
) -> SessionLocator {
    #[cfg(target_os = "macos")]
    {
        let grok_target = is_grok(harness)
            .then(|| grok_session_target(&locator))
            .flatten();
        let processes = observe_processes(harness, grok_target.as_ref());
        correlate_terminal_locator(harness, locator, &processes)
    }

    #[cfg(not(target_os = "macos"))]
    {
        let processes = observe_processes(harness);
        correlate_terminal_locator(harness, locator, &processes)
    }
}

fn is_grok(harness: &HarnessId) -> bool {
    matches!(harness, HarnessId::Other(id) if id == "grok_cli")
}

fn valid_grok_session_id(native_session_id: &str) -> bool {
    !native_session_id.is_empty()
        && native_session_id.len() <= MAX_GROK_SESSION_ID_BYTES
        && native_session_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

fn valid_grok_session_locator(locator: &SessionLocator) -> bool {
    locator.validate().is_ok() && grok_session_directory(locator).is_some()
}

fn grok_session_directory(locator: &SessionLocator) -> Option<&Path> {
    let native_locator = locator.native_locator.as_deref()?;
    if !valid_grok_session_id(&locator.native_session_id)
        || native_locator.len() > SessionLocator::MAX_TEXT_BYTES
        || native_locator.chars().any(char::is_control)
    {
        return None;
    }
    let path = Path::new(native_locator);
    let file_name = path.file_name()?.to_str()?;
    let session_directory = path.parent()?;
    (path.is_absolute()
        && matches!(file_name, "updates.jsonl" | "events.jsonl")
        && session_directory.file_name()?.to_str()? == locator.native_session_id
        && is_grok_sessions_path(session_directory))
    .then_some(session_directory)
}

#[cfg(target_os = "macos")]
fn grok_session_target(locator: &SessionLocator) -> Option<GrokSessionTarget<'_>> {
    grok_session_directory(locator).map(|session_directory| GrokSessionTarget {
        native_session_id: &locator.native_session_id,
        session_directory,
    })
}

#[cfg(target_os = "macos")]
struct GrokSessionTarget<'a> {
    native_session_id: &'a str,
    session_directory: &'a Path,
}

fn process_has_grok_session_proof(process: &ObservedProcess, locator: &SessionLocator) -> bool {
    matches!(
        process.grok_session_proof.as_ref(),
        Some(proof)
            if proof.native_session_id == locator.native_session_id
                && proof.matching_session_file_open
                && proof.open_session_count == 1
    )
}

fn unique_application(
    matches: &[(&ObservedProcess, HostApplication, TerminalKind)],
) -> Option<HostApplication> {
    let application = matches.first()?.1.clone();
    matches
        .iter()
        .all(|(_, candidate, _)| candidate == &application)
        .then_some(application)
}

fn clear_grok_exact_target(locator: &mut SessionLocator) {
    locator.process_id = None;
    locator.parent_process_id = None;
    locator.process_started_at_ms = None;
    locator.terminal = None;
    locator.tty = None;
    locator.tmux_pane = None;
    locator.tmux_inner_pane = None;
    locator.tmux_outer_client_tty = None;
    locator.terminal_window_id = None;
    locator.terminal_session_id = None;
    locator.terminal_pane_id = None;
    locator.terminal_control_endpoint = None;
}

fn harness_process(harness: &HarnessId, executable: &str) -> bool {
    let name = Path::new(executable)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(executable)
        .trim_start_matches('-');
    match harness {
        HarnessId::ClaudeCode => name.eq_ignore_ascii_case("claude"),
        HarnessId::Pi => name.eq_ignore_ascii_case("pi"),
        HarnessId::Other(id) => harness_profile(id).is_some_and(|profile| {
            profile
                .executables
                .iter()
                .any(|expected| name.eq_ignore_ascii_case(expected))
        }),
        HarnessId::Codex => false,
    }
}

fn terminal_return_supported(harness: &HarnessId) -> bool {
    match harness {
        HarnessId::ClaudeCode | HarnessId::Pi => true,
        HarnessId::Other(id) => harness_profile(id).is_some_and(|profile| {
            matches!(
                profile.return_surface,
                ReturnSurface::Terminal | ReturnSurface::ApplicationOrTerminal
            )
        }),
        HarnessId::Codex => false,
    }
}

fn ancestor_application(
    process: &ObservedProcess,
    by_id: &HashMap<u32, &ObservedProcess>,
) -> Option<HostApplication> {
    let mut parent = process.parent_process_id;
    for _ in 0..MAX_ANCESTORS {
        let ancestor = by_id.get(&parent)?;
        let executable = ancestor.executable.to_ascii_lowercase();
        if executable.contains("terminal.app/")
            || executable.ends_with("/terminal")
            || executable == "terminal"
        {
            return Some(HostApplication::MacosTerminal);
        }
        if executable.contains("iterm") {
            return Some(HostApplication::ITerm2);
        }
        if ancestor.parent_process_id == parent {
            return None;
        }
        parent = ancestor.parent_process_id;
    }
    None
}

fn valid_tty(tty: &str) -> bool {
    tty.len() <= 128
        && tty != "??"
        && !tty.is_empty()
        && tty.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '/' | '_' | '-')
        })
}

fn normalize_tty(tty: &str) -> Option<String> {
    valid_tty(tty).then(|| {
        if tty.starts_with("/dev/") {
            tty.to_owned()
        } else {
            format!("/dev/{tty}")
        }
    })
}

fn normalize_path(path: &str) -> &str {
    let trimmed = path.trim_end_matches(['/', '\\']);
    if trimmed.is_empty() {
        path
    } else {
        trimmed
    }
}

#[cfg(target_os = "macos")]
fn observe_processes(
    harness: &HarnessId,
    grok_target: Option<&GrokSessionTarget<'_>>,
) -> Vec<ObservedProcess> {
    let deadline = Instant::now() + PROCESS_DISCOVERY_DEADLINE;
    let args = ["-Ao", "pid=,ppid=,tty=,comm="]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let Ok(output) = run_bounded_for("/bin/ps", &args, Duration::from_millis(500)) else {
        return Vec::new();
    };
    if !output.status.success() || output.stdout.len() > 2 * 1024 * 1024 {
        return Vec::new();
    }

    let output = String::from_utf8_lossy(&output.stdout);
    if output.lines().nth(MAX_PROCESSES).is_some() {
        return Vec::new();
    }
    let mut processes = output
        .lines()
        .filter_map(parse_process_line)
        .collect::<Vec<_>>();
    let candidate_ids = processes
        .iter()
        .filter(|process| {
            harness_process(harness, &process.executable)
                && process.tty.as_deref().is_some_and(valid_tty)
        })
        .take(MAX_HARNESS_PROCESSES)
        .map(|process| process.process_id)
        .collect::<Vec<_>>();
    for process_id in candidate_ids {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        if let Some(process) = processes
            .iter_mut()
            .find(|process| process.process_id == process_id)
        {
            process.working_directory =
                working_directory(process_id, remaining.min(Duration::from_millis(250)));
            if let Some(target) = grok_target {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if !remaining.is_zero() {
                    process.grok_session_proof = grok_session_proof(
                        process_id,
                        target,
                        remaining.min(Duration::from_millis(250)),
                    );
                }
            }
        }
    }
    processes
}

#[cfg(not(target_os = "macos"))]
fn observe_processes(_harness: &HarnessId) -> Vec<ObservedProcess> {
    Vec::new()
}

#[cfg(target_os = "macos")]
fn parse_process_line(line: &str) -> Option<ObservedProcess> {
    let mut fields = line.split_whitespace();
    let process_id = fields.next()?.parse().ok()?;
    let parent_process_id = fields.next()?.parse().ok()?;
    let tty = fields.next()?;
    let executable = fields.collect::<Vec<_>>().join(" ");
    if executable.is_empty() {
        return None;
    }
    Some(ObservedProcess::new(
        process_id,
        parent_process_id,
        valid_tty(tty).then_some(tty),
        &executable,
        None,
    ))
}

#[cfg(target_os = "macos")]
fn working_directory(process_id: u32, timeout: Duration) -> Option<String> {
    let args = [
        "-nP".to_owned(),
        "-a".to_owned(),
        "-p".to_owned(),
        process_id.to_string(),
        "-d".to_owned(),
        "cwd".to_owned(),
        "-Fn".to_owned(),
    ];
    let output = run_bounded_for("/usr/sbin/lsof", &args, timeout).ok()?;
    if !output.status.success() || output.stdout.len() > 16 * 1024 {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix('n'))
        .filter(|path| path.starts_with('/') && !path.chars().any(char::is_control))
        .map(str::to_owned)
}

#[cfg(target_os = "macos")]
fn grok_session_proof(
    process_id: u32,
    target: &GrokSessionTarget<'_>,
    timeout: Duration,
) -> Option<GrokSessionProof> {
    let args = [
        "-nP".to_owned(),
        "-a".to_owned(),
        "-p".to_owned(),
        process_id.to_string(),
        "-Fn".to_owned(),
    ];
    let output = run_bounded_for("/usr/sbin/lsof", &args, timeout).ok()?;
    if !output.status.success() || output.stdout.len() > 64 * 1024 {
        return None;
    }

    let mut matching_session_file_open = false;
    let mut other_session_file_open = false;
    for path in String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.strip_prefix('n'))
    {
        let Some(directory) = grok_open_session_directory(path) else {
            continue;
        };
        if Path::new(directory) == target.session_directory {
            matching_session_file_open = true;
        } else {
            other_session_file_open = true;
        }
        if matching_session_file_open && other_session_file_open {
            // Retain no paths; an ambiguous process never needs more detail.
            break;
        }
    }

    Some(GrokSessionProof {
        native_session_id: target.native_session_id.to_owned(),
        matching_session_file_open,
        open_session_count: u8::from(matching_session_file_open)
            .saturating_add(u8::from(other_session_file_open)),
    })
}

#[cfg(target_os = "macos")]
fn grok_open_session_directory(path: &str) -> Option<&str> {
    if path.len() > SessionLocator::MAX_TEXT_BYTES || path.chars().any(char::is_control) {
        return None;
    }
    let path = Path::new(path);
    let session_directory = path.parent()?;
    (path.is_absolute()
        && matches!(
            path.file_name()?.to_str()?,
            "updates.jsonl" | "events.jsonl"
        )
        && session_directory
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(valid_grok_session_id)
        && is_grok_sessions_path(session_directory))
    .then(|| session_directory.to_str())
    .flatten()
}

fn is_grok_sessions_path(session_directory: &Path) -> bool {
    session_directory.ancestors().any(|ancestor| {
        ancestor.file_name().is_some_and(|name| name == "sessions")
            && ancestor
                .parent()
                .and_then(Path::file_name)
                .is_some_and(|name| name == ".grok")
    })
}
