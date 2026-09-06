//! Read-only discovery and lifecycle observation for xAI Grok Build.
//!
//! Grok Build stores each session under `~/.grok/sessions/<cwd>/<id>/` with a
//! `summary.json` index and an `updates.jsonl` conversation log. Cookbench
//! treats native files as authoritative: identity comes from `summary.json`,
//! and lifecycle comes from an allowlisted ACP `sessionUpdate` map over
//! `updates.jsonl`. Message text, tool arguments, and thoughts are never
//! retained.

mod discovery;
mod parser;

pub use discovery::{default_sessions_root, discover_sessions, session_from_path, GrokAdapter};
pub use parser::parse_record;
