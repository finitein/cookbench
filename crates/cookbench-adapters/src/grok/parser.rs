//! Allowlisted ACP sessionUpdate → Cookbench lifecycle mapping for Grok Build.
//!
//! Grok Build persists flattened `updates.jsonl` records. Cookbench reads only
//! bounded lifecycle fields (`sessionUpdate`, `status`, `state`, `stopReason`,
//! and plan entry statuses). Message text, tool arguments, and thoughts are
//! never retained.

use cookbench_core::domain::{EventKind, EventMetadata, EventSource, StoveEvent};
use serde_json::Value;

const MAX_RECORD_BYTES: usize = 64 * 1024;
const MAX_NESTING: usize = 16;
const MAX_FIELD_BYTES: usize = 256;
const MAX_PLAN_ENTRIES: usize = 64;

/// Parses one Grok Build `updates.jsonl` line into zero or more lifecycle events.
///
/// Unknown `sessionUpdate` values and content-bearing fields are ignored.
pub fn parse_record(line: &str, sequence: u64) -> Vec<StoveEvent> {
    if line.len() > MAX_RECORD_BYTES {
        return Vec::new();
    }
    let Ok(value) = serde_json::from_str::<Value>(line) else {
        return Vec::new();
    };
    if !within_depth(&value, 0, MAX_NESTING) {
        return Vec::new();
    }
    let Some(object) = value.as_object() else {
        return Vec::new();
    };
    let update = object
        .get("update")
        .and_then(Value::as_object)
        .unwrap_or(object);
    let Some(session_update) = bounded_string(update.get("sessionUpdate"), MAX_FIELD_BYTES) else {
        return Vec::new();
    };
    let metadata = EventMetadata::new(
        EventSource::StructuredSession,
        90,
        sequence,
        object
            .get("timestamp")
            .and_then(timestamp_ms)
            .or_else(|| update.get("timestamp").and_then(timestamp_ms))
            .unwrap_or(sequence),
    );

    match session_update.as_str() {
        "user_message_chunk" | "user_message" => {
            vec![StoveEvent::new(EventKind::UserPromptSubmitted, metadata)]
        }
        "agent_message_chunk" | "agent_message" => {
            // Fixture and live Grok logs mark turn starts with kind=started.
            // Treat that as cooking evidence; bare chunks stay content-free noise.
            match bounded_string(update.get("kind"), MAX_FIELD_BYTES).as_deref() {
                Some("started") => vec![StoveEvent::new(EventKind::UserPromptSubmitted, metadata)],
                _ => Vec::new(),
            }
        }
        "tool_call" | "tool_call_update" => match tool_status(session_update.as_str(), update) {
            ToolStatus::Started => vec![StoveEvent::new(EventKind::ToolStarted, metadata)],
            ToolStatus::Completed { succeeded } => {
                vec![StoveEvent::new(
                    EventKind::ToolCompleted { succeeded },
                    metadata,
                )]
            }
            ToolStatus::Unknown => Vec::new(),
        },
        "state_update" => match bounded_string(update.get("state"), MAX_FIELD_BYTES).as_deref() {
            Some("running") => vec![StoveEvent::new(EventKind::UserPromptSubmitted, metadata)],
            Some("requires_action") => {
                vec![StoveEvent::new(EventKind::PermissionRequested, metadata)]
            }
            Some("idle") => {
                match bounded_string(update.get("stopReason"), MAX_FIELD_BYTES).as_deref() {
                    Some("end_turn" | "cancelled") => {
                        vec![StoveEvent::new(EventKind::TurnCompleted, metadata)]
                    }
                    Some("refusal" | "max_tokens" | "max_turn_requests") => {
                        vec![StoveEvent::new(EventKind::SessionFailed, metadata)]
                    }
                    // Idle without a stop reason is not completion evidence.
                    _ => Vec::new(),
                }
            }
            _ => Vec::new(),
        },
        "plan" => plan_progress(update)
            .map(|(completed, total)| {
                StoveEvent::new(EventKind::PlanUpdated { completed, total }, metadata)
            })
            .into_iter()
            .collect(),
        // Thoughts, command catalogs, mode changes, and unknown updates stay ignored.
        "agent_thought_chunk"
        | "agent_thought"
        | "available_commands_update"
        | "current_mode_update"
        | "tool_call_content_chunk"
        | "terminal_update"
        | "terminal_output_chunk" => Vec::new(),
        _ => Vec::new(),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ToolStatus {
    Started,
    Completed { succeeded: bool },
    Unknown,
}

fn tool_status(session_update: &str, update: &serde_json::Map<String, Value>) -> ToolStatus {
    match bounded_string(update.get("status"), MAX_FIELD_BYTES).as_deref() {
        Some("pending" | "in_progress") => ToolStatus::Started,
        // A fresh tool_call without status is still cooking evidence; a later
        // tool_call_update that only patches content is not.
        None if session_update == "tool_call" => ToolStatus::Started,
        Some("completed") => ToolStatus::Completed { succeeded: true },
        Some("failed") => ToolStatus::Completed { succeeded: false },
        _ => ToolStatus::Unknown,
    }
}

fn plan_progress(update: &serde_json::Map<String, Value>) -> Option<(u32, u32)> {
    let entries = update.get("entries").and_then(Value::as_array)?;
    if entries.is_empty() || entries.len() > MAX_PLAN_ENTRIES {
        return None;
    }
    let total = u32::try_from(entries.len()).ok()?;
    let mut completed = 0u32;
    for entry in entries {
        let status = entry
            .as_object()
            .and_then(|object| bounded_string(object.get("status"), MAX_FIELD_BYTES))?;
        if matches!(status.as_str(), "completed" | "done") {
            completed = completed.saturating_add(1);
        }
    }
    Some((completed, total))
}

fn timestamp_ms(value: &Value) -> Option<u64> {
    value.as_u64().or_else(|| value.as_str()?.parse().ok())
}

fn bounded_string(value: Option<&Value>, limit: usize) -> Option<String> {
    let value = value?.as_str()?;
    (value.len() <= limit).then(|| value.to_owned())
}

fn within_depth(value: &Value, depth: usize, max_depth: usize) -> bool {
    if depth > max_depth {
        return false;
    }
    match value {
        Value::Array(items) => items
            .iter()
            .all(|item| within_depth(item, depth + 1, max_depth)),
        Value::Object(map) => map
            .values()
            .all(|item| within_depth(item, depth + 1, max_depth)),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_allowlisted_lifecycle_updates_and_ignores_noise() {
        let kind = |line: &str| {
            parse_record(line, 7)
                .into_iter()
                .map(|event| event.kind)
                .collect::<Vec<_>>()
        };

        assert_eq!(
            kind(r#"{"sessionUpdate":"user_message_chunk","sessionId":"s"}"#),
            vec![EventKind::UserPromptSubmitted]
        );
        assert_eq!(
            kind(r#"{"sessionUpdate":"agent_message_chunk","kind":"started"}"#),
            vec![EventKind::UserPromptSubmitted]
        );
        assert!(kind(
            r#"{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"secret"}}"#
        )
        .is_empty());
        assert_eq!(
            kind(r#"{"sessionUpdate":"tool_call","status":"pending","toolName":"list_dir"}"#),
            vec![EventKind::ToolStarted]
        );
        assert_eq!(
            kind(r#"{"sessionUpdate":"tool_call_update","status":"completed"}"#),
            vec![EventKind::ToolCompleted { succeeded: true }]
        );
        assert_eq!(
            kind(r#"{"sessionUpdate":"tool_call","status":"failed"}"#),
            vec![EventKind::ToolCompleted { succeeded: false }]
        );
        assert_eq!(
            kind(r#"{"sessionUpdate":"state_update","state":"requires_action"}"#),
            vec![EventKind::PermissionRequested]
        );
        assert_eq!(
            kind(r#"{"sessionUpdate":"state_update","state":"idle","stopReason":"end_turn"}"#),
            vec![EventKind::TurnCompleted]
        );
        assert_eq!(
            kind(r#"{"sessionUpdate":"state_update","state":"idle","stopReason":"refusal"}"#),
            vec![EventKind::SessionFailed]
        );
        assert_eq!(
            kind(
                r#"{"sessionUpdate":"plan","entries":[{"status":"completed"},{"status":"pending"}]}"#
            ),
            vec![EventKind::PlanUpdated {
                completed: 1,
                total: 2
            }]
        );
        assert!(kind(r#"{"sessionUpdate":"agent_thought_chunk","kind":"completed"}"#).is_empty());
        assert!(kind(r#"{"sessionUpdate":"available_commands_update"}"#).is_empty());
        assert!(kind(r#"{"sessionUpdate":"not_a_real_update"}"#).is_empty());
        assert!(kind("{not-json").is_empty());
    }

    #[test]
    fn accepts_nested_acp_update_envelopes() {
        let events = parse_record(
            r#"{"sessionId":"s","update":{"sessionUpdate":"state_update","state":"running"}}"#,
            3,
        );
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0].kind, EventKind::UserPromptSubmitted));
    }
}
