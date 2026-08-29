//! Minimal, privacy-preserving metadata used to return a user to a session.
//!
//! A locator deliberately contains only host-surface correlation data. It is
//! not a transcript, prompt, command history, credential store, or agent
//! control channel.

mod model;

pub use model::{HostApplication, LocatorValidationError, SessionLocator, TerminalKind};
