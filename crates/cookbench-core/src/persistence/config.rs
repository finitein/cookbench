use serde::{Deserialize, Serialize};

use crate::domain::HarnessId;

use super::Versioned;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BarLayout {
    #[serde(default)]
    pub global_bar_visible: bool,
    #[serde(default)]
    pub detached_stoves: Vec<String>,
}

impl Default for BarLayout {
    fn default() -> Self {
        Self {
            global_bar_visible: true,
            detached_stoves: Vec::new(),
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
        }
    }
}

impl Versioned for PersistedConfig {
    const CURRENT_VERSION: u32 = Self::CURRENT_VERSION;

    fn version(&self) -> u32 {
        self.version
    }
}
