//! Bounded projection of remote JSONL suffixes through first-party adapters.
//!
//! This preserves adapter parsing behavior without retaining a prompt, command,
//! code fragment, or complete remote conversation in the SSH source.

use cookbench_adapters::{claude, codex, io::TailLimits, pi};
use cookbench_core::domain::{EventKind, HarnessId};

use super::zero_install::{ParsedRemoteSession, RemoteLifecycleParser};

const MAX_RECORDS_PER_SUFFIX: usize = 256;
const MAX_RECORD_BYTES: usize = 64 * 1024;

#[derive(Default)]
pub struct FirstPartyRemoteParser;

impl RemoteLifecycleParser for FirstPartyRemoteParser {
    fn parse_suffix(&self, path: &str, suffix: &[u8]) -> Option<ParsedRemoteSession> {
        let text = std::str::from_utf8(suffix).ok()?;
        match harness_for_path(path) {
            Some(HarnessId::Codex) => parse_codex(path, text),
            Some(HarnessId::ClaudeCode) => parse_claude(path, text),
            Some(HarnessId::Pi) => parse_pi(path, text),
            Some(HarnessId::Other(_)) | None => parse_codex(path, text)
                .or_else(|| parse_claude(path, text))
                .or_else(|| parse_pi(path, text)),
        }
    }
}

fn parse_codex(path: &str, text: &str) -> Option<ParsedRemoteSession> {
    let mut native_session_id = None;
    let mut project_root = None;
    let mut events = Vec::new();
    for (index, line) in bounded_lines(text).enumerate() {
        if let Some(record) = codex::parse_record(line, index as u64 + 1, 24, 16 * 1024) {
            native_session_id = native_session_id.or(record.session_id);
            project_root = project_root.or(record.cwd);
            if let Some(event) = record.event {
                events.push(event.kind);
            }
        }
    }
    parsed(
        HarnessId::Codex,
        native_session_id.unwrap_or_else(|| file_stem(path)),
        project_root,
        events,
    )
}

fn parse_claude(path: &str, text: &str) -> Option<ParsedRemoteSession> {
    let limits = TailLimits {
        max_record_bytes: MAX_RECORD_BYTES,
        max_partial_bytes: MAX_RECORD_BYTES,
        max_json_nesting: 24,
        max_json_field_bytes: 16 * 1024,
        max_read_bytes_per_poll: MAX_RECORD_BYTES,
    };
    let mut events = Vec::new();
    for (index, line) in bounded_lines(text).enumerate() {
        if let Some(record) = claude::parse_record(line, limits, index as u64 + 1) {
            events.extend(record.events.into_iter().map(|event| event.kind));
        }
    }
    parsed(HarnessId::ClaudeCode, file_stem(path), None, events)
}

fn parse_pi(path: &str, text: &str) -> Option<ParsedRemoteSession> {
    let mut events = Vec::new();
    for (index, line) in bounded_lines(text).enumerate() {
        events.extend(
            pi::parse_record(line, index as u64 + 1)
                .into_iter()
                .map(|event| event.kind),
        );
    }
    parsed(HarnessId::Pi, file_stem(path), None, events)
}

fn bounded_lines(text: &str) -> impl Iterator<Item = &str> {
    text.lines()
        .filter(|line| !line.is_empty() && line.len() <= MAX_RECORD_BYTES)
        .take(MAX_RECORDS_PER_SUFFIX)
}

fn parsed(
    harness: HarnessId,
    native_session_id: String,
    project_root: Option<String>,
    events: Vec<EventKind>,
) -> Option<ParsedRemoteSession> {
    (!native_session_id.is_empty() && !events.is_empty()).then_some(ParsedRemoteSession {
        harness,
        native_session_id,
        project_root,
        events,
    })
}

fn harness_for_path(path: &str) -> Option<HarnessId> {
    if path.contains("/.codex/") || path.contains("/codex/") {
        Some(HarnessId::Codex)
    } else if path.contains("/.claude/") || path.contains("/claude/") {
        Some(HarnessId::ClaudeCode)
    } else if path.contains("/.pi/") || path.contains("/pi/") {
        Some(HarnessId::Pi)
    } else {
        None
    }
}

fn file_stem(path: &str) -> String {
    path.rsplit('/')
        .next()
        .unwrap_or_default()
        .strip_suffix(".jsonl")
        .unwrap_or_default()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use cookbench_core::domain::{EventKind, HarnessId};

    use super::{FirstPartyRemoteParser, RemoteLifecycleParser};

    #[test]
    fn parses_codex_claude_and_pi_suffixes_without_retaining_content() {
        let parser = FirstPartyRemoteParser;
        let codex = parser
            .parse_suffix(
                "/remote/.codex/sessions/codex-safe.jsonl",
                b"{\"type\":\"session_started\"}\n{\"type\":\"tool_started\"}\n",
            )
            .unwrap();
        let claude = parser
            .parse_suffix(
                "/remote/.claude/projects/claude-safe.jsonl",
                b"{\"type\":\"user\"}\n",
            )
            .unwrap();
        let pi = parser
            .parse_suffix(
                "/remote/.pi/agent/sessions/pi-safe.jsonl",
                b"{\"type\":\"prompt\"}\n",
            )
            .unwrap();

        assert_eq!(codex.harness, HarnessId::Codex);
        assert!(matches!(codex.events[1], EventKind::ToolStarted));
        assert_eq!(claude.harness, HarnessId::ClaudeCode);
        assert!(matches!(claude.events[0], EventKind::UserPromptSubmitted));
        assert_eq!(pi.harness, HarnessId::Pi);
        assert!(matches!(pi.events[0], EventKind::UserPromptSubmitted));
    }

    #[test]
    fn malformed_or_content_only_records_do_not_create_a_session() {
        let parser = FirstPartyRemoteParser;
        assert!(parser
            .parse_suffix("/remote/.codex/sessions/safe.jsonl", b"not json\n")
            .is_none());
    }
}
