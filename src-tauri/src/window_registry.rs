//! Testable ownership of detached Cookbench windows.
//!
//! This registry never inspects, starts, or controls an agent. It merely keeps
//! one Cookbench UI window per opaque stove key and its persisted layout.

use std::{collections::BTreeMap, fmt};

use cookbench_core::persistence::{
    DetachedStoveLayout, MonitorWorkArea, RestoredDetachedStoveLayout, WindowPosition,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DetachedWindowRecord {
    pub stove_key: String,
    pub label: String,
    pub layout: DetachedStoveLayout,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DetachOutcome {
    Created(DetachedWindowRecord),
    PresentedExisting(DetachedWindowRecord),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegistryError {
    EmptyStoveKey,
    NoMonitors,
    Host(String),
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyStoveKey => write!(f, "stove key must not be empty"),
            Self::NoMonitors => write!(f, "no graphical monitors are available"),
            Self::Host(error) => write!(f, "window host failed: {error}"),
        }
    }
}

impl std::error::Error for RegistryError {}

/// Small adapter boundary so registry behavior is unit-testable without Tauri.
pub trait DetachedWindowHost {
    type Error: fmt::Display;

    fn create(
        &mut self,
        record: &DetachedWindowRecord,
        position: WindowPosition,
    ) -> Result<(), Self::Error>;
    fn present(&mut self, label: &str) -> Result<(), Self::Error>;
    fn close(&mut self, label: &str) -> Result<(), Self::Error>;
}

#[derive(Default)]
pub struct WindowRegistry {
    global_bar_visible: bool,
    detached: BTreeMap<String, DetachedWindowRecord>,
}

impl WindowRegistry {
    pub fn new(global_bar_visible: bool) -> Self {
        Self {
            global_bar_visible,
            detached: BTreeMap::new(),
        }
    }

    pub fn global_bar_visible(&self) -> bool {
        self.global_bar_visible
    }

    pub fn set_global_bar_visible(&mut self, visible: bool) {
        self.global_bar_visible = visible;
    }

    pub fn detached(&self, stove_key: &str) -> Option<&DetachedWindowRecord> {
        self.detached.get(stove_key)
    }

    pub fn layouts(&self) -> Vec<DetachedStoveLayout> {
        self.detached
            .values()
            .map(|record| record.layout.clone())
            .collect()
    }

    pub fn detach<H: DetachedWindowHost>(
        &mut self,
        host: &mut H,
        layout: DetachedStoveLayout,
        monitors: &[MonitorWorkArea],
    ) -> Result<DetachOutcome, RegistryError> {
        validate_stove_key(&layout.stove_key)?;
        if let Some(existing) = self.detached.get(&layout.stove_key) {
            host.present(&existing.label).map_err(host_error)?;
            return Ok(DetachOutcome::PresentedExisting(existing.clone()));
        }

        let restored = layout.restore(monitors).ok_or(RegistryError::NoMonitors)?;
        self.create_detached(host, restored)
    }

    pub fn restore_all<H: DetachedWindowHost>(
        &mut self,
        host: &mut H,
        layouts: impl IntoIterator<Item = DetachedStoveLayout>,
        monitors: &[MonitorWorkArea],
    ) -> Result<Vec<DetachedWindowRecord>, RegistryError> {
        let mut restored = Vec::new();
        for layout in layouts {
            match self.detach(host, layout, monitors)? {
                DetachOutcome::Created(record) | DetachOutcome::PresentedExisting(record) => {
                    restored.push(record)
                }
            }
        }
        Ok(restored)
    }

    pub fn update_position(
        &mut self,
        stove_key: &str,
        monitor: &MonitorWorkArea,
        position: WindowPosition,
    ) -> bool {
        let Some(record) = self.detached.get_mut(stove_key) else {
            return false;
        };
        record.layout.update_from_absolute(monitor, position);
        true
    }

    /// Removes Cookbench's detached window when the user manually clears its
    /// stove. This does not modify native harness history or source sessions.
    pub fn clear_stove<H: DetachedWindowHost>(
        &mut self,
        host: &mut H,
        stove_key: &str,
    ) -> Result<bool, RegistryError> {
        let Some(record) = self.detached.get(stove_key) else {
            return Ok(false);
        };
        host.close(&record.label).map_err(host_error)?;
        self.detached.remove(stove_key);
        Ok(true)
    }

    fn create_detached<H: DetachedWindowHost>(
        &mut self,
        host: &mut H,
        restored: RestoredDetachedStoveLayout,
    ) -> Result<DetachOutcome, RegistryError> {
        let record = DetachedWindowRecord {
            label: detached_window_label(&restored.layout.stove_key),
            stove_key: restored.layout.stove_key.clone(),
            layout: restored.layout,
        };
        host.create(&record, restored.position)
            .map_err(host_error)?;
        self.detached
            .insert(record.stove_key.clone(), record.clone());
        Ok(DetachOutcome::Created(record))
    }
}

pub fn detached_window_label(stove_key: &str) -> String {
    let mut label = String::from("stove-");
    for byte in stove_key.as_bytes() {
        use std::fmt::Write as _;
        write!(&mut label, "{byte:02x}").expect("writing into String cannot fail");
    }
    label
}

fn validate_stove_key(stove_key: &str) -> Result<(), RegistryError> {
    if stove_key.trim().is_empty() {
        Err(RegistryError::EmptyStoveKey)
    } else {
        Ok(())
    }
}

fn host_error(error: impl fmt::Display) -> RegistryError {
    RegistryError::Host(error.to_string())
}
