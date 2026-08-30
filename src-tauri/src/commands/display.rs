//! Commands for Cookbench-owned Bar visibility and placement preferences.
//!
//! These controls affect only Cookbench windows. They never inspect, create,
//! or manage a harness session.

use cookbench_core::persistence::{
    GlobalBarPlacement, GlobalBarPosition, MonitorIdentity, MonitorWorkArea, PersistedConfig,
    RelativePosition, WindowPosition,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, PhysicalPosition, State};

use crate::{
    app_state::AppState,
    commands::windows::TauriWindowCommandService,
    platform::{apply_platform_overlay, OverlayError},
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetachedBarWire {
    pub stove_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplaySettingsWire {
    pub global_bar_visible: bool,
    pub global_bar_placement: GlobalBarPlacement,
    pub detached_bars: Vec<DetachedBarWire>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplaySettingsInput {
    pub global_bar_visible: bool,
    pub global_bar_placement: GlobalBarPlacement,
}

pub fn settings_wire(config: &PersistedConfig) -> DisplaySettingsWire {
    DisplaySettingsWire {
        global_bar_visible: config.layout.global_bar_visible,
        global_bar_placement: config.layout.global_bar_placement,
        detached_bars: config
            .layout
            .detached_layouts
            .iter()
            .map(|layout| DetachedBarWire {
                stove_id: layout.stove_key.clone(),
            })
            .collect(),
    }
}

#[tauri::command]
pub fn get_display_settings(state: State<'_, AppState>) -> DisplaySettingsWire {
    settings_wire(&state.persisted_config())
}

#[tauri::command]
pub fn configure_display_settings(
    input: DisplaySettingsInput,
    app: AppHandle,
    state: State<'_, AppState>,
    windows: State<'_, TauriWindowCommandService>,
) -> Result<DisplaySettingsWire, String> {
    apply_global_bar_preferences(
        &app,
        input.global_bar_visible,
        input.global_bar_placement,
        None,
    )?;
    let global_bar_position = input
        .global_bar_visible
        .then(|| capture_global_bar_position(&app))
        .transpose()?;
    windows
        .set_global_bar_visible(input.global_bar_visible)
        .map_err(|error| error.to_string())?;
    state.update_persisted_config(|config| {
        config.layout.global_bar_visible = input.global_bar_visible;
        config.layout.global_bar_placement = input.global_bar_placement;
        if input.global_bar_visible {
            config.layout.global_bar_position = global_bar_position;
        }
    })?;
    Ok(settings_wire(&state.persisted_config()))
}

/// Records a user drag without changing the selected placement anchor. The
/// next launch restores this explicit position before considering that anchor.
#[tauri::command]
pub fn record_global_bar_position(
    x: i32,
    y: i32,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let position = capture_global_bar_position_at(&app, WindowPosition { x, y })?;
    state.update_persisted_config(|config| config.layout.global_bar_position = Some(position))
}

/// Applies the persisted global-bar preference after startup and after a user
/// changes it. The operation intentionally leaves detached windows untouched.
pub fn apply_global_bar_preferences(
    app: &AppHandle,
    visible: bool,
    placement: GlobalBarPlacement,
    position: Option<&GlobalBarPosition>,
) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Cookbench global Bar window is unavailable".to_owned())?;
    if !visible {
        return window.hide().map_err(|error| error.to_string());
    }

    window.show().map_err(|error| error.to_string())?;
    match apply_platform_overlay(&window) {
        Ok(()) | Err(OverlayError::BestEffortWayland) => (),
        Err(error) => return Err(error.to_string()),
    }
    if let Some(position) = position {
        return restore_global_bar_position(&window, position);
    }
    position_global_bar(&window, placement)
}

fn capture_global_bar_position(app: &AppHandle) -> Result<GlobalBarPosition, String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Cookbench global Bar window is unavailable".to_owned())?;
    let position = window.outer_position().map_err(|error| error.to_string())?;
    capture_global_bar_position_at(
        app,
        WindowPosition {
            x: position.x,
            y: position.y,
        },
    )
}

fn capture_global_bar_position_at(
    app: &AppHandle,
    position: WindowPosition,
) -> Result<GlobalBarPosition, String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Cookbench global Bar window is unavailable".to_owned())?;
    let size = window.outer_size().map_err(|error| error.to_string())?;
    let monitors = monitors_for_window(&window)?;
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
        .ok_or_else(|| "no display is available for the Cookbench global Bar".to_owned())?;
    Ok(GlobalBarPosition {
        monitor: monitor.identity.clone(),
        relative_position: RelativePosition::from_absolute(
            position,
            monitor,
            cookbench_core::persistence::WindowSize {
                width: size.width,
                height: size.height,
            },
        ),
    })
}

fn restore_global_bar_position(
    window: &tauri::WebviewWindow,
    saved: &GlobalBarPosition,
) -> Result<(), String> {
    let size = window.outer_size().map_err(|error| error.to_string())?;
    let monitors = monitors_for_window(window)?;
    let monitor = monitors
        .iter()
        .find(|monitor| monitor.identity.id == saved.monitor.id)
        .or_else(|| monitors.iter().find(|monitor| monitor.primary))
        .or_else(|| monitors.first())
        .ok_or_else(|| "no display is available for the Cookbench global Bar".to_owned())?;
    let position = saved.relative_position.resolve(
        monitor,
        cookbench_core::persistence::WindowSize {
            width: size.width,
            height: size.height,
        },
    );
    window
        .set_position(PhysicalPosition::new(position.x, position.y))
        .map_err(|error| error.to_string())
}

fn monitors_for_window(window: &tauri::WebviewWindow) -> Result<Vec<MonitorWorkArea>, String> {
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
            let work_area = monitor.work_area();
            Ok(MonitorWorkArea {
                primary: name == primary_name,
                identity: MonitorIdentity {
                    id: name.clone().unwrap_or_else(|| format!("monitor-{index}")),
                    name,
                },
                x: work_area.position.x,
                y: work_area.position.y,
                width: work_area.size.width,
                height: work_area.size.height,
            })
        })
        .collect()
}

fn position_global_bar(
    window: &tauri::WebviewWindow,
    placement: GlobalBarPlacement,
) -> Result<(), String> {
    let monitor = window
        .current_monitor()
        .map_err(|error| error.to_string())?
        .or_else(|| window.primary_monitor().ok().flatten())
        .ok_or_else(|| "no display is available for the Cookbench global Bar".to_owned())?;
    let area = monitor.work_area();
    let size = window.outer_size().map_err(|error| error.to_string())?;
    let horizontal = match placement {
        GlobalBarPlacement::TopLeft | GlobalBarPlacement::BottomLeft => area.position.x,
        GlobalBarPlacement::TopCenter | GlobalBarPlacement::BottomCenter => area
            .position
            .x
            .saturating_add((area.size.width.saturating_sub(size.width) / 2) as i32),
        GlobalBarPlacement::TopRight | GlobalBarPlacement::BottomRight => area
            .position
            .x
            .saturating_add(area.size.width.saturating_sub(size.width) as i32),
    };
    let vertical = match placement {
        GlobalBarPlacement::TopLeft
        | GlobalBarPlacement::TopCenter
        | GlobalBarPlacement::TopRight => area.position.y,
        GlobalBarPlacement::BottomLeft
        | GlobalBarPlacement::BottomCenter
        | GlobalBarPlacement::BottomRight => area
            .position
            .y
            .saturating_add(area.size.height.saturating_sub(size.height) as i32),
    };
    window
        .set_position(PhysicalPosition::new(horizontal, vertical))
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use cookbench_core::persistence::{
        DetachedStoveLayout, GlobalBarPlacement, MonitorIdentity, MonitorWorkArea, WindowPosition,
        WindowSize,
    };

    use super::*;

    #[test]
    fn settings_wire_keeps_independent_bars_when_global_bar_is_hidden() {
        let monitor = MonitorWorkArea {
            identity: MonitorIdentity {
                id: "main".into(),
                name: Some("Main".into()),
            },
            primary: true,
            x: 0,
            y: 0,
            width: 1280,
            height: 800,
        };
        let mut config = PersistedConfig::default();
        config.layout.global_bar_visible = false;
        config.layout.global_bar_placement = GlobalBarPlacement::BottomRight;
        config
            .layout
            .detached_layouts
            .push(DetachedStoveLayout::from_absolute(
                "remote-a:session-1",
                &monitor,
                WindowPosition { x: 24, y: 24 },
                WindowSize {
                    width: 164,
                    height: 104,
                },
            ));

        assert_eq!(
            settings_wire(&config),
            DisplaySettingsWire {
                global_bar_visible: false,
                global_bar_placement: GlobalBarPlacement::BottomRight,
                detached_bars: vec![DetachedBarWire {
                    stove_id: "remote-a:session-1".into()
                }],
            }
        );
    }
}
