use std::collections::BTreeSet;

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

use crate::domain::HarnessId;
use crate::notifications::NotificationEventKind;

use super::Versioned;
use super::{DetachedStoveLayout, MonitorIdentity, RelativePosition, WindowSize};

/// The interface language selected by the user. `System` keeps first launch
/// lightweight while explicit choices remain stable across every window.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AppLocale {
    #[default]
    System,
    En,
    #[serde(rename = "zh-CN")]
    ZhCn,
    Ja,
    Ko,
}

/// The global Bar's screen-relative anchor. Detached Bars retain their own
/// monitor-relative positions independently of this preference.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GlobalBarPlacement {
    TopLeft,
    #[default]
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

/// The global Bar presentation density. Minimal mode deliberately remains a
/// display preference; it does not alter the observed Stove lifecycle.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GlobalBarMode {
    #[default]
    Full,
    Minimal,
}

pub const MAX_MAC_STATUS_STOVE_COUNT: u8 = 8;

const fn default_mac_status_stove_count() -> u8 {
    3
}

fn deserialize_mac_status_stove_count<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: Deserializer<'de>,
{
    let count = i64::deserialize(deserializer)?;
    Ok(count.clamp(0, i64::from(MAX_MAC_STATUS_STOVE_COUNT)) as u8)
}

/// The last user-dragged global Bar position, relative to a monitor work area.
/// It takes precedence over the placement anchor on restore and contains no
/// session, project, or harness data.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GlobalBarPosition {
    pub monitor: MonitorIdentity,
    pub relative_position: RelativePosition,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BarLayout {
    #[serde(default = "default_true")]
    pub global_bar_visible: bool,
    /// Detailed Stove metadata stays collapsed unless the user explicitly
    /// opts in. This keeps the overlay quiet on first launch.
    #[serde(default)]
    pub hover_details_enabled: bool,
    #[serde(default)]
    pub global_bar_placement: GlobalBarPlacement,
    #[serde(default)]
    pub global_bar_mode: GlobalBarMode,
    #[serde(
        default = "default_mac_status_stove_count",
        deserialize_with = "deserialize_mac_status_stove_count"
    )]
    pub mac_status_stove_count: u8,
    /// The last deliberate native window size. It is unset until the user
    /// resizes the Bar, keeping legacy installs on their platform default.
    #[serde(
        default,
        deserialize_with = "deserialize_global_bar_size",
        skip_serializing_if = "Option::is_none"
    )]
    pub global_bar_size: Option<WindowSize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub global_bar_position: Option<GlobalBarPosition>,
    #[serde(default)]
    pub detached_stoves: Vec<String>,
    #[serde(default)]
    pub detached_layouts: Vec<DetachedStoveLayout>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum PersistedGlobalBarSize {
    Freeform(WindowSize),
    LegacyPreset(String),
}

fn deserialize_global_bar_size<'de, D>(deserializer: D) -> Result<Option<WindowSize>, D::Error>
where
    D: Deserializer<'de>,
{
    let size = match Option::<PersistedGlobalBarSize>::deserialize(deserializer)? {
        Some(PersistedGlobalBarSize::Freeform(size)) => Some(size),
        Some(PersistedGlobalBarSize::LegacyPreset(preset))
            if matches!(preset.as_str(), "compact" | "standard" | "wide") =>
        {
            None
        }
        Some(PersistedGlobalBarSize::LegacyPreset(_)) => {
            return Err(de::Error::custom("unknown legacy global Bar size preset"));
        }
        None => None,
    };
    Ok(size)
}

const fn default_true() -> bool {
    true
}

const LOCAL_NOTIFICATION_EVENT_LIMIT: usize = 10;

fn default_local_notification_events() -> Vec<NotificationEventKind> {
    vec![
        NotificationEventKind::NeedsHuman,
        NotificationEventKind::Cooked,
        NotificationEventKind::Failed,
        NotificationEventKind::Disconnected,
    ]
}

fn normalize_local_notification_events(
    events: impl IntoIterator<Item = NotificationEventKind>,
) -> Vec<NotificationEventKind> {
    events
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .take(LOCAL_NOTIFICATION_EVENT_LIMIT)
        .collect()
}

fn deserialize_local_notification_events<'de, D>(
    deserializer: D,
) -> Result<Vec<NotificationEventKind>, D::Error>
where
    D: Deserializer<'de>,
{
    Vec::<NotificationEventKind>::deserialize(deserializer).map(normalize_local_notification_events)
}

fn serialize_local_notification_events<S>(
    events: &[NotificationEventKind],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    normalize_local_notification_events(events.iter().copied()).serialize(serializer)
}

impl Default for BarLayout {
    fn default() -> Self {
        Self {
            global_bar_visible: true,
            hover_details_enabled: false,
            global_bar_placement: GlobalBarPlacement::default(),
            global_bar_mode: GlobalBarMode::default(),
            mac_status_stove_count: default_mac_status_stove_count(),
            global_bar_size: None,
            global_bar_position: None,
            detached_stoves: Vec::new(),
            detached_layouts: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UserPreferences {
    #[serde(default)]
    pub always_on_top: bool,
    #[serde(default)]
    pub notifications_enabled: bool,
    #[serde(default)]
    pub locale: AppLocale,
    #[serde(default)]
    pub local_notifications: LocalNotificationPreferences,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            always_on_top: true,
            notifications_enabled: false,
            locale: AppLocale::default(),
            local_notifications: LocalNotificationPreferences::default(),
        }
    }
}

/// User-selected channels for local-only desktop alerts. These preferences
/// carry no Agent or session content and are intentionally independent from
/// outbound destination notifications.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LocalNotificationPreferences {
    #[serde(default = "default_true")]
    pub sound: bool,
    #[serde(default)]
    pub system_banner: bool,
    #[serde(default)]
    pub bar_flash: bool,
    #[serde(default)]
    pub system_attention: bool,
    #[serde(
        default = "default_local_notification_events",
        deserialize_with = "deserialize_local_notification_events",
        serialize_with = "serialize_local_notification_events"
    )]
    pub events: Vec<NotificationEventKind>,
}

impl LocalNotificationPreferences {
    pub const MAX_EVENTS: usize = LOCAL_NOTIFICATION_EVENT_LIMIT;

    pub fn normalize_events(
        events: impl IntoIterator<Item = NotificationEventKind>,
    ) -> Vec<NotificationEventKind> {
        normalize_local_notification_events(events)
    }
}

impl Default for LocalNotificationPreferences {
    fn default() -> Self {
        Self {
            sound: true,
            system_banner: false,
            bar_flash: false,
            system_attention: false,
            events: default_local_notification_events(),
        }
    }
}

/// A reference into an OS credential store. No secret material belongs in JSON.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CredentialReference {
    pub provider: String,
    pub account_id: String,
    pub secret_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NotificationDestinationConfig {
    pub id: String,
    pub provider: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub recipient: Option<String>,
    #[serde(default)]
    pub events: Vec<NotificationEventKind>,
    #[serde(default)]
    pub template: Option<String>,
    pub credential: CredentialReference,
}

/// A read-only OpenSSH source. Authentication remains entirely in the user's
/// existing SSH configuration and known_hosts files.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RemoteSourceConfig {
    pub id: String,
    pub alias: String,
    #[serde(default)]
    pub session_roots: Vec<String>,
    #[serde(default)]
    pub enabled: bool,
    /// Uses the explicitly selected, checksum-verified SSH stdio bridge rather
    /// than zero-install polling. The bridge is never enabled implicitly.
    #[serde(default)]
    pub bridge_enabled: bool,
    /// Optional path to a user-selected bridge binary for a different remote
    /// platform. When absent, the packaged same-platform sidecar is used.
    #[serde(default)]
    pub bridge_binary_path: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PersistedConfig {
    pub version: u32,
    #[serde(default)]
    pub layout: BarLayout,
    #[serde(default)]
    pub enabled_harnesses: Vec<HarnessId>,
    #[serde(default)]
    pub preferences: UserPreferences,
    #[serde(default)]
    pub credential_references: Vec<CredentialReference>,
    #[serde(default)]
    pub notification_destinations: Vec<NotificationDestinationConfig>,
    #[serde(default)]
    pub remote_sources: Vec<RemoteSourceConfig>,
}

impl PersistedConfig {
    pub const CURRENT_VERSION: u32 = 1;
}

impl Default for PersistedConfig {
    fn default() -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            layout: BarLayout::default(),
            enabled_harnesses: Vec::new(),
            preferences: UserPreferences::default(),
            credential_references: Vec::new(),
            notification_destinations: Vec::new(),
            remote_sources: Vec::new(),
        }
    }
}

impl Versioned for PersistedConfig {
    const CURRENT_VERSION: u32 = Self::CURRENT_VERSION;

    fn version(&self) -> u32 {
        self.version
    }
}
