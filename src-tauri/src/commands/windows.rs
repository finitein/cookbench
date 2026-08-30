//! Application commands for Cookbench-owned detached Stove bars.
//!
//! The service is deliberately parameterized over window and monitor adapters.
//! This keeps lifecycle rules testable without a desktop runtime and ensures the
//! command layer cannot reach into an agent, terminal, or session file.

use std::{fmt, sync::Mutex};

use cookbench_core::persistence::{
    DetachedStoveLayout, MonitorIdentity, MonitorWorkArea, WindowPosition, WindowSize,
};

use serde::Serialize;
use tauri::{
    AppHandle, LogicalSize, Manager, PhysicalPosition, Runtime, State, WebviewUrl,
    WebviewWindowBuilder,
};

use crate::window_registry::{DetachOutcome, DetachedWindowHost, RegistryError, WindowRegistry};

pub trait MonitorProvider {
    type Error: fmt::Display;

    fn monitors(&self) -> Result<Vec<MonitorWorkArea>, Self::Error>;
}

pub struct WindowCommandService<H, M> {
    registry: Mutex<WindowRegistry>,
    windows: Mutex<H>,
    monitors: M,
}

impl<H, M> WindowCommandService<H, M>
where
    H: DetachedWindowHost,
    M: MonitorProvider,
{
    pub fn new(registry: WindowRegistry, windows: H, monitors: M) -> Self {
        Self {
            registry: Mutex::new(registry),
            windows: Mutex::new(windows),
            monitors,
        }
    }

    pub fn detach(&self, layout: DetachedStoveLayout) -> Result<DetachOutcome, WindowCommandError> {
        let monitors = self
            .monitors
            .monitors()
            .map_err(|error| WindowCommandError::Monitors(error.to_string()))?;
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| WindowCommandError::Poisoned)?;
        let mut windows = self
            .windows
            .lock()
            .map_err(|_| WindowCommandError::Poisoned)?;
        registry
            .detach(&mut *windows, layout, &monitors)
            .map_err(WindowCommandError::Registry)
    }

    pub fn detach_stove_key(
        &self,
        stove_key: impl Into<String>,
    ) -> Result<DetachOutcome, WindowCommandError> {
        let stove_key = stove_key.into();
        let monitors = self
            .monitors
            .monitors()
            .map_err(|error| WindowCommandError::Monitors(error.to_string()))?;
        let monitor = monitors
            .iter()
            .find(|monitor| monitor.primary)
            .or_else(|| monitors.first())
            .ok_or(WindowCommandError::Registry(RegistryError::NoMonitors))?;
        let layout = DetachedStoveLayout::from_absolute(
            stove_key,
            monitor,
            WindowPosition {
                x: monitor.x.saturating_add(24),
                y: monitor.y.saturating_add(24),
            },
            cookbench_core::persistence::WindowSize {
                width: 164,
                height: 104,
            },
        );
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| WindowCommandError::Poisoned)?;
        let mut windows = self
            .windows
            .lock()
            .map_err(|_| WindowCommandError::Poisoned)?;
        registry
            .detach(&mut *windows, layout, &monitors)
            .map_err(WindowCommandError::Registry)
    }

    pub fn restore(&self, layouts: Vec<DetachedStoveLayout>) -> Result<(), WindowCommandError> {
        let monitors = self
            .monitors
            .monitors()
            .map_err(|error| WindowCommandError::Monitors(error.to_string()))?;
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| WindowCommandError::Poisoned)?;
        let mut windows = self
            .windows
            .lock()
            .map_err(|_| WindowCommandError::Poisoned)?;
        registry
            .restore_all(&mut *windows, layouts, &monitors)
            .map_err(WindowCommandError::Registry)?;
        Ok(())
    }

    pub fn moved(
        &self,
        stove_key: &str,
        monitor: &MonitorWorkArea,
        position: WindowPosition,
    ) -> Result<bool, WindowCommandError> {
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| WindowCommandError::Poisoned)?;
        Ok(registry.update_position(stove_key, monitor, position))
    }

    pub fn record_absolute_position(
        &self,
        stove_key: &str,
        position: WindowPosition,
    ) -> Result<bool, WindowCommandError> {
        let monitors = self
            .monitors
            .monitors()
            .map_err(|error| WindowCommandError::Monitors(error.to_string()))?;
        let monitor = monitors
            .iter()
            .find(|monitor| {
                position.x >= monitor.x
                    && position.y >= monitor.y
                    && position.x < monitor.x.saturating_add_unsigned(monitor.width)
                    && position.y < monitor.y.saturating_add_unsigned(monitor.height)
            })
            .or_else(|| monitors.iter().find(|monitor| monitor.primary))
            .or_else(|| monitors.first())
            .ok_or(WindowCommandError::Registry(RegistryError::NoMonitors))?;
        self.moved(stove_key, monitor, position)
    }

    /// Clearing a Cookbench Stove closes only its matching detached UI window.
    /// It neither deletes nor alters the harness-native session.
    pub fn clear_stove(&self, stove_key: &str) -> Result<bool, WindowCommandError> {
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| WindowCommandError::Poisoned)?;
        let mut windows = self
            .windows
            .lock()
            .map_err(|_| WindowCommandError::Poisoned)?;
        registry
            .clear_stove(&mut *windows, stove_key)
            .map_err(WindowCommandError::Registry)
    }

    pub fn persisted_layouts(&self) -> Result<Vec<DetachedStoveLayout>, WindowCommandError> {
        let registry = self
            .registry
            .lock()
            .map_err(|_| WindowCommandError::Poisoned)?;
        Ok(registry.layouts())
    }

    pub fn set_global_bar_visible(&self, visible: bool) -> Result<(), WindowCommandError> {
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| WindowCommandError::Poisoned)?;
        registry.set_global_bar_visible(visible);
        Ok(())
    }
}

#[derive(Debug)]
pub enum WindowCommandError {
    Monitors(String),
    Registry(RegistryError),
    Poisoned,
}

impl fmt::Display for WindowCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Monitors(error) => write!(f, "could not enumerate monitors: {error}"),
            Self::Registry(error) => error.fmt(f),
            Self::Poisoned => write!(f, "window state is unavailable after a previous failure"),
        }
    }
}

impl std::error::Error for WindowCommandError {}

/// Tauri adapter for the pure registry. Window labels and titles deliberately
/// exclude project names, prompts, and session content.
pub struct TauriDetachedWindowHost<R: Runtime = tauri::Wry> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriDetachedWindowHost<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> DetachedWindowHost for TauriDetachedWindowHost<R> {
    type Error = tauri::Error;

    fn create(
        &mut self,
        record: &crate::window_registry::DetachedWindowRecord,
        position: WindowPosition,
    ) -> Result<(), Self::Error> {
        if let Some(window) = self.app.get_webview_window(&record.label) {
            window.show()?;
            return Ok(());
        }

        let window = WebviewWindowBuilder::new(
            &self.app,
            &record.label,
            WebviewUrl::App("index.html".into()),
        )
        .title("Cookbench Stove")
        .decorations(false)
        .resizable(false)
        .skip_taskbar(true)
        .inner_size(
            record.layout.size.width as f64,
            record.layout.size.height as f64,
        )
        .build()?;
        window.set_position(PhysicalPosition::new(position.x, position.y))?;
        window.show()?;
        // Detached bars are the same floating Cookbench surface as the global
        // bar: macOS gets all-workspaces, Windows gets topmost, and Wayland
        // remains an honest best-effort presentation.
        match crate::platform::apply_platform_overlay(&window) {
            Ok(()) | Err(crate::platform::OverlayError::BestEffortWayland) => Ok(()),
            Err(crate::platform::OverlayError::Tauri(error)) => Err(error),
            Err(_) => Ok(()),
        }
    }

    fn present(&mut self, label: &str) -> Result<(), Self::Error> {
        if let Some(window) = self.app.get_webview_window(label) {
            window.show()?;
            // Focus is convenience only. The detached view never targets the
            // source harness, terminal, IDE, or SSH connection.
            let _ = window.set_focus();
        }
        Ok(())
    }

    fn close(&mut self, label: &str) -> Result<(), Self::Error> {
        if let Some(window) = self.app.get_webview_window(label) {
            window.close()?;
        }
        Ok(())
    }
}

pub struct TauriMonitorProvider<R: Runtime = tauri::Wry> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriMonitorProvider<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> MonitorProvider for TauriMonitorProvider<R> {
    type Error = tauri::Error;

    fn monitors(&self) -> Result<Vec<MonitorWorkArea>, Self::Error> {
        let primary_name = self
            .app
            .primary_monitor()?
            .and_then(|monitor| monitor.name().cloned());
        self.app
            .available_monitors()?
            .into_iter()
            .enumerate()
            .map(|(index, monitor)| {
                let name = monitor.name().cloned();
                // Tauri exposes a display name, but not a cross-platform UUID.
                // A missing name intentionally falls back after topology changes.
                let id = name.clone().unwrap_or_else(|| format!("monitor-{index}"));
                let area = monitor.work_area();
                Ok(MonitorWorkArea {
                    primary: name == primary_name,
                    identity: MonitorIdentity { id, name },
                    x: area.position.x,
                    y: area.position.y,
                    width: area.size.width,
                    height: area.size.height,
                })
            })
            .collect()
    }
}

pub type TauriWindowCommandService<R = tauri::Wry> =
    WindowCommandService<TauriDetachedWindowHost<R>, TauriMonitorProvider<R>>;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetachedWindowResponse {
    pub stove_id: String,
    pub label: String,
}

#[tauri::command]
pub fn detach_stove(
    stove_id: String,
    app_state: State<'_, crate::app_state::AppState>,
    windows: State<'_, TauriWindowCommandService>,
) -> Result<DetachedWindowResponse, String> {
    if !app_state
        .stoves
        .snapshot()
        .stoves
        .iter()
        .any(|stove| stove.id == stove_id)
    {
        return Err("Cookbench does not have that Stove".to_owned());
    }
    let record = match windows
        .detach_stove_key(stove_id.clone())
        .map_err(|error| error.to_string())?
    {
        DetachOutcome::Created(record) | DetachOutcome::PresentedExisting(record) => record,
    };
    persist_layouts(&app_state, &windows)?;
    Ok(DetachedWindowResponse {
        stove_id,
        label: record.label,
    })
}

#[tauri::command]
pub fn clear_detached_stove(
    stove_id: String,
    app: AppHandle,
    app_state: State<'_, crate::app_state::AppState>,
    windows: State<'_, TauriWindowCommandService>,
) -> Result<bool, String> {
    let identity = app_state
        .stoves
        .core_stove(&stove_id)
        .map(|stove| stove.identity);
    app_state
        .clear_cooked_and_emit(&app, &stove_id)
        .map_err(|error| error.to_string())?;
    if let (Some(identity), Some(remote)) = (
        identity,
        app.try_state::<crate::remote::runtime::RemoteRuntimeState>(),
    ) {
        remote.forget(identity);
    }
    let closed = windows
        .clear_stove(&stove_id)
        .map_err(|error| error.to_string())?;
    persist_layouts(&app_state, &windows)?;
    Ok(closed)
}

/// Closes a detached Cookbench view without clearing or changing its Stove.
/// This is used by display settings so an active harness session remains
/// observable in the global Bar or can be detached again later.
#[tauri::command]
pub fn close_detached_bar(
    stove_id: String,
    app_state: State<'_, crate::app_state::AppState>,
    windows: State<'_, TauriWindowCommandService>,
) -> Result<bool, String> {
    let closed = windows
        .clear_stove(&stove_id)
        .map_err(|error| error.to_string())?;
    persist_layouts(&app_state, &windows)?;
    Ok(closed)
}

/// Records a completed user resize. It never sets the window size, which keeps
/// native edge and corner drags authoritative.
#[tauri::command]
pub fn record_global_bar_size(
    width: f64,
    height: f64,
    state: State<'_, crate::app_state::AppState>,
) -> Result<(), String> {
    let size = normalized_global_bar_size(width, height)?;
    state.update_persisted_config(|config| config.layout.global_bar_size = Some(size))
}

/// Raises the native lower bound as wrapped Stove content grows. Width stays
/// freely resizable above a small usable floor; only an undersized current
/// height is expanded so no Stove is clipped and no scrollbar is required.
#[tauri::command]
pub fn set_global_bar_minimum_size(
    app: AppHandle,
    width: f64,
    height: f64,
    preferred_height: Option<f64>,
) -> Result<(), String> {
    let minimum = normalized_global_bar_size(width, height)?;
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Cookbench global Bar window is unavailable".to_owned())?;
    window
        .set_min_size(Some(LogicalSize::new(
            f64::from(minimum.width),
            f64::from(minimum.height),
        )))
        .map_err(|error| error.to_string())?;
    let scale = window.scale_factor().map_err(|error| error.to_string())?;
    let current = window.outer_size().map_err(|error| error.to_string())?;
    let minimum_height = (f64::from(minimum.height) * scale).ceil() as u32;
    let minimum_width = (f64::from(minimum.width) * scale).ceil() as u32;
    let preferred_height = preferred_height
        .filter(|height| height.is_finite())
        .map(|height| height.ceil().max(f64::from(minimum.height)));
    let target_height = preferred_height
        .map(|height| (height * scale).ceil() as u32)
        .unwrap_or(minimum_height);
    if current.height < minimum_height
        || preferred_height.is_some_and(|_| current.height.abs_diff(target_height) > 1)
    {
        window
            .set_size(LogicalSize::new(
                f64::from(current.width.max(minimum_width)) / scale,
                f64::from(target_height) / scale,
            ))
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn normalized_global_bar_size(width: f64, height: f64) -> Result<WindowSize, String> {
    fn normalize(value: f64, minimum: u32) -> Result<u32, String> {
        if !value.is_finite() {
            return Err("Cookbench global Bar dimensions must be finite".to_owned());
        }
        Ok(value
            .ceil()
            .max(f64::from(minimum))
            .min(f64::from(u32::MAX)) as u32)
    }

    Ok(WindowSize {
        width: normalize(width, 280)?,
        height: normalize(height, 80)?,
    })
}

#[tauri::command]
pub fn record_detached_stove_position(
    stove_id: String,
    x: i32,
    y: i32,
    app_state: State<'_, crate::app_state::AppState>,
    windows: State<'_, TauriWindowCommandService>,
) -> Result<bool, String> {
    if !app_state
        .stoves
        .snapshot()
        .stoves
        .iter()
        .any(|stove| stove.id == stove_id)
    {
        return Err("Cookbench does not have that Stove".to_owned());
    }
    let updated = windows
        .record_absolute_position(&stove_id, WindowPosition { x, y })
        .map_err(|error| error.to_string())?;
    if updated {
        persist_layouts(&app_state, &windows)?;
    }
    Ok(updated)
}

pub(super) fn persist_layouts(
    app_state: &crate::app_state::AppState,
    windows: &TauriWindowCommandService,
) -> Result<(), String> {
    let layouts = windows
        .persisted_layouts()
        .map_err(|error| error.to_string())?;
    app_state.update_persisted_config(|config| {
        config.layout.detached_layouts = layouts;
        config.layout.detached_stoves.clear();
    })
}
