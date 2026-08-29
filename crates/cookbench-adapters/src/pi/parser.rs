use std::path::Path;

use cookbench_core::domain::{EventKind, EventMetadata, EventSource, StoveEvent};
use serde_json::Value;

use crate::{
    io::{JsonlTailer, TailLimits, TailRecord},
    AdapterError,
};

const MAX_RECORD_BYTES: usize = 64 * 1024;
const MAX_DEPTH: usize = 24;
const MAX_EVENTS_PER_SESSION: usize = 4_096;

/// A bounded, content-free projection of a Pi session file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedPiSession {
    pub native_session_id: String,
    pub title: Option<String>,
    pub project: Option<String>,
    pub events: Vec<StoveEvent>,
}

pub fn parse_session_file(path: &Path) -> Result<ParsedPiSession, AdapterError> {
    let root = path
        .parent()
        .ok_or_else(|| AdapterError::Message("Pi session has no parent directory".into()))?;
    let limits = TailLimits {
        max_record_bytes: MAX_RECORD_BYTES,
        max_partial_bytes: MAX_RECORD_BYTES,
        ..TailLimits::default()
    };
    let mut tailer = JsonlTailer::open(root, path, limits)
        .map_err(|error| AdapterError::Message(error.to_string()))?;
    let mut state = initial_state(path);
    loop {
        let records = tailer
            .poll()
            .map_err(|error| AdapterError::Message(error.to_string()))?;
        if records.is_empty() {
            break;
        }
        for record in records {
            if let TailRecord::Record(line) = record {
                parse_line(&line, &mut state);
            }
        }
    }
    Ok(finish(state))
}

/// Parses one appended Pi record into lifecycle events without retaining its
/// content. Runtime observers use this rather than reloading a full session.
pub fn parse_record(line: &str, sequence: u64) -> Vec<StoveEvent> {
    let mut state = ParseState {
        native_session_id: String::new(),
        title: None,
        project: None,
        events: Vec::new(),
        sequence: 0,
    };
    parse_line(line, &mut state);
    state
        .events
        .into_iter()
        .enumerate()
        .map(|(index, mut event)| {
            event.metadata.sequence = sequence.saturating_add(index as u64);
            event
        })
        .collect()
}

#[cfg(test)]
fn parse_session_text(path: &Path, contents: &str) -> Result<ParsedPiSession, AdapterError> {
    let mut state = initial_state(path);
    for line in contents.lines() {
        parse_line(line, &mut state);
    }
    Ok(finish(state))
}

fn initial_state(path: &Path) -> ParseState {
    let fallback_id = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("pi-session")
        .to_owned();
    ParseState {
        native_session_id: fallback_id,
        title: None,
        project: None,
        events: Vec::new(),
        sequence: 0,
    }
}

fn parse_line(line: &str, state: &mut ParseState) {
    if line.len() > MAX_RECORD_BYTES {
        return;
    }
    let Ok(value) = serde_json::from_str::<Value>(line) else {
        return;
    };
    visit_value(&value, state, 0);
}

fn finish(state: ParseState) -> ParsedPiSession {
    ParsedPiSession {
        native_session_id: state.native_session_id,
        title: state.title,
        project: state.project,
        events: state.events,
    }
}

struct ParseState {
    native_session_id: String,
    title: Option<String>,
    project: Option<String>,
    events: Vec<StoveEvent>,
    sequence: u64,
}

fn visit_value(value: &Value, state: &mut ParseState, depth: usize) {
    if depth > MAX_DEPTH {
        return;
    }
    let Value::Object(object) = value else {
        if let Value::Array(values) = value {
            for value in values {
                visit_value(value, state, depth + 1);
            }
        }
        return;
    };

    populate_metadata(object, state);
    if state.events.len() < MAX_EVENTS_PER_SESSION {
        if let Some(kind) = event_kind(object) {
            state.sequence += 1;
            state.events.push(StoveEvent::new(
                kind,
                EventMetadata::new(
                    EventSource::StructuredSession,
                    100,
                    state.sequence,
                    timestamp_ms(object),
                ),
            ));
        }
    }
    for value in object.values() {
        if value.is_array() || value.is_object() {
            visit_value(value, state, depth + 1);
        }
    }
}

fn populate_metadata(object: &serde_json::Map<String, Value>, state: &mut ParseState) {
    let is_session_header = record_type(object)
        .is_some_and(|kind| matches!(kind.as_str(), "session" | "session_start" | "sessionstart"));
    let id_fields = if is_session_header {
        &["sessionId", "session_id", "id"][..]
    } else {
        &["sessionId", "session_id"][..]
    };
    if let Some(id) = bounded_string(object, id_fields) {
        state.native_session_id = id;
    }
    if state.title.is_none() {
        let title_fields = if is_session_header {
            &["sessionName", "session_name", "title", "name"][..]
        } else {
            &["sessionName", "session_name", "title"][..]
        };
        state.title = bounded_string(object, title_fields);
    }
    if state.project.is_none() {
        state.project = bounded_string(
            object,
            &[
                "cwd",
                "projectPath",
                "project_path",
                "workingDirectory",
                "working_directory",
            ],
        );
    }
}

fn event_kind(object: &serde_json::Map<String, Value>) -> Option<EventKind> {
    let record_type = record_type(object)?;
    match record_type.as_str() {
        "session" | "session_start" | "sessionstart" => Some(EventKind::SessionDiscovered),
        "prompt" | "user_prompt" | "user_message" => Some(EventKind::UserPromptSubmitted),
        "message"
            if string_field(object, "role")
                .is_some_and(|role| role.eq_ignore_ascii_case("user")) =>
        {
            Some(EventKind::UserPromptSubmitted)
        }
        "tool_call" | "toolcall" | "tool_start" | "toolstarted" => Some(EventKind::ToolStarted),
        "tool_result" | "toolresult" | "tool_completed" | "toolcompleted" => {
            Some(EventKind::ToolCompleted {
                succeeded: !is_failure(object),
            })
        }
        "question" | "question_asked" | "questionasked" => Some(EventKind::QuestionAsked),
        "permission" | "permission_request" | "permissionrequested" => {
            Some(EventKind::PermissionRequested)
        }
        "error" | "failed" | "session_failed" => Some(EventKind::SessionFailed),
        "turn_completed" | "turncomplete" | "completion" | "session_complete"
            if is_normal_completion(object) =>
        {
            Some(EventKind::TurnCompleted)
        }
        "todo" | "todos" | "task" | "task_list" | "custom" => todo_progress(object)
            .map(|(completed, total)| EventKind::PlanUpdated { completed, total }),
        _ => None,
    }
}

fn record_type(object: &serde_json::Map<String, Value>) -> Option<String> {
    bounded_string(
        object,
        &["type", "event", "kind", "entryType", "entry_type"],
    )
    .map(|value| value.to_ascii_lowercase().replace(['-', ' '], "_"))
}

fn todo_progress(object: &serde_json::Map<String, Value>) -> Option<(u32, u32)> {
    let entries = ["items", "todos", "tasks", "entries"]
        .into_iter()
        .find_map(|key| object.get(key).and_then(Value::as_array))?;
    let total = u32::try_from(entries.len()).ok()?;
    if total == 0 {
        return None;
    }
    let completed = entries
        .iter()
        .filter(|entry| {
            entry.as_object().is_some_and(|entry| {
                entry.get("completed").and_then(Value::as_bool) == Some(true)
                    || entry.get("done").and_then(Value::as_bool) == Some(true)
                    || entry
                        .get("status")
                        .and_then(Value::as_str)
                        .is_some_and(|status| {
                            matches!(
                                status.to_ascii_lowercase().as_str(),
                                "done" | "completed" | "complete"
                            )
                        })
            })
        })
        .count();
    Some((u32::try_from(completed).ok()?, total))
}

fn is_normal_completion(object: &serde_json::Map<String, Value>) -> bool {
    ["status", "outcome", "reason"].into_iter().any(|key| {
        string_field(object, key).is_some_and(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "normal" | "success" | "succeeded" | "complete" | "completed"
            )
        })
    })
}

fn is_failure(object: &serde_json::Map<String, Value>) -> bool {
    object.get("success").and_then(Value::as_bool) == Some(false)
        || object.get("failed").and_then(Value::as_bool) == Some(true)
        || string_field(object, "status").is_some_and(|status| {
            matches!(
                status.to_ascii_lowercase().as_str(),
                "error" | "failed" | "failure"
            )
        })
}

fn timestamp_ms(object: &serde_json::Map<String, Value>) -> u64 {
    ["timestampMs", "timestamp_ms", "timestamp"]
        .into_iter()
        .find_map(|key| object.get(key))
        .and_then(|value| value.as_u64())
        .unwrap_or(0)
}

fn bounded_string(object: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| string_field(object, key))
        .filter(|value| !value.is_empty() && value.len() <= 512)
        .map(ToOwned::to_owned)
}

fn string_field<'a>(object: &'a serde_json::Map<String, Value>, key: &str) -> Option<&'a str> {
    object.get(key)?.as_str()
}

#[cfg(test)]
mod tests {
    use super::parse_session_text;
    use cookbench_core::domain::EventKind;
    use std::path::Path;

    #[test]
    fn excludes_prompt_content_from_the_projection() {
        let parsed = parse_session_text(
            Path::new("safe.jsonl"),
            r#"{"type":"prompt","content":"must never be retained"}"#,
        )
        .unwrap();
        assert!(matches!(
            parsed.events[0].kind,
            EventKind::UserPromptSubmitted
        ));
        assert_ne!(format!("{parsed:?}"), "must never be retained");
    }
}
