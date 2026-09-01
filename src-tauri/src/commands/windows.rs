//! Application commands for Cookbench-owned detached Stove bars.
//!
//! The service is deliberately parameterized over window and monitor adapters.
//! This keeps lifecycle rules testable without a desktop runtime and ensures the
//! command layer cannot reach into an agent, terminal, or session file.

use std::{fmt, sync::Mutex};

use cookbench_core::persistence::{
    resolve_top_dock, select_dock_monitor, top_dock_decision, DetachedStoveLayout,
    DockMonitorWorkArea, GlobalBarPosition, GlobalBarTopDock, MonitorIdentity, MonitorWorkArea,
    RelativePosition, TopDockDecision, TopDockInput, WindowPosition, WindowSize,
};

use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, Runtime, State, WebviewUrl,
    WebviewWindowBuilder,
};

use crate::{
    platform::{apply_platform_overlay, OverlayError},
    window_registry::{DetachOutcome, DetachedWindowHost, RegistryError, WindowRegistry},
};

pub(crate) fn native_monitor_identity(
    name: Option<String>,
    index: usize,
    duplicate_occurrence: Option<usize>,
) -> MonitorIdentity {
    // Display names are not unique (two identical external panels commonly
    // report the same name). Keep v0.3 name IDs where unambiguous; only a
    // duplicate gets a stable occurrence discriminator.
    let name_key = name.as_deref().unwrap_or("monitor");
    MonitorIdentity {
        id: duplicate_occurrence
            .map(|occurrence| format!("{name_key}#{occurrence}"))
            .unwrap_or_else(|| name.clone().unwrap_or_else(|| format!("monitor-{index}"))),
        name,
    }
}

pub(crate) fn duplicate_name_occurrence(names: &[Option<String>], index: usize) -> Option<usize> {
    let name = names.get(index)?.as_ref()?;
    (names
        .iter()
        .filter(|candidate| candidate.as_ref() == Some(name))
        .count()
        > 1)
    .then(|| {
        names[..index]
            .iter()
            .filter(|candidate| candidate.as_ref() == Some(name))
            .count()
    })
}

pub(crate) fn same_native_monitor(left: &tauri::Monitor, right: &tauri::Monitor) -> bool {
    let left_area = left.work_area();
    let right_area = right.work_area();
    left.name() == right.name()
        && left_area.position == right_area.position
        && left_area.size == right_area.size
        && left.scale_factor() == right.scale_factor()
}

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
    next_drag_token: u64,
    active_drag: Option<u64>,
    guards: GlobalBarDockGuards,
    generation: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DockGeometryCandidate {
    generation: u64,
    dock: GlobalBarTopDock,
    operation: DockGeometryOperation,
    expanded_position: Option<WindowPosition>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DockGeometryOperation {
    Collapse,
    Refresh { collapsed: bool },
}

impl DockGeometryCandidate {
    fn collapsed(&self) -> bool {
        match self.operation {
            DockGeometryOperation::Collapse => true,
            DockGeometryOperation::Refresh { collapsed } => collapsed,
        }
    }

    fn with_expanded_position(mut self, position: WindowPosition) -> Self {
        self.expanded_position = Some(position);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum CollapseCompensation {
    Expand(GlobalBarTopDock),
    MoveVisible(WindowPosition),
    KeepVisible,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DragFinishPlan {
    Reliable,
    BestEffortVisible,
}

fn drag_finish_plan(reliable_positioning: bool) -> DragFinishPlan {
    if reliable_positioning {
        DragFinishPlan::Reliable
    } else {
        DragFinishPlan::BestEffortVisible
    }
}

fn execute_drag_finish_plan<T>(
    plan: DragFinishPlan,
    best_effort: impl FnOnce() -> Result<T, String>,
    reliable: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    match plan {
        DragFinishPlan::BestEffortVisible => best_effort(),
        DragFinishPlan::Reliable => reliable(),
    }
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalBarDragStartWire {
    pub token: u64,
    pub completed: bool,
    pub release_confirmed: bool,
    pub state: Option<GlobalBarDockStateWire>,
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
    fn drag_reveal_needed(&self) -> Option<GlobalBarTopDock> {
        let controller = self.0.lock().expect("global bar dock lock poisoned");
        (controller.phase == GlobalBarDockPhase::DockedCollapsed)
            .then(|| controller.dock.clone())
            .flatten()
    }
    fn begin_drag_after_reveal(&self) -> u64 {
        let mut controller = self.0.lock().expect("global bar dock lock poisoned");
        if controller.dock.is_some() {
            controller.phase = GlobalBarDockPhase::DockedExpanded;
        }
        controller.generation = controller.generation.wrapping_add(1);
        controller.next_drag_token = controller.next_drag_token.wrapping_add(1).max(1);
        controller.active_drag = Some(controller.next_drag_token);
        controller.next_drag_token
    }
    fn cancel_drag(&self, token: u64) {
        let mut controller = self.0.lock().expect("global bar dock lock poisoned");
        if controller.active_drag == Some(token) {
            controller.active_drag = None;
            controller.generation = controller.generation.wrapping_add(1);
        }
    }
    fn consume_drag(&self, token: u64) -> Option<Option<GlobalBarTopDock>> {
        let mut controller = self.0.lock().expect("global bar dock lock poisoned");
        if controller.active_drag != Some(token) {
            return None;
        }
        controller.active_drag = None;
        controller.generation = controller.generation.wrapping_add(1);
        Some(controller.dock.clone())
    }
    pub(crate) fn commit_dock(&self, dock: Option<GlobalBarTopDock>) {
        let mut controller = self.0.lock().expect("global bar dock lock poisoned");
        controller.dock = dock;
        controller.active_drag = None;
        controller.generation = controller.generation.wrapping_add(1);
        controller.phase = if controller.dock.is_some() {
            GlobalBarDockPhase::DockedExpanded
        } else {
            GlobalBarDockPhase::Undocked
        };
    }
    fn set_guards(&self, guards: GlobalBarDockGuards) -> bool {
        let mut controller = self.0.lock().expect("global bar dock lock poisoned");
        controller.guards = guards;
        controller.generation = controller.generation.wrapping_add(1);
        guards.blocks_collapse() && controller.phase == GlobalBarDockPhase::DockedCollapsed
    }
    fn collapse_candidate(&self) -> Option<DockGeometryCandidate> {
        let controller = self.0.lock().expect("global bar dock lock poisoned");
        if controller.dock.is_none()
            || controller.phase != GlobalBarDockPhase::DockedExpanded
            || controller.active_drag.is_some()
            || controller.guards.blocks_collapse()
            || !reliable_top_dock_positioning()
        {
            return None;
        }
        controller.dock.clone().map(|dock| DockGeometryCandidate {
            generation: controller.generation,
            dock,
            operation: DockGeometryOperation::Collapse,
            expanded_position: None,
        })
    }
    fn refresh_candidate(&self) -> Option<DockGeometryCandidate> {
        let controller = self.0.lock().expect("global bar dock lock poisoned");
        controller.dock.clone().map(|dock| DockGeometryCandidate {
            generation: controller.generation,
            dock,
            operation: DockGeometryOperation::Refresh {
                collapsed: controller.phase == GlobalBarDockPhase::DockedCollapsed,
            },
            expanded_position: None,
        })
    }
    fn commit_geometry(&self, candidate: &DockGeometryCandidate) -> bool {
        let mut controller = self.0.lock().expect("global bar dock lock poisoned");
        if controller.generation != candidate.generation
            || controller.dock.as_ref() != Some(&candidate.dock)
            || (candidate.operation == DockGeometryOperation::Collapse
                && (controller.phase != GlobalBarDockPhase::DockedExpanded
                    || controller.guards.blocks_collapse()))
        {
            return false;
        }
        if candidate.operation == DockGeometryOperation::Collapse {
            controller.phase = GlobalBarDockPhase::DockedCollapsed;
            controller.generation = controller.generation.wrapping_add(1);
        }
        true
    }
    fn commit_revealed(&self) -> bool {
        let mut controller = self.0.lock().expect("global bar dock lock poisoned");
        if controller.dock.is_some() && controller.phase == GlobalBarDockPhase::DockedCollapsed {
            controller.phase = GlobalBarDockPhase::DockedExpanded;
            controller.generation = controller.generation.wrapping_add(1);
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
    fn collapse_compensation(
        &self,
        fallback_position: Option<WindowPosition>,
    ) -> CollapseCompensation {
        self.dock()
            .map(CollapseCompensation::Expand)
            .or_else(|| fallback_position.map(CollapseCompensation::MoveVisible))
            .unwrap_or(CollapseCompensation::KeepVisible)
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
        let primary = self.app.primary_monitor()?;
        let monitors = self.app.available_monitors()?;
        let names = monitors
            .iter()
            .map(|monitor| monitor.name().cloned())
            .collect::<Vec<_>>();
        monitors
            .iter()
            .enumerate()
            .map(|(index, monitor)| {
                let name = monitor.name().cloned();
                let area = monitor.work_area();
                Ok(MonitorWorkArea {
                    primary: primary
                        .as_ref()
                        .is_some_and(|value| same_native_monitor(value, monitor)),
                    identity: native_monitor_identity(
                        name,
                        index,
                        duplicate_name_occurrence(&names, index),
                    ),
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
    let primary = window
        .primary_monitor()
        .map_err(|error| error.to_string())?;
    let monitors = window
        .available_monitors()
        .map_err(|error| error.to_string())?;
    let names = monitors
        .iter()
        .map(|monitor| monitor.name().cloned())
        .collect::<Vec<_>>();
    monitors
        .iter()
        .enumerate()
        .map(|(index, monitor)| {
            let name = monitor.name().cloned();
            let area = monitor.work_area();
            Ok(DockMonitorWorkArea::new(
                MonitorWorkArea {
                    primary: primary
                        .as_ref()
                        .is_some_and(|value| same_native_monitor(value, monitor)),
                    identity: native_monitor_identity(
                        name,
                        index,
                        duplicate_name_occurrence(&names, index),
                    ),
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
    let monitor = select_dock_monitor(monitors, position, size, None)
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

fn expanded_dock_position<R: Runtime>(
    window: &tauri::WebviewWindow<R>,
    dock: &GlobalBarTopDock,
) -> Result<WindowPosition, String> {
    let size = window.outer_size().map_err(|error| error.to_string())?;
    resolve_top_dock(
        dock,
        WindowSize {
            width: size.width,
            height: size.height,
        },
        &dock_monitors(window)?,
    )
    .map(|geometry| geometry.expanded_position)
    .ok_or_else(|| "no display is available for the Cookbench global Bar".to_owned())
}

fn compensate_stale_geometry<R: Runtime>(
    window: &tauri::WebviewWindow<R>,
    runtime: &GlobalBarDockRuntime,
    fallback_position: Option<WindowPosition>,
) -> Result<(), String> {
    // A native move is not transactional. Once another interaction invalidates
    // its candidate, put the window at the current expanded dock instead of
    // leaving it physically hidden behind the trigger strip.
    let compensation = runtime.collapse_compensation(fallback_position);
    execute_geometry_compensation(
        compensation,
        || window.show().map_err(|error| error.to_string()),
        |dock| move_to_dock_geometry(window, dock, false),
        |position| {
            window
                .set_position(PhysicalPosition::new(position.x, position.y))
                .map_err(|error| error.to_string())
        },
    )
}

fn execute_geometry_compensation(
    compensation: CollapseCompensation,
    show: impl FnOnce() -> Result<(), String>,
    expand: impl FnOnce(&GlobalBarTopDock) -> Result<(), String>,
    move_visible: impl FnOnce(WindowPosition) -> Result<(), String>,
) -> Result<(), String> {
    show()?;
    match compensation {
        CollapseCompensation::Expand(dock) => expand(&dock),
        CollapseCompensation::MoveVisible(position) => move_visible(position),
        CollapseCompensation::KeepVisible => Ok(()),
    }
}

#[tauri::command]
pub fn get_global_bar_dock_state(
    runtime: State<'_, GlobalBarDockRuntime>,
) -> GlobalBarDockStateWire {
    runtime.state()
}

fn begin_global_bar_drag(app: AppHandle, runtime: &GlobalBarDockRuntime) -> Result<u64, String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Cookbench global Bar window is unavailable".to_owned())?;
    if let Some(dock) = runtime.drag_reveal_needed() {
        move_to_dock_geometry(&window, &dock, false)?;
        runtime.commit_revealed();
        emit_dock_state(&app, runtime);
    }
    let token = runtime.begin_drag_after_reveal();
    if let Err(error) = window.start_dragging() {
        runtime.cancel_drag(token);
        return Err(error.to_string());
    }
    Ok(token)
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub fn start_global_bar_drag(
    app: AppHandle,
    state: State<'_, crate::app_state::AppState>,
    runtime: State<'_, GlobalBarDockRuntime>,
) -> Result<GlobalBarDragStartWire, String> {
    let token = begin_global_bar_drag(app.clone(), runtime.inner())?;
    // This synchronous command preserves AppKit's blocking drag loop.
    let state = finish_global_bar_drag_inner(token, &app, state.inner(), runtime.inner())?;
    Ok(GlobalBarDragStartWire {
        token,
        completed: true,
        release_confirmed: true,
        state: Some(state),
    })
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub async fn start_global_bar_drag(
    app: AppHandle,
    state: State<'_, crate::app_state::AppState>,
    runtime: State<'_, GlobalBarDockRuntime>,
) -> Result<GlobalBarDragStartWire, String> {
    let token = begin_global_bar_drag(app.clone(), runtime.inner())?;
    let evidence =
        tauri::async_runtime::spawn_blocking(crate::platform::wait_for_local_drag_release)
            .await
            .unwrap_or(crate::platform::DragReleaseEvidence::Unavailable);
    if evidence == crate::platform::DragReleaseEvidence::Released {
        let state = finish_global_bar_drag_inner(token, &app, state.inner(), runtime.inner())?;
        Ok(GlobalBarDragStartWire {
            token,
            completed: true,
            release_confirmed: true,
            state: Some(state),
        })
    } else {
        runtime.cancel_drag(token);
        Ok(GlobalBarDragStartWire {
            token,
            completed: true,
            release_confirmed: false,
            state: Some(runtime.state()),
        })
    }
}

#[tauri::command]
pub fn finish_global_bar_drag(
    token: u64,
    app: AppHandle,
    state: State<'_, crate::app_state::AppState>,
    runtime: State<'_, GlobalBarDockRuntime>,
) -> Result<GlobalBarDockStateWire, String> {
    finish_global_bar_drag_inner(token, &app, state.inner(), runtime.inner())
}

#[tauri::command]
pub async fn wait_for_global_bar_pointer_release() -> bool {
    matches!(
        tauri::async_runtime::spawn_blocking(crate::platform::wait_for_local_drag_release).await,
        Ok(crate::platform::DragReleaseEvidence::Released)
    )
}

fn finish_global_bar_drag_inner(
    token: u64,
    app: &AppHandle,
    state: &crate::app_state::AppState,
    runtime: &GlobalBarDockRuntime,
) -> Result<GlobalBarDockStateWire, String> {
    // Persistence is the recovery authority. Snapshot it before consuming the
    // one-shot token so a failed native finish always knows where to return.
    let prior_dock = state.persisted_config().layout.global_bar_top_dock;
    let Some(_runtime_prior) = runtime.consume_drag(token) else {
        return Ok(runtime.state());
    };
    execute_drag_finish_plan(
        drag_finish_plan(reliable_top_dock_positioning()),
        || {
            let window = app
                .get_webview_window("main")
                .ok_or_else(|| "Cookbench global Bar window is unavailable".to_owned())?;
            window.show().map_err(|error| error.to_string())?;
            emit_dock_state(app, runtime);
            Ok(runtime.state())
        },
        || {
            let original_position = (|| -> Result<WindowPosition, String> {
                let window = app
                    .get_webview_window("main")
                    .ok_or_else(|| "Cookbench global Bar window is unavailable".to_owned())?;
                let position = window.outer_position().map_err(|error| error.to_string())?;
                Ok(WindowPosition {
                    x: position.x,
                    y: position.y,
                })
            })();
            execute_drag_transition(
                prior_dock.as_ref(),
                original_position.as_ref().ok().copied(),
                || {
                    let original_position = original_position.clone()?;
                    let window = app
                        .get_webview_window("main")
                        .ok_or_else(|| "Cookbench global Bar window is unavailable".to_owned())?;
                    let native_size = window.outer_size().map_err(|error| error.to_string())?;
                    let position = original_position;
                    let size = WindowSize {
                        width: native_size.width,
                        height: native_size.height,
                    };
                    let monitors = dock_monitors(&window)?;
                    match top_dock_decision(TopDockInput {
                        position,
                        size,
                        monitors: &monitors,
                        prior_dock: prior_dock.as_ref(),
                        reliable_positioning: true,
                    }) {
                        TopDockDecision::Dock(dock) | TopDockDecision::RemainDocked(dock) => {
                            move_to_dock_geometry(&window, &dock, false)?;
                            state.update_persisted_config(|config| {
                                config.layout.global_bar_top_dock = Some(dock.clone())
                            })?;
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
                    emit_dock_state(app, runtime);
                    Ok(runtime.state())
                },
                |old| {
                    let window = app.get_webview_window("main").ok_or_else(|| {
                        "Cookbench global Bar window is unavailable for rollback".to_owned()
                    })?;
                    move_to_dock_geometry(&window, old, false)
                },
                |position| {
                    let window = app.get_webview_window("main").ok_or_else(|| {
                        "Cookbench global Bar window is unavailable for rollback".to_owned()
                    })?;
                    window
                        .set_position(PhysicalPosition::new(position.x, position.y))
                        .map_err(|error| error.to_string())
                },
            )
        },
    )
}

fn execute_drag_transition<T>(
    prior_dock: Option<&GlobalBarTopDock>,
    prior_position: Option<WindowPosition>,
    transition: impl FnOnce() -> Result<T, String>,
    rollback_dock: impl FnOnce(&GlobalBarTopDock) -> Result<(), String>,
    rollback_position: impl FnOnce(WindowPosition) -> Result<(), String>,
) -> Result<T, String> {
    match transition() {
        Ok(value) => Ok(value),
        Err(primary) => match prior_dock {
            Some(prior) => match rollback_dock(prior) {
                Ok(()) => Err(primary),
                Err(rollback_error) => Err(format!(
                    "{primary}; could not restore the prior expanded Global Bar: {rollback_error}"
                )),
            },
            None => match prior_position {
                Some(position) => match rollback_position(position) {
                    Ok(()) => Err(primary),
                    Err(rollback_error) => Err(format!(
                        "{primary}; could not restore the prior freeform Global Bar: {rollback_error}"
                    )),
                },
                None => Err(primary),
            },
        },
    }
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
    if let Some(candidate) = runtime.collapse_candidate() {
        let window = app
            .get_webview_window("main")
            .ok_or_else(|| "Cookbench global Bar window is unavailable".to_owned())?;
        let expanded_position = window.outer_position().map_err(|error| error.to_string())?;
        let candidate = candidate.with_expanded_position(WindowPosition {
            x: expanded_position.x,
            y: expanded_position.y,
        });
        move_to_dock_geometry(&window, &candidate.dock, true)?;
        if runtime.commit_geometry(&candidate) {
            emit_dock_state(&app, &runtime);
        } else if let Err(rollback_error) =
            compensate_stale_geometry(&window, runtime.inner(), candidate.expanded_position)
        {
            return Err(format!(
                "Global Bar collapse was superseded; could not restore the expanded Bar: {rollback_error}"
            ));
        }
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
    if let Some(dock) = runtime.dock() {
        move_to_dock_geometry(&window, &dock, false)?;
        runtime.commit_revealed();
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
    if let Some(candidate) = runtime.refresh_candidate() {
        let window = app
            .get_webview_window("main")
            .ok_or_else(|| "Cookbench global Bar window is unavailable".to_owned())?;
        let expanded_position = expanded_dock_position(&window, &candidate.dock)?;
        let candidate = candidate.with_expanded_position(expanded_position);
        let collapsed = candidate.collapsed() && reliable_top_dock_positioning();
        move_to_dock_geometry(&window, &candidate.dock, collapsed)?;
        if !runtime.commit_geometry(&candidate) {
            compensate_stale_geometry(&window, runtime.inner(), candidate.expanded_position)
                .map_err(|error| {
                format!("Global Bar refresh was superseded; could not restore the expanded Bar: {error}")
            })?;
        }
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
    if config.layout.global_bar_visible {
        if let Err(error) = (|| -> Result<(), String> {
            window.show().map_err(|error| error.to_string())?;
            match apply_platform_overlay(&window) {
                Ok(()) | Err(OverlayError::BestEffortWayland) => (),
                Err(error) => return Err(error.to_string()),
            }
            move_to_dock_geometry(&window, &dock, false)
        })() {
            runtime.commit_dock(None);
            return Err(error);
        }
    } else {
        window.hide().map_err(|error| error.to_string())?;
    }
    runtime.commit_dock(Some(dock));
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
        let collapse = runtime.collapse_candidate().unwrap();
        assert!(runtime.commit_geometry(&collapse));
        assert_eq!(runtime.drag_reveal_needed(), Some(dock()));
        runtime.commit_revealed();
        let token = runtime.begin_drag_after_reveal();
        assert_eq!(runtime.state().phase, GlobalBarDockPhase::DockedExpanded);
        assert_eq!(runtime.consume_drag(token), Some(Some(dock())));
        assert_eq!(runtime.consume_drag(token), None);
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
            assert!(runtime.collapse_candidate().is_none());
            assert_eq!(runtime.state().phase, GlobalBarDockPhase::DockedExpanded);
        }
    }

    #[test]
    fn active_drag_blocks_collapse_until_its_token_finishes() {
        let runtime = GlobalBarDockRuntime::default();
        runtime.commit_dock(Some(dock()));
        let token = runtime.begin_drag_after_reveal();
        assert!(runtime.collapse_candidate().is_none());
        assert_eq!(runtime.consume_drag(token), Some(Some(dock())));
        assert!(runtime.collapse_candidate().is_some());
    }

    #[test]
    fn guard_reveals_a_collapsed_dock() {
        let runtime = GlobalBarDockRuntime::default();
        runtime.commit_dock(Some(dock()));
        let collapse = runtime.collapse_candidate().unwrap();
        assert!(runtime.commit_geometry(&collapse));
        assert!(runtime.set_guards(GlobalBarDockGuards {
            focused: true,
            ..Default::default()
        }));
        // Guard entry marks a reveal request; the phase changes only after
        // native movement succeeds.
        assert_eq!(runtime.state().phase, GlobalBarDockPhase::DockedCollapsed);
        runtime.commit_revealed();
        assert_eq!(runtime.state().phase, GlobalBarDockPhase::DockedExpanded);
    }

    #[test]
    fn empty_runtime_never_collapses_or_reveals() {
        let runtime = GlobalBarDockRuntime::default();
        assert!(runtime.collapse_candidate().is_none());
        assert!(!runtime.commit_revealed());
        assert_eq!(runtime.state().phase, GlobalBarDockPhase::Undocked);
    }

    #[test]
    fn overlapping_and_stale_drag_tokens_are_rejected() {
        let runtime = GlobalBarDockRuntime::default();
        let first = runtime.begin_drag_after_reveal();
        let second = runtime.begin_drag_after_reveal();
        assert_ne!(first, second);
        assert_eq!(runtime.consume_drag(first), None);
        runtime.cancel_drag(second);
        assert_eq!(runtime.consume_drag(second), None);
    }

    #[test]
    fn failed_native_drag_start_cancels_its_token_and_rejects_stale_finish() {
        let runtime = GlobalBarDockRuntime::default();
        let token = runtime.begin_drag_after_reveal();
        // This is the model operation used by start_global_bar_drag when
        // Tauri rejects start_dragging.
        runtime.cancel_drag(token);
        assert_eq!(runtime.consume_drag(token), None);
    }

    #[test]
    fn stale_collapse_and_refresh_request_expanded_compensation() {
        let runtime = GlobalBarDockRuntime::default();
        runtime.commit_dock(Some(dock()));
        let collapse = runtime.collapse_candidate().unwrap();
        runtime.set_guards(GlobalBarDockGuards {
            pointer_inside: true,
            ..Default::default()
        });
        assert!(!runtime.commit_geometry(&collapse));
        assert_eq!(
            runtime.collapse_compensation(None),
            CollapseCompensation::Expand(dock())
        );

        let refresh = runtime.refresh_candidate().unwrap();
        runtime.commit_dock(None);
        assert!(!runtime.commit_geometry(&refresh));
        assert_eq!(
            runtime.collapse_compensation(None),
            CollapseCompensation::KeepVisible
        );
    }

    #[test]
    fn stale_collapse_without_a_dock_restores_the_captured_on_screen_position() {
        let runtime = GlobalBarDockRuntime::default();
        runtime.commit_dock(Some(dock()));
        let on_screen = WindowPosition { x: 120, y: 24 };
        let collapse = runtime
            .collapse_candidate()
            .unwrap()
            .with_expanded_position(on_screen);
        runtime.commit_dock(None);
        assert!(!runtime.commit_geometry(&collapse));
        assert_eq!(
            runtime.collapse_compensation(collapse.expanded_position),
            CollapseCompensation::MoveVisible(on_screen)
        );
    }

    #[test]
    fn stale_collapsed_refresh_moves_a_fake_window_back_on_screen() {
        let runtime = GlobalBarDockRuntime::default();
        runtime.commit_dock(Some(dock()));
        let collapse = runtime.collapse_candidate().unwrap();
        assert!(runtime.commit_geometry(&collapse));
        let expanded = WindowPosition { x: 120, y: 24 };
        let refresh = runtime
            .refresh_candidate()
            .unwrap()
            .with_expanded_position(expanded);
        runtime.commit_dock(None);
        assert!(!runtime.commit_geometry(&refresh));

        let mut physical_position = WindowPosition { x: 120, y: -97 };
        execute_geometry_compensation(
            runtime.collapse_compensation(refresh.expanded_position),
            || Ok(()),
            |_| Ok(()),
            |position| {
                physical_position = position;
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(physical_position, expanded);
    }

    #[test]
    fn best_effort_finish_never_requests_global_coordinates() {
        let coordinate_queries = std::cell::Cell::new(0);
        let result = execute_drag_finish_plan(
            drag_finish_plan(false),
            || Ok("visible"),
            || {
                coordinate_queries.set(coordinate_queries.get() + 1);
                Ok("reliable")
            },
        );
        assert_eq!(result, Ok("visible"));
        assert_eq!(coordinate_queries.get(), 0);
    }

    #[test]
    fn collapsed_refresh_commits_without_revealing() {
        let runtime = GlobalBarDockRuntime::default();
        runtime.commit_dock(Some(dock()));
        let collapse = runtime.collapse_candidate().unwrap();
        assert!(runtime.commit_geometry(&collapse));
        let refresh = runtime.refresh_candidate().unwrap();
        assert!(refresh.collapsed());
        assert!(runtime.commit_geometry(&refresh));
        assert_eq!(runtime.state().phase, GlobalBarDockPhase::DockedCollapsed);
    }

    #[test]
    fn drag_transition_rolls_back_query_move_and_save_failures() {
        for stage in ["query", "move", "save"] {
            let disk = Some(dock());
            let mut rollbacks = 0;
            let result = execute_drag_transition(
                disk.as_ref(),
                None,
                || Err::<(), _>(format!("{stage} failed")),
                |_| {
                    rollbacks += 1;
                    Ok(())
                },
                |_| Ok(()),
            );
            assert_eq!(result, Err(format!("{stage} failed")));
            assert_eq!(rollbacks, 1);
            assert_eq!(disk, Some(dock()));
        }
    }

    #[test]
    fn drag_transition_preserves_primary_and_rollback_errors() {
        let prior = dock();
        let result = execute_drag_transition(
            Some(&prior),
            None,
            || Err::<(), _>("save failed".to_owned()),
            |_| Err("restore move failed".to_owned()),
            |_| Ok(()),
        );
        assert_eq!(
            result,
            Err(
                "save failed; could not restore the prior expanded Global Bar: restore move failed"
                    .to_owned()
            )
        );
    }

    #[test]
    fn undocked_save_failure_restores_the_exact_original_position() {
        let original = WindowPosition { x: 320, y: 180 };
        let mut restored = None;
        let result = execute_drag_transition(
            None,
            Some(original),
            || Err::<(), _>("save failed".to_owned()),
            |_| Ok(()),
            |position| {
                restored = Some(position);
                Ok(())
            },
        );
        assert_eq!(result, Err("save failed".to_owned()));
        assert_eq!(restored, Some(original));
    }

    #[test]
    fn native_monitor_identity_keeps_duplicate_names_distinct() {
        let first = native_monitor_identity(Some("Panel".into()), 0, Some(0));
        let second = native_monitor_identity(Some("Panel".into()), 1, Some(1));
        assert_ne!(first.id, second.id);
        assert_eq!(first.name, second.name);
    }

    #[test]
    fn monitor_occurrences_preserve_legacy_unique_and_unnamed_ids() {
        let names = vec![
            Some("Main".into()),
            Some("Panel".into()),
            Some("Panel".into()),
            None,
        ];
        assert_eq!(duplicate_name_occurrence(&names, 0), None);
        assert_eq!(duplicate_name_occurrence(&names, 1), Some(0));
        assert_eq!(duplicate_name_occurrence(&names, 2), Some(1));
        assert_eq!(
            native_monitor_identity(names[0].clone(), 0, None).id,
            "Main"
        );
        assert_eq!(native_monitor_identity(None, 3, None).id, "monitor-3");
    }
}
