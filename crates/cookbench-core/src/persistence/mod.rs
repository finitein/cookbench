//! Persistence for data owned by Cookbench.
//!
//! Native harness session files remain authoritative. These files contain only
//! Cookbench preferences, completion retention, and clear cursors.

mod atomic_file;
mod config;
mod layout;
mod state;

pub use atomic_file::{AtomicJsonFile, PersistenceError, Versioned};
pub use config::{
    BarLayout, CredentialReference, GlobalBarPlacement, GlobalBarPosition,
    LocalNotificationPreferences, NotificationDestinationConfig, PersistedConfig,
    RemoteSourceConfig, UserPreferences,
};
pub use layout::{
    DetachedStoveLayout, MonitorIdentity, MonitorWorkArea, RelativePosition,
    RestoredDetachedStoveLayout, WindowPosition, WindowSize,
};
pub use state::{
    ArchiveReason, ArchivedSession, ClearCursor, PersistedState, PinnedSession, RetainedStove,
    RetainedStovePresentation, SessionRecord,
};
