//! Persisted placement for Cookbench-owned detached Stove bars.
//!
//! Coordinates are relative to a monitor's available work area so a display
//! arrangement change does not strand a bar at an unusable global pixel.

use serde::{Deserialize, Serialize};

const RELATIVE_SCALE: u32 = 10_000;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MonitorIdentity {
    /// Stable platform-provided identifier where available.
    pub id: String,
    /// Human-readable name is only a migration aid, never the primary key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RelativePosition {
    /// Position in ten-thousandths of the usable horizontal travel range.
    pub x: u16,
    /// Position in ten-thousandths of the usable vertical travel range.
    pub y: u16,
}

impl RelativePosition {
    pub const TOP_LEFT: Self = Self { x: 0, y: 0 };

    pub fn from_absolute(
        position: WindowPosition,
        monitor: &MonitorWorkArea,
        size: WindowSize,
    ) -> Self {
        Self {
            x: relative_axis(position.x, monitor.x, monitor.width, size.width),
            y: relative_axis(position.y, monitor.y, monitor.height, size.height),
        }
    }

    pub fn resolve(self, monitor: &MonitorWorkArea, size: WindowSize) -> WindowPosition {
        clamp_window_position_to_work_area(
            WindowPosition {
                x: resolve_axis(self.x, monitor.x, monitor.width, size.width),
                y: resolve_axis(self.y, monitor.y, monitor.height, size.height),
            },
            size,
            monitor,
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WindowPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MonitorWorkArea {
    pub identity: MonitorIdentity,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub primary: bool,
}

/// Resolves a persisted monitor without stranding legacy name-only records.
/// An exact ID wins; a display name is used only when it identifies exactly
/// one currently connected monitor.
pub fn resolve_saved_monitor<'a>(
    saved: &MonitorIdentity,
    monitors: &'a [MonitorWorkArea],
) -> Option<&'a MonitorWorkArea> {
    monitors
        .iter()
        .find(|candidate| candidate.identity.id == saved.id)
        .or_else(|| {
            saved.name.as_ref().and_then(|name| {
                let mut matches = monitors
                    .iter()
                    .filter(|candidate| candidate.identity.name.as_ref() == Some(name));
                let only = matches.next()?;
                matches.next().is_none().then_some(only)
            })
        })
        .or_else(|| monitors.iter().find(|candidate| candidate.primary))
        .or_else(|| monitors.first())
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DetachedStoveLayout {
    /// Opaque Cookbench stove key; no transcript or prompt content is stored.
    pub stove_key: String,
    pub monitor: MonitorIdentity,
    pub relative_position: RelativePosition,
    pub size: WindowSize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoredDetachedStoveLayout {
    pub layout: DetachedStoveLayout,
    pub position: WindowPosition,
    pub used_fallback_monitor: bool,
}

impl DetachedStoveLayout {
    pub fn from_absolute(
        stove_key: impl Into<String>,
        monitor: &MonitorWorkArea,
        position: WindowPosition,
        size: WindowSize,
    ) -> Self {
        Self {
            stove_key: stove_key.into(),
            monitor: monitor.identity.clone(),
            relative_position: RelativePosition::from_absolute(position, monitor, size),
            size,
        }
    }

    /// Restores on the saved monitor when possible and otherwise uses the
    /// primary display (or the first reported display), always clamped on-screen.
    pub fn restore(&self, monitors: &[MonitorWorkArea]) -> Option<RestoredDetachedStoveLayout> {
        let monitor = resolve_saved_monitor(&self.monitor, monitors)?;
        let used_fallback_monitor = monitor.identity.id != self.monitor.id;
        let mut layout = self.clone();
        if used_fallback_monitor {
            layout.monitor = monitor.identity.clone();
        }
        let position = layout.relative_position.resolve(monitor, layout.size);

        Some(RestoredDetachedStoveLayout {
            layout,
            position,
            used_fallback_monitor,
        })
    }

    pub fn update_from_absolute(&mut self, monitor: &MonitorWorkArea, position: WindowPosition) {
        self.monitor = monitor.identity.clone();
        self.relative_position = RelativePosition::from_absolute(position, monitor, self.size);
    }
}

/// Keeps a window's top-left inside the work area so the full frame stays
/// on-screen when it fits, and pinned to the origin when it is larger than
/// the display. Used when restoring freeform Global Bar / detached positions.
pub fn clamp_window_position_to_work_area(
    position: WindowPosition,
    size: WindowSize,
    monitor: &MonitorWorkArea,
) -> WindowPosition {
    let max_x = if size.width >= monitor.width {
        monitor.x
    } else {
        monitor
            .x
            .saturating_add((monitor.width - size.width) as i32)
    };
    let max_y = if size.height >= monitor.height {
        monitor.y
    } else {
        monitor
            .y
            .saturating_add((monitor.height - size.height) as i32)
    };
    WindowPosition {
        x: position.x.clamp(monitor.x.min(max_x), monitor.x.max(max_x)),
        y: position.y.clamp(monitor.y.min(max_y), monitor.y.max(max_y)),
    }
}

fn relative_axis(position: i32, origin: i32, available: u32, window: u32) -> u16 {
    let travel = available.saturating_sub(window) as i64;
    if travel == 0 {
        return 0;
    }
    let offset = (i64::from(position) - i64::from(origin)).clamp(0, travel);
    ((offset * i64::from(RELATIVE_SCALE) + travel / 2) / travel) as u16
}

fn resolve_axis(relative: u16, origin: i32, available: u32, window: u32) -> i32 {
    let travel = available.saturating_sub(window) as i64;
    let offset = (i64::from(relative.min(RELATIVE_SCALE as u16)) * travel
        + i64::from(RELATIVE_SCALE) / 2)
        / i64::from(RELATIVE_SCALE);
    i64::from(origin)
        .saturating_add(offset)
        .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(id: &str, x: i32, width: u32, primary: bool) -> MonitorWorkArea {
        MonitorWorkArea {
            identity: MonitorIdentity {
                id: id.into(),
                name: None,
            },
            x,
            y: 0,
            width,
            height: 900,
            primary,
        }
    }

    #[test]
    fn restores_relative_position_on_a_resized_monitor() {
        let wide = monitor("wide", 1920, 2560, false);
        let layout = DetachedStoveLayout::from_absolute(
            "stove-1",
            &wide,
            WindowPosition { x: 3000, y: 400 },
            WindowSize {
                width: 360,
                height: 104,
            },
        );
        let resized = monitor("wide", 1920, 1600, false);
        let restored = layout.restore(&[resized]).unwrap();

        assert_eq!(restored.position.x, 2529);
        assert_eq!(restored.position.y, 400);
    }

    #[test]
    fn missing_monitor_falls_back_and_clamps() {
        let detached = monitor("detached", 1920, 2560, false);
        let layout = DetachedStoveLayout::from_absolute(
            "stove-1",
            &detached,
            WindowPosition { x: 4100, y: 2000 },
            WindowSize {
                width: 360,
                height: 104,
            },
        );
        let primary = monitor("primary", 0, 1200, true);
        let restored = layout.restore(&[primary]).unwrap();

        assert!(restored.used_fallback_monitor);
        assert_eq!(restored.layout.monitor.id, "primary");
        assert_eq!(restored.position.x, 832);
        assert_eq!(restored.position.y, 796);
    }

    #[test]
    fn clamps_freeform_restore_that_would_spill_past_the_right_edge() {
        // Dogfood D16: a near-right relative restore with a usable Bar width
        // must not land at x≈1251 on a 1280-wide display.
        let monitor = monitor("primary", 0, 1280, true);
        let size = WindowSize {
            width: 280,
            height: 104,
        };
        let spilled = WindowPosition { x: 1251, y: 40 };
        let clamped = clamp_window_position_to_work_area(spilled, size, &monitor);
        assert_eq!(clamped, WindowPosition { x: 1000, y: 40 });
    }

    #[test]
    fn clamps_all_four_edges_and_pins_oversized_windows() {
        let monitor = MonitorWorkArea {
            identity: MonitorIdentity {
                id: "primary".into(),
                name: None,
            },
            x: 100,
            y: 50,
            width: 1280,
            height: 800,
            primary: true,
        };
        let size = WindowSize {
            width: 320,
            height: 120,
        };
        assert_eq!(
            clamp_window_position_to_work_area(WindowPosition { x: -40, y: -10 }, size, &monitor),
            WindowPosition { x: 100, y: 50 }
        );
        assert_eq!(
            clamp_window_position_to_work_area(WindowPosition { x: 5000, y: 5000 }, size, &monitor),
            WindowPosition { x: 1060, y: 730 }
        );
        let oversized = WindowSize {
            width: 2000,
            height: 1200,
        };
        assert_eq!(
            clamp_window_position_to_work_area(
                WindowPosition { x: 400, y: 300 },
                oversized,
                &monitor
            ),
            WindowPosition { x: 100, y: 50 }
        );
    }

    #[test]
    fn resolve_clamps_corrupted_far_right_relative_units() {
        let monitor = monitor("primary", 0, 1280, true);
        let size = WindowSize {
            width: 280,
            height: 104,
        };
        // Relative units past the scale still resolve on-screen.
        let position = RelativePosition {
            x: 10_000,
            y: 10_000,
        }
        .resolve(&monitor, size);
        assert_eq!(position, WindowPosition { x: 1000, y: 796 });
        assert!(position.x + size.width as i32 <= monitor.width as i32);
        assert!(position.y + size.height as i32 <= monitor.height as i32);
    }
}
