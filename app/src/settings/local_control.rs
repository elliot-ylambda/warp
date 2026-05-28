use settings::macros::define_settings_group;
use settings::{SupportedPlatforms, SyncToCloud};

use local_control::protocol::PermissionCategory;

define_settings_group!(LocalControlSettings, settings: [
    outside_warp_control_enabled: OutsideWarpControlEnabled {
        type: bool,
        default: false,
        supported_platforms: SupportedPlatforms::DESKTOP,
        sync_to_cloud: SyncToCloud::Never,
        private: true,
        storage_key: "LocalControlOutsideWarpEnabled",
    },
    outside_warp_metadata_reads_enabled: OutsideWarpMetadataReadsEnabled {
        type: bool,
        default: false,
        supported_platforms: SupportedPlatforms::DESKTOP,
        sync_to_cloud: SyncToCloud::Never,
        private: true,
        storage_key: "LocalControlOutsideWarpMetadataReadsEnabled",
    },
    outside_warp_underlying_data_reads_enabled: OutsideWarpUnderlyingDataReadsEnabled {
        type: bool,
        default: false,
        supported_platforms: SupportedPlatforms::DESKTOP,
        sync_to_cloud: SyncToCloud::Never,
        private: true,
        storage_key: "LocalControlOutsideWarpUnderlyingDataReadsEnabled",
    },
    outside_warp_app_state_mutations_enabled: OutsideWarpAppStateMutationsEnabled {
        type: bool,
        default: false,
        supported_platforms: SupportedPlatforms::DESKTOP,
        sync_to_cloud: SyncToCloud::Never,
        private: true,
        storage_key: "LocalControlOutsideWarpAppStateMutationsEnabled",
    },
    outside_warp_metadata_mutations_enabled: OutsideWarpMetadataMutationsEnabled {
        type: bool,
        default: false,
        supported_platforms: SupportedPlatforms::DESKTOP,
        sync_to_cloud: SyncToCloud::Never,
        private: true,
        storage_key: "LocalControlOutsideWarpMetadataMutationsEnabled",
    },
    outside_warp_underlying_data_mutations_enabled: OutsideWarpUnderlyingDataMutationsEnabled {
        type: bool,
        default: false,
        supported_platforms: SupportedPlatforms::DESKTOP,
        sync_to_cloud: SyncToCloud::Never,
        private: true,
        storage_key: "LocalControlOutsideWarpUnderlyingDataMutationsEnabled",
    }
]);

impl LocalControlSettings {
    pub fn outside_warp_permission_enabled(&self, category: PermissionCategory) -> bool {
        if !*self.outside_warp_control_enabled {
            return false;
        }
        match category {
            PermissionCategory::ReadMetadata => *self.outside_warp_metadata_reads_enabled,
            PermissionCategory::ReadUnderlyingData => {
                *self.outside_warp_underlying_data_reads_enabled
            }
            PermissionCategory::MutateAppState => *self.outside_warp_app_state_mutations_enabled,
            PermissionCategory::MutateMetadata => *self.outside_warp_metadata_mutations_enabled,
            PermissionCategory::MutateUnderlyingData => {
                *self.outside_warp_underlying_data_mutations_enabled
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use settings::{PrivatePreferences, PublicPreferences, Setting as _, SettingsManager};
    use warpui::SingletonEntity as _;
    use warpui_extras::user_preferences;

    use super::*;

    fn init_test_app(ctx: &mut warpui::AppContext) {
        ctx.add_singleton_model(move |_| {
            PublicPreferences::new(
                Box::<user_preferences::in_memory::InMemoryPreferences>::default(),
            )
        });
        ctx.add_singleton_model(move |_| -> PrivatePreferences {
            PrivatePreferences::new(
                Box::<user_preferences::in_memory::InMemoryPreferences>::default(),
            )
        });
        ctx.add_singleton_model(|_| SettingsManager::default());
        LocalControlSettings::register(ctx);
    }

    #[test]
    fn defaults_are_disabled_and_private_local_only() {
        warpui::App::test((), |mut app| async move {
            app.update(init_test_app);
            app.read(|ctx| {
                let settings = LocalControlSettings::as_ref(ctx);
                assert!(!*settings.outside_warp_control_enabled);
                assert!(!settings.outside_warp_permission_enabled(PermissionCategory::ReadMetadata));

                assert!(OutsideWarpControlEnabled::is_private());
                assert_eq!(OutsideWarpControlEnabled::sync_to_cloud(), SyncToCloud::Never);
                assert_eq!(OutsideWarpControlEnabled::toml_path(), None);
                assert!(OutsideWarpAppStateMutationsEnabled::is_private());
                assert_eq!(OutsideWarpAppStateMutationsEnabled::sync_to_cloud(), SyncToCloud::Never);
                assert_eq!(OutsideWarpAppStateMutationsEnabled::toml_path(), None);
            });
        });
    }

    #[test]
    fn top_level_gate_controls_granular_permissions() {
        warpui::App::test((), |mut app| async move {
            app.update(init_test_app);
            app.update(|ctx| {
                LocalControlSettings::handle(ctx).update(ctx, |settings, ctx| {
                    settings
                        .outside_warp_metadata_reads_enabled
                        .set_value(true, ctx)
                        .unwrap();
                });
            });
            app.read(|ctx| {
                let settings = LocalControlSettings::as_ref(ctx);
                assert!(!settings.outside_warp_permission_enabled(PermissionCategory::ReadMetadata));
            });
            app.update(|ctx| {
                LocalControlSettings::handle(ctx).update(ctx, |settings, ctx| {
                    settings
                        .outside_warp_control_enabled
                        .set_value(true, ctx)
                        .unwrap();
                });
            });
            app.read(|ctx| {
                let settings = LocalControlSettings::as_ref(ctx);
                assert!(settings.outside_warp_permission_enabled(PermissionCategory::ReadMetadata));
                assert!(
                    !settings.outside_warp_permission_enabled(PermissionCategory::MutateAppState)
                );
            });
        });
    }
}
