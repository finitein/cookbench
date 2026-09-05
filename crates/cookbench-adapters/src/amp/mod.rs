//! Read-only discovery and lifecycle observation for Amp (ampcode.com).
//!
//! Local-first Amp CLIs wrote one JSON thread document per session under
//! `~/.local/share/amp/threads/T-*.json`. Cookbench treats those native files
//! as authoritative when present: identity comes from top-level `id` / `title`
//! / `env`, and lifecycle comes from an allowlisted role/content-type map.
//! Message text, tool arguments, and thoughts are never retained.
//!
//! Modern Amp is often server-authoritative and may not read or write this
//! directory. Discovery therefore stays honest about the on-disk legacy mirror
//! and does not invent cloud thread APIs.

mod discovery;
mod parser;

pub use discovery::{default_threads_root, discover_sessions, session_from_path, AmpAdapter};
pub use parser::parse_thread;
