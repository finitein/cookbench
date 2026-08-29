use cookbench_core::domain::{EventKind, EventMetadata, EventSource, StoveEvent};
use serde_json::Value;

use crate::io::TailLimits;

use super::tasks::extract_task_progress;

/// Sanitized, normalized result of one Claude native record. It deliberately
/// contains events and bounded metadata only, never transcript content.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedRecord {
    pub title: Option<String>,
    pub events: Vec<StoveEvent>,
}

pub fn parse_record(record: &str, limits: TailLimits, sequence: u64) -> Option<ParsedRecord> {
    if record.len() > limits.max_record_bytes {
        return None;
    }
    let value: Value = serde_json::from_str(record).ok()?;
    if !within_depth(&value, 0, limits.max_json_nesting) {
        return None;
    }
    let object = value.as_object()?;
    let record_type = bounded_string(object.get("type"), limits.max_json_field_bytes)?;
    let subtype = object
        .get("subtype")
        .and_then(|value| bounded_string(Some(value), limits.max_json_field_bytes));
    let timestamp_ms = object
        .get("timestamp")
        .and_then(timestamp_ms)
        .unwrap_or(sequence);
    let metadata = EventMetadata::new(EventSource::StructuredSession, 90, sequence, timestamp_ms);
    let mut events = Vec::new();

    if let Some(progress) = extract_task_progress(&value, 10_000) {
        events.push(StoveEvent::new(
            EventKind::PlanUpdated {
                completed: progress.completed,
                total: progress.total,
            },
            metadata.clone(),
        ));
    }

    match (record_type.as_str(), subtype.as_deref()) {
        ("user", _) => events.push(StoveEvent::new(
            EventKind::UserPromptSubmitted,
            metadata.clone(),
        )),
        ("assistant", _) if contains_tool_use(&value) => {
            events.push(StoveEvent::new(EventKind::ToolStarted, metadata.clone()))
        }
        ("tool_result", _) => events.push(StoveEvent::new(
            EventKind::ToolCompleted {
                succeeded: !tool_result_failed(&value),
            },
            metadata.clone(),
        )),
        (_, Some("permission_request" | "permission_requested")) | ("permission", _) => {
            events.push(StoveEvent::new(
                EventKind::PermissionRequested,
                metadata.clone(),
            ));
        }
        (_, Some("question" | "ask_user_question" | "input_required")) | ("question", _) => {
            events.push(StoveEvent::new(EventKind::QuestionAsked, metadata.clone()));
        }
        (_, Some("stop" | "turn_completed" | "success")) => {
            events.push(StoveEvent::new(EventKind::TurnCompleted, metadata.clone()));
        }
        (_, Some("error" | "failed")) | ("error", _) => {
            events.push(StoveEvent::new(EventKind::SessionFailed, metadata.clone()));
        }
        // Subagents are activity records, not failures or completion of the
        // parent session; their lifecycle keeps the parent cooking.
        (_, Some("subagent_started" | "subagent")) | ("subagent", _) => {
            events.push(StoveEvent::new(EventKind::ToolStarted, metadata.clone()));
        }
        (_, Some("subagent_completed")) => {
            events.push(StoveEvent::new(
                EventKind::ToolCompleted { succeeded: true },
                metadata.clone(),
            ));
        }
        _ => {}
    }

    let title = ["session_name", "title", "name"]
        .iter()
        .find_map(|key| bounded_string(object.get(*key), limits.max_json_field_bytes));
    (!events.is_empty() || title.is_some()).then_some(ParsedRecord { title, events })
}

fn timestamp_ms(value: &Value) -> Option<u64> {
    value.as_u64().or_else(|| value.as_str()?.parse().ok())
}

fn bounded_string(value: Option<&Value>, limit: usize) -> Option<String> {
    let value = value?.as_str()?;
    (value.len() <= limit).then(|| value.to_owned())
}

fn contains_tool_use(value: &Value) -> bool {
    let Some(content) = value.pointer("/message/content").and_then(Value::as_array) else {
        return false;
    };
    content
        .iter()
        .any(|entry| entry.get("type").and_then(Value::as_str) == Some("tool_use"))
}

fn tool_result_failed(value: &Value) -> bool {
    value
        .get("is_error")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || value
            .pointer("/content/is_error")
            .and_then(Value::as_bool)
            .unwrap_or(false)
}

fn within_depth(value: &Value, depth: usize, maximum: usize) -> bool {
    if depth > maximum {
        return false;
    }
    match value {
        Value::Array(values) => values
            .iter()
            .all(|child| within_depth(child, depth + 1, maximum)),
        Value::Object(values) => values
            .values()
            .all(|child| within_depth(child, depth + 1, maximum)),
        _ => true,
    }
}
