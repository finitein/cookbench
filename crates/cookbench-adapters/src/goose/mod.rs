//! Read-only discovery and lifecycle observation for Block Goose.
//!
//! Legacy Goose sessions remain on disk as `~/.local/share/goose/sessions/*.jsonl`
//! even after the SQLite migration. Cookbench treats those native files as
//! authoritative: identity comes from the first metadata line, and lifecycle
//! comes from an allowlisted role/content-type map. Message text is never
//! retained. The SQLite `sessions.db` file is ignored.

mod discovery;
mod parser;

pub use discovery::{default_sessions_root, discover_sessions, session_from_path, GooseAdapter};
pub use parser::parse_record;
