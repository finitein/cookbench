//! Allowlisted Goose legacy JSONL → Cookbench lifecycle mapping.
//!
//! Goose legacy sessions store a metadata header followed by role/content
//! records. Cookbench reads only bounded role and content-type discriminators.
//! Message text, tool arguments, and thoughts are never retained.

use cookbench_core::domain::{EventKind, EventMetadata, EventSource, StoveEvent};
use serde_json::Value;

const MAX_RECORD_BYTES: usize = 64 * 1024;
const MAX_NESTING: usize = 16;
const MAX_FIELD_BYTES: usize = 256;
const MAX_CONTENT_BLOCKS: usize = 64;

/// Parses one Goose legacy JSONL line into zero or more lifecycle events.
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

    // Metadata headers are discovery-only.
    if object.contains_key("working_dir")
        || object.contains_key("description")
        || (object.contains_key("id") && !object.contains_key("role"))
    {
        return Vec::new();
    }

    let metadata = EventMetadata::new(
        EventSource::StructuredSession,
        85,
        sequence,
        object
            .get("created")
            .and_then(timestamp_ms)
            .or_else(|| object.get("created_at").and_then(timestamp_ms))
            .unwrap_or(sequence),
    );

    let mut events = Vec::new();
    if let Some(role) = bounded_string(object.get("role"), MAX_FIELD_BYTES) {
        match role.as_str() {
            "user" => events.push(StoveEvent::new(
                EventKind::UserPromptSubmitted,
                metadata.clone(),
            )),
            "assistant" => {
                // Assistant text alone is not completion evidence.
            }
            _ => {}
        }
    }

    if let Some(blocks) = object.get("content").and_then(Value::as_array) {
        if blocks.len() <= MAX_CONTENT_BLOCKS {
            for block in blocks {
                let Some(block) = block.as_object() else {
                    continue;
                };
                let Some(kind) = bounded_string(block.get("type"), MAX_FIELD_BYTES) else {
                    continue;
                };
                match kind.as_str() {
                    "toolRequest" | "tool_use" | "toolCall" | "tool_call" => {
                        events.push(StoveEvent::new(EventKind::ToolStarted, metadata.clone()));
                    }
                    "toolResponse" | "tool_result" | "toolResult" | "tool_call_result" => {
                        let succeeded =
                            !matches!(block.get("is_error").and_then(Value::as_bool), Some(true));
                        events.push(StoveEvent::new(
                            EventKind::ToolCompleted { succeeded },
                            metadata.clone(),
                        ));
                    }
                    // Never retain text / image / thinking payloads.
                    "text" | "image" | "thinking" | "redacted_thinking" => {}
                    _ => {}
                }
            }
        }
    }

    // Optional allowlisted Goose system notifications (content-free).
    if let Some(notification) = bounded_string(object.get("notification"), MAX_FIELD_BYTES) {
        match notification.as_str() {
            "permission_required" | "waiting_for_user" => {
                events.push(StoveEvent::new(
                    EventKind::PermissionRequested,
                    metadata.clone(),
                ));
            }
            "turn_completed" | "session_finished" => {
                events.push(StoveEvent::new(EventKind::TurnCompleted, metadata.clone()));
            }
            "error" | "failed" => {
                events.push(StoveEvent::new(EventKind::SessionFailed, metadata));
            }
            _ => {}
        }
    }

    events
}

fn timestamp_ms(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .map(|seconds_or_ms| {
            if seconds_or_ms < 1_000_000_000_000 {
                seconds_or_ms.saturating_mul(1_000)
            } else {
                seconds_or_ms
            }
        })
        .or_else(|| value.as_str()?.parse().ok())
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
    fn maps_allowlisted_roles_and_ignores_message_text() {
        let kind = |line: &str| {
            parse_record(line, 3)
                .into_iter()
                .map(|event| event.kind)
                .collect::<Vec<_>>()
        };

        assert!(kind(
            r#"{"id":"20260906_1","description":"Synthetic","working_dir":"/synthetic"}"#
        )
        .is_empty());
        assert_eq!(
            kind(
                r#"{"role":"user","created":1710000000,"content":[{"type":"text","text":"secret"}]}"#
            ),
            vec![EventKind::UserPromptSubmitted]
        );
        assert_eq!(
            kind(
                r#"{"role":"assistant","content":[{"type":"toolRequest","id":"1","toolCall":{"name":"list"}}]}"#
            ),
            vec![EventKind::ToolStarted]
        );
        assert_eq!(
            kind(
                r#"{"role":"user","content":[{"type":"toolResponse","id":"1","is_error":false}]}"#
            ),
            vec![
                EventKind::UserPromptSubmitted,
                EventKind::ToolCompleted { succeeded: true }
            ]
        );
        assert_eq!(
            kind(r#"{"notification":"permission_required"}"#),
            vec![EventKind::PermissionRequested]
        );
        assert_eq!(
            kind(r#"{"notification":"turn_completed"}"#),
            vec![EventKind::TurnCompleted]
        );
        assert_eq!(
            kind(r#"{"notification":"failed"}"#),
            vec![EventKind::SessionFailed]
        );
        assert!(kind(r#"{"role":"assistant","content":[{"type":"text","text":"hi"}]}"#).is_empty());
    }
}
