use super::organization_telemetry_policy;
use crate::features::FeatureFlag;
use crate::workspaces::workspace::{
    OrganizationTelemetryPolicy, TelemetryEnablementSetting as NativeTelemetryEnablementSetting,
};

#[test]
fn force_disabled_wins_over_force_enabled() {
    let _flag = FeatureFlag::EnterpriseTelemetryPolicy.override_enabled(true);
    assert_eq!(
        organization_telemetry_policy(true, true),
        OrganizationTelemetryPolicy::Enforced(NativeTelemetryEnablementSetting::Disabled)
    );
}

#[test]
fn force_enabled_enforces_enabled() {
    let _flag = FeatureFlag::EnterpriseTelemetryPolicy.override_enabled(true);
    assert_eq!(
        organization_telemetry_policy(true, false),
        OrganizationTelemetryPolicy::Enforced(NativeTelemetryEnablementSetting::Enabled)
    );
}

#[test]
fn neither_force_boolean_is_unmanaged() {
    let _flag = FeatureFlag::EnterpriseTelemetryPolicy.override_enabled(true);
    assert_eq!(
        organization_telemetry_policy(false, false),
        OrganizationTelemetryPolicy::Unmanaged
    );
}

#[test]
fn rollout_off_ignores_force_disabled_and_preserves_force_enabled() {
    let _flag = FeatureFlag::EnterpriseTelemetryPolicy.override_enabled(false);
    assert_eq!(
        organization_telemetry_policy(true, true),
        OrganizationTelemetryPolicy::Enforced(NativeTelemetryEnablementSetting::Enabled)
    );
    assert_eq!(
        organization_telemetry_policy(false, true),
        OrganizationTelemetryPolicy::Unmanaged
    );
}
