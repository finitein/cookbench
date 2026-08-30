//! Platform-neutral jump fallback orchestration.
//!
//! Actions are generated as bounded program/argument vectors or native focus
//! requests. This module never runs a shell, opens a port, or controls an
//! agent. Platform backends may decline any request and fall through.

pub mod linux;
pub mod macos;
mod terminal;
pub mod tmux;
pub mod vscode;
pub mod windows;

use std::{
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use cookbench_core::locator::{HostApplication, SessionLocator, TerminalKind};
use serde::Serialize;

pub use terminal::{correlate_terminal_locator, correlate_with_running_processes, ObservedProcess};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JumpAction {
    ExactPane {
        program: &'static str,
        args: Vec<String>,
    },
    ExactTerminalTab {
        terminal: TerminalKind,
        tty: String,
    },
    /// Requests a bounded Codex Desktop deep link. The OS URL dispatcher is
    /// asynchronous, so this never claims a verified thread focus.
    CodexDesktopThread {
        native_session_id: String,
    },
    ExactGhosttyTerminal {
        terminal_id: String,
    },
    ExactWezTermPane {
        pane_id: u64,
        control_endpoint: Option<String>,
    },
    ExactZellijPane {
        session_name: String,
        pane_id: String,
    },
    ExactCmuxPanel {
        panel_id: String,
        control_endpoint: Option<String>,
    },
    ApplicationWindow {
        application: &'static str,
    },
    ProjectDirectory {
        path: String,
    },
    ResumeInstructions {
        native_session_id: String,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JumpOutcome {
    /// A driver proved that its unique target is now selected.
    FocusedExact,
    VisibleFallback,
    NotRunning,
    NotFound,
    Ambiguous,
    PermissionDenied,
    TimedOut,
    VerificationFailed,
    Unsupported,
}

pub trait JumpExecutor {
    fn perform(&mut self, action: &JumpAction) -> JumpOutcome;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JumpResult {
    pub action: JumpAction,
    pub outcome: JumpOutcome,
}

/// The only data sent to the UI after an activation attempt. It never exposes
/// command arguments, paths, terminal metadata, or a native locator record.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocatorActivationResult {
    pub target: LocatorActivationTarget,
    pub status: LocatorActivationStatus,
    pub resume_session_id: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LocatorActivationTarget {
    ExactPane,
    ExactThread,
    ApplicationWindow,
    ProjectDirectory,
    ResumeInstructions,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LocatorActivationStatus {
    Focused,
    VisibleFallback,
    Unavailable,
}

/// Runs the documented order. A permission boundary or an elevated target is
/// not a terminal failure: users still receive a lower-precision jump option.
pub fn jump_with<E: JumpExecutor>(locator: &SessionLocator, executor: &mut E) -> JumpResult {
    for action in actions_for(locator) {
        let outcome = executor.perform(&action);
        if matches!(
            outcome,
            JumpOutcome::FocusedExact | JumpOutcome::VisibleFallback
        ) || matches!(action, JumpAction::ResumeInstructions { .. })
        {
            return JumpResult { action, outcome };
        }
    }

    unreachable!("a locator always ends with resume instructions")
}

pub fn activate_with<E: JumpExecutor>(
    locator: &SessionLocator,
    executor: &mut E,
) -> LocatorActivationResult {
    let JumpResult { action, outcome } = jump_with(locator, executor);
    let (target, resume_session_id) = match action {
        JumpAction::ExactPane { .. } => (LocatorActivationTarget::ExactPane, None),
        JumpAction::ExactTerminalTab { .. } => (LocatorActivationTarget::ExactPane, None),
        JumpAction::ExactGhosttyTerminal { .. }
        | JumpAction::ExactWezTermPane { .. }
        | JumpAction::ExactZellijPane { .. }
        | JumpAction::ExactCmuxPanel { .. } => (LocatorActivationTarget::ExactPane, None),
        JumpAction::CodexDesktopThread { .. } => (LocatorActivationTarget::ExactThread, None),
        JumpAction::ApplicationWindow { .. } => (LocatorActivationTarget::ApplicationWindow, None),
        JumpAction::ProjectDirectory { .. } => (LocatorActivationTarget::ProjectDirectory, None),
        JumpAction::ResumeInstructions { native_session_id } => (
            LocatorActivationTarget::ResumeInstructions,
            Some(native_session_id),
        ),
    };
    let status = match outcome {
        JumpOutcome::FocusedExact => LocatorActivationStatus::Focused,
        JumpOutcome::VisibleFallback => LocatorActivationStatus::VisibleFallback,
        JumpOutcome::NotRunning
        | JumpOutcome::NotFound
        | JumpOutcome::Ambiguous
        | JumpOutcome::PermissionDenied
        | JumpOutcome::TimedOut
        | JumpOutcome::VerificationFailed
        | JumpOutcome::Unsupported => LocatorActivationStatus::Unavailable,
    };
    LocatorActivationResult {
        target,
        status,
        resume_session_id,
    }
}

/// Executes only Cookbench-owned focus attempts. `Command` receives a program
/// and argument vector directly, so locator values are never shell source.
pub struct NativeJumpExecutor;

impl JumpExecutor for NativeJumpExecutor {
    fn perform(&mut self, action: &JumpAction) -> JumpOutcome {
        match action {
            JumpAction::ExactPane { program, args } => run(program, args),
            JumpAction::ExactTerminalTab { terminal, tty } => focus_terminal_tab(terminal, tty),
            JumpAction::CodexDesktopThread { native_session_id } => {
                open_codex_thread(native_session_id)
            }
            JumpAction::ExactGhosttyTerminal { terminal_id } => focus_ghostty_terminal(terminal_id),
            JumpAction::ExactWezTermPane {
                pane_id,
                control_endpoint,
            } => focus_wezterm_pane(*pane_id, control_endpoint.as_deref()),
            JumpAction::ExactZellijPane {
                session_name,
                pane_id,
            } => focus_zellij_pane(session_name, pane_id),
            JumpAction::ExactCmuxPanel {
                panel_id,
                control_endpoint,
            } => focus_cmux_panel(panel_id, control_endpoint.as_deref()),
            JumpAction::ApplicationWindow { application } => activate_application(application),
            JumpAction::ProjectDirectory { path } => open_project_directory(path),
            JumpAction::ResumeInstructions { .. } => JumpOutcome::VisibleFallback,
        }
    }
}

#[cfg(target_os = "macos")]
fn open_codex_thread(native_session_id: &str) -> JumpOutcome {
    let Some(encoded_id) = percent_encode_url_path_segment(native_session_id) else {
        return JumpOutcome::Unsupported;
    };
    run_visible_fallback(
        "open",
        &[
            "-b".to_owned(),
            macos::CODEX_DESKTOP_BUNDLE_ID.to_owned(),
            format!("codex://threads/{encoded_id}"),
        ],
    )
}

#[cfg(target_os = "windows")]
fn open_codex_thread(native_session_id: &str) -> JumpOutcome {
    let Some(encoded_id) = percent_encode_url_path_segment(native_session_id) else {
        return JumpOutcome::Unsupported;
    };
    run_visible_fallback("explorer.exe", &[format!("codex://threads/{encoded_id}")])
}

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
fn open_codex_thread(native_session_id: &str) -> JumpOutcome {
    let Some(encoded_id) = percent_encode_url_path_segment(native_session_id) else {
        return JumpOutcome::Unsupported;
    };
    run_visible_fallback("xdg-open", &[format!("codex://threads/{encoded_id}")])
}

fn percent_encode_url_path_segment(value: &str) -> Option<String> {
    if value.is_empty()
        || value.len() > SessionLocator::MAX_TEXT_BYTES
        || value.chars().any(char::is_control)
    {
        return None;
    }

    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                use std::fmt::Write;
                let _ = write!(encoded, "%{byte:02X}");
            }
        }
    }
    Some(encoded)
}

const FOCUS_TIMEOUT: Duration = Duration::from_secs(2);

fn run(program: &str, args: &[String]) -> JumpOutcome {
    let output = match run_bounded(program, args) {
        Ok(output) => output,
        Err(outcome) => return outcome,
    };
    if output.status.success() {
        JumpOutcome::FocusedExact
    } else {
        command_failure(&output.stderr)
    }
}

fn run_visible_fallback(program: &str, args: &[String]) -> JumpOutcome {
    let output = match run_bounded(program, args) {
        Ok(output) => output,
        Err(outcome) => return outcome,
    };
    if output.status.success() {
        JumpOutcome::VisibleFallback
    } else {
        command_failure(&output.stderr)
    }
}

fn run_bounded(program: &str, args: &[String]) -> Result<std::process::Output, JumpOutcome> {
    run_bounded_with_env(program, args, &[])
}

fn run_bounded_with_env(
    program: &str,
    args: &[String],
    environment: &[(&str, &str)],
) -> Result<std::process::Output, JumpOutcome> {
    run_bounded_for_with_env(program, args, environment, FOCUS_TIMEOUT)
}

#[cfg(any(target_os = "macos", test))]
pub(super) fn run_bounded_for(
    program: &str,
    args: &[String],
    timeout: Duration,
) -> Result<std::process::Output, JumpOutcome> {
    run_bounded_for_with_env(program, args, &[], timeout)
}

fn run_bounded_for_with_env(
    program: &str,
    args: &[String],
    environment: &[(&str, &str)],
    timeout: Duration,
) -> Result<std::process::Output, JumpOutcome> {
    let mut command = Command::new(program);
    let mut child = command
        .args(args)
        .envs(environment.iter().copied())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => JumpOutcome::Unsupported,
            std::io::ErrorKind::PermissionDenied => JumpOutcome::PermissionDenied,
            _ => JumpOutcome::Unsupported,
        })?;
    let deadline = Instant::now() + timeout;
    loop {
        if child
            .try_wait()
            .map_err(|_| JumpOutcome::VerificationFailed)?
            .is_some()
        {
            return child
                .wait_with_output()
                .map_err(|_| JumpOutcome::VerificationFailed);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(JumpOutcome::TimedOut);
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn command_failure(stderr: &[u8]) -> JumpOutcome {
    let stderr = String::from_utf8_lossy(stderr);
    if stderr.contains("-1743") || stderr.contains("Not authorized") {
        JumpOutcome::PermissionDenied
    } else if stderr.contains("-600") || stderr.contains("isn't running") {
        JumpOutcome::NotRunning
    } else {
        JumpOutcome::VerificationFailed
    }
}

#[cfg(target_os = "macos")]
fn focus_terminal_tab(terminal: &TerminalKind, tty: &str) -> JumpOutcome {
    const TERMINAL_SCRIPT: &str = r#"on run argv
set targetTTY to item 1 of argv
if application id "com.apple.Terminal" is not running then return "not-running"
tell application "Terminal"
  set matchCount to 0
  set matchedWindow to missing value
  set matchedTab to missing value
  repeat with targetWindow in windows
    repeat with targetTab in tabs of targetWindow
      if tty of targetTab is targetTTY then
        set matchCount to matchCount + 1
        set matchedWindow to targetWindow
        set matchedTab to targetTab
      end if
    end repeat
  end repeat
  if matchCount is 0 then return "not-found"
  if matchCount is not 1 then return "ambiguous"
  set selected tab of matchedWindow to matchedTab
  set frontmost of matchedWindow to true
  activate
  delay 0.05
  if tty of selected tab of front window is not targetTTY then return "verification-failed"
  return "focused"
end tell
end run"#;
    const ITERM_SCRIPT: &str = r#"on run argv
set targetTTY to item 1 of argv
if application id "com.googlecode.iterm2" is not running then return "not-running"
tell application "iTerm2"
  set matchCount to 0
  set matchedWindow to missing value
  set matchedTab to missing value
  set matchedSession to missing value
  repeat with targetWindow in windows
    repeat with targetTab in tabs of targetWindow
      repeat with targetSession in sessions of targetTab
        if tty of targetSession is targetTTY then
          set matchCount to matchCount + 1
          set matchedWindow to targetWindow
          set matchedTab to targetTab
          set matchedSession to targetSession
        end if
      end repeat
    end repeat
  end repeat
  if matchCount is 0 then return "not-found"
  if matchCount is not 1 then return "ambiguous"
  select matchedSession
  select matchedTab
  select matchedWindow
  activate
  delay 0.05
  if tty of current session of current tab of current window is not targetTTY then return "verification-failed"
  return "focused"
end tell
end run"#;
    let script = match terminal {
        TerminalKind::MacosTerminal => TERMINAL_SCRIPT,
        TerminalKind::ITerm2 => ITERM_SCRIPT,
        _ => return JumpOutcome::Unsupported,
    };
    let output = match run_bounded(
        "/usr/bin/osascript",
        &[
            "-e".to_owned(),
            script.to_owned(),
            "--".to_owned(),
            tty.to_owned(),
        ],
    ) {
        Ok(output) => output,
        Err(outcome) => return outcome,
    };
    if !output.status.success() {
        return command_failure(&output.stderr);
    }
    terminal_script_outcome(&output.stdout)
}

#[cfg(target_os = "macos")]
fn terminal_script_outcome(stdout: &[u8]) -> JumpOutcome {
    match String::from_utf8_lossy(stdout).trim() {
        "focused" => JumpOutcome::FocusedExact,
        "not-running" => JumpOutcome::NotRunning,
        "not-found" => JumpOutcome::NotFound,
        "ambiguous" => JumpOutcome::Ambiguous,
        "verification-failed" => JumpOutcome::VerificationFailed,
        _ => JumpOutcome::VerificationFailed,
    }
}

#[cfg(target_os = "macos")]
fn focus_ghostty_terminal(terminal_id: &str) -> JumpOutcome {
    const GHOSTTY_SCRIPT: &str = r#"on run argv
set targetID to item 1 of argv
if application "Ghostty" is not running then return "not-running"
tell application "Ghostty"
  set matchCount to 0
  set matchedTerminal to missing value
  repeat with targetTerminal in terminals
    if (id of targetTerminal as text) is targetID then
      set matchCount to matchCount + 1
      set matchedTerminal to targetTerminal
    end if
  end repeat
  if matchCount is 0 then return "not-found"
  if matchCount is not 1 then return "ambiguous"
  focus matchedTerminal
  delay 0.05
  set focusedTerminal to focused terminal of selected tab of front window
  if (id of focusedTerminal as text) is not targetID then return "verification-failed"
  return "focused"
end tell
end run"#;
    let output = match run_bounded(
        "/usr/bin/osascript",
        &[
            "-e".to_owned(),
            GHOSTTY_SCRIPT.to_owned(),
            "--".to_owned(),
            terminal_id.to_owned(),
        ],
    ) {
        Ok(output) => output,
        Err(outcome) => return outcome,
    };
    if !output.status.success() {
        return command_failure(&output.stderr);
    }
    ghostty_script_outcome(&output.stdout)
}

#[cfg(target_os = "macos")]
fn ghostty_script_outcome(stdout: &[u8]) -> JumpOutcome {
    terminal_script_outcome(stdout)
}

#[cfg(not(target_os = "macos"))]
fn focus_ghostty_terminal(_terminal_id: &str) -> JumpOutcome {
    JumpOutcome::Unsupported
}

fn focus_wezterm_pane(pane_id: u64, control_endpoint: Option<&str>) -> JumpOutcome {
    let env = control_endpoint
        .map(|endpoint| vec![("WEZTERM_UNIX_SOCKET", endpoint)])
        .unwrap_or_default();
    let list_args = vec![
        "cli".to_owned(),
        "list".to_owned(),
        "--format".to_owned(),
        "json".to_owned(),
    ];
    let before = match run_bounded_with_env("wezterm", &list_args, &env) {
        Ok(output) if output.status.success() => output,
        Ok(output) => return command_failure(&output.stderr),
        Err(outcome) => return outcome,
    };
    if !json_has_integer_field(&before.stdout, "pane_id", pane_id) {
        return JumpOutcome::NotFound;
    }
    let focus_args = vec![
        "cli".to_owned(),
        "activate-pane".to_owned(),
        "--pane-id".to_owned(),
        pane_id.to_string(),
    ];
    let focused = match run_bounded_with_env("wezterm", &focus_args, &env) {
        Ok(output) if output.status.success() => output,
        Ok(output) => return command_failure(&output.stderr),
        Err(outcome) => return outcome,
    };
    let _ = focused;
    let clients_args = vec![
        "cli".to_owned(),
        "list-clients".to_owned(),
        "--format".to_owned(),
        "json".to_owned(),
    ];
    match run_bounded_with_env("wezterm", &clients_args, &env) {
        Ok(output)
            if output.status.success()
                && json_has_integer_field(&output.stdout, "focused_pane_id", pane_id) =>
        {
            JumpOutcome::FocusedExact
        }
        Ok(output) if output.status.success() => JumpOutcome::VerificationFailed,
        Ok(output) => command_failure(&output.stderr),
        Err(outcome) => outcome,
    }
}

fn focus_zellij_pane(session_name: &str, pane_id: &str) -> JumpOutcome {
    let list_args = vec![
        "--session".to_owned(),
        session_name.to_owned(),
        "list-panes".to_owned(),
        "--json".to_owned(),
    ];
    let before = match run_bounded("zellij", &list_args) {
        Ok(output) if output.status.success() => output,
        Ok(output) => return command_failure(&output.stderr),
        Err(outcome) => return outcome,
    };
    if !json_has_string_field(&before.stdout, "terminal_id", pane_id) {
        return JumpOutcome::NotFound;
    }
    let focus_args = vec![
        "--session".to_owned(),
        session_name.to_owned(),
        "action".to_owned(),
        "focus-pane-id".to_owned(),
        pane_id.to_owned(),
    ];
    match run_bounded("zellij", &focus_args) {
        Ok(output) if output.status.success() => {}
        Ok(output) => return command_failure(&output.stderr),
        Err(outcome) => return outcome,
    }
    match run_bounded("zellij", &list_args) {
        Ok(output)
            if output.status.success()
                && json_has_focused_string_field(&output.stdout, "terminal_id", pane_id) =>
        {
            JumpOutcome::FocusedExact
        }
        Ok(output) if output.status.success() => JumpOutcome::VerificationFailed,
        Ok(output) => command_failure(&output.stderr),
        Err(outcome) => outcome,
    }
}

fn focus_cmux_panel(panel_id: &str, control_endpoint: Option<&str>) -> JumpOutcome {
    let env = control_endpoint
        .map(|endpoint| vec![("CMUX_SOCKET", endpoint)])
        .unwrap_or_default();
    let list_args = vec!["list-panels".to_owned(), "--json".to_owned()];
    let before = match run_bounded_with_env("cmux", &list_args, &env) {
        Ok(output) if output.status.success() => output,
        Ok(output) => return command_failure(&output.stderr),
        Err(outcome) => return outcome,
    };
    if !json_has_string_field(&before.stdout, "id", panel_id) {
        return JumpOutcome::NotFound;
    }
    let focus_args = vec![
        "focus-panel".to_owned(),
        "--panel".to_owned(),
        panel_id.to_owned(),
    ];
    match run_bounded_with_env("cmux", &focus_args, &env) {
        Ok(output) if output.status.success() => {}
        Ok(output) => return command_failure(&output.stderr),
        Err(outcome) => return outcome,
    }
    match run_bounded_with_env("cmux", &list_args, &env) {
        Ok(output)
            if output.status.success()
                && json_has_focused_string_field(&output.stdout, "id", panel_id) =>
        {
            JumpOutcome::FocusedExact
        }
        Ok(output) if output.status.success() => JumpOutcome::VerificationFailed,
        Ok(output) => command_failure(&output.stderr),
        Err(outcome) => outcome,
    }
}

fn json_has_integer_field(input: &[u8], field: &str, target: u64) -> bool {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(input) else {
        return false;
    };
    json_value_has_integer_field(&value, field, target)
}

fn json_has_string_field(input: &[u8], field: &str, target: &str) -> bool {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(input) else {
        return false;
    };
    json_value_has_string_field(&value, field, target)
}

fn json_value_has_string_field(value: &serde_json::Value, field: &str, target: &str) -> bool {
    match value {
        serde_json::Value::Object(map) => {
            map.get(field).and_then(serde_json::Value::as_str) == Some(target)
                || map
                    .values()
                    .any(|child| json_value_has_string_field(child, field, target))
        }
        serde_json::Value::Array(values) => values
            .iter()
            .any(|child| json_value_has_string_field(child, field, target)),
        _ => false,
    }
}

fn json_has_focused_string_field(input: &[u8], field: &str, target: &str) -> bool {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(input) else {
        return false;
    };
    json_value_has_focused_string_field(&value, field, target)
}

fn json_value_has_focused_string_field(
    value: &serde_json::Value,
    field: &str,
    target: &str,
) -> bool {
    match value {
        serde_json::Value::Object(map) => {
            let matches_target = map.get(field).and_then(serde_json::Value::as_str) == Some(target);
            let is_focused = ["focused", "is_focused", "active"]
                .iter()
                .any(|key| map.get(*key).and_then(serde_json::Value::as_bool) == Some(true));
            (matches_target && is_focused)
                || map
                    .values()
                    .any(|child| json_value_has_focused_string_field(child, field, target))
        }
        serde_json::Value::Array(values) => values
            .iter()
            .any(|child| json_value_has_focused_string_field(child, field, target)),
        _ => false,
    }
}

fn json_value_has_integer_field(value: &serde_json::Value, field: &str, target: u64) -> bool {
    match value {
        serde_json::Value::Object(map) => {
            map.get(field).and_then(serde_json::Value::as_u64) == Some(target)
                || map
                    .values()
                    .any(|child| json_value_has_integer_field(child, field, target))
        }
        serde_json::Value::Array(values) => values
            .iter()
            .any(|child| json_value_has_integer_field(child, field, target)),
        _ => false,
    }
}

#[cfg(not(target_os = "macos"))]
fn focus_terminal_tab(_terminal: &TerminalKind, _tty: &str) -> JumpOutcome {
    JumpOutcome::Unsupported
}

#[cfg(target_os = "macos")]
fn activate_application(application: &str) -> JumpOutcome {
    let flag = if application.contains('.') {
        "-b"
    } else {
        "-a"
    };
    run_visible_fallback("open", &[flag.to_owned(), application.to_owned()])
}

#[cfg(target_os = "windows")]
fn activate_application(application: &str) -> JumpOutcome {
    let Some(process_name) = windows_process_name(application) else {
        return JumpOutcome::Unsupported;
    };
    let args = [
        "-NoProfile".to_owned(),
        "-NonInteractive".to_owned(),
        "-Command".to_owned(),
        WINDOWS_ACTIVATION_SCRIPT.to_owned(),
    ];
    match run_bounded_with_env(
        "powershell.exe",
        &args,
        &[("COOKBENCH_TARGET_PROCESS", process_name)],
    ) {
        Ok(output) if output.status.success() => JumpOutcome::FocusedExact,
        Ok(output) => match output.status.code() {
            Some(3) => JumpOutcome::NotRunning,
            Some(4) => JumpOutcome::Ambiguous,
            Some(5) => JumpOutcome::PermissionDenied,
            Some(6) => JumpOutcome::VerificationFailed,
            _ => command_failure(&output.stderr),
        },
        Err(outcome) => outcome,
    }
}

#[cfg(any(target_os = "windows", test))]
const WINDOWS_ACTIVATION_SCRIPT: &str = r#"
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class CookbenchWindow {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
}
'@
$matches = @(Get-Process -Name $env:COOKBENCH_TARGET_PROCESS -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 })
if ($matches.Count -eq 0) { exit 3 }
if ($matches.Count -ne 1) { exit 4 }
$handle = $matches[0].MainWindowHandle
if (-not [CookbenchWindow]::SetForegroundWindow($handle)) { exit 5 }
Start-Sleep -Milliseconds 80
if ([CookbenchWindow]::GetForegroundWindow() -ne $handle) { exit 6 }
"#;

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
fn activate_application(application: &str) -> JumpOutcome {
    linux_desktop_id(application)
        .map(|desktop_id| run_visible_fallback("gtk-launch", &[desktop_id.to_owned()]))
        .unwrap_or(JumpOutcome::Unsupported)
}

#[cfg(any(target_os = "windows", test))]
fn windows_process_name(application: &str) -> Option<&'static str> {
    match application {
        windows::WINDOWS_TERMINAL_APP => Some("WindowsTerminal"),
        vscode::VSCODE_APP => Some("Code"),
        macos::WEZTERM_APP => Some("wezterm-gui"),
        macos::GHOSTTY_APP => Some("ghostty"),
        macos::CODEX_DESKTOP_BUNDLE_ID => Some("Codex"),
        _ => None,
    }
}

#[cfg(any(all(not(target_os = "macos"), not(target_os = "windows")), test))]
fn linux_desktop_id(application: &str) -> Option<&'static str> {
    match application {
        linux::GNOME_TERMINAL_APP => Some("org.gnome.Terminal.desktop"),
        linux::KONSOLE_APP => Some("org.kde.konsole.desktop"),
        linux::XFCE_TERMINAL_APP => Some("xfce4-terminal.desktop"),
        vscode::VSCODE_APP => Some("code.desktop"),
        macos::WEZTERM_APP => Some("org.wezfurlong.wezterm.desktop"),
        macos::GHOSTTY_APP => Some("com.mitchellh.ghostty.desktop"),
        _ => None,
    }
}

#[cfg(target_os = "macos")]
fn open_project_directory(path: &str) -> JumpOutcome {
    run_visible_fallback("open", &[path.to_owned()])
}

#[cfg(target_os = "windows")]
fn open_project_directory(path: &str) -> JumpOutcome {
    run_visible_fallback("explorer.exe", &[path.to_owned()])
}

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
fn open_project_directory(path: &str) -> JumpOutcome {
    run_visible_fallback("xdg-open", &[path.to_owned()])
}

/// The truthful fallback sequence for a locator. Unsupported terminal and IDE
/// integrations simply omit an exact target rather than pretending precision.
pub fn actions_for(locator: &SessionLocator) -> Vec<JumpAction> {
    if locator.validate().is_err() {
        return vec![JumpAction::ResumeInstructions {
            native_session_id: locator.native_session_id.clone(),
        }];
    }

    let mut actions = Vec::with_capacity(5);

    if matches!(
        locator.host_application,
        Some(HostApplication::CodexDesktop)
    ) {
        actions.push(JumpAction::CodexDesktopThread {
            native_session_id: locator.native_session_id.clone(),
        });
    }

    if matches!(locator.terminal.as_ref(), Some(TerminalKind::Tmux)) {
        if let Some(action) = tmux::exact_pane_action(locator.tmux_pane.as_deref()) {
            actions.push(action);
        }
    }

    if let (Some(terminal), Some(tty)) = (locator.terminal.as_ref(), locator.tty.as_ref()) {
        if matches!(terminal, TerminalKind::MacosTerminal | TerminalKind::ITerm2) {
            actions.push(JumpAction::ExactTerminalTab {
                terminal: terminal.clone(),
                tty: tty.clone(),
            });
        }
    }

    match locator.terminal.as_ref() {
        Some(TerminalKind::Ghostty) => {
            if let Some(terminal_id) = valid_opaque_selector(locator.terminal_pane_id.as_deref()) {
                actions.push(JumpAction::ExactGhosttyTerminal { terminal_id });
            }
        }
        Some(TerminalKind::WezTerm) => {
            if let Some(pane_id) = locator
                .terminal_pane_id
                .as_deref()
                .and_then(|value| value.parse::<u64>().ok())
            {
                actions.push(JumpAction::ExactWezTermPane {
                    pane_id,
                    control_endpoint: valid_opaque_selector(
                        locator.terminal_control_endpoint.as_deref(),
                    ),
                });
            }
        }
        Some(TerminalKind::Zellij) => {
            if let (Some(session_name), Some(pane_id)) = (
                valid_opaque_selector(locator.terminal_session_id.as_deref()),
                valid_opaque_selector(locator.terminal_pane_id.as_deref()),
            ) {
                actions.push(JumpAction::ExactZellijPane {
                    session_name,
                    pane_id,
                });
            }
        }
        Some(TerminalKind::Cmux) => {
            if let Some(panel_id) = valid_opaque_selector(locator.terminal_pane_id.as_deref()) {
                actions.push(JumpAction::ExactCmuxPanel {
                    panel_id,
                    control_endpoint: valid_opaque_selector(
                        locator.terminal_control_endpoint.as_deref(),
                    ),
                });
            }
        }
        _ => {}
    }

    if let Some(application) = application_for(locator.host_application.as_ref()) {
        actions.push(JumpAction::ApplicationWindow { application });
    }

    if let Some(path) = locator
        .ide_workspace
        .as_ref()
        .or(locator.working_directory.as_ref())
    {
        actions.push(JumpAction::ProjectDirectory { path: path.clone() });
    }

    actions.push(JumpAction::ResumeInstructions {
        native_session_id: locator.native_session_id.clone(),
    });
    actions
}

fn valid_opaque_selector(value: Option<&str>) -> Option<String> {
    let value = value?;
    (value.len() <= SessionLocator::MAX_TEXT_BYTES
        && !value.is_empty()
        && !value.chars().any(char::is_control))
    .then(|| value.to_owned())
}

fn application_for(application: Option<&HostApplication>) -> Option<&'static str> {
    match application {
        Some(HostApplication::CodexDesktop) => Some(macos::CODEX_DESKTOP_BUNDLE_ID),
        Some(HostApplication::MacosTerminal) => Some(macos::TERMINAL_APP),
        Some(HostApplication::ITerm2) => Some(macos::ITERM_APP),
        Some(HostApplication::Ghostty) => Some(macos::GHOSTTY_APP),
        Some(HostApplication::WezTerm) => Some(macos::WEZTERM_APP),
        Some(HostApplication::Zellij) => None,
        Some(HostApplication::Cmux) => Some(macos::CMUX_APP),
        Some(HostApplication::WindowsTerminal) => Some(windows::WINDOWS_TERMINAL_APP),
        Some(HostApplication::VisualStudioCode) => Some(vscode::VSCODE_APP),
        Some(HostApplication::GnomeTerminal) => Some(linux::GNOME_TERMINAL_APP),
        Some(HostApplication::Konsole) => Some(linux::KONSOLE_APP),
        Some(HostApplication::XfceTerminal) => Some(linux::XFCE_TERMINAL_APP),
        Some(HostApplication::Other(_)) | None => None,
    }
}

#[cfg(test)]
mod driver_tests {
    use super::{
        command_failure, json_has_focused_string_field, json_has_integer_field, linux_desktop_id,
        percent_encode_url_path_segment, run_bounded_for, windows_process_name, JumpOutcome,
        WINDOWS_ACTIVATION_SCRIPT,
    };
    use std::time::Duration;

    #[test]
    fn classifies_automation_permission_failures_without_claiming_focus() {
        assert_eq!(
            command_failure(b"execution error: Not authorized to send Apple events. (-1743)"),
            JumpOutcome::PermissionDenied
        );
    }

    #[test]
    fn bounded_runner_kills_a_timed_out_driver() {
        #[cfg(windows)]
        let (program, args) = (
            "powershell.exe",
            vec![
                "-NoProfile".to_owned(),
                "-Command".to_owned(),
                "Start-Sleep -Seconds 1".to_owned(),
            ],
        );
        #[cfg(not(windows))]
        let (program, args) = ("/bin/sleep", vec!["1".to_owned()]);

        let result = run_bounded_for(program, &args, Duration::from_millis(5));
        assert_eq!(result.unwrap_err(), JumpOutcome::TimedOut);
    }

    #[test]
    fn driver_postconditions_require_the_exact_selected_identity() {
        assert!(json_has_integer_field(
            br#"[{"pane_id":42}]"#,
            "pane_id",
            42
        ));
        assert!(json_has_focused_string_field(
            br#"[{"terminal_id":"terminal_3","is_focused":true}]"#,
            "terminal_id",
            "terminal_3"
        ));
        assert!(!json_has_focused_string_field(
            br#"[{"terminal_id":"terminal_3","is_focused":false}]"#,
            "terminal_id",
            "terminal_3"
        ));
    }

    #[test]
    fn platform_application_fallbacks_are_allowlisted_not_user_commands() {
        assert_eq!(
            windows_process_name("Windows Terminal"),
            Some("WindowsTerminal")
        );
        assert_eq!(
            linux_desktop_id("GNOME Terminal"),
            Some("org.gnome.Terminal.desktop")
        );
        assert_eq!(windows_process_name("arbitrary.exe --unsafe"), None);
        assert_eq!(linux_desktop_id("arbitrary.desktop"), None);
        assert!(WINDOWS_ACTIVATION_SCRIPT.contains("$env:COOKBENCH_TARGET_PROCESS"));
        assert!(!WINDOWS_ACTIVATION_SCRIPT.contains("$args[0]"));
    }

    #[test]
    fn codex_thread_identifier_is_a_path_segment_not_url_source() {
        assert_eq!(
            percent_encode_url_path_segment("thread/with?query#fragment").as_deref(),
            Some("thread%2Fwith%3Fquery%23fragment")
        );
        assert_eq!(percent_encode_url_path_segment("thread\nother"), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn terminal_driver_reports_only_a_postcondition_as_exact() {
        assert_eq!(
            super::terminal_script_outcome(b"focused\n"),
            JumpOutcome::FocusedExact
        );
        assert_eq!(
            super::terminal_script_outcome(b"ambiguous\n"),
            JumpOutcome::Ambiguous
        );
        assert_eq!(
            super::terminal_script_outcome(b"verification-failed\n"),
            JumpOutcome::VerificationFailed
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn ghostty_driver_requires_the_selected_terminal_id_postcondition() {
        assert_eq!(
            super::ghostty_script_outcome(b"focused\n"),
            JumpOutcome::FocusedExact
        );
        assert_eq!(
            super::ghostty_script_outcome(b"ambiguous\n"),
            JumpOutcome::Ambiguous
        );
        assert_eq!(
            super::ghostty_script_outcome(b"verification-failed\n"),
            JumpOutcome::VerificationFailed
        );
    }
}
