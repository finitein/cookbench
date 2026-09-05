//! Read-only discovery for xAI Grok Build native sessions.
//!
//! Grok Build stores each session under `~/.grok/sessions/<cwd>/<id>/` with a
//! `summary.json` index and an `updates.jsonl` conversation log. Cookbench
//! treats native files as authoritative and only projects bounded identity
//! metadata from `summary.json`. Lifecycle normalization from `updates.jsonl`
//! is intentionally deferred until an allowlisted ACP event map is verified.

mod discovery;

pub use discovery::{default_sessions_root, discover_sessions, session_from_path, GrokAdapter};
