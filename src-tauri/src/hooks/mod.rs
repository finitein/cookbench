//! Optional harness-hook configuration.
//!
//! This module only manages Cookbench-owned hook entries.  It never invokes a
//! shell, replaces an existing single callback, or stores hook payloads.

use std::{
    env,
    fs::{self, OpenOptions},
    io::{self, BufReader, Read, Write},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use cookbench_adapters::{
    claude::{install_hooks_with_command, uninstall_all_cookbench_hooks},
    codex::{inspect_notify_hook, NotifyHookPlan},
};
use serde::{Deserialize, Serialize};

const CLAUDE_HOOK_ARGS: &[&str] = &["--harness", "claude-code"];
const STALE_HOOK_AFTER: Duration = Duration::from_secs(15 * 60);
const MAX_HOOK_HELPER_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum HookHarness {
    Codex,
    ClaudeCode,
    Pi,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum HookHealth {
    Detected,
    NotInstalled,
    Healthy,
    Outdated,
    Conflicted,
    Unwritable,
    NoRecentEvents,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HookStatus {
    pub harness: HookHarness,
    pub label: &'static str,
    pub health: HookHealth,
    pub config_display: String,
    pub detail: String,
    pub can_install: bool,
    pub can_repair: bool,
    pub can_uninstall: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HookAction {
    PreviewInstall,
    Install,
    Repair,
    Uninstall,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HookActionResult {
    pub status: HookStatus,
    pub changed: bool,
    pub preview: Option<String>,
    pub backup_display: Option<String>,
}

#[derive(Debug)]
pub enum HookError {
    Unsupported(HookHarness),
    Conflict(String),
    InvalidConfiguration(String),
    Io(io::Error),
}

impl std::fmt::Display for HookError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(harness) => write!(
                formatter,
                "{harness:?} does not expose a safe hook configuration"
            ),
            Self::Conflict(message) | Self::InvalidConfiguration(message) => {
                formatter.write_str(message)
            }
            Self::Io(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for HookError {}

impl From<io::Error> for HookError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

pub fn statuses() -> Vec<HookStatus> {
    let helper = hook_program();
    let helper_is_current = helper_is_current(&packaged_hook_program(), &helper);
    [HookHarness::Codex, HookHarness::ClaudeCode, HookHarness::Pi]
        .into_iter()
        .map(|harness| with_helper_health(status_with_helper(harness, &helper), helper_is_current))
        .collect()
}

pub fn status(harness: HookHarness) -> HookStatus {
    let helper = hook_program();
    with_helper_health(
        status_with_helper(harness, &helper),
        helper_is_current(&packaged_hook_program(), &helper),
    )
}

fn status_with_helper(harness: HookHarness, helper: &Path) -> HookStatus {
    match harness {
        HookHarness::Codex => codex_status_with_helper(&codex_config_path(), helper),
        HookHarness::ClaudeCode => claude_status_with_helper(&claude_config_path(), helper),
        HookHarness::Pi => pi_status_with_helper(&pi_extension_path(), helper),
    }
}

pub fn apply(harness: HookHarness, action: HookAction) -> Result<HookActionResult, HookError> {
    let helper = hook_program();
    if matches!(action, HookAction::Install | HookAction::Repair) {
        ensure_managed_helper(&packaged_hook_program(), &helper)?;
    }
    match harness {
        HookHarness::Codex => apply_codex_with_helper(action, &codex_config_path(), &helper),
        HookHarness::ClaudeCode => apply_claude_with_helper(action, &claude_config_path(), &helper),
        HookHarness::Pi => apply_pi_with_helper(action, &pi_extension_path(), &helper),
    }
}

#[cfg(test)]
fn codex_status(path: &Path) -> HookStatus {
    codex_status_with_helper(path, &hook_program())
}

fn codex_status_with_helper(path: &Path, helper: &Path) -> HookStatus {
    let base = HookStatus {
        harness: HookHarness::Codex,
        label: "Codex",
        health: HookHealth::NotInstalled,
        config_display: display_path(path),
        detail: "No Cookbench notify hook is installed.".into(),
        can_install: true,
        can_repair: false,
        can_uninstall: false,
    };
    let Ok(config) = fs::read_to_string(path) else {
        return if path.exists() {
            unwritable(base)
        } else {
            base
        };
    };
    let Some(expected) = codex_hook_command(helper) else {
        return helper_unavailable(base);
    };
    match inspect_notify_hook(&config, &expected) {
        NotifyHookPlan::NotConfigured => base,
        NotifyHookPlan::Chain {
            existing_command,
            cookbench_command,
        } if existing_command == cookbench_command => with_event_health(HookStatus {
            health: HookHealth::Healthy,
            detail: "Cookbench receives lifecycle notifications through Codex notify.".into(),
            can_install: false,
            can_repair: true,
            can_uninstall: true,
            ..base
        }),
        NotifyHookPlan::Chain { existing_command, .. }
            if existing_command.iter().any(|part| part.contains("cookbench-hook")) =>
        {
            HookStatus {
                health: HookHealth::Outdated,
                detail: "An older Cookbench notify hook is present; repair will replace only the Cookbench-owned callback."
                    .into(),
                can_install: false,
                can_repair: true,
                can_uninstall: true,
                ..base
            }
        }
        NotifyHookPlan::Chain { .. } => HookStatus {
            health: HookHealth::Conflicted,
            detail: "Codex has another notify callback. Cookbench will not replace or shell-chain it."
                .into(),
            can_install: false,
            ..base
        },
        NotifyHookPlan::ReadOnlyFallback { reason } => HookStatus {
            health: HookHealth::Conflicted,
            detail: reason.to_owned(),
            can_install: false,
            ..base
        },
    }
}

fn claude_status_with_helper(path: &Path, helper: &Path) -> HookStatus {
    let base = HookStatus {
        harness: HookHarness::ClaudeCode,
        label: "Claude Code",
        health: HookHealth::NotInstalled,
        config_display: display_path(path),
        detail: "No Cookbench lifecycle hooks are installed.".into(),
        can_install: true,
        can_repair: false,
        can_uninstall: false,
    };
    let Ok(config) = fs::read_to_string(path) else {
        return if path.exists() {
            unwritable(base)
        } else {
            base
        };
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&config) else {
        return HookStatus {
            health: HookHealth::Conflicted,
            detail: "Claude settings are not valid JSON; Cookbench will not rewrite them.".into(),
            can_install: false,
            ..base
        };
    };
    let Some(helper) = helper.to_str() else {
        return helper_unavailable(base);
    };
    let has_other_cookbench_hook = contains_other_cookbench_hook(&value, helper, CLAUDE_HOOK_ARGS);
    match install_hooks_with_command(&value, helper, CLAUDE_HOOK_ARGS) {
        Ok(mutation) if !mutation.changed && !has_other_cookbench_hook => {
            with_event_health(HookStatus {
                health: HookHealth::Healthy,
                detail: "Cookbench lifecycle hooks are installed.".into(),
                can_install: false,
                can_repair: true,
                can_uninstall: true,
                ..base
            })
        }
        Ok(_) if contains_cookbench_hook(&value) => HookStatus {
            health: HookHealth::Outdated,
            detail:
                "An older Cookbench hook entry is present; repair preserves unrelated Claude hooks."
                    .into(),
            can_install: false,
            can_repair: true,
            can_uninstall: true,
            ..base
        },
        Ok(_) => base,
        Err(error) => HookStatus {
            health: HookHealth::Conflicted,
            detail: error.to_string(),
            can_install: false,
            ..base
        },
    }
}

#[cfg(test)]
fn apply_codex(action: HookAction, path: &Path) -> Result<HookActionResult, HookError> {
    apply_codex_with_helper(action, path, &hook_program())
}

fn apply_codex_with_helper(
    action: HookAction,
    path: &Path,
    helper: &Path,
) -> Result<HookActionResult, HookError> {
    let current = read_optional(path)?;
    let expected = codex_hook_command(helper).ok_or_else(|| {
        HookError::InvalidConfiguration("packaged hook helper path is not valid UTF-8".into())
    })?;
    let plan = inspect_notify_hook(&current, &expected);
    let next = match action {
        HookAction::Uninstall => remove_codex_hook(&current, &expected)?,
        HookAction::PreviewInstall | HookAction::Install | HookAction::Repair => match plan {
            NotifyHookPlan::NotConfigured => append_codex_hook(&current, &expected)?,
            NotifyHookPlan::Chain {
                existing_command,
                cookbench_command,
            } if existing_command == cookbench_command => current.clone(),
            NotifyHookPlan::Chain {
                existing_command, ..
            } if action == HookAction::Repair
                && existing_command
                    .iter()
                    .any(|part| part.contains("cookbench-hook")) =>
            {
                append_codex_hook(&remove_codex_hook(&current, &expected)?, &expected)?
            }
            NotifyHookPlan::Chain { .. } => {
                return Err(HookError::Conflict(
                    "Codex has an existing notify callback; Cookbench refuses to replace it."
                        .into(),
                ))
            }
            NotifyHookPlan::ReadOnlyFallback { reason } => {
                return Err(HookError::Conflict(reason.into()))
            }
        },
    };
    finish_action(HookHarness::Codex, action, path, current, next)
}

#[cfg(test)]
fn apply_claude(action: HookAction, path: &Path) -> Result<HookActionResult, HookError> {
    apply_claude_with_helper(action, path, &hook_program())
}

fn apply_claude_with_helper(
    action: HookAction,
    path: &Path,
    helper: &Path,
) -> Result<HookActionResult, HookError> {
    let current = read_optional(path)?;
    let value = if current.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&current)
            .map_err(|error| HookError::InvalidConfiguration(error.to_string()))?
    };
    let helper = helper.to_str().ok_or_else(|| {
        HookError::InvalidConfiguration("packaged hook helper path is not valid UTF-8".into())
    })?;
    let mutation = match action {
        HookAction::Uninstall => uninstall_all_cookbench_hooks(&value),
        HookAction::Repair => {
            let cleaned = uninstall_all_cookbench_hooks(&value)
                .map_err(|error| HookError::InvalidConfiguration(error.to_string()))?
                .configuration;
            install_hooks_with_command(&cleaned, helper, CLAUDE_HOOK_ARGS)
        }
        HookAction::PreviewInstall | HookAction::Install => {
            install_hooks_with_command(&value, helper, CLAUDE_HOOK_ARGS)
        }
    }
    .map_err(|error| HookError::InvalidConfiguration(error.to_string()))?;
    let next = serde_json::to_string_pretty(&mutation.configuration)
        .map_err(|error| HookError::InvalidConfiguration(error.to_string()))?
        + "\n";
    finish_action(HookHarness::ClaudeCode, action, path, current, next)
}

fn pi_status_with_helper(path: &Path, helper: &Path) -> HookStatus {
    let base = HookStatus {
        harness: HookHarness::Pi,
        label: "Pi",
        health: HookHealth::NotInstalled,
        config_display: display_path(path),
        detail: "No Cookbench Pi lifecycle extension is installed.".into(),
        can_install: true,
        can_repair: false,
        can_uninstall: false,
    };
    let Ok(current) = fs::read_to_string(path) else {
        return if path.exists() {
            unwritable(base)
        } else {
            base
        };
    };
    let Ok(expected) = pi_extension_source(helper) else {
        return helper_unavailable(base);
    };
    if current == expected {
        return with_event_health(HookStatus {
            health: HookHealth::Healthy,
            detail: "Cookbench receives metadata-only Pi lifecycle events.".into(),
            can_install: false,
            can_repair: true,
            can_uninstall: true,
            ..base
        });
    }
    if is_cookbench_pi_extension(&current) {
        return HookStatus {
            health: HookHealth::Outdated,
            detail: "An older Cookbench Pi extension is present; repair replaces only that file."
                .into(),
            can_install: false,
            can_repair: true,
            can_uninstall: true,
            ..base
        };
    }
    HookStatus {
        health: HookHealth::Conflicted,
        detail: "A non-Cookbench file already uses the Cookbench Pi extension path.".into(),
        can_install: false,
        ..base
    }
}

fn apply_pi_with_helper(
    action: HookAction,
    path: &Path,
    helper: &Path,
) -> Result<HookActionResult, HookError> {
    let current = read_optional(path)?;
    let expected = pi_extension_source(helper)?;
    if !current.is_empty() && current != expected && !is_cookbench_pi_extension(&current) {
        return Err(HookError::Conflict(
            "Cookbench refuses to replace a non-Cookbench Pi extension file.".into(),
        ));
    }
    if action == HookAction::Uninstall {
        let changed = !current.is_empty();
        let backup_display = if changed {
            Some(backup_and_remove(path)?)
        } else {
            None
        };
        return Ok(HookActionResult {
            status: pi_status_with_helper(path, helper),
            changed,
            preview: None,
            backup_display,
        });
    }
    finish_action(HookHarness::Pi, action, path, current, expected)
}

fn pi_extension_source(helper: &Path) -> Result<String, HookError> {
    let helper = helper.to_str().ok_or_else(|| {
        HookError::InvalidConfiguration("packaged hook helper path is not valid UTF-8".into())
    })?;
    if helper.is_empty() || helper.len() > 512 || helper.chars().any(char::is_control) {
        return Err(HookError::InvalidConfiguration(
            "packaged hook helper path is invalid".into(),
        ));
    }
    let helper = serde_json::to_string(helper)
        .map_err(|error| HookError::InvalidConfiguration(error.to_string()))?;
    Ok(format!(
        r#"// Cookbench managed Pi extension v{}
import {{ spawn }} from "node:child_process";
import {{ basename }} from "node:path";

const helper = {};

function emit(eventType: string, ctx: any) {{
  const transcriptPath = ctx.sessionManager.getSessionFile();
  if (!transcriptPath) return;
  const sessionId = basename(transcriptPath).replace(/\.jsonl$/, "");
  const payload = JSON.stringify({{
    event_type: eventType,
    session_id: sessionId,
    transcript_path: transcriptPath,
    cwd: ctx.cwd,
  }});
  const child = spawn(helper, ["--harness", "pi"], {{
    stdio: ["pipe", "ignore", "ignore"],
    windowsHide: true,
  }});
  child.on("error", () => {{}});
  child.stdin.end(payload);
}}

export default function cookbench(pi: any) {{
  pi.on("session_start", async (_event: any, ctx: any) => emit("session_discovered", ctx));
  pi.on("agent_start", async (_event: any, ctx: any) => emit("user_prompt_submitted", ctx));
  pi.on("tool_execution_start", async (_event: any, ctx: any) => emit("tool_started", ctx));
  pi.on("tool_execution_end", async (_event: any, ctx: any) => emit("tool_completed", ctx));
  pi.on("agent_end", async (_event: any, ctx: any) => emit("turn_completed", ctx));
  pi.on("session_shutdown", async (_event: any, ctx: any) => emit("process_exited", ctx));
}}
"#,
        env!("CARGO_PKG_VERSION"),
        helper
    ))
}

fn is_cookbench_pi_extension(contents: &str) -> bool {
    contents.starts_with("// Cookbench managed Pi extension v")
        && contents.contains("cookbench-hook")
}

fn finish_action(
    harness: HookHarness,
    action: HookAction,
    path: &Path,
    current: String,
    next: String,
) -> Result<HookActionResult, HookError> {
    let changed = current != next;
    if action == HookAction::PreviewInstall {
        return Ok(HookActionResult {
            status: status(harness),
            changed,
            preview: Some(preview_summary(harness, changed)),
            backup_display: None,
        });
    }
    let backup_display = if changed {
        Some(write_with_backup(path, next.as_bytes())?)
    } else {
        None
    };
    Ok(HookActionResult {
        status: status(harness),
        changed,
        preview: None,
        backup_display,
    })
}

fn append_codex_hook(current: &str, command: &[String]) -> Result<String, HookError> {
    let separator = if current.is_empty() || current.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    let encoded = command
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| HookError::InvalidConfiguration(error.to_string()))?;
    Ok(format!(
        "{current}{separator}notify = [{}]\n",
        encoded.join(", ")
    ))
}

fn remove_codex_hook(current: &str, expected: &[String]) -> Result<String, HookError> {
    match inspect_notify_hook(current, expected) {
        NotifyHookPlan::NotConfigured => Ok(current.to_owned()),
        NotifyHookPlan::Chain {
            existing_command,
            cookbench_command,
        } if existing_command == cookbench_command => {
            let lines = current
                .lines()
                .filter(|line| !line.trim_start().starts_with("notify"))
                .collect::<Vec<_>>();
            Ok(if lines.is_empty() {
                String::new()
            } else {
                format!("{}\n", lines.join("\n"))
            })
        }
        NotifyHookPlan::Chain {
            existing_command, ..
        } if existing_command
            .iter()
            .any(|part| part.contains("cookbench-hook")) =>
        {
            let lines = current
                .lines()
                .filter(|line| !line.trim_start().starts_with("notify"))
                .collect::<Vec<_>>();
            Ok(if lines.is_empty() {
                String::new()
            } else {
                format!("{}\n", lines.join("\n"))
            })
        }
        _ => Err(HookError::Conflict(
            "Codex notify is not Cookbench-owned; refusing to remove it.".into(),
        )),
    }
}

fn read_optional(path: &Path) -> Result<String, HookError> {
    match fs::read_to_string(path) {
        Ok(content) => Ok(content),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(HookError::Io(error)),
    }
}

fn write_with_backup(path: &Path, contents: &[u8]) -> Result<String, HookError> {
    let parent = path.parent().ok_or_else(|| {
        HookError::InvalidConfiguration("configuration path has no parent directory".into())
    })?;
    fs::create_dir_all(parent)?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let backup = path.with_extension(format!("cookbench-backup-{stamp}"));
    if path.exists() {
        fs::copy(path, &backup)?;
    }
    let temporary = parent.join(format!(
        ".{}.cookbench-{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("settings"),
        std::process::id()
    ));
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(contents)?;
        file.sync_all()?;
    }
    if let Err(error) = replace_config_file(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(HookError::Io(error));
    }
    Ok(display_path(&backup))
}

#[cfg(not(windows))]
fn replace_config_file(temporary: &Path, destination: &Path) -> io::Result<()> {
    fs::rename(temporary, destination)
}

#[cfg(windows)]
fn replace_config_file(temporary: &Path, destination: &Path) -> io::Result<()> {
    if !destination.exists() {
        return fs::rename(temporary, destination);
    }

    use std::{ffi::c_void, os::windows::ffi::OsStrExt};

    extern "system" {
        fn ReplaceFileW(
            replaced_file_name: *const u16,
            replacement_file_name: *const u16,
            backup_file_name: *const u16,
            replace_flags: u32,
            exclude: *mut c_void,
            reserved: *mut c_void,
        ) -> i32;
    }

    let wide = |path: &Path| {
        path.as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>()
    };
    let destination = wide(destination);
    let temporary = wide(temporary);
    let replaced = unsafe {
        ReplaceFileW(
            destination.as_ptr(),
            temporary.as_ptr(),
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if replaced == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn backup_and_remove(path: &Path) -> Result<String, HookError> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let backup = path.with_extension(format!("cookbench-backup-{stamp}"));
    fs::copy(path, &backup)?;
    fs::remove_file(path)?;
    Ok(display_path(&backup))
}

fn unwritable(mut status: HookStatus) -> HookStatus {
    status.health = HookHealth::Unwritable;
    status.detail = "Cookbench cannot read this configuration file.".into();
    status.can_install = false;
    status
}

fn helper_unavailable(mut status: HookStatus) -> HookStatus {
    status.health = HookHealth::Unwritable;
    status.detail = "The packaged Cookbench hook helper is unavailable.".into();
    status.can_install = false;
    status.can_repair = false;
    status
}

fn hook_program() -> PathBuf {
    if let Some(path) = env::var_os("COOKBENCH_HOOK_BINARY") {
        return PathBuf::from(path);
    }
    default_app_data_dir()
        .join("bin")
        .join(hook_executable_name())
}

fn packaged_hook_program() -> PathBuf {
    if let Some(path) = env::var_os("COOKBENCH_HOOK_BINARY") {
        return PathBuf::from(path);
    }
    env::current_exe()
        .ok()
        .and_then(|path| {
            path.parent()
                .map(|parent| parent.join(hook_executable_name()))
        })
        .unwrap_or_else(|| PathBuf::from(hook_executable_name()))
}

fn hook_executable_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "cookbench-hook.exe"
    } else {
        "cookbench-hook"
    }
}

fn default_app_data_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        home().join("Library/Application Support/app.cookbench.desktop")
    }
    #[cfg(target_os = "windows")]
    {
        env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(home)
            .join("app.cookbench.desktop")
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home().join(".local/share"))
            .join("app.cookbench.desktop")
    }
}

fn with_helper_health(status: HookStatus, helper_is_current: bool) -> HookStatus {
    if helper_is_current
        || !matches!(
            status.health,
            HookHealth::Healthy | HookHealth::NoRecentEvents
        )
    {
        return status;
    }
    HookStatus {
        health: HookHealth::Outdated,
        detail: "The managed Cookbench hook helper is missing or outdated; repair will refresh it."
            .into(),
        can_repair: true,
        ..status
    }
}

fn helper_is_current(packaged: &Path, managed: &Path) -> bool {
    if !valid_helper_file(packaged) || !valid_helper_file(managed) {
        return false;
    }
    if packaged == managed {
        return true;
    }
    files_equal_bounded(packaged, managed).unwrap_or(false)
}

fn valid_helper_file(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|metadata| {
        metadata.is_file()
            && metadata.len() > 0
            && metadata.len() <= MAX_HOOK_HELPER_BYTES
            && helper_permissions_are_executable(&metadata)
    })
}

#[cfg(unix)]
fn helper_permissions_are_executable(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn helper_permissions_are_executable(_metadata: &fs::Metadata) -> bool {
    true
}

fn files_equal_bounded(left: &Path, right: &Path) -> io::Result<bool> {
    let left_metadata = fs::metadata(left)?;
    let right_metadata = fs::metadata(right)?;
    if !left_metadata.is_file()
        || !right_metadata.is_file()
        || left_metadata.len() == 0
        || left_metadata.len() > MAX_HOOK_HELPER_BYTES
        || left_metadata.len() != right_metadata.len()
    {
        return Ok(false);
    }
    let mut left = BufReader::new(fs::File::open(left)?);
    let mut right = BufReader::new(fs::File::open(right)?);
    let mut left_buffer = [0_u8; 8192];
    let mut right_buffer = [0_u8; 8192];
    loop {
        let left_read = left.read(&mut left_buffer)?;
        let right_read = right.read(&mut right_buffer)?;
        if left_read != right_read || left_buffer[..left_read] != right_buffer[..right_read] {
            return Ok(false);
        }
        if left_read == 0 {
            return Ok(true);
        }
    }
}

fn ensure_managed_helper(packaged: &Path, managed: &Path) -> Result<(), HookError> {
    if helper_is_current(packaged, managed) {
        return Ok(());
    }
    let metadata = fs::metadata(packaged)?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_HOOK_HELPER_BYTES {
        return Err(HookError::InvalidConfiguration(
            "The packaged Cookbench hook helper is unavailable or invalid.".into(),
        ));
    }
    if packaged == managed {
        return Ok(());
    }
    let parent = managed.parent().ok_or_else(|| {
        HookError::InvalidConfiguration("The managed hook helper path has no parent.".into())
    })?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(
        ".{}.tmp-{}-{}",
        hook_executable_name(),
        std::process::id(),
        now_ms()
    ));
    let result = (|| -> io::Result<()> {
        let source = fs::File::open(packaged)?;
        let mut destination = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        let copied = io::copy(
            &mut source.take(MAX_HOOK_HELPER_BYTES + 1),
            &mut destination,
        )?;
        if copied != metadata.len() || copied > MAX_HOOK_HELPER_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "packaged hook helper changed while being copied",
            ));
        }
        destination.sync_all()?;
        drop(destination);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&temporary, fs::Permissions::from_mode(0o700))?;
        }
        replace_config_file(&temporary, managed)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(HookError::Io)
}

fn codex_hook_command(helper: &Path) -> Option<Vec<String>> {
    let helper = helper.to_str()?;
    if helper.is_empty() || helper.len() > 512 || helper.chars().any(char::is_control) {
        return None;
    }
    Some(vec![
        helper.to_owned(),
        "--harness".to_owned(),
        "codex".to_owned(),
    ])
}

fn home() -> PathBuf {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn codex_config_path() -> PathBuf {
    env::var_os("COOKBENCH_CODEX_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".codex/config.toml"))
}
fn claude_config_path() -> PathBuf {
    env::var_os("COOKBENCH_CLAUDE_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".claude/settings.json"))
}
fn pi_extension_path() -> PathBuf {
    env::var_os("COOKBENCH_PI_EXTENSION")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".pi/agent/extensions/cookbench.ts"))
}

fn display_path(path: &Path) -> String {
    path.strip_prefix(home())
        .map(|relative| format!("~/{}", relative.to_string_lossy()))
        .unwrap_or_else(|_| {
            format!(
                "Custom location/{}",
                path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("settings")
            )
        })
}

fn preview_summary(harness: HookHarness, changed: bool) -> String {
    if !changed {
        return "Cookbench's managed hook already matches this action. No configuration would change."
            .into();
    }
    match harness {
        HookHarness::Codex => format!(
            "Would add the Cookbench Codex notify hook (helper version {}). Existing configuration is not shown.",
            env!("CARGO_PKG_VERSION")
        ),
        HookHarness::ClaudeCode => format!(
            "Would add Cookbench lifecycle entries for SessionStart, UserPromptSubmit, PreToolUse, PostToolUse, PermissionRequest, Stop, SubagentStart, SubagentStop, Notification, and SessionEnd (helper version {}). Existing configuration is not shown.",
            env!("CARGO_PKG_VERSION")
        ),
        HookHarness::Pi => format!(
            "Would add Cookbench's metadata-only Pi lifecycle extension (helper version {}). Existing session content is never included.",
            env!("CARGO_PKG_VERSION")
        ),
    }
}

fn contains_cookbench_hook(configuration: &serde_json::Value) -> bool {
    match configuration {
        serde_json::Value::Object(object) => {
            let is_handler = object.get("type").and_then(serde_json::Value::as_str)
                == Some("command")
                && object
                    .get("command")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|command| command.contains("cookbench-hook"));
            is_handler || object.values().any(contains_cookbench_hook)
        }
        serde_json::Value::Array(values) => values.iter().any(contains_cookbench_hook),
        _ => false,
    }
}

fn contains_other_cookbench_hook(
    configuration: &serde_json::Value,
    expected_command: &str,
    expected_args: &[&str],
) -> bool {
    match configuration {
        serde_json::Value::Object(object) => {
            let command = object.get("command").and_then(serde_json::Value::as_str);
            let is_cookbench = object.get("type").and_then(serde_json::Value::as_str)
                == Some("command")
                && command.is_some_and(|command| command.contains("cookbench-hook"));
            let exact_args = object
                .get("args")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|args| {
                    args.len() == expected_args.len()
                        && args
                            .iter()
                            .zip(expected_args)
                            .all(|(value, expected)| value.as_str() == Some(*expected))
                });
            (is_cookbench && !(command == Some(expected_command) && exact_args))
                || object.values().any(|value| {
                    contains_other_cookbench_hook(value, expected_command, expected_args)
                })
        }
        serde_json::Value::Array(values) => values
            .iter()
            .any(|value| contains_other_cookbench_hook(value, expected_command, expected_args)),
        _ => false,
    }
}

fn with_event_health(status: HookStatus) -> HookStatus {
    let harness = status.harness;
    health_with_last_event(status, now_ms(), last_event_ms(harness))
}

fn health_with_last_event(status: HookStatus, now: u64, last_event: Option<u64>) -> HookStatus {
    if status.health != HookHealth::Healthy {
        return status;
    }
    match last_event {
        Some(last_event) if now.saturating_sub(last_event) <= STALE_HOOK_AFTER.as_millis() as u64 => status,
        _ => HookStatus {
            health: HookHealth::NoRecentEvents,
            detail: "The hook is installed, but Cookbench has not received a recent metadata-only lifecycle event."
                .into(),
            ..status
        },
    }
}

fn last_event_ms(harness: HookHarness) -> Option<u64> {
    let bytes = fs::read(hook_health_ledger_path()).ok()?;
    let ledger: HookHealthLedger = serde_json::from_slice(&bytes).ok()?;
    ledger.last_event_ms.get(harness_key(harness)).copied()
}

#[derive(Deserialize)]
struct HookHealthLedger {
    #[serde(default)]
    last_event_ms: std::collections::BTreeMap<String, u64>,
}

fn hook_health_ledger_path() -> PathBuf {
    env::var_os("COOKBENCH_HOOK_HEALTH_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(default_hook_spool_dir)
        .join("hook-health.json")
}

fn default_hook_spool_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        home().join("Library/Application Support/app.cookbench.desktop/hook-spool")
    }
    #[cfg(target_os = "windows")]
    {
        env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(home)
            .join("app.cookbench.desktop/hook-spool")
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home().join(".local/share"))
            .join("app.cookbench.desktop/hook-spool")
    }
}

fn harness_key(harness: HookHarness) -> &'static str {
    match harness {
        HookHarness::Codex => "codex",
        HookHarness::ClaudeCode => "claudeCode",
        HookHarness::Pi => "pi",
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_dir(name: &str) -> PathBuf {
        let path = env::temp_dir().join(format!(
            "cookbench-hooks-{name}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn claude_install_preserves_existing_groups_and_creates_backup() {
        let dir = unique_dir("claude");
        let path = dir.join("settings.json");
        fs::write(&path, r#"{"hooks":{"Stop":[{"matcher":"*","hooks":[{"type":"command","command":"existing"}]}]}}"#).unwrap();
        let result = apply_claude(HookAction::Install, &path).unwrap();
        assert!(result.changed);
        let installed = fs::read_to_string(&path).unwrap();
        assert!(installed.contains("existing"));
        let installed: serde_json::Value = serde_json::from_str(&installed).unwrap();
        let handler = &installed["hooks"]["Stop"][1]["hooks"][0];
        assert!(handler["command"]
            .as_str()
            .is_some_and(|command| command.ends_with(hook_executable_name())));
        assert_eq!(
            handler["args"],
            serde_json::json!(["--harness", "claude-code"])
        );
        assert!(result.backup_display.is_some());
    }

    #[test]
    fn installs_the_packaged_helper_as_an_exec_form_command() {
        let dir = unique_dir("packaged-helper");
        let helper = dir.join("Cookbench Helper").join("cookbench-hook");
        let claude_path = dir.join("settings.json");
        let codex_path = dir.join("config.toml");

        apply_claude_with_helper(HookAction::Install, &claude_path, &helper).unwrap();
        let claude: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&claude_path).unwrap()).unwrap();
        let handler = &claude["hooks"]["Stop"][0]["hooks"][0];
        assert_eq!(handler["command"], helper.to_string_lossy().as_ref());
        assert_eq!(
            handler["args"],
            serde_json::json!(["--harness", "claude-code"])
        );

        apply_codex_with_helper(HookAction::Install, &codex_path, &helper).unwrap();
        let codex = fs::read_to_string(&codex_path).unwrap();
        assert!(codex.contains(&serde_json::to_string(helper.to_string_lossy().as_ref()).unwrap()));
        assert!(!codex.contains("notify = [\"cookbench-hook\""));
    }

    #[test]
    fn codex_refuses_to_replace_single_existing_callback() {
        let dir = unique_dir("codex-conflict");
        let path = dir.join("config.toml");
        fs::write(&path, "notify = [\"existing-helper\"]\n").unwrap();
        assert!(matches!(
            apply_codex(HookAction::Install, &path),
            Err(HookError::Conflict(_))
        ));
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "notify = [\"existing-helper\"]\n"
        );
    }

    #[test]
    fn preview_does_not_mutate_and_uninstall_is_idempotent() {
        let dir = unique_dir("preview");
        let path = dir.join("config.toml");
        let preview = apply_codex(HookAction::PreviewInstall, &path).unwrap();
        assert!(preview.changed);
        assert!(!path.exists());
        assert!(!preview.preview.unwrap().contains("notify ="));
        apply_codex(HookAction::Install, &path).unwrap();
        assert!(apply_codex(HookAction::Uninstall, &path).unwrap().changed);
        assert!(!apply_codex(HookAction::Uninstall, &path).unwrap().changed);
    }

    #[test]
    fn managed_but_stale_hook_is_reported_without_a_real_clock() {
        let status = HookStatus {
            harness: HookHarness::Codex,
            label: "Codex",
            health: HookHealth::Healthy,
            config_display: "~/.codex/config.toml".into(),
            detail: "healthy".into(),
            can_install: false,
            can_repair: true,
            can_uninstall: true,
        };
        assert_eq!(
            health_with_last_event(status.clone(), 1_000_000, Some(999_999)).health,
            HookHealth::Healthy
        );
        assert_eq!(
            health_with_last_event(status, 1_000_000, Some(1)).health,
            HookHealth::NoRecentEvents
        );
    }

    #[test]
    fn installed_hook_with_a_missing_or_stale_managed_helper_is_outdated() {
        let status = HookStatus {
            harness: HookHarness::Codex,
            label: "Codex",
            health: HookHealth::Healthy,
            config_display: "~/.codex/config.toml".into(),
            detail: "healthy".into(),
            can_install: false,
            can_repair: true,
            can_uninstall: true,
        };
        assert_eq!(
            with_helper_health(status.clone(), false).health,
            HookHealth::Outdated
        );
        assert_eq!(with_helper_health(status, true).health, HookHealth::Healthy);
    }

    #[test]
    fn packaged_helper_is_copied_atomically_to_a_stable_executable_path() {
        let dir = unique_dir("managed-helper");
        let packaged = dir.join("package/cookbench-hook");
        let managed = dir.join("data/bin/cookbench-hook");
        fs::create_dir_all(packaged.parent().unwrap()).unwrap();
        fs::write(&packaged, b"first helper").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&packaged, fs::Permissions::from_mode(0o700)).unwrap();
        }

        ensure_managed_helper(&packaged, &managed).unwrap();
        assert_eq!(fs::read(&managed).unwrap(), b"first helper");
        assert!(helper_is_current(&packaged, &managed));

        fs::write(&packaged, b"second helper").unwrap();
        assert!(!helper_is_current(&packaged, &managed));
        ensure_managed_helper(&packaged, &managed).unwrap();
        assert_eq!(fs::read(&managed).unwrap(), b"second helper");
        assert!(helper_is_current(&packaged, &managed));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&managed).unwrap().permissions().mode() & 0o777,
                0o700
            );
            fs::set_permissions(&managed, fs::Permissions::from_mode(0o600)).unwrap();
            assert!(!helper_is_current(&packaged, &managed));
            ensure_managed_helper(&packaged, &managed).unwrap();
            assert_eq!(
                fs::metadata(&managed).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
    }

    #[test]
    fn old_cookbench_callback_is_outdated_not_an_unrelated_conflict() {
        let dir = unique_dir("outdated");
        let path = dir.join("config.toml");
        fs::write(&path, "notify = [\"cookbench-hook\", \"codex\"]\n").unwrap();
        assert_eq!(codex_status(&path).health, HookHealth::Outdated);
    }

    #[test]
    fn claude_distinguishes_missing_and_legacy_cookbench_hooks() {
        let dir = unique_dir("claude-status");
        let path = dir.join("settings.json");
        let helper = dir.join("cookbench-hook");

        fs::write(&path, "{}\n").unwrap();
        assert_eq!(
            claude_status_with_helper(&path, &helper).health,
            HookHealth::NotInstalled
        );

        let legacy = cookbench_adapters::claude::install_hooks(&serde_json::json!({})).unwrap();
        fs::write(&path, serde_json::to_vec(&legacy.configuration).unwrap()).unwrap();
        assert_eq!(
            claude_status_with_helper(&path, &helper).health,
            HookHealth::Outdated
        );

        let helper_text = helper.to_string_lossy();
        let exact = cookbench_adapters::claude::install_hooks_with_command(
            &serde_json::json!({}),
            &helper_text,
            CLAUDE_HOOK_ARGS,
        )
        .unwrap();
        let mixed = cookbench_adapters::claude::install_hooks(&exact.configuration).unwrap();
        fs::write(&path, serde_json::to_vec(&mixed.configuration).unwrap()).unwrap();
        assert_eq!(
            claude_status_with_helper(&path, &helper).health,
            HookHealth::Outdated
        );
    }

    #[test]
    fn pi_extension_install_is_owned_content_free_and_reversible() {
        let dir = unique_dir("pi-extension");
        let path = dir.join("cookbench.ts");
        let helper = dir.join("Cookbench Helper").join("cookbench-hook");

        assert_eq!(
            pi_status_with_helper(&path, &helper).health,
            HookHealth::NotInstalled
        );
        let preview = apply_pi_with_helper(HookAction::PreviewInstall, &path, &helper).unwrap();
        assert!(preview.changed);
        assert!(!path.exists());

        apply_pi_with_helper(HookAction::Install, &path, &helper).unwrap();
        let extension = fs::read_to_string(&path).unwrap();
        assert!(extension.contains("agent_end"));
        assert!(
            extension.contains(&serde_json::to_string(helper.to_string_lossy().as_ref()).unwrap())
        );
        assert!(!extension.contains("event.prompt"));
        assert!(!extension.contains("event.args"));
        assert!(pi_status_with_helper(&path, &helper).can_uninstall);

        apply_pi_with_helper(HookAction::Uninstall, &path, &helper).unwrap();
        assert!(!path.exists());
    }
}
