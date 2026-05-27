use super::{LocalControlMode, LocalControlModeSetting, LocalControlSettings};
use settings::Setting as _;

fn default_settings() -> LocalControlSettings {
    LocalControlSettings {
        local_control_mode: LocalControlModeSetting::new(None),
    }
}

#[test]
fn defaults_disable_warp_control() {
    let settings = default_settings();

    assert_eq!(LocalControlMode::default(), LocalControlMode::Disabled);
    assert_eq!(settings.mode(), LocalControlMode::Disabled);
    assert!(!settings.inside_warp_control_enabled());
    assert!(!settings.outside_warp_control_enabled());
}
