//! Persistence for data owned by Cookbench.
//!
//! Native harness session files remain authoritative. These files contain only
//! Cookbench preferences, completion retention, and clear cursors.

mod atomic_file;
mod config;
mod dock;
mod layout;
mod state;

pub use atomic_file::{AtomicJsonFile, PersistenceError, Versioned};
pub use config::{
    AppLocale, BarLayout, CredentialReference, GlobalBarMode, GlobalBarPlacement,
    GlobalBarPosition, LocalNotificationPreferences, NotificationDestinationConfig,
    PersistedConfig, RemoteSourceConfig, UserPreferences, MAX_MAC_STATUS_STOVE_COUNT,
};
pub use dock::{
    dock_threshold_physical, dock_upper_threshold_physical, resolve_top_dock, top_dock_decision,
    DockMonitorWorkArea, GlobalBarTopDock, TopDockDecision, TopDockGeometry, TopDockInput,
    TOP_DOCK_HIDE_DELAY_MS, TOP_DOCK_THRESHOLD_LOGICAL_PX, TOP_DOCK_TRIGGER_LOGICAL_PX,
    TOP_UNDOCK_THRESHOLD_LOGICAL_PX,
};
pub use layout::{
    resolve_saved_monitor, DetachedStoveLayout, MonitorIdentity, MonitorWorkArea, RelativePosition,
    RestoredDetachedStoveLayout, WindowPosition, WindowSize,
};
pub use state::{
    ArchiveReason, ArchivedSession, ClearCursor, CookedAttentionCursor, PersistedState,
    PinnedSession, RetainedStove, RetainedStovePresentation, SessionRecord,
};
