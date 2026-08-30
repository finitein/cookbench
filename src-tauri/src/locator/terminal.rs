use std::{collections::HashMap, path::Path};

#[cfg(target_os = "macos")]
use std::time::{Duration, Instant};

use cookbench_core::{
    domain::HarnessId,
    locator::{HostApplication, SessionLocator, TerminalKind},
};

#[cfg(target_os = "macos")]
use super::run_bounded_for;

const MAX_ANCESTORS: usize = 64;
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
        }
    }
}

/// Adds an exact terminal target only when one running harness process is an
/// unambiguous match. Ambiguity preserves the existing lower-precision locator.
pub fn correlate_terminal_locator(
    harness: &HarnessId,
    mut locator: SessionLocator,
    processes: &[ObservedProcess],
) -> SessionLocator {
    if !matches!(harness, HarnessId::ClaudeCode | HarnessId::Pi) {
        return locator;
    }

    let by_id = processes
        .iter()
        .map(|process| (process.process_id, process))
        .collect::<HashMap<_, _>>();
    let expected_directory = locator.working_directory.as_deref().map(normalize_path);
    let mut matches = processes
        .iter()
        .filter(|process| harness_process(harness, &process.executable))
        .filter(|process| process.tty.as_deref().is_some_and(valid_tty))
        .filter(|process| match expected_directory {
            Some(expected) => process
                .working_directory
                .as_deref()
                .is_some_and(|actual| normalize_path(actual) == expected),
            None => true,
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
    matches.sort_by_key(|(process, _, _)| process.process_id);
    matches.dedup_by(|left, right| left.0.tty == right.0.tty);

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
    let processes = observe_processes(harness);
    correlate_terminal_locator(harness, locator, &processes)
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
        HarnessId::Codex | HarnessId::Other(_) => false,
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
fn observe_processes(harness: &HarnessId) -> Vec<ObservedProcess> {
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

    let mut processes = String::from_utf8_lossy(&output.stdout)
        .lines()
        .take(MAX_PROCESSES)
        .filter_map(parse_process_line)
        .collect::<Vec<_>>();
    let candidate_ids = processes
        .iter()
        .filter(|process| harness_process(harness, &process.executable))
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
