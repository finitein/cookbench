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
        WindowPosition {
            x: resolve_axis(self.x, monitor.x, monitor.width, size.width),
            y: resolve_axis(self.y, monitor.y, monitor.height, size.height),
        }
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
        let monitor = monitors
            .iter()
            .find(|candidate| candidate.identity.id == self.monitor.id)
            .or_else(|| monitors.iter().find(|candidate| candidate.primary))
            .or_else(|| monitors.first())?;
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
}
