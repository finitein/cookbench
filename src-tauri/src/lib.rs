pub mod app_state;
pub mod commands;
pub mod events;
pub mod locator;
pub mod notifications;
pub mod platform;
pub mod remote;
pub mod secrets;
pub mod window_registry;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use tauri::Manager;

    let app = tauri::Builder::default()
        .manage(app_state::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::stoves::get_stoves_snapshot
        ])
        .setup(|app| {
            let overlay = platform::TauriOverlayController::new(app.handle().clone());
            let state = app.state::<app_state::AppState>();
            platform::publish_optional_gnome_snapshot(&state.stoves.snapshot());
            // Wayland can show the window but cannot promise a compositor-level
            // overlay. The capability model exposes that distinction to UI code.
            match platform::OverlayController::show_global_bar(&overlay) {
                Ok(()) | Err(platform::OverlayError::BestEffortWayland) => Ok(()),
                Err(error) => Err(error.into()),
            }
        })
        .build(tauri::generate_context!())
        .expect("Cookbench desktop shell failed to build");
    app.run(|_, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            platform::clear_optional_gnome_snapshot();
        }
    });
}
