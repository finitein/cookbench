/// The stable menu identifiers passed to Tauri's tray handler.
pub const SHOW_BAR_MENU_ID: &str = "show-bar";
pub const HIDE_BAR_MENU_ID: &str = "hide-bar";
pub const OPEN_SETTINGS_MENU_ID: &str = "open-settings";
pub const QUIT_MENU_ID: &str = "quit";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrayAction {
    ShowBar,
    HideBar,
    OpenSettings,
    Quit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TrayMenuItem {
    pub id: &'static str,
    pub title: &'static str,
    pub action: TrayAction,
}

pub const fn tray_menu() -> [TrayMenuItem; 4] {
    [
        TrayMenuItem {
            id: SHOW_BAR_MENU_ID,
            title: "Show Bar",
            action: TrayAction::ShowBar,
        },
        TrayMenuItem {
            id: HIDE_BAR_MENU_ID,
            title: "Hide Bar",
            action: TrayAction::HideBar,
        },
        TrayMenuItem {
            id: OPEN_SETTINGS_MENU_ID,
            title: "Open Settings",
            action: TrayAction::OpenSettings,
        },
        TrayMenuItem {
            id: QUIT_MENU_ID,
            title: "Quit",
            action: TrayAction::Quit,
        },
    ]
}

pub fn tray_action(id: &str) -> Option<TrayAction> {
    tray_menu()
        .into_iter()
        .find_map(|item| (item.id == id).then_some(item.action))
}

/// A platform-neutral accelerator understood by Tauri's official plugin.
pub const fn default_toggle_shortcut() -> &'static str {
    "CommandOrControl+Shift+B"
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopShellDiagnostic {
    pub code: &'static str,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShortcutPlan {
    Register { shortcut: &'static str },
    Unavailable(DesktopShellDiagnostic),
}

/// Registration failures are intentionally non-fatal: another application may
/// already own the accelerator. Cookbench remains reachable from its tray.
pub fn shortcut_plan(registration_result: Result<(), impl std::fmt::Display>) -> ShortcutPlan {
    match registration_result {
        Ok(()) => ShortcutPlan::Register {
            shortcut: default_toggle_shortcut(),
        },
        Err(_error) => ShortcutPlan::Unavailable(DesktopShellDiagnostic {
            code: "globalShortcutUnavailable",
            message: format!(
                "Cookbench could not reserve {}. Use the tray menu to show or hide the Bar.",
                default_toggle_shortcut()
            ),
        }),
    }
}

pub const fn default_autostart_enabled() -> bool {
    false
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AutostartTransition {
    Enable,
    Disable,
    NoChange,
}

pub const fn autostart_transition(
    currently_enabled: bool,
    requested_enabled: bool,
) -> AutostartTransition {
    match (currently_enabled, requested_enabled) {
        (false, true) => AutostartTransition::Enable,
        (true, false) => AutostartTransition::Disable,
        _ => AutostartTransition::NoChange,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_menu_has_only_cookbench_presentation_actions() {
        assert_eq!(
            tray_menu().map(|item| item.id),
            [
                SHOW_BAR_MENU_ID,
                HIDE_BAR_MENU_ID,
                OPEN_SETTINGS_MENU_ID,
                QUIT_MENU_ID
            ]
        );
        assert_eq!(
            tray_action(OPEN_SETTINGS_MENU_ID),
            Some(TrayAction::OpenSettings)
        );
        assert_eq!(tray_action("unknown"), None);
    }

    #[test]
    fn shortcut_conflict_is_a_nonfatal_tray_fallback() {
        let plan = shortcut_plan(Err("reserved by another application"));
        assert!(matches!(plan, ShortcutPlan::Unavailable(_)));
        let ShortcutPlan::Unavailable(diagnostic) = plan else {
            unreachable!()
        };
        assert_eq!(diagnostic.code, "globalShortcutUnavailable");
        assert!(diagnostic.message.contains("tray menu"));
    }

    #[test]
    fn shortcut_success_requests_the_default_toggle_once() {
        assert_eq!(
            shortcut_plan(Ok::<(), &str>(())),
            ShortcutPlan::Register {
                shortcut: "CommandOrControl+Shift+B"
            }
        );
    }

    #[test]
    fn autostart_is_opt_in_and_only_changes_when_requested_state_differs() {
        assert!(!default_autostart_enabled());
        assert_eq!(
            autostart_transition(false, false),
            AutostartTransition::NoChange
        );
        assert_eq!(
            autostart_transition(false, true),
            AutostartTransition::Enable
        );
        assert_eq!(
            autostart_transition(true, false),
            AutostartTransition::Disable
        );
    }
}
