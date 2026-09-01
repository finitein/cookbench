//! Tauri 2 desktop-shell integration. Call `install` after the official
//! autostart and global-shortcut plugins have been registered on the builder.

use cookbench_core::persistence::AppLocale;
#[cfg(target_os = "macos")]
use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
use tauri::{
    menu::{IsMenuItem, Menu, MenuItem},
    tray::TrayIconBuilder,
    App, AppHandle, Manager, Runtime,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use super::{default_toggle_shortcut, tray_action, tray_menu, DesktopShellDiagnostic, TrayAction};

pub fn install(app: &App, locale: AppLocale) -> tauri::Result<Option<DesktopShellDiagnostic>> {
    let menu = build_menu(app.handle(), locale, &[])?;
    TrayIconBuilder::with_id("cookbench")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|handle, event| dispatch_tray_action(handle, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| handle_tray_icon_event(tray.app_handle(), event))
        .build(app)?;

    refresh_status_stoves(app.handle());

    match app
        .global_shortcut()
        .on_shortcut(default_toggle_shortcut(), |handle, _, event| {
            if event.state == ShortcutState::Pressed {
                let _ = toggle_bar(handle);
            }
        }) {
        Ok(()) => Ok(None),
        Err(_error) => Ok(Some(DesktopShellDiagnostic {
            code: "globalShortcutUnavailable",
            message: format!(
                "Cookbench could not reserve {}. Use the tray menu to show or hide the Bar.",
                default_toggle_shortcut()
            ),
        })),
    }
}

fn build_menu<R: Runtime>(
    app: &AppHandle<R>,
    locale: AppLocale,
    stoves: &[super::status_stoves::StatusMenuStove],
) -> tauri::Result<Menu<R>> {
    let mut items = status_menu_items(app, stoves)?;
    items.extend(
        tray_menu(locale)
            .into_iter()
            .map(|item| MenuItem::with_id(app, item.id, item.title, true, None::<&str>))
            .collect::<tauri::Result<Vec<_>>>()?,
    );
    let item_refs = items
        .iter()
        .map(|item| item as &dyn IsMenuItem<_>)
        .collect::<Vec<_>>();
    Menu::with_items(app, &item_refs)
}

pub fn update_menu<R: Runtime>(app: &AppHandle<R>, locale: AppLocale) -> tauri::Result<()> {
    #[cfg(target_os = "macos")]
    {
        let _ = locale;
        refresh_status_stoves(app);
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let Some(tray) = app.tray_by_id("cookbench") else {
            return Ok(());
        };
        tray.set_menu(Some(build_menu(app, locale, &[])?))
    }
}

fn status_menu_items<R: Runtime>(
    app: &AppHandle<R>,
    stoves: &[super::status_stoves::StatusMenuStove],
) -> tauri::Result<Vec<MenuItem<R>>> {
    #[cfg(target_os = "macos")]
    {
        stoves
            .iter()
            .map(|stove| MenuItem::with_id(app, &stove.menu_id, &stove.label, true, None::<&str>))
            .collect()
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, stoves);
        Ok(Vec::new())
    }
}

/// Updates only the macOS combined status item. Other platforms retain the
/// static tray icon created by Tauri at startup.
pub fn refresh_status_stoves<R: Runtime>(app: &AppHandle<R>) {
    let snapshot = app.state::<crate::app_state::AppState>().snapshot();
    refresh_status_stoves_snapshot(app, &snapshot);
}

/// Enqueues native work after the caller has returned to Tauri's event loop.
/// AppState invokes this while serializing an observation, so synchronously
/// calling menu APIs there could deadlock with a simultaneous status click.
pub fn queue_status_stoves_refresh<R: Runtime>(
    app: &AppHandle<R>,
    snapshot: crate::app_state::StoveSnapshot,
) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let callback_app = app.clone();
        let _ = app.run_on_main_thread(move || {
            refresh_status_stoves_snapshot(&callback_app, &snapshot);
        });
    });
}

/// Refreshes from the caller's immutable snapshot so AppState can publish
/// while it owns its serialization lock without recursively taking it.
pub fn refresh_status_stoves_snapshot<R: Runtime>(
    app: &AppHandle<R>,
    _snapshot: &crate::app_state::StoveSnapshot,
) {
    #[cfg(target_os = "macos")]
    {
        let Some(tray) = app.tray_by_id("cookbench") else {
            return;
        };
        let state = app.state::<crate::app_state::AppState>();
        let count = state.persisted_config().layout.mac_status_stove_count;
        let status = app.state::<super::status_stoves::StatusStovesState>();
        if !status.accepts_revision(_snapshot.revision) {
            return;
        }
        let rendered = status.presentation(_snapshot, count);
        let locale = app.state::<crate::i18n::NativeLocaleState>().current();
        let menu_stoves = super::status_stoves::all_stove_menu_for_locale(_snapshot, locale);
        let icon_result = match rendered {
            Some(ref presentation) => tray.set_icon(Some(tauri::image::Image::new_owned(
                presentation.image.rgba.clone(),
                presentation.image.width,
                presentation.image.height,
            ))),
            None => tray.set_icon(app.default_window_icon().cloned()),
        };
        if icon_result.is_ok() {
            status.commit_presentation(_snapshot.revision, rendered.as_ref());
        } else {
            let _ = tray.set_icon(app.default_window_icon().cloned());
            status.commit_presentation(_snapshot.revision, None);
        }
        let _ = tray.set_visible(true);
        let _ = tray.set_tooltip(Some(super::status_stoves::accessibility_label_for_locale(
            _snapshot, count, locale,
        )));
        if let Ok(menu) = build_menu(app, locale, &menu_stoves) {
            if tray.set_menu(Some(menu)).is_ok() {
                status.commit_menu(_snapshot.revision, &menu_stoves);
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
    }
}

#[cfg(target_os = "macos")]
fn handle_tray_icon_event(app: &AppHandle, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        position,
        rect,
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        let local_x = crate::platform::status_item_image_x(position, rect.position);
        let item_width = f64::from(rect.size.to_physical::<i32>(1.0).width);
        let stove_id = app
            .state::<super::status_stoves::StatusStovesState>()
            .stove_at_status_x(local_x, item_width);
        if let Some(stove_id) = stove_id {
            activate_status_stove(app.clone(), stove_id);
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn handle_tray_icon_event(_app: &AppHandle, _event: tauri::tray::TrayIconEvent) {}

#[cfg(target_os = "macos")]
fn activate_status_stove(app: AppHandle, stove_id: String) {
    // Refuse stale slots before acknowledgement or locator work. The native
    // status image may legitimately lag one event-loop turn behind a snapshot.
    if !app
        .state::<crate::app_state::AppState>()
        .snapshot()
        .stoves
        .iter()
        .any(|stove| stove.id == stove_id)
    {
        return;
    }
    let state = app.state::<crate::app_state::AppState>();
    let _ = state.acknowledge_cooked_and_emit(&app, &stove_id);
    tauri::async_runtime::spawn(async move {
        let _ = crate::commands::locator::activate_stove_locator(stove_id, app).await;
    });
}

fn dispatch_tray_action(app: &AppHandle, id: &str) {
    #[cfg(target_os = "macos")]
    if let Some(stove_id) = app
        .state::<super::status_stoves::StatusStovesState>()
        .menu_target(id)
    {
        activate_status_stove(app.clone(), stove_id);
        return;
    }
    match tray_action(id) {
        Some(TrayAction::ShowBar) => {
            let _ = show_bar(app);
        }
        Some(TrayAction::HideBar) => {
            let _ = hide_bar(app);
        }
        Some(TrayAction::OpenSettings) => {
            let _ = crate::commands::notifications::open_notification_settings(app.clone());
        }
        Some(TrayAction::Quit) => app.exit(0),
        None => (),
    }
}

pub fn show_bar<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let window = app
        .get_webview_window("main")
        .ok_or(tauri::Error::WindowNotFound)?;
    window.show()?;
    if let Some(runtime) = app.try_state::<crate::commands::windows::GlobalBarDockRuntime>() {
        crate::commands::windows::reveal_global_bar_dock(app, runtime.inner())
            .map_err(|error| tauri::Error::Anyhow(std::io::Error::other(error).into()))?;
    }
    window.set_focus()?;
    Ok(())
}

pub fn hide_bar<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    app.get_webview_window("main")
        .ok_or(tauri::Error::WindowNotFound)?
        .hide()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ToggleBarAction {
    Hide,
    ShowReveal,
    Reveal,
}

pub(crate) fn toggle_bar_action(visible: bool, collapsed: bool) -> ToggleBarAction {
    match (visible, collapsed) {
        (false, _) => ToggleBarAction::ShowReveal,
        (true, true) => ToggleBarAction::Reveal,
        (true, false) => ToggleBarAction::Hide,
    }
}

pub fn toggle_bar<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let window = app
        .get_webview_window("main")
        .ok_or(tauri::Error::WindowNotFound)?;
    let collapsed = app
        .try_state::<crate::commands::windows::GlobalBarDockRuntime>()
        .is_some_and(|runtime| runtime.state().collapsed);
    match toggle_bar_action(window.is_visible()?, collapsed) {
        ToggleBarAction::Hide => window.hide(),
        ToggleBarAction::ShowReveal | ToggleBarAction::Reveal => {
            window.show()?;
            if let Some(runtime) = app.try_state::<crate::commands::windows::GlobalBarDockRuntime>()
            {
                crate::commands::windows::reveal_global_bar_dock(app, runtime.inner())
                    .map_err(|error| tauri::Error::Anyhow(std::io::Error::other(error).into()))?;
            }
            window.set_focus()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_action_reveals_hidden_or_collapsed_bar_and_hides_only_expanded_bar() {
        assert_eq!(toggle_bar_action(false, false), ToggleBarAction::ShowReveal);
        assert_eq!(toggle_bar_action(false, true), ToggleBarAction::ShowReveal);
        assert_eq!(toggle_bar_action(true, true), ToggleBarAction::Reveal);
        assert_eq!(toggle_bar_action(true, false), ToggleBarAction::Hide);
    }
}
