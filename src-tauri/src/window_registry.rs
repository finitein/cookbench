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

/// Opaque stove keys written by Cookbench follow
/// `{local|ssh}:{host_id}:{harness_id}:{native_session_id}`.
/// Reject control characters and incomplete identities so a corrupt layout
/// never becomes an empty ghost window.
pub fn stove_key_identity_is_valid(stove_key: &str) -> bool {
    if stove_key.is_empty()
        || stove_key.len() > 256
        || stove_key.chars().any(char::is_control)
        || stove_key != stove_key.trim()
    {
        return false;
    }
    let mut parts = stove_key.splitn(4, ':');
    let Some(kind) = parts.next() else {
        return false;
    };
    let Some(host) = parts.next() else {
        return false;
    };
    let Some(harness) = parts.next() else {
        return false;
    };
    let Some(session) = parts.next() else {
        return false;
    };
    matches!(kind, "local" | "ssh")
        && !host.is_empty()
        && !harness.is_empty()
        && !session.is_empty()
}

/// Keeps layouts that still match a known live/archived session identity.
/// Layouts with invalid stove keys or unknown sessions are dropped so startup
/// restore cannot open empty "Cookbench Stove" ghost windows (dogfood D24).
pub fn filter_detached_layouts_for_known_sessions(
    layouts: impl IntoIterator<Item = DetachedStoveLayout>,
    known_stove_keys: &std::collections::BTreeSet<String>,
) -> Vec<DetachedStoveLayout> {
    layouts
        .into_iter()
        .filter(|layout| {
            stove_key_identity_is_valid(&layout.stove_key)
                && known_stove_keys.contains(&layout.stove_key)
        })
        .collect()
}

#[cfg(test)]
mod identity_filter_tests {
    use super::*;
    use cookbench_core::persistence::{MonitorIdentity, RelativePosition, WindowSize};

    fn layout(stove_key: &str) -> DetachedStoveLayout {
        DetachedStoveLayout {
            stove_key: stove_key.into(),
            monitor: MonitorIdentity {
                id: "primary".into(),
                name: None,
            },
            relative_position: RelativePosition { x: 0, y: 0 },
            size: WindowSize {
                width: 164,
                height: 104,
            },
        }
    }

    #[test]
    fn accepts_cookbench_stove_identities_and_rejects_garbage() {
        assert!(stove_key_identity_is_valid("local:local:codex:rollout-abc"));
        assert!(stove_key_identity_is_valid("ssh:jump:claudeCode:session-1"));
        assert!(stove_key_identity_is_valid("local:local:grok:thread-9"));
        assert!(!stove_key_identity_is_valid(""));
        assert!(!stove_key_identity_is_valid("session-a"));
        assert!(!stove_key_identity_is_valid("local:local:codex:"));
        assert!(!stove_key_identity_is_valid("ftp:local:codex:x"));
        assert!(!stove_key_identity_is_valid(
            "local:local:codex:bad\u{0001}"
        ));
    }

    #[test]
    fn prunes_stale_and_invalid_layouts_before_restore() {
        let mut known = std::collections::BTreeSet::new();
        known.insert("local:local:codex:live-1".into());
        known.insert("local:local:amp:archived-2".into());
        let kept = filter_detached_layouts_for_known_sessions(
            vec![
                layout("local:local:codex:live-1"),
                layout("local:local:amp:archived-2"),
                layout("local:local:goose:missing-3"),
                layout("orphan-not-an-identity"),
                layout(""),
            ],
            &known,
        );
        assert_eq!(
            kept.iter()
                .map(|layout| layout.stove_key.as_str())
                .collect::<Vec<_>>(),
            vec!["local:local:codex:live-1", "local:local:amp:archived-2"]
        );
    }
}
