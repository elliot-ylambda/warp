use serde::{Deserialize, Serialize};
use settings::macros::define_settings_group;
use settings::{SupportedPlatforms, SyncToCloud};

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[schemars(
    description = "Private local Warp control settings.",
    rename_all = "snake_case"
)]
pub struct LocalControlPermissionSettings {
    pub metadata_read: bool,
    pub underlying_data_read: bool,
    pub app_state_mutation: bool,
    pub metadata_configuration_mutation: bool,
    pub underlying_data_mutation: bool,
}

define_settings_group!(LocalControlSettings, settings: [
    outside_warp_control_enabled: OutsideWarpControlEnabled {
        type: bool,
        default: false,
        supported_platforms: SupportedPlatforms::DESKTOP,
        sync_to_cloud: SyncToCloud::Never,
        private: true,
        storage_key: "LocalControlOutsideWarpControlEnabled",
    },
    outside_warp_permissions: OutsideWarpPermissions {
        type: LocalControlPermissionSettings,
        default: LocalControlPermissionSettings::default(),
        supported_platforms: SupportedPlatforms::DESKTOP,
        sync_to_cloud: SyncToCloud::Never,
        private: true,
        storage_key: "LocalControlOutsideWarpPermissions",
    }
]);

#[cfg(test)]
mod tests {
    use settings::Setting as _;

    use super::*;

    #[test]
    fn local_control_settings_default_fail_closed() {
        assert!(!OutsideWarpControlEnabled::default_value());
        let permissions = OutsideWarpPermissions::new(None);
        assert!(!permissions.value().metadata_read);
        assert!(!permissions.value().app_state_mutation);
    }

    #[test]
    fn local_control_settings_are_private_and_non_synced() {
        assert!(OutsideWarpControlEnabled::is_private());
        assert_eq!(
            OutsideWarpControlEnabled::sync_to_cloud(),
            SyncToCloud::Never
        );
        assert!(OutsideWarpPermissions::is_private());
        assert_eq!(OutsideWarpPermissions::sync_to_cloud(), SyncToCloud::Never);
    }
}
