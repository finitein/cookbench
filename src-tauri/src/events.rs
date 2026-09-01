use serde::{Deserialize, Serialize};
use tauri::Emitter;

use crate::app_state::StoveWire;

pub const STOVE_CHANGED_EVENT: &str = "cookbench://stove-changed";

/// One ordered change following a snapshot. A missing revision is deliberately
/// recoverable: clients must fetch another snapshot rather than guess state.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoveChange {
    pub revision: u64,
    pub stove: Option<StoveWire>,
    pub removed_stove_id: Option<String>,
    pub attention_order: Vec<String>,
}

impl StoveChange {
    pub fn upsert(revision: u64, stove: StoveWire) -> Self {
        Self {
            revision,
            stove: Some(stove),
            removed_stove_id: None,
            attention_order: Vec::new(),
        }
    }

    pub fn remove(revision: u64, stove_id: String) -> Self {
        Self {
            revision,
            stove: None,
            removed_stove_id: Some(stove_id),
            attention_order: Vec::new(),
        }
    }
}

pub fn emit_stove_change<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    change: StoveChange,
) -> tauri::Result<()> {
    app.emit(STOVE_CHANGED_EVENT, change)
}
