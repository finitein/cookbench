//! Read-only support for native Codex session files.
//!
//! This module observes JSONL records written by Codex. It never starts,
//! controls, or rewrites a Codex process or its configuration.

mod discovery;
mod hook;
mod parser;
mod progress;

pub use discovery::{
    codex_home_from, correlate_processes, default_codex_home, CodexAdapter, CodexProcess,
};
pub use hook::{inspect_notify_hook, NotifyHookPlan};
pub use parser::{parse_record, sanitize_fixture_record, CodexRecord};
pub use progress::plan_progress;
