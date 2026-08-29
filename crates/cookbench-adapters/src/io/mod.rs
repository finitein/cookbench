//! Bounded, read-only helpers for native JSONL session files.
//!
//! These helpers intentionally expose records one line at a time. They do not
//! retain transcripts, parse harness-specific payloads, or make a file source
//! authoritative over the native harness that owns it.

mod directory_watch;
mod jsonl_tailer;
mod limits;

pub use directory_watch::{discover_jsonl_files, DirectoryWatch, DirectoryWatchError};
pub use jsonl_tailer::{JsonlTailer, TailError, TailRecord, TailRejection};
pub use limits::TailLimits;
