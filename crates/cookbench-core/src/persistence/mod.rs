//! Persistence for data owned by Cookbench.
//!
//! Native harness session files remain authoritative. These files contain only
//! Cookbench preferences, completion retention, and clear cursors.

mod atomic_file;
mod config;
mod state;

pub use atomic_file::{AtomicJsonFile, PersistenceError, Versioned};
pub use config::{BarLayout, CredentialReference, PersistedConfig, UserPreferences};
pub use state::{ClearCursor, PersistedState, RetainedStove};
