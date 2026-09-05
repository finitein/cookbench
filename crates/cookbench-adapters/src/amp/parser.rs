//! Allowlisted Amp thread JSON → Cookbench lifecycle mapping.
//!
//! Amp stores one pretty-printed (or minified) JSON document per thread.
//! Cookbench reads only bounded role and content-type discriminators from the
//! `messages` array. Message text, tool arguments, thinking blocks, and usage
//! payloads are never retained.

use cookbench_core::domain::{EventKind, EventMetadata, EventSource, StoveEvent};
use serde_json::Value;

const MAX_DOCUMENT_BYTES: usize = 256 * 1024;
const MAX_NESTING: usize = 16;
const MAX_FIELD_BYTES: usize = 256;
const MAX_MESSAGES: usize = 512;
const MAX_CONTENT_BLOCKS: usize = 64;

/// Parses one Amp thread document into zero or more lifecycle events.
pub fn parse_thread(document: &str) -> Vec<StoveEvent> {
    if document.len() > MAX_DOCUMENT_BYTES {
        return Vec::new();
    }
    let Ok(value) = serde_json::from_str::<Value>(document) else {
        return Vec::new();
    };
    if !within_depth(&value, 0, MAX_NESTING) {
        return Vec::new();
    }
    let Some(object) = value.as_object() else {
        return Vec::new();
    };
    // Format sniff: Amp thread documents carry a revision counter `v`.
    if !object.contains_key("v") {
        return Vec::new();
    }
    let Some(messages) = object.get("messages").and_then(Value::as_array) else {
        return Vec::new();
    };
    if messages.len() > MAX_MESSAGES {
        return Vec::new();
    }

    let mut events = Vec::new();
    for (index, message) in messages.iter().enumerate() {
        let Some(message) = message.as_object() else {
            continue;
        };
        let sequence = message
            .get("messageId")
            .and_then(Value::as_u64)
            .unwrap_or(index as u64)
            .saturating_add(1);
        let timestamp_ms = message
            .get("meta")
            .and_then(|meta| meta.get("sentAt"))
            .and_then(timestamp_ms)
            .or_else(|| object.get("created").and_then(timestamp_ms))
            .unwrap_or(sequence);
        let metadata =
            EventMetadata::new(EventSource::StructuredSession, 85, sequence, timestamp_ms);

        let role = bounded_string(message.get("role"), MAX_FIELD_BYTES);
        let mut saw_user_text = false;
        let mut saw_tool_use = false;
        let mut tool_completed: Option<bool> = None;

        if let Some(blocks) = message.get("content").and_then(Value::as_array) {
            if blocks.len() <= MAX_CONTENT_BLOCKS {
                for block in blocks {
                    let Some(block) = block.as_object() else {
                        continue;
                    };
                    let Some(kind) = bounded_string(block.get("type"), MAX_FIELD_BYTES) else {
                        continue;
                    };
                    match kind.as_str() {
                        "text" | "image" | "thinking" | "redacted_thinking" => {
                            if role.as_deref() == Some("user") {
                                saw_user_text = true;
                            }
                            // Never retain payloads.
                        }
                        "tool_use" | "toolUse" | "tool_call" | "toolCall" => {
                            saw_tool_use = true;
                        }
                        "tool_result" | "toolResult" | "tool_call_result" => {
                            let succeeded = tool_result_succeeded(block);
                            tool_completed = Some(succeeded);
                        }
                        _ => {}
                    }
                }
            }
        }

        match role.as_deref() {
            Some("user") => {
                if saw_user_text {
                    events.push(StoveEvent::new(
                        EventKind::UserPromptSubmitted,
                        metadata.clone(),
                    ));
                }
                if let Some(succeeded) = tool_completed {
                    events.push(StoveEvent::new(
                        EventKind::ToolCompleted { succeeded },
                        metadata.clone(),
                    ));
                }
            }
            Some("assistant") => {
                if saw_tool_use {
                    events.push(StoveEvent::new(EventKind::ToolStarted, metadata.clone()));
                }
                if let Some(state) = message.get("state").and_then(Value::as_object) {
                    let state_type = bounded_string(state.get("type"), MAX_FIELD_BYTES);
                    let stop = bounded_string(state.get("stopReason"), MAX_FIELD_BYTES);
                    match (state_type.as_deref(), stop.as_deref()) {
                        (Some("complete"), Some("end_turn") | Some("stop") | None) => {
                            // end_turn / stop without tool_use means the turn finished.
                            if !saw_tool_use {
                                events.push(StoveEvent::new(
                                    EventKind::TurnCompleted,
                                    metadata.clone(),
                                ));
                            }
                        }
                        (Some("complete"), Some("tool_use")) => {}
                        (Some("cancelled") | Some("aborted"), _) => {
                            events
                                .push(StoveEvent::new(EventKind::SessionFailed, metadata.clone()));
                        }
                        (Some("error") | Some("failed"), _) => {
                            events
                                .push(StoveEvent::new(EventKind::SessionFailed, metadata.clone()));
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    events
}

fn tool_result_succeeded(block: &serde_json::Map<String, Value>) -> bool {
    if matches!(block.get("is_error").and_then(Value::as_bool), Some(true)) {
        return false;
    }
    if let Some(run) = block.get("run").and_then(Value::as_object) {
        if let Some(status) = bounded_string(run.get("status"), MAX_FIELD_BYTES) {
            if matches!(status.as_str(), "error" | "failed" | "cancelled") {
                return false;
            }
        }
        if let Some(result) = run.get("result").and_then(Value::as_object) {
            if matches!(result.get("exitCode").and_then(Value::as_i64), Some(code) if code != 0) {
                return false;
            }
        }
    }
    true
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
        let kinds = |document: &str| {
            parse_thread(document)
                .into_iter()
                .map(|event| event.kind)
                .collect::<Vec<_>>()
        };

        assert!(kinds(r#"{"id":"T-1","title":"x"}"#).is_empty());

        let document = r#"{
          "v": 1,
          "id": "T-0199aaaa-bbbb-7ccc-8ddd-eeeeffff0001",
          "created": 1757116800000,
          "title": "Synthetic",
          "messages": [
            {"role":"user","messageId":0,"meta":{"sentAt":1757116801000},"content":[{"type":"text","text":"secret"}]},
            {"role":"assistant","messageId":1,"content":[{"type":"tool_use","id":"t1","name":"Bash","input":{"cmd":"secret"}}],"state":{"type":"complete","stopReason":"tool_use"}},
            {"role":"user","messageId":2,"content":[{"type":"tool_result","toolUseID":"t1","run":{"status":"done","result":{"exitCode":0}}}]},
            {"role":"assistant","messageId":3,"content":[{"type":"text","text":"done"}],"state":{"type":"complete","stopReason":"end_turn"}}
          ]
        }"#;

        let mapped = kinds(document);
        assert!(mapped.contains(&EventKind::UserPromptSubmitted));
        assert!(mapped.contains(&EventKind::ToolStarted));
        assert!(mapped.contains(&EventKind::ToolCompleted { succeeded: true }));
        assert!(mapped.contains(&EventKind::TurnCompleted));
        assert!(!mapped
            .iter()
            .any(|kind| matches!(kind, EventKind::SessionFailed)));
    }

    #[test]
    fn maps_cancelled_assistant_state_to_session_failed() {
        let document = r#"{
          "v": 2,
          "id": "T-fail",
          "messages": [
            {"role":"assistant","messageId":0,"content":[],"state":{"type":"cancelled"}}
          ]
        }"#;
        assert_eq!(
            parse_thread(document)
                .into_iter()
                .map(|event| event.kind)
                .collect::<Vec<_>>(),
            vec![EventKind::SessionFailed]
        );
    }
}
