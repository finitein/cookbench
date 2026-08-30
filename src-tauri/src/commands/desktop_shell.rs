//! Commands for explicit, opt-in desktop shell preferences.

use serde::Serialize;
use tauri::{AppHandle, Runtime};
use tauri_plugin_autostart::ManagerExt;

use crate::desktop_shell::{autostart_transition, default_autostart_enabled, AutostartTransition};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchAtLoginWire {
    pub enabled: bool,
    pub default_enabled: bool,
}

#[tauri::command]
pub fn get_launch_at_login<R: Runtime>(app: AppHandle<R>) -> Result<LaunchAtLoginWire, String> {
    Ok(LaunchAtLoginWire {
        enabled: app
            .autolaunch()
            .is_enabled()
            .map_err(|error| error.to_string())?,
        default_enabled: default_autostart_enabled(),
    })
}

#[tauri::command]
pub fn set_launch_at_login<R: Runtime>(
    enabled: bool,
    app: AppHandle<R>,
) -> Result<LaunchAtLoginWire, String> {
    let manager = app.autolaunch();
    let current = manager.is_enabled().map_err(|error| error.to_string())?;
    match autostart_transition(current, enabled) {
        AutostartTransition::Enable => manager.enable().map_err(|error| error.to_string())?,
        AutostartTransition::Disable => manager.disable().map_err(|error| error.to_string())?,
        AutostartTransition::NoChange => (),
    }
    Ok(LaunchAtLoginWire {
        enabled,
        default_enabled: default_autostart_enabled(),
    })
}
