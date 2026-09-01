//! Commands for Cookbench-owned Bar visibility and placement preferences.
//!
//! These controls affect only Cookbench windows. They never inspect, create,
//! or manage a harness session.

use cookbench_core::persistence::{
    AppLocale, GlobalBarMode, GlobalBarPlacement, GlobalBarPosition, MonitorWorkArea,
    PersistedConfig, RelativePosition, WindowPosition, MAX_MAC_STATUS_STOVE_COUNT,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, State};

use crate::{
    app_state::AppState,
    commands::windows::TauriWindowCommandService,
    i18n::NativeLocaleState,
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
    pub global_bar_mode: GlobalBarMode,
    pub mac_status_stove_count: u8,
    pub mac_status_available: bool,
    pub hover_details_enabled: bool,
    pub locale: AppLocale,
    pub detached_bars: Vec<DetachedBarWire>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplaySettingsInput {
    pub global_bar_visible: bool,
    pub global_bar_placement: GlobalBarPlacement,
    pub global_bar_mode: GlobalBarMode,
    pub mac_status_stove_count: u8,
    pub hover_details_enabled: bool,
    pub locale: AppLocale,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplaySettingsPatch {
    pub global_bar_visible: Option<bool>,
    pub global_bar_placement: Option<GlobalBarPlacement>,
    pub global_bar_mode: Option<GlobalBarMode>,
    pub mac_status_stove_count: Option<u8>,
    pub hover_details_enabled: Option<bool>,
    pub locale: Option<AppLocale>,
}

pub const DISPLAY_SETTINGS_CHANGED_EVENT: &str = "cookbench://display-settings-changed";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DisplayPatchEffects {
    apply_window_preferences: bool,
    set_visibility: bool,
    placement_changed: bool,
}

fn apply_display_patch(
    config: &mut PersistedConfig,
    patch: DisplaySettingsPatch,
) -> Result<DisplayPatchEffects, String> {
    if let Some(count) = patch.mac_status_stove_count {
        validate_mac_status_stove_count(count)?;
    }
    // An explicit placement is a reset command, even when it selects the
    // already active anchor. It must clear both saved geometric overrides.
    let placement_changed = patch.global_bar_placement.is_some();
    if let Some(value) = patch.global_bar_visible {
        config.layout.global_bar_visible = value;
    }
    if let Some(value) = patch.global_bar_placement {
        config.layout.global_bar_placement = value;
    }
    if let Some(value) = patch.global_bar_mode {
        config.layout.global_bar_mode = value;
    }
    if let Some(value) = patch.mac_status_stove_count {
        config.layout.mac_status_stove_count = value;
    }
    if let Some(value) = patch.hover_details_enabled {
        config.layout.hover_details_enabled = value;
    }
    if let Some(value) = patch.locale {
        config.preferences.locale = value;
    }
    if placement_changed {
        config.layout.global_bar_position = None;
        config.layout.global_bar_top_dock = None;
    }
    Ok(DisplayPatchEffects {
        apply_window_preferences: patch.global_bar_visible.is_some()
            || patch.global_bar_placement.is_some(),
        set_visibility: patch.global_bar_visible.is_some(),
        placement_changed,
    })
}

pub fn settings_wire(config: &PersistedConfig) -> DisplaySettingsWire {
    DisplaySettingsWire {
        global_bar_visible: config.layout.global_bar_visible,
        global_bar_placement: config.layout.global_bar_placement,
        global_bar_mode: config.layout.global_bar_mode,
        mac_status_stove_count: config.layout.mac_status_stove_count,
        mac_status_available: cfg!(target_os = "macos"),
        hover_details_enabled: config.layout.hover_details_enabled,
        locale: config.preferences.locale,
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
    native_locale: State<'_, NativeLocaleState>,
    windows: State<'_, TauriWindowCommandService>,
) -> Result<DisplaySettingsWire, String> {
    patch_display_settings(
        DisplaySettingsPatch {
            global_bar_visible: Some(input.global_bar_visible),
            global_bar_placement: Some(input.global_bar_placement),
            global_bar_mode: Some(input.global_bar_mode),
            mac_status_stove_count: Some(input.mac_status_stove_count),
            hover_details_enabled: Some(input.hover_details_enabled),
            locale: Some(input.locale),
        },
        app,
        state,
        native_locale,
        windows,
    )
}

#[tauri::command]
pub fn patch_display_settings(
    patch: DisplaySettingsPatch,
    app: AppHandle,
    state: State<'_, AppState>,
    native_locale: State<'_, NativeLocaleState>,
    windows: State<'_, TauriWindowCommandService>,
) -> Result<DisplaySettingsWire, String> {
    let previous_locale = state.persisted_config().preferences.locale;
    let (effects, committed) =
        state.update_persisted_config_with(|config| apply_display_patch(config, patch))?;
    let wire = settings_wire(&committed);
    if previous_locale != committed.preferences.locale {
        let locale = native_locale.set_preference(committed.preferences.locale);
        apply_native_locale(&app, locale);
    }
    app.emit(DISPLAY_SETTINGS_CHANGED_EVENT, &wire)
        .map_err(|error| error.to_string())?;
    crate::desktop_shell::runtime::refresh_status_stoves(&app);
    if effects.apply_window_preferences {
        let current = committed.layout;
        if effects.placement_changed {
            if let Some(runtime) = app.try_state::<crate::commands::windows::GlobalBarDockRuntime>()
            {
                runtime.commit_dock(None);
                let _ = app.emit(
                    crate::commands::windows::GLOBAL_BAR_DOCK_STATE_CHANGED_EVENT,
                    runtime.state(),
                );
            }
        }
        apply_global_bar_preferences(
            &app,
            current.global_bar_visible,
            current.global_bar_placement,
            if effects.placement_changed {
                None
            } else {
                current.global_bar_position.as_ref()
            },
        )?;
        if current.global_bar_visible {
            if let Some(runtime) = app.try_state::<crate::commands::windows::GlobalBarDockRuntime>()
            {
                crate::commands::windows::reveal_global_bar_dock(&app, runtime.inner())?;
            }
        }
        if effects.set_visibility {
            windows
                .set_global_bar_visible(current.global_bar_visible)
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(wire)
}

fn validate_mac_status_stove_count(count: u8) -> Result<(), String> {
    if count > MAX_MAC_STATUS_STOVE_COUNT {
        return Err(format!(
            "macOS status Stove count must be between 0 and {MAX_MAC_STATUS_STOVE_COUNT}"
        ));
    }
    Ok(())
}

/// Synchronizes the browser-resolved system locale to native surfaces. The
/// persisted preference remains `System`; only the runtime translation used
/// by the tray, window title, and local notifications changes.
#[tauri::command]
pub fn sync_native_locale(
    locale: AppLocale,
    app: AppHandle,
    native_locale: State<'_, NativeLocaleState>,
) -> Result<(), String> {
    let locale = native_locale.set_resolved(locale)?;
    apply_native_locale(&app, locale);
    Ok(())
}

fn apply_native_locale(app: &AppHandle, locale: AppLocale) {
    let _ = crate::desktop_shell::runtime::update_menu(app, locale);
    if let Some(settings) = app.get_webview_window("settings") {
        let _ = settings.set_title(crate::i18n::settings_window_title(locale));
    }
}

pub fn restore_global_bar_size(
    app: &AppHandle,
    size: Option<cookbench_core::persistence::WindowSize>,
) -> Result<(), String> {
    let Some(size) = size else {
        return Ok(());
    };
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Cookbench global Bar window is unavailable".to_owned())?;
    window
        .set_size(LogicalSize::new(
            f64::from(size.width),
            f64::from(size.height),
        ))
        .map_err(|error| error.to_string())
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
    // Dock moves are programmatic. Their coordinates must never replace the
    // last freeform restore point; explicit dock transitions use the native
    // drag-token command instead.
    if state
        .persisted_config()
        .layout
        .global_bar_top_dock
        .is_some()
    {
        return Ok(());
    }
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
            let work_area = monitor.work_area();
            Ok(MonitorWorkArea {
                primary: primary.as_ref().is_some_and(|value| {
                    crate::commands::windows::same_native_monitor(value, monitor)
                }),
                identity: crate::commands::windows::native_monitor_identity(
                    name,
                    index,
                    crate::commands::windows::duplicate_name_occurrence(&names, index),
                ),
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
                global_bar_mode: GlobalBarMode::Full,
                mac_status_stove_count: 3,
                mac_status_available: cfg!(target_os = "macos"),
                hover_details_enabled: false,
                locale: AppLocale::System,
                detached_bars: vec![DetachedBarWire {
                    stove_id: "remote-a:session-1".into()
                }],
            }
        );
    }

    #[test]
    fn settings_wire_exposes_compile_time_mac_status_capability() {
        assert_eq!(
            settings_wire(&PersistedConfig::default()).mac_status_available,
            cfg!(target_os = "macos")
        );
    }

    #[test]
    fn command_input_rejects_an_out_of_range_mac_status_count() {
        assert!(validate_mac_status_stove_count(8).is_ok());
        assert_eq!(
            validate_mac_status_stove_count(9),
            Err("macOS status Stove count must be between 0 and 8".to_owned())
        );
    }

    #[test]
    fn partial_patch_preserves_unrelated_preferences_and_skips_window_effects() {
        let mut config = PersistedConfig::default();
        let effects = apply_display_patch(
            &mut config,
            DisplaySettingsPatch {
                global_bar_mode: Some(GlobalBarMode::Minimal),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(config.layout.global_bar_mode, GlobalBarMode::Minimal);
        assert_eq!(config.layout.mac_status_stove_count, 3);
        assert!(!effects.apply_window_preferences);
        assert!(!effects.set_visibility);
        for patch in [
            DisplaySettingsPatch {
                mac_status_stove_count: Some(2),
                ..Default::default()
            },
            DisplaySettingsPatch {
                hover_details_enabled: Some(true),
                ..Default::default()
            },
            DisplaySettingsPatch {
                locale: Some(AppLocale::En),
                ..Default::default()
            },
        ] {
            assert!(
                !apply_display_patch(&mut config, patch)
                    .unwrap()
                    .apply_window_preferences
            );
        }
    }

    #[test]
    fn window_patches_declare_effects_and_invalid_counts_do_not_mutate() {
        let mut config = PersistedConfig::default();
        let original = config.clone();
        assert!(apply_display_patch(
            &mut config,
            DisplaySettingsPatch {
                mac_status_stove_count: Some(9),
                ..Default::default()
            }
        )
        .is_err());
        assert_eq!(config, original);
        assert!(
            apply_display_patch(
                &mut config,
                DisplaySettingsPatch {
                    global_bar_visible: Some(false),
                    ..Default::default()
                }
            )
            .unwrap()
            .set_visibility
        );
        let effects = apply_display_patch(
            &mut config,
            DisplaySettingsPatch {
                global_bar_placement: Some(GlobalBarPlacement::BottomLeft),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(effects.apply_window_preferences && effects.placement_changed);
        assert_eq!(config.layout.global_bar_position, None);
    }

    #[test]
    fn explicit_same_placement_clears_saved_freeform_and_dock_overrides() {
        let mut config = PersistedConfig::default();
        config.layout.global_bar_placement = GlobalBarPlacement::TopCenter;
        config.layout.global_bar_position = Some(GlobalBarPosition {
            monitor: MonitorIdentity {
                id: "primary".into(),
                name: Some("Primary".into()),
            },
            relative_position: RelativePosition { x: 5_000, y: 0 },
        });
        config.layout.global_bar_top_dock = Some(cookbench_core::persistence::GlobalBarTopDock {
            monitor: MonitorIdentity {
                id: "primary".into(),
                name: Some("Primary".into()),
            },
            relative_x: 5_000,
        });

        let effects = apply_display_patch(
            &mut config,
            DisplaySettingsPatch {
                global_bar_placement: Some(GlobalBarPlacement::TopCenter),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(effects.apply_window_preferences);
        assert!(effects.placement_changed);
        assert_eq!(config.layout.global_bar_position, None);
        assert_eq!(config.layout.global_bar_top_dock, None);

        config.layout.global_bar_position = Some(GlobalBarPosition {
            monitor: MonitorIdentity {
                id: "primary".into(),
                name: Some("Primary".into()),
            },
            relative_position: RelativePosition { x: 5_000, y: 0 },
        });
        config.layout.global_bar_top_dock = Some(cookbench_core::persistence::GlobalBarTopDock {
            monitor: MonitorIdentity {
                id: "primary".into(),
                name: Some("Primary".into()),
            },
            relative_x: 5_000,
        });
        apply_display_patch(
            &mut config,
            DisplaySettingsPatch {
                global_bar_mode: Some(GlobalBarMode::Minimal),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(config.layout.global_bar_position.is_some());
        assert!(config.layout.global_bar_top_dock.is_some());
    }
}
