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
        schema_version: 1,
        source: "hook",
        received_at_ms,
        event: SanitizedEvent {
            event_type: hook.event_type,
            session_id: hook.session_id,
            harness: hook.harness.as_str(),
            sequence: hook.sequence,
            progress: hook.progress,
        },
    })
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
}
