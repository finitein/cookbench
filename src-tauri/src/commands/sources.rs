use tauri::State;

use crate::runtime::{LocalSourceStatusResponse, LocalSourceStatusState};

#[tauri::command]
pub fn get_local_source_status(
    status: State<'_, LocalSourceStatusState>,
) -> LocalSourceStatusResponse {
    status.snapshot()
}
