//! Tauri commands for outbound notification configuration and synthetic tests.
//! Secret values cross IPC only on an explicit save and are written directly
//! to the native credential store; replies and JSON configuration never echo
//! them.

use std::{collections::BTreeSet, sync::Arc};

use cookbench_core::{
    notifications::{
        DestinationId, NotificationEventKind, NotificationRule, NotificationSettings, RuleScope,
        Template,
    },
    persistence::{
        CredentialReference, LocalNotificationPreferences, NotificationDestinationConfig,
        PersistedConfig,
    },
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::{
    app_state::AppState,
    notifications::{
        local::{
            LocalAlertChannel, LocalAlertDispatcher, LocalAlertPayload, LocalAlertResult,
            TauriLocalAlertEffects,
        },
        sender::{DestinationKind, ReqwestTransport},
        service::{DestinationConfiguration, NotificationService},
    },
    secrets::{NativeSecretStore, SecretReference},
};

pub type NotificationRuntime = NotificationService<ReqwestTransport, NativeSecretStore>;

pub struct NotificationCommandState(pub Arc<NotificationRuntime>);

pub struct LocalAlertCommandState(pub Arc<LocalAlertDispatcher>);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NotificationEventWire {
    SessionAppeared,
    CookingStarted,
    PhaseChanged,
    NeedsHuman,
    ProgressMilestone,
    Cooked,
    Failed,
    Disconnected,
    ConnectionRestored,
    StoveCleared,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationDestinationWire {
    pub destination: String,
    pub enabled: bool,
    pub configured: bool,
    pub recipient: Option<String>,
    pub events: Vec<NotificationEventWire>,
    pub template: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationDestinationInput {
    pub destination: String,
    pub enabled: bool,
    pub secret: Option<String>,
    pub recipient: Option<String>,
    pub events: Vec<NotificationEventWire>,
    pub template: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalNotificationSettingsWire {
    pub sound: bool,
    pub system_banner: bool,
    pub bar_flash: bool,
    pub system_attention: bool,
    pub events: Vec<NotificationEventWire>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalNotificationInput {
    pub sound: bool,
    pub system_banner: bool,
    pub bar_flash: bool,
    pub system_attention: bool,
    pub events: Vec<NotificationEventWire>,
}

/// Label for the Settings WebviewWindow. Frontend uses `label === "settings"`.
pub const SETTINGS_WINDOW_LABEL: &str = "settings";

/// Same app entry as the main window (`WebviewUrl::default()` → `index.html`).
/// Keeping this explicit documents the Windows custom-protocol path contract.
pub fn settings_webview_url() -> WebviewUrl {
    WebviewUrl::App("index.html".into())
}

fn settings_window_needs_reload(url: &str) -> bool {
    // A prior sync/event-handler create on Windows/WebView2 can leave the
    // shell at about:blank; reopen must navigate/recreate so the panel mounts.
    url == "about:blank" || url.starts_with("about:")
}

/// Ensure the Settings window exists and shows the shared frontend.
///
/// Must not run on the Windows UI/event thread while holding it: Tauri/WebView2
/// deadlocks when `WebviewWindowBuilder::build` is called from a synchronous
/// command or tray/event handler. Callers must use the async command or spawn
/// a background thread (see tray Open Settings).
pub fn ensure_settings_window(app: &AppHandle) -> Result<(), String> {
    let locale = app.state::<crate::i18n::NativeLocaleState>().current();
    let title = crate::i18n::settings_window_title(locale);

    if let Some(window) = app.get_webview_window(SETTINGS_WINDOW_LABEL) {
        let blank = window
            .url()
            .map(|url| settings_window_needs_reload(url.as_str()))
            .unwrap_or(true);
        if blank {
            // Destroy the blank shell so we can recreate with a real App URL.
            window.destroy().map_err(|error| error.to_string())?;
        } else {
            let _ = window.set_title(title);
            window
                .set_always_on_top(true)
                .map_err(|error| error.to_string())?;
            window.show().map_err(|error| error.to_string())?;
            let _ = window.set_focus();
            return Ok(());
        }
    }

    WebviewWindowBuilder::new(app, SETTINGS_WINDOW_LABEL, settings_webview_url())
        .title(title)
        .inner_size(620.0, 720.0)
        .min_inner_size(420.0, 520.0)
        .resizable(true)
        .always_on_top(true)
        .build()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

/// Open Settings from the frontend. Async so Windows/WebView2 does not deadlock
/// the way a synchronous command would when building a secondary webview.
#[tauri::command]
pub async fn open_notification_settings(app: AppHandle) -> Result<(), String> {
    ensure_settings_window(&app)
}

/// Open Settings from tray/menu handlers without blocking the UI thread.
pub fn open_settings_window_off_thread(app: AppHandle) {
    std::thread::spawn(move || {
        if let Err(error) = ensure_settings_window(&app) {
            eprintln!("Cookbench failed to open Settings window: {error}");
        }
    });
}

#[tauri::command]
pub fn get_notification_settings(state: State<'_, AppState>) -> Vec<NotificationDestinationWire> {
    settings_wire(&state.persisted_config())
}

#[tauri::command]
pub fn get_local_notification_settings(
    state: State<'_, AppState>,
) -> LocalNotificationSettingsWire {
    local_notification_settings_wire(&state.persisted_config())
}

#[tauri::command]
pub fn configure_local_notification_settings(
    input: LocalNotificationInput,
    app: AppHandle,
    app_state: State<'_, AppState>,
) -> Result<LocalNotificationSettingsWire, String> {
    let current = app_state.persisted_config();
    if input.system_banner && !current.preferences.local_notifications.system_banner {
        let effects = TauriLocalAlertEffects::new(&app);
        match effects.request_banner_permission() {
            LocalAlertResult::Delivered | LocalAlertResult::Queued => {}
            LocalAlertResult::PermissionDenied => {
                return Err("System notifications are not allowed".to_owned());
            }
            LocalAlertResult::Unavailable => {
                return Err("System notifications are unavailable".to_owned());
            }
        }
    }

    let mut updated = current;
    apply_local_notification_input(&mut updated, input)?;
    app_state.update_persisted_config(|config| *config = updated.clone())?;
    Ok(local_notification_settings_wire(&updated))
}

#[tauri::command]
pub fn test_local_notification(
    channel: LocalAlertChannel,
    app: AppHandle,
    state: State<'_, LocalAlertCommandState>,
    app_state: State<'_, AppState>,
    native_locale: State<'_, crate::i18n::NativeLocaleState>,
) -> LocalAlertResult {
    let effects = TauriLocalAlertEffects::new(&app);
    if channel == LocalAlertChannel::SystemBanner {
        let permission = effects.request_banner_permission();
        if permission != LocalAlertResult::Delivered {
            return permission;
        }
    }
    let snapshot = app_state.stoves.snapshot();
    let (stove_id, project) = snapshot
        .stoves
        .first()
        .map(|stove| (stove.id.as_str(), stove.project_label.as_str()))
        .unwrap_or(("__cookbench_test__", "Cookbench"));
    let locale = native_locale.current();
    state.0.test_channel(
        channel,
        &LocalAlertPayload::new(stove_id, project, NotificationEventKind::Cooked)
            .with_locale(locale),
        &effects,
    )
}

#[tauri::command]
pub fn configure_notification_destination(
    input: NotificationDestinationInput,
    app_state: State<'_, AppState>,
    notifications: State<'_, NotificationCommandState>,
) -> Result<Vec<NotificationDestinationWire>, String> {
    let (kind, id) = parse_destination(&input.destination)?;
    let template = input
        .template
        .as_ref()
        .filter(|value| !value.is_empty())
        .map(|value| {
            if value.len() > 1_024 || value.chars().any(char::is_control) {
                return Err("invalid notification template".to_owned());
            }
            Template::parse(value.clone())
                .map(|_| value.clone())
                .map_err(|_| "invalid notification template".to_owned())
        })
        .transpose()?;
    if input.events.len() > 10
        || input
            .recipient
            .as_ref()
            .is_some_and(|value| value.len() > 256 || value.chars().any(char::is_control))
    {
        return Err("invalid notification destination settings".to_owned());
    }
    if kind == DestinationKind::Telegram
        && input.enabled
        && input.recipient.as_deref().unwrap_or_default().is_empty()
    {
        return Err("Telegram requires a chat ID".to_owned());
    }

    let secret_reference = SecretReference::new("Cookbench", format!("notification-{id}"))
        .map_err(|error| error.to_string())?;
    let credential = CredentialReference {
        provider: "native".to_owned(),
        account_id: secret_reference.account().to_owned(),
        secret_id: secret_reference.redacted(),
    };
    let supplied_secret = input.secret.as_deref().filter(|value| !value.is_empty());
    if let Some(secret) = supplied_secret {
        if secret.len() > 4_096 || secret.chars().any(|character| character == '\0') {
            return Err("notification secret is invalid".to_owned());
        }
        notifications
            .0
            .set_secret(&secret_reference, secret)
            .map_err(|error| error.to_string())?;
    }

    let mut updated = app_state.persisted_config();
    let already_configured = updated
        .credential_references
        .iter()
        .any(|reference| reference.secret_id == credential.secret_id);
    if input.enabled && supplied_secret.is_none() && !already_configured {
        return Err("save a destination credential before enabling notifications".to_owned());
    }
    updated
        .notification_destinations
        .retain(|candidate| candidate.id != id);
    updated
        .notification_destinations
        .push(NotificationDestinationConfig {
            id: id.to_owned(),
            provider: provider_name(kind).to_owned(),
            enabled: input.enabled,
            recipient: input.recipient,
            events: input.events.into_iter().map(Into::into).collect(),
            template,
            credential: credential.clone(),
        });
    if supplied_secret.is_some() && !already_configured {
        updated.credential_references.push(credential);
    }
    app_state.update_persisted_config(|config| *config = updated.clone())?;
    configure_runtime(&updated, &notifications.0)?;
    Ok(settings_wire(&updated))
}

#[tauri::command]
pub fn send_test_notification(
    state: State<'_, NotificationCommandState>,
    destination: String,
) -> Result<(), String> {
    let (_, destination) = parse_destination(&destination)?;
    state
        .0
        .send_test(&DestinationId::new(destination))
        .map_err(|error| error.to_string())
}

pub fn configure_runtime(
    config: &PersistedConfig,
    runtime: &NotificationRuntime,
) -> Result<(), String> {
    let mut settings = NotificationSettings {
        global: NotificationRule::enabled_for(BTreeSet::new()),
        rules: Vec::new(),
    };
    let mut destinations = Vec::new();
    for configured in config.notification_destinations.iter().take(16) {
        let Ok((kind, id)) = parse_destination(&configured.id) else {
            continue;
        };
        let Ok(secret) = SecretReference::new("Cookbench", format!("notification-{id}")) else {
            continue;
        };
        let mut rule = NotificationRule::for_scope(RuleScope::Destination(DestinationId::new(id)))
            .with_enabled(configured.enabled)
            .with_events(configured.events.iter().copied());
        if let Some(template) = configured
            .template
            .as_ref()
            .and_then(|source| Template::parse(source.clone()).ok())
        {
            rule = rule.with_template(template);
        }
        settings.rules.push(rule);
        destinations.push(DestinationConfiguration {
            id: DestinationId::new(id),
            kind,
            enabled: configured.enabled,
            secret,
            recipient: configured.recipient.clone(),
        });
    }
    runtime.configure(settings, destinations);
    Ok(())
}

fn settings_wire(config: &PersistedConfig) -> Vec<NotificationDestinationWire> {
    ["telegram", "slack", "discord", "lark", "generic"]
        .into_iter()
        .map(|id| {
            let configured = config
                .notification_destinations
                .iter()
                .find(|candidate| candidate.id == id);
            let has_credential = configured.is_some_and(|candidate| {
                config
                    .credential_references
                    .iter()
                    .any(|reference| reference.secret_id == candidate.credential.secret_id)
            });
            NotificationDestinationWire {
                destination: id.to_owned(),
                enabled: configured.is_some_and(|candidate| candidate.enabled),
                configured: has_credential,
                recipient: configured.and_then(|candidate| candidate.recipient.clone()),
                events: configured
                    .map(|candidate| {
                        candidate
                            .events
                            .iter()
                            .filter_map(|event| NotificationEventWire::try_from(*event).ok())
                            .collect()
                    })
                    .unwrap_or_else(default_events),
                template: configured.and_then(|candidate| candidate.template.clone()),
            }
        })
        .collect()
}

pub fn local_notification_settings_wire(config: &PersistedConfig) -> LocalNotificationSettingsWire {
    let preferences = &config.preferences.local_notifications;
    LocalNotificationSettingsWire {
        sound: preferences.sound,
        system_banner: preferences.system_banner,
        bar_flash: preferences.bar_flash,
        system_attention: preferences.system_attention,
        events: preferences
            .events
            .iter()
            .filter_map(|event| NotificationEventWire::try_from(*event).ok())
            .collect(),
    }
}

pub fn apply_local_notification_input(
    config: &mut PersistedConfig,
    input: LocalNotificationInput,
) -> Result<(), String> {
    if input.events.len() > LocalNotificationPreferences::MAX_EVENTS {
        return Err("Too many local notification events".to_owned());
    }
    config.preferences.local_notifications = LocalNotificationPreferences {
        sound: input.sound,
        system_banner: input.system_banner,
        bar_flash: input.bar_flash,
        system_attention: input.system_attention,
        events: LocalNotificationPreferences::normalize_events(
            input.events.into_iter().map(Into::into),
        ),
    };
    Ok(())
}

fn parse_destination(value: &str) -> Result<(DestinationKind, &'static str), String> {
    match value {
        "telegram" => Ok((DestinationKind::Telegram, "telegram")),
        "slack" => Ok((DestinationKind::Slack, "slack")),
        "discord" => Ok((DestinationKind::Discord, "discord")),
        "lark" => Ok((DestinationKind::Lark, "lark")),
        "generic" => Ok((DestinationKind::Generic, "generic")),
        _ => Err("invalid notification destination".to_owned()),
    }
}

fn provider_name(kind: DestinationKind) -> &'static str {
    match kind {
        DestinationKind::Telegram => "telegram",
        DestinationKind::Slack => "slack",
        DestinationKind::Discord => "discord",
        DestinationKind::Lark => "lark",
        DestinationKind::Generic => "generic",
    }
}

fn default_events() -> Vec<NotificationEventWire> {
    vec![
        NotificationEventWire::NeedsHuman,
        NotificationEventWire::Cooked,
        NotificationEventWire::Failed,
        NotificationEventWire::Disconnected,
    ]
}

impl From<NotificationEventWire> for NotificationEventKind {
    fn from(value: NotificationEventWire) -> Self {
        match value {
            NotificationEventWire::SessionAppeared => Self::SessionAppeared,
            NotificationEventWire::CookingStarted => Self::CookingStarted,
            NotificationEventWire::PhaseChanged => Self::PhaseChanged,
            NotificationEventWire::NeedsHuman => Self::NeedsHuman,
            NotificationEventWire::ProgressMilestone => Self::ProgressMilestone,
            NotificationEventWire::Cooked => Self::Cooked,
            NotificationEventWire::Failed => Self::Failed,
            NotificationEventWire::Disconnected => Self::Disconnected,
            NotificationEventWire::ConnectionRestored => Self::ConnectionRestored,
            NotificationEventWire::StoveCleared => Self::StoveCleared,
        }
    }
}

impl TryFrom<NotificationEventKind> for NotificationEventWire {
    type Error = ();

    fn try_from(value: NotificationEventKind) -> Result<Self, Self::Error> {
        match value {
            NotificationEventKind::SessionAppeared => Ok(Self::SessionAppeared),
            NotificationEventKind::CookingStarted => Ok(Self::CookingStarted),
            NotificationEventKind::PhaseChanged => Ok(Self::PhaseChanged),
            NotificationEventKind::NeedsHuman => Ok(Self::NeedsHuman),
            NotificationEventKind::ProgressMilestone => Ok(Self::ProgressMilestone),
            NotificationEventKind::Cooked => Ok(Self::Cooked),
            NotificationEventKind::Failed => Ok(Self::Failed),
            NotificationEventKind::Disconnected => Ok(Self::Disconnected),
            NotificationEventKind::ConnectionRestored => Ok(Self::ConnectionRestored),
            NotificationEventKind::StoveCleared => Ok(Self::StoveCleared),
        }
    }
}

#[cfg(test)]
mod settings_window_tests {
    use super::{settings_webview_url, settings_window_needs_reload, SETTINGS_WINDOW_LABEL};
    use tauri::WebviewUrl;

    #[test]
    fn settings_label_matches_frontend_detached_window_contract() {
        assert_eq!(SETTINGS_WINDOW_LABEL, "settings");
    }

    #[test]
    fn settings_url_matches_default_app_entry() {
        assert!(matches!(
            settings_webview_url(),
            WebviewUrl::App(path) if path.to_str() == Some("index.html")
        ));
        assert!(matches!(
            WebviewUrl::default(),
            WebviewUrl::App(path) if path.to_str() == Some("index.html")
        ));
    }

    #[test]
    fn blank_settings_shell_is_detected_for_reload() {
        assert!(settings_window_needs_reload("about:blank"));
        assert!(settings_window_needs_reload("about:srcdoc"));
        assert!(!settings_window_needs_reload("http://tauri.localhost/"));
        assert!(!settings_window_needs_reload(
            "https://tauri.localhost/index.html"
        ));
    }
}
