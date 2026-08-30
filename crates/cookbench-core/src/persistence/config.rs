use serde::{Deserialize, Serialize};

use crate::domain::HarnessId;
use crate::notifications::NotificationEventKind;

use super::Versioned;
use super::{DetachedStoveLayout, MonitorIdentity, RelativePosition};

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

/// A bounded width choice for the Global Bar. Presets keep the floating
/// surface readable and prevent a transparent native window from growing into
/// an accidental desktop-sized hit target.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GlobalBarSize {
    Compact,
    #[default]
    Standard,
    Wide,
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
    #[serde(default)]
    pub global_bar_placement: GlobalBarPlacement,
    #[serde(default)]
    pub global_bar_size: GlobalBarSize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub global_bar_position: Option<GlobalBarPosition>,
    #[serde(default)]
    pub detached_stoves: Vec<String>,
    #[serde(default)]
    pub detached_layouts: Vec<DetachedStoveLayout>,
}

const fn default_true() -> bool {
    true
}

impl Default for BarLayout {
    fn default() -> Self {
        Self {
            global_bar_visible: true,
            global_bar_placement: GlobalBarPlacement::default(),
            global_bar_size: GlobalBarSize::default(),
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
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            always_on_top: true,
            notifications_enabled: false,
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
