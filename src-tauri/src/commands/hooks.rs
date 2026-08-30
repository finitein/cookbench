use serde::Deserialize;

use crate::hooks::{self, HookAction, HookActionResult, HookError, HookHarness, HookStatus};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookActionRequest {
    pub harness: HookHarness,
    pub action: HookActionWire,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HookActionWire {
    PreviewInstall,
    Install,
    Repair,
    Uninstall,
}

impl From<HookActionWire> for HookAction {
    fn from(value: HookActionWire) -> Self {
        match value {
            HookActionWire::PreviewInstall => Self::PreviewInstall,
            HookActionWire::Install => Self::Install,
            HookActionWire::Repair => Self::Repair,
            HookActionWire::Uninstall => Self::Uninstall,
        }
    }
}

#[tauri::command]
pub fn get_hook_status() -> Vec<HookStatus> {
    hooks::statuses()
}

#[tauri::command]
pub fn manage_hook(request: HookActionRequest) -> Result<HookActionResult, String> {
    hooks::apply(request.harness, request.action.into()).map_err(hook_error)
}

fn hook_error(error: HookError) -> String {
    error.to_string()
}
