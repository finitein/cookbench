use std::env;

use serde::{Deserialize, Serialize};

pub const MAX_INPUT_BYTES: usize = 16 * 1024;
pub const MAX_SESSION_ID_BYTES: usize = 256;
pub const MAX_HARNESS_BYTES: usize = 32;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HookInput {
    event_type: LifecycleEvent,
    session_id: String,
    harness: Harness,
    #[serde(default)]
    sequence: Option<u64>,
    #[serde(default)]
    progress: Option<Progress>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleEvent {
    SessionDiscovered,
    UserPromptSubmitted,
    PlanUpdated,
    ToolStarted,
    ToolCompleted,
    QuestionAsked,
    PermissionRequested,
    TurnCompleted,
    SessionFailed,
    ProcessExited,
    ConnectionLost,
    ConnectionRestored,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Harness {
    Codex,
    ClaudeCode,
    Pi,
}

impl Harness {
    const fn as_str(&self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::ClaudeCode => "claude_code",
            Self::Pi => "pi",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Progress {
    completed: u32,
    total: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct EventEnvelope {
    pub schema_version: u8,
    pub source: &'static str,
    pub received_at_ms: u64,
    pub event: SanitizedEvent,
}

#[derive(Clone, Debug, Serialize)]
pub struct SanitizedEvent {
    pub event_type: LifecycleEvent,
    pub session_id: String,
    pub harness: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<Progress>,
    /// A bounded, metadata-only return locator. It never contains command
    /// arguments, prompt content, agent output, or environment snapshots.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locator: Option<LocatorProjection>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct LocatorProjection {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_locator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_pane_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_control_endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tmux_pane: Option<String>,
}

impl LocatorProjection {
    fn from_native(object: &serde_json::Map<String, serde_json::Value>) -> Option<Self> {
        let mut locator = Self::from_environment();
        locator.native_locator = optional_safe_string(object, "transcript_path");
        locator.working_directory = optional_safe_string(object, "cwd");
        locator.non_empty()
    }

    fn from_environment() -> Self {
        Self::from_pairs(&[
            ("ITERM_SESSION_ID", "iterm_session"),
            ("WEZTERM_PANE", "wezterm_pane"),
            ("WEZTERM_UNIX_SOCKET", "wezterm_socket"),
            ("ZELLIJ_SESSION_NAME", "zellij_session"),
            ("ZELLIJ_PANE_ID", "zellij_pane"),
            ("TMUX_PANE", "tmux_pane"),
            ("CMUX_WORKSPACE_ID", "cmux_workspace"),
            ("CMUX_SURFACE_ID", "cmux_surface"),
            ("CMUX_SOCKET_PATH", "cmux_socket"),
            ("TERM_PROGRAM", "term_program"),
        ])
    }

    fn from_pairs(pairs: &[(&str, &str)]) -> Self {
        let value = |name: &str| env::var(name).ok().and_then(safe_text);
        let named = |key: &str| {
            pairs
                .iter()
                .find_map(|(name, alias)| (*alias == key).then(|| value(name)))
                .flatten()
        };
        let term_program = named("term_program");
        let mut locator = Self {
            // The hook helper is deliberately short-lived. Its PID is not the
            // terminal/agent PID and therefore must never enter a locator.
            process_id: None,
            terminal: term_program.as_deref().and_then(terminal_name),
            terminal_session_id: named("iterm_session")
                .or_else(|| named("zellij_session"))
                .or_else(|| named("cmux_workspace")),
            terminal_pane_id: named("wezterm_pane")
                .or_else(|| named("zellij_pane"))
                .or_else(|| named("cmux_surface")),
            terminal_control_endpoint: named("wezterm_socket").or_else(|| named("cmux_socket")),
            tmux_pane: named("tmux_pane"),
            ..Self::default()
        };
        // A terminal-specific variable is sufficient to identify WezTerm,
        // Zellij, or cmux even when TERM_PROGRAM is absent inside tmux.
        if locator.terminal.is_none() {
            locator.terminal =
                if named("wezterm_pane").is_some() || named("wezterm_socket").is_some() {
                    Some("wezterm".into())
                } else if named("zellij_session").is_some() || named("zellij_pane").is_some() {
                    Some("zellij".into())
                } else if named("cmux_workspace").is_some()
                    || named("cmux_surface").is_some()
                    || named("cmux_socket").is_some()
                {
                    Some("cmux".into())
                } else {
                    None
                };
        }
        locator
    }

    fn non_empty(self) -> Option<Self> {
        (self.native_locator.is_some()
            || self.working_directory.is_some()
            || self.process_id.is_some()
            || self.terminal.is_some()
            || self.terminal_session_id.is_some()
            || self.terminal_pane_id.is_some()
            || self.terminal_control_endpoint.is_some()
            || self.tmux_pane.is_some())
        .then_some(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnvelopeError {
    Malformed,
    SensitiveField,
    InvalidField,
}

impl EnvelopeError {
    pub const fn diagnostic(self) -> &'static str {
        match self {
            Self::Malformed => "invalid hook event",
            Self::SensitiveField => "hook event contains a restricted field",
            Self::InvalidField => "hook event contains an invalid field",
        }
    }
}

pub fn parse(input: &[u8], received_at_ms: u64) -> Result<EventEnvelope, EnvelopeError> {
    if input.is_empty() || input.len() > MAX_INPUT_BYTES {
        return Err(EnvelopeError::Malformed);
    }

    let value: serde_json::Value =
        serde_json::from_slice(input).map_err(|_| EnvelopeError::Malformed)?;
    if contains_sensitive_field(&value) {
        return Err(EnvelopeError::SensitiveField);
    }
    let hook: HookInput = serde_json::from_value(value).map_err(|_| EnvelopeError::Malformed)?;

    if hook.session_id.is_empty()
        || hook.session_id.len() > MAX_SESSION_ID_BYTES
        || !hook.session_id.is_ascii()
        || hook.harness.as_str().len() > MAX_HARNESS_BYTES
        || hook
            .progress
            .as_ref()
            .is_some_and(|progress| progress.total == 0 || progress.completed > progress.total)
    {
        return Err(EnvelopeError::InvalidField);
    }

    Ok(EventEnvelope {
        schema_version: 2,
        source: "hook",
        received_at_ms,
        event: SanitizedEvent {
            event_type: hook.event_type,
            session_id: hook.session_id,
            harness: hook.harness.as_str(),
            sequence: hook.sequence,
            progress: hook.progress,
            locator: LocatorProjection::from_environment().non_empty(),
        },
    })
}

/// Converts a native harness hook payload into Cookbench's metadata-only
/// envelope. Native payloads deliberately contain transcript paths, prompts,
/// tool inputs, and results; this projection reads only the event name and
/// native session identity and discards every other field before persistence.
pub fn parse_native(
    input: &[u8],
    harness: &str,
    received_at_ms: u64,
) -> Result<EventEnvelope, EnvelopeError> {
    if input.is_empty() || input.len() > MAX_INPUT_BYTES {
        return Err(EnvelopeError::Malformed);
    }
    let value: serde_json::Value =
        serde_json::from_slice(input).map_err(|_| EnvelopeError::Malformed)?;
    let object = value.as_object().ok_or(EnvelopeError::Malformed)?;

    let (harness, session_id, event_type) = match harness {
        "claude" | "claude-code" => {
            let session_id = required_string(object, "session_id")?;
            let hook_event = required_string(object, "hook_event_name")?;
            let event_type = match hook_event {
                "SessionStart" => LifecycleEvent::SessionDiscovered,
                "UserPromptSubmit" => LifecycleEvent::UserPromptSubmitted,
                "PreToolUse" | "SubagentStart" => LifecycleEvent::ToolStarted,
                "PostToolUse" | "SubagentStop" => LifecycleEvent::ToolCompleted,
                "PermissionRequest" => LifecycleEvent::PermissionRequested,
                "Stop" => LifecycleEvent::TurnCompleted,
                "SessionEnd" => LifecycleEvent::ProcessExited,
                "Notification" => match required_string(object, "notification_type")? {
                    "permission_prompt" => LifecycleEvent::PermissionRequested,
                    "idle_prompt" | "elicitation_dialog" => LifecycleEvent::QuestionAsked,
                    _ => return Err(EnvelopeError::InvalidField),
                },
                _ => return Err(EnvelopeError::InvalidField),
            };
            ("claude_code", session_id, event_type)
        }
        "codex" => {
            let session_id = required_string(object, "thread-id")?;
            let event_type = match required_string(object, "type")? {
                "agent-turn-complete" => LifecycleEvent::TurnCompleted,
                "approval-requested" => LifecycleEvent::PermissionRequested,
                "user-input-requested" => LifecycleEvent::QuestionAsked,
                _ => return Err(EnvelopeError::InvalidField),
            };
            ("codex", session_id, event_type)
        }
        "pi" => {
            let session_id = required_string(object, "session_id")?;
            let event_type = match required_string(object, "event_type")? {
                "session_discovered" => LifecycleEvent::SessionDiscovered,
                "user_prompt_submitted" => LifecycleEvent::UserPromptSubmitted,
                "tool_started" => LifecycleEvent::ToolStarted,
                "tool_completed" => LifecycleEvent::ToolCompleted,
                "question_asked" => LifecycleEvent::QuestionAsked,
                "permission_requested" => LifecycleEvent::PermissionRequested,
                "turn_completed" => LifecycleEvent::TurnCompleted,
                "session_failed" => LifecycleEvent::SessionFailed,
                "process_exited" => LifecycleEvent::ProcessExited,
                _ => return Err(EnvelopeError::InvalidField),
            };
            ("pi", session_id, event_type)
        }
        _ => return Err(EnvelopeError::InvalidField),
    };

    validate_session_id(session_id)?;
    Ok(EventEnvelope {
        schema_version: 2,
        source: "hook",
        received_at_ms,
        event: SanitizedEvent {
            event_type,
            session_id: session_id.to_owned(),
            harness,
            sequence: None,
            progress: None,
            locator: LocatorProjection::from_native(object),
        },
    })
}

fn required_string<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    field: &str,
) -> Result<&'a str, EnvelopeError> {
    object
        .get(field)
        .and_then(serde_json::Value::as_str)
        .ok_or(EnvelopeError::Malformed)
}

fn optional_safe_string(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &str,
) -> Option<String> {
    object
        .get(field)
        .and_then(serde_json::Value::as_str)
        .and_then(safe_text)
}

fn safe_text(value: impl AsRef<str>) -> Option<String> {
    let value = value.as_ref();
    (value.len() <= 4096 && !value.chars().any(char::is_control)).then(|| value.to_owned())
}

fn terminal_name(term_program: &str) -> Option<String> {
    match term_program.to_ascii_lowercase().as_str() {
        "iterm.app" | "iterm2" => Some("iterm2".into()),
        "wezterm" => Some("wezterm".into()),
        "ghostty" => Some("ghostty".into()),
        "zellij" => Some("zellij".into()),
        "cmux" => Some("cmux".into()),
        "apple_terminal" | "terminal.app" => Some("macos_terminal".into()),
        "vscode" => Some("visual_studio_code".into()),
        "gnome-terminal" => Some("gnome_terminal".into()),
        "konsole" => Some("konsole".into()),
        "xfce4-terminal" => Some("xfce_terminal".into()),
        _ => None,
    }
}

fn validate_session_id(session_id: &str) -> Result<(), EnvelopeError> {
    if session_id.is_empty() || session_id.len() > MAX_SESSION_ID_BYTES || !session_id.is_ascii() {
        return Err(EnvelopeError::InvalidField);
    }
    Ok(())
}

fn contains_sensitive_field(value: &serde_json::Value) -> bool {
    const RESTRICTED: &[&str] = &[
        "prompt",
        "output",
        "code",
        "command",
        "token",
        "credential",
        "password",
        "secret",
        "authorization",
        "api_key",
    ];

    match value {
        serde_json::Value::Object(object) => object.iter().any(|(key, value)| {
            let normalized = key.to_ascii_lowercase();
            RESTRICTED
                .iter()
                .any(|restricted| normalized.contains(restricted))
                || contains_sensitive_field(value)
        }),
        serde_json::Value::Array(values) => values.iter().any(contains_sensitive_field),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_minimal_lifecycle_event() {
        let event = parse(
            br#"{"event_type":"turn_completed","session_id":"session-42","harness":"codex"}"#,
            10,
        )
        .expect("event should parse");
        assert_eq!(event.event.session_id, "session-42");
        assert_eq!(event.received_at_ms, 10);
    }

    #[test]
    fn rejects_sensitive_fields_at_any_depth() {
        assert!(matches!(
            parse(
                br#"{"event_type":"turn_completed","session_id":"session-42","harness":"codex","metadata":{"token":"hidden"}}"#,
                10,
            ),
            Err(EnvelopeError::SensitiveField)
        ));
    }

    #[test]
    fn only_allowlisted_locator_values_are_projected() {
        let object = serde_json::json!({
            "transcript_path": "/safe/session.jsonl",
            "cwd": "/safe/project",
            "prompt": "must never survive"
        });
        let locator = LocatorProjection::from_native(object.as_object().unwrap()).unwrap();
        assert_eq!(
            locator.native_locator.as_deref(),
            Some("/safe/session.jsonl")
        );
        assert_eq!(locator.working_directory.as_deref(), Some("/safe/project"));
        let serialized = serde_json::to_string(&locator).unwrap();
        assert!(!serialized.contains("prompt"));
        assert!(!serialized.contains("must never survive"));
    }
}
