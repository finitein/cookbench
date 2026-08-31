//! Tauri 2 desktop-shell integration. Call `install` after the official
//! autostart and global-shortcut plugins have been registered on the builder.

use cookbench_core::persistence::AppLocale;
use tauri::{
    menu::{IsMenuItem, Menu, MenuItem},
    tray::TrayIconBuilder,
    App, AppHandle, Manager, Runtime,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use super::{default_toggle_shortcut, tray_action, tray_menu, DesktopShellDiagnostic, TrayAction};

pub fn install(app: &App, locale: AppLocale) -> tauri::Result<Option<DesktopShellDiagnostic>> {
    let menu = build_menu(app.handle(), locale)?;
    TrayIconBuilder::with_id("cookbench")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|handle, event| dispatch_tray_action(handle, event.id().as_ref()))
        .build(app)?;

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

fn build_menu<R: Runtime>(app: &AppHandle<R>, locale: AppLocale) -> tauri::Result<Menu<R>> {
    let items = tray_menu(locale)
        .into_iter()
        .map(|item| MenuItem::with_id(app, item.id, item.title, true, None::<&str>))
        .collect::<tauri::Result<Vec<_>>>()?;
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
    tray.set_menu(Some(build_menu(app, locale)?))
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
    window.set_focus()?;
    Ok(())
}

pub fn hide_bar<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    app.get_webview_window("main")
        .ok_or(tauri::Error::WindowNotFound)?
        .hide()
}

pub fn toggle_bar<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let window = app
        .get_webview_window("main")
        .ok_or(tauri::Error::WindowNotFound)?;
    if window.is_visible()? {
        window.hide()
    } else {
        window.show()?;
        window.set_focus()
    }
}
