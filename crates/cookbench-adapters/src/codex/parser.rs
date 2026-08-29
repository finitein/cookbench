use cookbench_core::domain::{EventKind, EventMetadata, EventSource, StoveEvent};
use serde_json::Value;

use super::progress::plan_progress;

/// A parsed Codex record. Content-bearing fields are intentionally absent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexRecord {
    pub session_id: Option<String>,
    pub cwd: Option<String>,
    pub event: Option<StoveEvent>,
}

/// Parses only fixture-backed Codex JSONL variants. Unknown/malformed records
/// are ignored so a new Codex record type cannot break an entire stove.
pub fn parse_record(
    line: &str,
    sequence: u64,
    max_depth: usize,
    max_field_bytes: usize,
) -> Option<CodexRecord> {
    if line.len() > max_field_bytes.saturating_mul(4) {
        return None;
    }
    let value: Value = serde_json::from_str(line).ok()?;
    if value_depth(&value) > max_depth {
        return None;
    }
    let payload = value.get("payload").unwrap_or(&value);
    let session_id = bounded_string(
        payload
            .get("id")
            .or_else(|| payload.get("session_id"))
            .or_else(|| value.get("session_id")),
        max_field_bytes,
    );
    let cwd = bounded_string(
        payload
            .get("cwd")
            .or_else(|| payload.get("worktree"))
            .or_else(|| value.get("cwd")),
        max_field_bytes,
    );
    let timestamp_ms = timestamp_ms(value.get("timestamp")).unwrap_or(sequence);
    let record_type = value.get("type").and_then(Value::as_str)?;
    let payload_type = payload.get("type").and_then(Value::as_str);
    let kind = match (record_type, payload_type) {
        ("response_item", Some("message"))
            if payload.get("role").and_then(Value::as_str) == Some("user") =>
        {
            Some(EventKind::UserPromptSubmitted)
        }
        ("response_item", Some("function_call"))
            if payload.get("name").and_then(Value::as_str) == Some("update_plan") =>
        {
            update_plan_progress(payload)
                .map(|(completed, total)| EventKind::PlanUpdated { completed, total })
        }
        ("response_item", Some("function_call"))
            if matches!(
                payload.get("name").and_then(Value::as_str),
                Some("request_user_input" | "ask_user_question")
            ) =>
        {
            Some(EventKind::QuestionAsked)
        }
        ("response_item", Some("function_call")) => Some(EventKind::ToolStarted),
        ("response_item", Some("function_call_output")) => Some(EventKind::ToolCompleted {
            succeeded: !payload
                .get("success")
                .is_some_and(|success| success == false),
        }),
        ("event_msg", Some("task_complete")) => Some(EventKind::TurnCompleted),
        ("event_msg", Some("question" | "input_required" | "request_user_input")) => {
            Some(EventKind::QuestionAsked)
        }
        ("event_msg", Some("permission_request" | "permission_requested")) => {
            Some(EventKind::PermissionRequested)
        }
        ("event_msg", Some("turn_failed" | "session_failed")) => Some(EventKind::SessionFailed),
        ("session_meta" | "session_started", _) => Some(EventKind::SessionDiscovered),
        ("user_message" | "user_prompt", _) => Some(EventKind::UserPromptSubmitted),
        ("tool_started" | "function_call", _) => Some(EventKind::ToolStarted),
        ("tool_completed" | "function_call_output", _) => Some(EventKind::ToolCompleted {
            succeeded: !payload
                .get("success")
                .is_some_and(|success| success == false),
        }),
        ("update_plan", _) => plan_progress(payload)
            .map(|(completed, total)| EventKind::PlanUpdated { completed, total }),
        ("turn_completed" | "task_complete", _) => Some(EventKind::TurnCompleted),
        ("question" | "input_required", _) => Some(EventKind::QuestionAsked),
        ("permission_request" | "permission_requested", _) => Some(EventKind::PermissionRequested),
        ("turn_failed" | "error", _) => Some(EventKind::SessionFailed),
        _ => None,
    };
    Some(CodexRecord {
        session_id,
        cwd,
        event: kind.map(|kind| {
            StoveEvent::new(
                kind,
                EventMetadata::new(EventSource::StructuredSession, 100, sequence, timestamp_ms),
            )
        }),
    })
}

fn update_plan_progress(payload: &Value) -> Option<(u32, u32)> {
    plan_progress(payload).or_else(|| {
        let arguments = payload.get("arguments")?.as_str()?;
        let value: Value = serde_json::from_str(arguments).ok()?;
        plan_progress(&value)
    })
}

/// Produces a fixture-safe JSON record. It preserves record shape, identifier,
/// time, path shape and plan status while replacing prompt/output/command/code
/// material. This is a development helper; callers must never commit raw logs.
pub fn sanitize_fixture_record(line: &str) -> Option<String> {
    let mut value: Value = serde_json::from_str(line).ok()?;
    sanitize_value(&mut value, None);
    serde_json::to_string(&value).ok()
}

fn sanitize_value(value: &mut Value, key: Option<&str>) {
    const CONTENT_KEYS: &[&str] = &[
        "text",
        "content",
        "prompt",
        "input",
        "output",
        "command",
        "arguments",
        "code",
        "message",
        "summary",
        "description",
        "result",
        "reason",
        "step",
    ];
    match value {
        Value::Object(map) => {
            for (child_key, child) in map.iter_mut() {
                sanitize_value(child, Some(child_key));
            }
        }
        Value::Array(values) => {
            for child in values {
                sanitize_value(child, key);
            }
        }
        Value::String(_) if key.is_some_and(|key| CONTENT_KEYS.contains(&key)) => {
            *value = Value::String("[redacted]".to_owned());
        }
        _ => {}
    }
}

fn bounded_string(value: Option<&Value>, max: usize) -> Option<String> {
    let value = value?.as_str()?;
    (value.len() <= max).then(|| value.to_owned())
}

fn value_depth(value: &Value) -> usize {
    match value {
        Value::Array(items) => 1 + items.iter().map(value_depth).max().unwrap_or(0),
        Value::Object(items) => 1 + items.values().map(value_depth).max().unwrap_or(0),
        _ => 1,
    }
}

fn timestamp_ms(value: Option<&Value>) -> Option<u64> {
    let value = value?;
    if let Some(number) = value.as_u64() {
        return Some(if number < 10_000_000_000 {
            number.saturating_mul(1000)
        } else {
            number
        });
    }
    // Sequence remains the authoritative order when a record uses RFC3339.
    // Do not invent an epoch value or use a hash that could reorder events.
    None
}
