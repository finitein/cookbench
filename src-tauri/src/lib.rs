pub mod platform;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let overlay = platform::TauriOverlayController::new(app.handle().clone());
            // Wayland can show the window but cannot promise a compositor-level
            // overlay. The capability model exposes that distinction to UI code.
            match platform::OverlayController::show_global_bar(&overlay) {
                Ok(()) | Err(platform::OverlayError::BestEffortWayland) => Ok(()),
                Err(error) => Err(error.into()),
            }
        })
        .run(tauri::generate_context!())
        .expect("Cookbench desktop shell failed to run");
}
