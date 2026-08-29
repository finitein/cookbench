//! Read-only integration for Pi native sessions.
//!
//! Pi session files remain the authority. This module deliberately exposes only
//! bounded metadata and lifecycle state, never transcript contents.

mod discovery;
mod extension;
mod parser;

pub use discovery::PiAdapter;
pub use extension::{ExtensionEnvelope, ExtensionEvent};
pub use parser::{parse_record, parse_session_file, ParsedPiSession};
