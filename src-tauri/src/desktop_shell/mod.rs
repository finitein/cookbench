//! Native desktop-shell policy for Cookbench-owned presentation only.
//!
//! Runtime wiring lives at the application boundary. This module keeps menu,
//! shortcut, and startup decisions deterministic and testable without exiting
//! the application, registering an OS shortcut, or changing login items.

mod controller;
pub mod runtime;
pub mod status_stoves;

pub use controller::{
    autostart_transition, default_autostart_enabled, default_toggle_shortcut, shortcut_plan,
    tray_action, tray_menu, AutostartTransition, DesktopShellDiagnostic, ShortcutPlan, TrayAction,
    TrayMenuItem,
};
