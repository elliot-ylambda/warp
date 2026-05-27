//! Private local setting that gates local-control invocation contexts.
//!
//! This setting is local-only, kept out of the user-visible settings file, and
//! marked `private: true` in the settings definition. It is the authoritative
//! enablement bit for local control.
use serde::{Deserialize, Serialize};
use settings::{macros::define_settings_group, SupportedPlatforms, SyncToCloud};

/// User-selected local-control availability.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    Eq,
    PartialEq,
    schemars::JsonSchema,
    Serialize,
    settings_value::SettingsValue,
)]
#[schemars(
    description = "Which local-control invocation contexts are allowed.",
    rename_all = "snake_case"
)]
pub enum LocalControlMode {
    #[default]
    Disabled,
    EnabledWithinWarp,
    EnabledEverywhere,
}

impl LocalControlMode {
    pub const ALL: [Self; 3] = [
        Self::Disabled,
        Self::EnabledWithinWarp,
        Self::EnabledEverywhere,
    ];

    pub fn allows_inside_warp(self) -> bool {
        matches!(self, Self::EnabledWithinWarp | Self::EnabledEverywhere)
    }

    pub fn allows_outside_warp(self) -> bool {
        matches!(self, Self::EnabledEverywhere)
    }

    pub fn as_dropdown_label(self) -> &'static str {
        match self {
            Self::Disabled => "Disabled",
            Self::EnabledWithinWarp => "Enabled within Warp",
            Self::EnabledEverywhere => "Enabled everywhere",
        }
    }
}

define_settings_group!(LocalControlSettings, settings: [
    local_control_mode: LocalControlModeSetting {
        type: LocalControlMode,
        default: LocalControlMode::Disabled,
        supported_platforms: SupportedPlatforms::DESKTOP,
        sync_to_cloud: SyncToCloud::Never,
        private: true,
        storage_key: "LocalControlMode",
        description: "Whether Warp local control is disabled, enabled within Warp, or enabled everywhere including outside Warp.",
    },
]);

impl LocalControlSettings {
    pub fn mode(&self) -> LocalControlMode {
        *self.local_control_mode
    }

    pub fn inside_warp_control_enabled(&self) -> bool {
        self.mode().allows_inside_warp()
    }

    pub fn outside_warp_control_enabled(&self) -> bool {
        self.mode().allows_outside_warp()
    }
}

#[cfg(test)]
#[path = "local_control_tests.rs"]
mod tests;
