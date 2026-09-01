//! Pure, persisted geometry for the optional Global Bar top dock.
//!
//! Native windows report physical coordinates, while the product thresholds
//! are logical pixels. Keeping the conversion here makes platform adapters
//! small and lets the state machine stay deterministic in tests.

use serde::{Deserialize, Deserializer, Serialize};

use super::{MonitorIdentity, MonitorWorkArea, RelativePosition, WindowPosition, WindowSize};

pub const TOP_DOCK_THRESHOLD_LOGICAL_PX: u32 = 12;
pub const TOP_UNDOCK_THRESHOLD_LOGICAL_PX: u32 = 24;
pub const TOP_DOCK_TRIGGER_LOGICAL_PX: u32 = 3;
pub const TOP_DOCK_HIDE_DELAY_MS: u64 = 600;

const RELATIVE_SCALE: u16 = 10_000;

/// The monitor-relative horizontal anchor that survives docked monitor resize.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GlobalBarTopDock {
    pub monitor: MonitorIdentity,
    /// Position in ten-thousandths of the usable horizontal travel range.
    pub relative_x: u16,
}

impl<'de> Deserialize<'de> for GlobalBarTopDock {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawGlobalBarTopDock {
            monitor: MonitorIdentity,
            relative_x: i64,
        }

        let raw = RawGlobalBarTopDock::deserialize(deserializer)?;
        Ok(Self {
            monitor: raw.monitor,
            relative_x: raw.relative_x.clamp(0, i64::from(RELATIVE_SCALE)) as u16,
        })
    }
}

impl GlobalBarTopDock {
    pub fn from_absolute(
        monitor: &MonitorWorkArea,
        position: WindowPosition,
        size: WindowSize,
    ) -> Self {
        Self {
            monitor: monitor.identity.clone(),
            relative_x: RelativePosition::from_absolute(position, monitor, size).x,
        }
    }
}

/// A monitor work area paired with the platform scale factor used for physical
/// window coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct DockMonitorWorkArea {
    pub work_area: MonitorWorkArea,
    pub scale_factor: f64,
}

impl DockMonitorWorkArea {
    pub fn new(work_area: MonitorWorkArea, scale_factor: f64) -> Self {
        Self {
            work_area,
            scale_factor,
        }
    }

    fn physical_threshold(&self, logical_pixels: u32) -> u32 {
        dock_threshold_physical(logical_pixels, self.scale_factor)
    }

    fn dock_upper_threshold(&self) -> u32 {
        dock_upper_threshold_physical(TOP_DOCK_THRESHOLD_LOGICAL_PX, self.scale_factor)
    }
}

/// Inputs needed to choose a top-dock transition without querying native APIs.
pub struct TopDockInput<'a> {
    pub position: WindowPosition,
    pub size: WindowSize,
    pub monitors: &'a [DockMonitorWorkArea],
    pub prior_dock: Option<&'a GlobalBarTopDock>,
    /// False for platform backends where coordinates cannot be trusted.
    pub reliable_positioning: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TopDockDecision {
    Dock(GlobalBarTopDock),
    RemainDocked(GlobalBarTopDock),
    Undock,
    Freeform,
    /// Coordinates are unavailable or unreliable. Keep the persisted state
    /// and window visible rather than guessing a placement transition.
    BestEffortVisible,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TopDockGeometry {
    pub monitor: MonitorIdentity,
    pub expanded_position: WindowPosition,
    pub collapsed_position: WindowPosition,
}

/// Converts a logical threshold to physical coordinates, rounding outward so
/// a high-DPI screen never has a smaller active region than requested.
pub fn dock_threshold_physical(logical_pixels: u32, scale_factor: f64) -> u32 {
    scaled_logical_pixels(logical_pixels, scale_factor).ceil() as u32
}

/// Converts an inclusive docking upper bound to physical pixels. Flooring
/// ensures a physical coordinate is accepted only when it is no more than the
/// requested logical distance from the monitor top.
pub fn dock_upper_threshold_physical(logical_pixels: u32, scale_factor: f64) -> u32 {
    scaled_logical_pixels(logical_pixels, scale_factor).floor() as u32
}

fn scaled_logical_pixels(logical_pixels: u32, scale_factor: f64) -> f64 {
    let scale = if scale_factor.is_finite() && scale_factor > 0.0 {
        scale_factor
    } else {
        1.0
    };
    (f64::from(logical_pixels) * scale).min(f64::from(u32::MAX))
}

/// Chooses whether a moved Global Bar docks, stays docked, or returns to
/// freeform placement. Unreliable coordinates intentionally never infer dock.
pub fn top_dock_decision(input: TopDockInput<'_>) -> TopDockDecision {
    if !input.reliable_positioning {
        return TopDockDecision::BestEffortVisible;
    }
    let Some(monitor) =
        select_monitor(input.monitors, input.position, input.size, input.prior_dock)
    else {
        return TopDockDecision::Freeform;
    };
    let down_from_top = input.position.y.saturating_sub(monitor.work_area.y).max(0) as u32;

    if input.prior_dock.is_some() {
        if down_from_top >= monitor.physical_threshold(TOP_UNDOCK_THRESHOLD_LOGICAL_PX) {
            TopDockDecision::Undock
        } else {
            TopDockDecision::RemainDocked(GlobalBarTopDock::from_absolute(
                &monitor.work_area,
                input.position,
                input.size,
            ))
        }
    } else if down_from_top <= monitor.dock_upper_threshold() {
        TopDockDecision::Dock(GlobalBarTopDock::from_absolute(
            &monitor.work_area,
            input.position,
            input.size,
        ))
    } else {
        TopDockDecision::Freeform
    }
}

/// Resolves the expanded and collapsed coordinates for a persisted dock.
/// The collapsed location leaves exactly the scaled trigger strip visible.
pub fn resolve_top_dock(
    dock: &GlobalBarTopDock,
    size: WindowSize,
    monitors: &[DockMonitorWorkArea],
) -> Option<TopDockGeometry> {
    let monitor = monitors
        .iter()
        .find(|candidate| candidate.work_area.identity.id == dock.monitor.id)
        .or_else(|| {
            monitors
                .iter()
                .find(|candidate| candidate.work_area.primary)
        })
        .or_else(|| monitors.first())?;
    let relative = RelativePosition {
        x: dock.relative_x.min(RELATIVE_SCALE),
        y: 0,
    };
    let x = relative.resolve(&monitor.work_area, size).x;
    let trigger = monitor
        .physical_threshold(TOP_DOCK_TRIGGER_LOGICAL_PX)
        .min(size.height) as i64;
    let collapsed_y = i64::from(monitor.work_area.y)
        .saturating_sub(i64::from(size.height))
        .saturating_add(trigger)
        .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
    Some(TopDockGeometry {
        monitor: monitor.work_area.identity.clone(),
        expanded_position: WindowPosition {
            x,
            y: monitor.work_area.y,
        },
        collapsed_position: WindowPosition { x, y: collapsed_y },
    })
}

fn select_monitor<'a>(
    monitors: &'a [DockMonitorWorkArea],
    position: WindowPosition,
    size: WindowSize,
    prior_dock: Option<&GlobalBarTopDock>,
) -> Option<&'a DockMonitorWorkArea> {
    monitors
        .iter()
        .max_by_key(|candidate| intersection_area(position, size, &candidate.work_area))
        .filter(|candidate| intersection_area(position, size, &candidate.work_area) > 0)
        .or_else(|| {
            prior_dock.and_then(|dock| {
                monitors
                    .iter()
                    .find(|candidate| candidate.work_area.identity.id == dock.monitor.id)
            })
        })
        .or_else(|| {
            monitors
                .iter()
                .find(|candidate| candidate.work_area.primary)
        })
        .or_else(|| monitors.first())
}

fn intersection_area(position: WindowPosition, size: WindowSize, monitor: &MonitorWorkArea) -> i64 {
    let left = i64::from(position.x).max(i64::from(monitor.x));
    let top = i64::from(position.y).max(i64::from(monitor.y));
    let right = i64::from(position.x)
        .saturating_add(i64::from(size.width))
        .min(i64::from(monitor.x).saturating_add(i64::from(monitor.width)));
    let bottom = i64::from(position.y)
        .saturating_add(i64::from(size.height))
        .min(i64::from(monitor.y).saturating_add(i64::from(monitor.height)));
    right
        .saturating_sub(left)
        .max(0)
        .saturating_mul(bottom.saturating_sub(top).max(0))
}
