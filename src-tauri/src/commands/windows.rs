//! Application commands for Cookbench-owned detached Stove bars.
//!
//! The service is deliberately parameterized over window and monitor adapters.
//! This keeps lifecycle rules testable without a desktop runtime and ensures the
//! command layer cannot reach into an agent, terminal, or session file.

use std::{fmt, sync::Mutex};

use cookbench_core::persistence::{
    resolve_top_dock, top_dock_decision, DetachedStoveLayout, DockMonitorWorkArea,
    GlobalBarPosition, GlobalBarTopDock, MonitorIdentity, MonitorWorkArea, RelativePosition,
    TopDockDecision, TopDockInput, WindowPosition, WindowSize,
};

use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, Runtime, State, WebviewUrl,
    WebviewWindowBuilder,
};

use crate::window_registry::{DetachOutcome, DetachedWindowHost, RegistryError, WindowRegistry};

pub const GLOBAL_BAR_DOCK_STATE_CHANGED_EVENT: &str = "cookbench://global-bar-dock-state-changed";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GlobalBarDockPhase {
    #[default]
    Undocked,
    DockedExpanded,
    DockedCollapsed,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GlobalBarDockGuards {
    pub pointer_inside: bool,
    pub focused: bool,
    pub menu_open: bool,
    pub resizing: bool,
}

impl GlobalBarDockGuards {
    fn blocks_collapse(self) -> bool {
        self.pointer_inside || self.focused || self.menu_open || self.resizing
    }
}

#[derive(Clone, Debug, Default)]
struct GlobalBarDockController {
    phase: GlobalBarDockPhase,
    dock: Option<GlobalBarTopDock>,
    active_drag: bool,
    guards: GlobalBarDockGuards,
}

#[derive(Default)]
pub struct GlobalBarDockRuntime(Mutex<GlobalBarDockController>);

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalBarDockStateWire {
    pub phase: GlobalBarDockPhase,
    pub docked: bool,
    pub collapsed: bool,
    pub best_effort: bool,
}

impl GlobalBarDockRuntime {
    pub(crate) fn state(&self) -> GlobalBarDockStateWire {
        let controller = self.0.lock().expect("global bar dock lock poisoned");
        GlobalBarDockStateWire {
            phase: controller.phase,
            docked: controller.dock.is_some(),
            collapsed: controller.phase == GlobalBarDockPhase::DockedCollapsed,
            best_effort: !reliable_top_dock_positioning(),
        }
    }
    fn begin_drag(&self) -> bool {
        let mut controller = self.0.lock().expect("global bar dock lock poisoned");
        let revealed = controller.phase == GlobalBarDockPhase::DockedCollapsed;
        if controller.dock.is_some() {
            controller.phase = GlobalBarDockPhase::DockedExpanded;
        }
        controller.active_drag = true;
        revealed
    }
    fn consume_drag(&self) -> Option<Option<GlobalBarTopDock>> {
        let mut controller = self.0.lock().expect("global bar dock lock poisoned");
        if !controller.active_drag {
            return None;
        }
        controller.active_drag = false;
        Some(controller.dock.clone())
    }
    pub(crate) fn commit_dock(&self, dock: Option<GlobalBarTopDock>) {
        let mut controller = self.0.lock().expect("global bar dock lock poisoned");
        controller.dock = dock;
        controller.phase = if controller.dock.is_some() {
            GlobalBarDockPhase::DockedExpanded
        } else {
            GlobalBarDockPhase::Undocked
        };
    }
    fn set_guards(&self, guards: GlobalBarDockGuards) -> bool {
        let mut controller = self.0.lock().expect("global bar dock lock poisoned");
        controller.guards = guards;
        let reveal =
            guards.blocks_collapse() && controller.phase == GlobalBarDockPhase::DockedCollapsed;
        if reveal {
            controller.phase = GlobalBarDockPhase::DockedExpanded;
        }
        reveal
    }
    fn request_collapse(&self) -> bool {
        let mut controller = self.0.lock().expect("global bar dock lock poisoned");
        if controller.dock.is_none()
            || controller.phase != GlobalBarDockPhase::DockedExpanded
            || controller.guards.blocks_collapse()
            || !reliable_top_dock_positioning()
        {
            return false;
        }
        controller.phase = GlobalBarDockPhase::DockedCollapsed;
        true
    }
    fn reveal(&self) -> bool {
        let mut controller = self.0.lock().expect("global bar dock lock poisoned");
        if controller.dock.is_some() && controller.phase == GlobalBarDockPhase::DockedCollapsed {
            controller.phase = GlobalBarDockPhase::DockedExpanded;
            true
        } else {
            false
        }
    }
    fn dock(&self) -> Option<GlobalBarTopDock> {
        self.0
            .lock()
            .expect("global bar dock lock poisoned")
            .dock
            .clone()
    }
}

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

fn reliable_top_dock_positioning() -> bool {
    #[cfg(target_os = "linux")]
    {
        !std::env::var("XDG_SESSION_TYPE")
            .is_ok_and(|session| session.eq_ignore_ascii_case("wayland"))
            && std::env::var_os("WAYLAND_DISPLAY").is_none()
    }
    #[cfg(not(target_os = "linux"))]
    {
        true
    }
}

fn dock_monitors<R: Runtime>(
    window: &tauri::WebviewWindow<R>,
) -> Result<Vec<DockMonitorWorkArea>, String> {
    let primary_name = window
        .primary_monitor()
        .map_err(|error| error.to_string())?
        .and_then(|monitor| monitor.name().cloned());
    window
        .available_monitors()
        .map_err(|error| error.to_string())?
        .into_iter()
        .enumerate()
        .map(|(index, monitor)| {
            let name = monitor.name().cloned();
            let area = monitor.work_area();
            Ok(DockMonitorWorkArea::new(
                MonitorWorkArea {
                    primary: name == primary_name,
                    identity: MonitorIdentity {
                        id: name.clone().unwrap_or_else(|| format!("monitor-{index}")),
                        name,
                    },
                    x: area.position.x,
                    y: area.position.y,
                    width: area.size.width,
                    height: area.size.height,
                },
                monitor.scale_factor(),
            ))
        })
        .collect()
}

fn global_bar_position(
    position: WindowPosition,
    size: WindowSize,
    monitors: &[DockMonitorWorkArea],
) -> Result<GlobalBarPosition, String> {
    let monitor = monitors
        .iter()
        .find(|candidate| {
            let area = &candidate.work_area;
            position.x >= area.x
                && position.y >= area.y
                && position.x < area.x.saturating_add_unsigned(area.width)
                && position.y < area.y.saturating_add_unsigned(area.height)
        })
        .or_else(|| {
            monitors
                .iter()
                .find(|candidate| candidate.work_area.primary)
        })
        .or_else(|| monitors.first())
        .ok_or_else(|| "no display is available for the Cookbench global Bar".to_owned())?;
    Ok(GlobalBarPosition {
        monitor: monitor.work_area.identity.clone(),
        relative_position: RelativePosition::from_absolute(position, &monitor.work_area, size),
    })
}

fn emit_dock_state<R: Runtime>(app: &AppHandle<R>, runtime: &GlobalBarDockRuntime) {
    let _ = app.emit(GLOBAL_BAR_DOCK_STATE_CHANGED_EVENT, runtime.state());
}

fn move_to_dock_geometry<R: Runtime>(
    window: &tauri::WebviewWindow<R>,
    dock: &GlobalBarTopDock,
    collapsed: bool,
) -> Result<(), String> {
    let size = window.outer_size().map_err(|error| error.to_string())?;
    let geometry = resolve_top_dock(
        dock,
        WindowSize {
            width: size.width,
            height: size.height,
        },
        &dock_monitors(window)?,
    )
    .ok_or_else(|| "no display is available for the Cookbench global Bar".to_owned())?;
    let position = if collapsed {
        geometry.collapsed_position
    } else {
        geometry.expanded_position
    };
    window
        .set_position(PhysicalPosition::new(position.x, position.y))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_global_bar_dock_state(
    runtime: State<'_, GlobalBarDockRuntime>,
) -> GlobalBarDockStateWire {
    runtime.state()
}

#[tauri::command]
pub fn start_global_bar_drag(
    app: AppHandle,
    runtime: State<'_, GlobalBarDockRuntime>,
) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Cookbench global Bar window is unavailable".to_owned())?;
    if runtime.begin_drag() {
        if let Some(dock) = runtime.dock() {
            move_to_dock_geometry(&window, &dock, false)?;
        }
        emit_dock_state(&app, &runtime);
    }
    window.start_dragging().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn finish_global_bar_drag(
    app: AppHandle,
    state: State<'_, crate::app_state::AppState>,
    runtime: State<'_, GlobalBarDockRuntime>,
) -> Result<GlobalBarDockStateWire, String> {
    let Some(prior_dock) = runtime.consume_drag() else {
        return Ok(runtime.state());
    };
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Cookbench global Bar window is unavailable".to_owned())?;
    if !reliable_top_dock_positioning() {
        if let Some(dock) = prior_dock {
            move_to_dock_geometry(&window, &dock, false)?;
        }
        emit_dock_state(&app, &runtime);
        return Ok(runtime.state());
    }
    let outer = window.outer_position().map_err(|error| error.to_string())?;
    let size = window.outer_size().map_err(|error| error.to_string())?;
    let position = WindowPosition {
        x: outer.x,
        y: outer.y,
    };
    let size = WindowSize {
        width: size.width,
        height: size.height,
    };
    let monitors = dock_monitors(&window)?;
    let decision = top_dock_decision(TopDockInput {
        position,
        size,
        monitors: &monitors,
        prior_dock: prior_dock.as_ref(),
        reliable_positioning: true,
    });
    match decision {
        TopDockDecision::Dock(dock) | TopDockDecision::RemainDocked(dock) => {
            let previous = state.persisted_config();
            state.update_persisted_config(|config| {
                config.layout.global_bar_top_dock = Some(dock.clone())
            })?;
            if let Err(error) = move_to_dock_geometry(&window, &dock, false) {
                let _ = state.update_persisted_config(|config| config.layout = previous.layout);
                return Err(error);
            }
            runtime.commit_dock(Some(dock));
        }
        TopDockDecision::Undock | TopDockDecision::Freeform => {
            let saved = global_bar_position(position, size, &monitors)?;
            state.update_persisted_config(|config| {
                config.layout.global_bar_top_dock = None;
                config.layout.global_bar_position = Some(saved);
            })?;
            runtime.commit_dock(None);
        }
        TopDockDecision::BestEffortVisible => {}
    }
    emit_dock_state(&app, &runtime);
    Ok(runtime.state())
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalBarDockGuardsInput {
    pub pointer_inside: bool,
    pub focused: bool,
    pub menu_open: bool,
    pub resizing: bool,
}

#[tauri::command]
pub fn set_global_bar_dock_guards(
    input: GlobalBarDockGuardsInput,
    app: AppHandle,
    runtime: State<'_, GlobalBarDockRuntime>,
) -> Result<GlobalBarDockStateWire, String> {
    if runtime.set_guards(GlobalBarDockGuards {
        pointer_inside: input.pointer_inside,
        focused: input.focused,
        menu_open: input.menu_open,
        resizing: input.resizing,
    }) {
        reveal_global_bar_dock(&app, runtime.inner())?;
    }
    emit_dock_state(&app, &runtime);
    Ok(runtime.state())
}

#[tauri::command]
pub fn request_global_bar_dock_collapse(
    app: AppHandle,
    runtime: State<'_, GlobalBarDockRuntime>,
) -> Result<GlobalBarDockStateWire, String> {
    if runtime.request_collapse() {
        let window = app
            .get_webview_window("main")
            .ok_or_else(|| "Cookbench global Bar window is unavailable".to_owned())?;
        let Some(dock) = runtime.dock() else {
            return Ok(runtime.state());
        };
        if let Err(error) = move_to_dock_geometry(&window, &dock, true) {
            runtime.reveal();
            return Err(error);
        }
        emit_dock_state(&app, &runtime);
    }
    Ok(runtime.state())
}

pub fn reveal_global_bar_dock<R: Runtime>(
    app: &AppHandle<R>,
    runtime: &GlobalBarDockRuntime,
) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Cookbench global Bar window is unavailable".to_owned())?;
    window.show().map_err(|error| error.to_string())?;
    if runtime.reveal() || runtime.dock().is_some() {
        if let Some(dock) = runtime.dock() {
            move_to_dock_geometry(&window, &dock, false)?;
        }
    }
    emit_dock_state(app, runtime);
    Ok(())
}

#[tauri::command]
pub fn reveal_global_bar_dock_command(
    app: AppHandle,
    runtime: State<'_, GlobalBarDockRuntime>,
) -> Result<GlobalBarDockStateWire, String> {
    reveal_global_bar_dock(&app, runtime.inner())?;
    Ok(runtime.state())
}

#[tauri::command]
pub fn refresh_global_bar_dock_geometry(
    app: AppHandle,
    runtime: State<'_, GlobalBarDockRuntime>,
) -> Result<GlobalBarDockStateWire, String> {
    if let Some(dock) = runtime.dock() {
        let window = app
            .get_webview_window("main")
            .ok_or_else(|| "Cookbench global Bar window is unavailable".to_owned())?;
        let collapsed = runtime.state().collapsed && reliable_top_dock_positioning();
        move_to_dock_geometry(&window, &dock, collapsed)?;
    }
    Ok(runtime.state())
}

pub fn restore_global_bar_top_dock<R: Runtime>(
    app: &AppHandle<R>,
    state: &crate::app_state::AppState,
    runtime: &GlobalBarDockRuntime,
) -> Result<bool, String> {
    let config = state.persisted_config();
    let Some(dock) = config.layout.global_bar_top_dock else {
        return Ok(false);
    };
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Cookbench global Bar window is unavailable".to_owned())?;
    runtime.commit_dock(Some(dock.clone()));
    if config.layout.global_bar_visible {
        window.show().map_err(|error| error.to_string())?;
        move_to_dock_geometry(&window, &dock, false)?;
    } else {
        window.hide().map_err(|error| error.to_string())?;
    }
    emit_dock_state(app, runtime);
    Ok(true)
}

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

pub(crate) fn persist_layouts(
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

#[cfg(test)]
mod dock_tests {
    use super::*;

    fn dock() -> GlobalBarTopDock {
        GlobalBarTopDock {
            monitor: MonitorIdentity {
                id: "primary".into(),
                name: Some("Primary".into()),
            },
            relative_x: 5_000,
        }
    }

    #[test]
    fn drag_token_is_single_use_and_reveals_a_collapsed_dock() {
        let runtime = GlobalBarDockRuntime::default();
        runtime.commit_dock(Some(dock()));
        assert!(runtime.request_collapse());
        assert!(runtime.begin_drag());
        assert_eq!(runtime.state().phase, GlobalBarDockPhase::DockedExpanded);
        assert_eq!(runtime.consume_drag(), Some(Some(dock())));
        assert_eq!(runtime.consume_drag(), None);
    }

    #[test]
    fn every_interaction_guard_blocks_collapse_and_stale_collapse_is_inert() {
        let guards = [
            GlobalBarDockGuards {
                pointer_inside: true,
                ..Default::default()
            },
            GlobalBarDockGuards {
                focused: true,
                ..Default::default()
            },
            GlobalBarDockGuards {
                menu_open: true,
                ..Default::default()
            },
            GlobalBarDockGuards {
                resizing: true,
                ..Default::default()
            },
        ];
        for guard in guards {
            let runtime = GlobalBarDockRuntime::default();
            runtime.commit_dock(Some(dock()));
            runtime.set_guards(guard);
            assert!(!runtime.request_collapse());
            assert_eq!(runtime.state().phase, GlobalBarDockPhase::DockedExpanded);
        }
    }

    #[test]
    fn guard_reveals_a_collapsed_dock() {
        let runtime = GlobalBarDockRuntime::default();
        runtime.commit_dock(Some(dock()));
        assert!(runtime.request_collapse());
        assert!(runtime.set_guards(GlobalBarDockGuards {
            focused: true,
            ..Default::default()
        }));
        assert_eq!(runtime.state().phase, GlobalBarDockPhase::DockedExpanded);
    }

    #[test]
    fn empty_runtime_never_collapses_or_reveals() {
        let runtime = GlobalBarDockRuntime::default();
        assert!(!runtime.request_collapse());
        assert!(!runtime.reveal());
        assert_eq!(runtime.state().phase, GlobalBarDockPhase::Undocked);
    }
}
