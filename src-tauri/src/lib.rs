pub mod app_state;
pub mod commands;
pub mod desktop_shell;
pub mod diagnostics;
pub mod events;
pub mod hook_spool;
pub mod hooks;
pub mod locator;
pub mod notifications;
pub mod persistence;
pub mod platform;
pub mod remote;
pub mod runtime;
pub mod secrets;
pub mod window_registry;

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

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
        mut locator: SessionLocator,
        _title: Option<String>,
        summary: runtime::ObservationSummary,
        origin: runtime::ObservationOrigin,
        event: StoveEvent,
    ) {
        if locator.working_directory.is_none()
            && std::path::Path::new(&project.canonical_root).is_absolute()
        {
            locator.working_directory = Some(project.canonical_root.clone());
        }
        if locator.native_session_id.is_empty() {
            locator.native_session_id = identity.native_session_id.clone();
        }
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
        )
        .with_source_modified_at_ms(summary.source_modified_at_ms);
        let state = self.app.state::<app_state::AppState>();
        let _ = match origin {
            runtime::ObservationOrigin::Replay => state.apply_replay_observation_and_emit(
                &self.app,
                identity,
                project,
                app_state::LocatorCapability::Available,
                Some(locator),
                Some(summary),
                event,
            ),
            runtime::ObservationOrigin::Live => state.apply_observation_and_emit(
                &self.app,
                identity,
                project,
                app_state::LocatorCapability::Available,
                Some(locator),
                Some(summary),
                event,
            ),
        };
    }
}

pub(crate) struct LocalRuntimeState(Mutex<Option<runtime::RuntimeHandle>>);

impl LocalRuntimeState {
    pub(crate) fn add_pinned_path(&self, path: std::path::PathBuf) -> bool {
        self.0
            .lock()
            .expect("local runtime lock poisoned")
            .as_ref()
            .is_some_and(|runtime| runtime.add_pinned_path(path))
    }

    pub(crate) fn refresh_path(&self, path: std::path::PathBuf) -> bool {
        self.0
            .lock()
            .expect("local runtime lock poisoned")
            .as_ref()
            .is_some_and(|runtime| runtime.refresh_path(path))
    }
}
struct HookRuntimeState(Mutex<Option<hook_spool::HookSpoolHandle>>);
struct ExpiryRuntimeState(Mutex<Option<ExpiryRuntimeHandle>>);

struct ExpiryRuntimeHandle {
    cancelled: Arc<AtomicBool>,
    join: Option<thread::JoinHandle<()>>,
}

impl ExpiryRuntimeHandle {
    fn cancel(mut self) {
        self.cancelled.store(true, Ordering::Release);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn start_expiry_runtime(app: tauri::AppHandle) -> ExpiryRuntimeHandle {
    let cancelled = Arc::new(AtomicBool::new(false));
    let thread_cancelled = Arc::clone(&cancelled);
    let join = thread::spawn(move || {
        while !thread_cancelled.load(Ordering::Acquire) {
            for _ in 0..600 {
                if thread_cancelled.load(Ordering::Acquire) {
                    return;
                }
                thread::sleep(Duration::from_millis(100));
            }
            let state = app.state::<app_state::AppState>();
            let _ = state.reconcile_expired_and_emit(&app);
        }
    });
    ExpiryRuntimeHandle {
        cancelled,
        join: Some(join),
    }
}

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
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(app_state::AppState::default())
        .manage(notification_runtime)
        .manage(commands::notifications::LocalAlertCommandState(Arc::new(
            notifications::local::LocalAlertDispatcher::default(),
        )))
        .manage(remote::runtime::RemoteRuntimeState::default())
        .manage(runtime::LocalSourceStatusState::default())
        .manage(LocalRuntimeState(Mutex::new(None)))
        .manage(HookRuntimeState(Mutex::new(None)))
        .manage(ExpiryRuntimeState(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            commands::stoves::get_stoves_snapshot,
            commands::stoves::clear_cooked_stove,
            commands::stoves::set_stove_pinned,
            commands::stoves::archive_stove,
            commands::stoves::get_archived_sessions,
            commands::stoves::restore_archived_session,
            commands::windows::detach_stove,
            commands::windows::clear_detached_stove,
            commands::windows::close_detached_bar,
            commands::windows::record_detached_stove_position,
            commands::windows::record_global_bar_size,
            commands::windows::set_global_bar_minimum_size,
            commands::display::get_display_settings,
            commands::display::configure_display_settings,
            commands::display::record_global_bar_position,
            commands::locator::activate_stove_locator,
            commands::notifications::open_notification_settings,
            commands::notifications::get_notification_settings,
            commands::notifications::get_local_notification_settings,
            commands::notifications::configure_local_notification_settings,
            commands::notifications::test_local_notification,
            commands::notifications::configure_notification_destination,
            commands::notifications::send_test_notification,
            commands::remote::get_remote_sources,
            commands::remote::configure_remote_source,
            commands::remote::remove_remote_source,
            commands::sources::get_local_source_status,
            commands::hooks::get_hook_status,
            commands::hooks::manage_hook,
            commands::desktop_shell::get_launch_at_login,
            commands::desktop_shell::set_launch_at_login,
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
                window_registry::WindowRegistry::new(
                    state.persisted_config().layout.global_bar_visible,
                ),
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
            let mut local_config =
                runtime::LocalObservationConfig::from_environment(HostIdentity::local("local"));
            local_config.pinned_local_paths = state.pinned_local_paths();
            let local_status = app
                .state::<runtime::LocalSourceStatusState>()
                .inner()
                .clone();
            local_status.configure(&local_config);
            let handle = runtime::start(local_config, observer, local_status);
            *app.state::<LocalRuntimeState>()
                .0
                .lock()
                .expect("local runtime lock poisoned") = Some(handle);
            *app.state::<ExpiryRuntimeState>()
                .0
                .lock()
                .expect("expiry runtime lock poisoned") =
                Some(start_expiry_runtime(app.handle().clone()));

            let hook_directory = app_data.join("hook-spool");
            if let Ok(spool) =
                hook_spool::HookSpool::create(hook_directory, HostIdentity::local("local"))
            {
                let hook_app = app.handle().clone();
                let consumer = Arc::new(move |observation: hook_spool::HookObservation| {
                    let state = hook_app.state::<app_state::AppState>();
                    // Hooks enrich authoritative native sessions; they never
                    // create Stoves on their own. This also prevents Codex
                    // child-agent notify events from bypassing adapter filters.
                    if !state.stoves.contains_identity(&observation.identity) {
                        return;
                    }
                    let locator = observation.locator;
                    let project = locator
                        .as_ref()
                        .and_then(|locator| locator.working_directory.as_ref())
                        .filter(|root| std::path::Path::new(root).is_absolute())
                        .map(|root| {
                            ProjectIdentity::new(observation.identity.host.clone(), root.clone())
                        })
                        .unwrap_or(observation.project);
                    let capability = if locator.is_some() {
                        app_state::LocatorCapability::Available
                    } else {
                        app_state::LocatorCapability::Unavailable
                    };
                    let _ = state.apply_observation_and_emit(
                        &hook_app,
                        observation.identity,
                        project,
                        capability,
                        locator,
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

            platform::publish_optional_gnome_snapshot(&state.stoves.snapshot());
            let layout = state.persisted_config().layout;
            if let Err(error) = commands::display::restore_global_bar_size(
                &app.handle().clone(),
                layout.global_bar_size,
            ) {
                eprintln!("Cookbench could not restore global Bar size: {error}");
            }
            if let Err(error) = commands::display::apply_global_bar_preferences(
                &app.handle().clone(),
                layout.global_bar_visible,
                layout.global_bar_placement,
                layout.global_bar_position.as_ref(),
            ) {
                eprintln!("Cookbench could not restore global Bar display preferences: {error}");
            }
            match desktop_shell::runtime::install(app) {
                Ok(Some(diagnostic)) => {
                    eprintln!("Cookbench desktop integration: {}", diagnostic.message);
                }
                Ok(None) => {}
                Err(_) => {
                    eprintln!(
                        "Cookbench desktop integration is unavailable; the Bar remains usable"
                    );
                }
            }
            Ok(())
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
            if let Some(handle) = app
                .state::<ExpiryRuntimeState>()
                .0
                .lock()
                .expect("expiry runtime lock poisoned")
                .take()
            {
                handle.cancel();
            }
        }
    });
}
