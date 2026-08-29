pub mod app_state;
pub mod commands;
pub mod diagnostics;
pub mod events;
pub mod hook_spool;
pub mod locator;
pub mod notifications;
pub mod persistence;
pub mod platform;
pub mod remote;
pub mod runtime;
pub mod secrets;
pub mod window_registry;

use std::sync::{Arc, Mutex};

use cookbench_core::{
    domain::{HostIdentity, ProjectIdentity, StoveEvent, StoveIdentity},
    locator::SessionLocator,
};
use tauri::Manager;

struct TauriObservationSink {
    app: tauri::AppHandle,
}

impl runtime::ObservationSink for TauriObservationSink {
    fn apply(
        &self,
        identity: StoveIdentity,
        project: ProjectIdentity,
        _native_locator: String,
        _title: Option<String>,
        summary: runtime::ObservationSummary,
        event: StoveEvent,
    ) {
        let locator = SessionLocator {
            working_directory: std::path::Path::new(&project.canonical_root)
                .is_absolute()
                .then(|| project.canonical_root.clone()),
            native_session_id: identity.native_session_id.clone(),
            ..SessionLocator::default()
        };
        let summary = app_state::StoveSummary::new(
            project
                .canonical_root
                .rsplit(['/', '\\'])
                .find(|part| !part.is_empty())
                .unwrap_or("Project"),
            &project.canonical_root,
            summary.task_title,
            summary.current_action,
            summary.next_action,
            summary.elapsed_ms,
        );
        let state = self.app.state::<app_state::AppState>();
        let _ = state.apply_observation_and_emit(
            &self.app,
            identity,
            project,
            app_state::LocatorCapability::Available,
            Some(locator),
            Some(summary),
            event,
        );
    }
}

struct LocalRuntimeState(Mutex<Option<runtime::RuntimeHandle>>);
struct HookRuntimeState(Mutex<Option<hook_spool::HookSpoolHandle>>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let notification_runtime = notifications::sender::ReqwestTransport::new()
        .map(|transport| {
            commands::notifications::NotificationCommandState(Arc::new(
                notifications::service::NotificationService::new(
                    transport,
                    secrets::NativeSecretStore,
                ),
            ))
        })
        .expect("Cookbench outbound HTTPS client failed to initialize");

    let app = tauri::Builder::default()
        .manage(app_state::AppState::default())
        .manage(notification_runtime)
        .manage(remote::runtime::RemoteRuntimeState::default())
        .manage(LocalRuntimeState(Mutex::new(None)))
        .manage(HookRuntimeState(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            commands::stoves::get_stoves_snapshot,
            commands::stoves::clear_cooked_stove,
            commands::windows::detach_stove,
            commands::windows::clear_detached_stove,
            commands::windows::record_detached_stove_position,
            commands::locator::activate_stove_locator,
            commands::notifications::open_notification_settings,
            commands::notifications::get_notification_settings,
            commands::notifications::configure_notification_destination,
            commands::notifications::send_test_notification,
            commands::remote::get_remote_sources,
            commands::remote::configure_remote_source,
            commands::remote::remove_remote_source,
        ])
        .setup(|app| {
            let app_data = app.path().app_data_dir()?;
            let state = app.state::<app_state::AppState>();
            state.initialize_persistence(&app_data);
            if commands::notifications::configure_runtime(
                &state.persisted_config(),
                &app.state::<commands::notifications::NotificationCommandState>()
                    .0,
            )
            .is_err()
            {
                eprintln!("Cookbench skipped invalid persisted notification settings");
            }
            if app
                .state::<remote::runtime::RemoteRuntimeState>()
                .reconfigure(
                    app.handle().clone(),
                    &state.persisted_config().remote_sources,
                )
                .is_err()
            {
                eprintln!("Cookbench skipped invalid persisted SSH settings");
            }

            app.manage(commands::windows::TauriWindowCommandService::new(
                window_registry::WindowRegistry::new(true),
                commands::windows::TauriDetachedWindowHost::new(app.handle().clone()),
                commands::windows::TauriMonitorProvider::new(app.handle().clone()),
            ));
            let layouts = state.persisted_config().layout.detached_layouts;
            app.state::<commands::windows::TauriWindowCommandService>()
                .restore(layouts)
                .map_err(|error| error.to_string())?;

            let observer = Arc::new(TauriObservationSink {
                app: app.handle().clone(),
            });
            let handle = runtime::start(
                runtime::LocalObservationConfig::from_environment(HostIdentity::local("local")),
                observer,
            );
            *app.state::<LocalRuntimeState>()
                .0
                .lock()
                .expect("local runtime lock poisoned") = Some(handle);

            let hook_directory = app_data.join("hook-spool");
            if let Ok(spool) =
                hook_spool::HookSpool::create(hook_directory, HostIdentity::local("local"))
            {
                let hook_app = app.handle().clone();
                let consumer = Arc::new(move |observation: hook_spool::HookObservation| {
                    let state = hook_app.state::<app_state::AppState>();
                    let _ = state.apply_observation_and_emit(
                        &hook_app,
                        observation.identity,
                        observation.project,
                        app_state::LocatorCapability::Unavailable,
                        None,
                        None,
                        observation.event,
                    );
                });
                let handle = hook_spool::start(spool, consumer);
                *app.state::<HookRuntimeState>()
                    .0
                    .lock()
                    .expect("hook runtime lock poisoned") = Some(handle);
            }

            let overlay = platform::TauriOverlayController::new(app.handle().clone());
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
    app.run(|app, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            platform::clear_optional_gnome_snapshot();
            if let Some(handle) = app
                .state::<HookRuntimeState>()
                .0
                .lock()
                .expect("hook runtime lock poisoned")
                .take()
            {
                handle.cancel();
            }
        }
    });
}
