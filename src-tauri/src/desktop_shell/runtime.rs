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
    stoves: &[crate::app_state::StoveWire],
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
    let Some(tray) = app.tray_by_id("cookbench") else {
        return Ok(());
    };
    tray.set_menu(Some(build_menu(app, locale, &status_menu_stoves(app))?))
}

fn status_menu_items<R: Runtime>(
    app: &AppHandle<R>,
    stoves: &[crate::app_state::StoveWire],
) -> tauri::Result<Vec<MenuItem<R>>> {
    #[cfg(target_os = "macos")]
    {
        stoves
            .iter()
            .enumerate()
            .map(|(index, stove)| {
                MenuItem::with_id(
                    app,
                    format!("status-stove-{index}"),
                    safe_status_label(stove),
                    false,
                    None::<&str>,
                )
            })
            .collect()
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, stoves);
        Ok(Vec::new())
    }
}

fn status_menu_stoves<R: Runtime>(app: &AppHandle<R>) -> Vec<crate::app_state::StoveWire> {
    #[cfg(target_os = "macos")]
    {
        let state = app.state::<crate::app_state::AppState>();
        let snapshot = state.snapshot();
        let count = state.persisted_config().layout.mac_status_stove_count;
        status_stoves_for(&snapshot, count)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Vec::new()
    }
}

#[cfg(target_os = "macos")]
fn status_stoves_for(
    snapshot: &crate::app_state::StoveSnapshot,
    requested_count: u8,
) -> Vec<crate::app_state::StoveWire> {
    snapshot
        .attention_order
        .iter()
        .filter_map(|id| snapshot.stoves.iter().find(|stove| stove.id == *id))
        .take(usize::from(requested_count.min(8)))
        .cloned()
        .collect()
}

#[cfg(target_os = "macos")]
fn safe_status_label(stove: &crate::app_state::StoveWire) -> String {
    let state = match stove.state {
        crate::app_state::StoveStateWire::Starting => "Starting",
        crate::app_state::StoveStateWire::Planning => "Planning",
        crate::app_state::StoveStateWire::Cooking => "Cooking",
        crate::app_state::StoveStateWire::NeedsHuman => "Needs Human",
        crate::app_state::StoveStateWire::Cooked => "Cooked",
        crate::app_state::StoveStateWire::Failed => "Failed",
        crate::app_state::StoveStateWire::Disconnected => "Disconnected",
    };
    let mut label = stove.project_label.chars().take(64).collect::<String>();
    if label.is_empty() {
        label = "Project".into();
    }
    format!("{state}: {label}")
}

/// Updates only the macOS combined status item. Other platforms retain the
/// static tray icon created by Tauri at startup.
pub fn refresh_status_stoves<R: Runtime>(app: &AppHandle<R>) {
    let snapshot = app.state::<crate::app_state::AppState>().snapshot();
    refresh_status_stoves_snapshot(app, &snapshot);
}

/// Refreshes from the caller's immutable snapshot so AppState can publish
/// while it owns its serialization lock without recursively taking it.
pub fn refresh_status_stoves_snapshot<R: Runtime>(
    app: &AppHandle<R>,
    snapshot: &crate::app_state::StoveSnapshot,
) {
    #[cfg(target_os = "macos")]
    {
        let Some(tray) = app.tray_by_id("cookbench") else {
            return;
        };
        let state = app.state::<crate::app_state::AppState>();
        let count = state.persisted_config().layout.mac_status_stove_count;
        let rendered = super::status_stoves::presentation(snapshot, count);
        let slots = rendered
            .as_ref()
            .map(|presentation| presentation.slots.clone());
        let icon_result = match rendered {
            Some(presentation) => tray.set_icon(Some(tauri::image::Image::new_owned(
                presentation.image.rgba,
                presentation.image.width,
                presentation.image.height,
            ))),
            None => tray.set_icon(app.default_window_icon().cloned()),
        };
        let status = app.state::<super::status_stoves::StatusStovesState>();
        if icon_result.is_ok() {
            status.replace_slots(slots.unwrap_or_default());
        } else {
            let _ = tray.set_icon(app.default_window_icon().cloned());
            status.clear();
        }
        let _ = tray.set_visible(count != 0);
        if let Ok(menu) = build_menu(
            app,
            state.persisted_config().preferences.locale,
            &status_stoves_for(snapshot, count),
        ) {
            let _ = tray.set_menu(Some(menu));
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
        let image_x = crate::platform::status_item_image_x(position, rect.position);
        let stove_id = app
            .state::<super::status_stoves::StatusStovesState>()
            .stove_at(image_x);
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
