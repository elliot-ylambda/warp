use local_control::auth::PermissionGrant;
use local_control::catalog::action_metadata;
use local_control::protocol::{ControlAction, ControlError, ErrorCode, ScopedCredential};
use settings::Setting as _;
use warpui::{AppContext, SingletonEntity};

use crate::settings::LocalControlSettings;

pub fn outside_warp_enabled(ctx: &AppContext) -> bool {
    *LocalControlSettings::as_ref(ctx).outside_warp_control_enabled
}

pub fn outside_warp_permissions(ctx: &AppContext) -> PermissionGrant {
    let settings = LocalControlSettings::as_ref(ctx)
        .outside_warp_permissions
        .value()
        .to_owned();
    PermissionGrant {
        metadata_read: settings.metadata_read,
        underlying_data_read: settings.underlying_data_read,
        app_state_mutation: settings.app_state_mutation,
        metadata_configuration_mutation: settings.metadata_configuration_mutation,
        underlying_data_mutation: settings.underlying_data_mutation,
    }
}

pub fn verify_request_policy(
    action: &ControlAction,
    credential: &ScopedCredential,
    ctx: &AppContext,
) -> Result<(), ControlError> {
    if !outside_warp_enabled(ctx) {
        return Err(ControlError::new(
            ErrorCode::LocalControlDisabled,
            "outside-Warp local control is disabled",
        ));
    }
    let metadata = action_metadata(action);
    if credential.action != *action || credential.permission != metadata.permission {
        return Err(ControlError::new(
            ErrorCode::InsufficientPermissions,
            "credential does not authorize the requested action",
        ));
    }
    if !outside_warp_permissions(ctx).allows(&metadata.permission) {
        return Err(ControlError::new(
            ErrorCode::InsufficientPermissions,
            "outside-Warp local-control permission is disabled for this action category",
        ));
    }
    if metadata.requires_authenticated_user && !credential.requires_authenticated_user {
        return Err(ControlError::new(
            ErrorCode::AuthenticatedUserRequired,
            "authenticated-user local-control grants are not implemented in the foundation slice",
        ));
    }
    Ok(())
}
