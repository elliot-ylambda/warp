use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use uuid::Uuid;

use crate::catalog::action_metadata;
use crate::protocol::{
    ControlAction, ControlError, CredentialRequest, ErrorCode, ExecutionContextProof,
    InvocationContext, PermissionCategory, ScopedCredential,
};

#[derive(Clone, Debug, Default)]
pub struct PermissionGrant {
    pub metadata_read: bool,
    pub underlying_data_read: bool,
    pub app_state_mutation: bool,
    pub metadata_configuration_mutation: bool,
    pub underlying_data_mutation: bool,
}

impl PermissionGrant {
    pub fn allows(&self, permission: &PermissionCategory) -> bool {
        match permission {
            PermissionCategory::MetadataRead => self.metadata_read,
            PermissionCategory::UnderlyingDataRead => self.underlying_data_read,
            PermissionCategory::AppStateMutation => self.app_state_mutation,
            PermissionCategory::MetadataConfigurationMutation => {
                self.metadata_configuration_mutation
            }
            PermissionCategory::UnderlyingDataMutation => self.underlying_data_mutation,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CredentialIssuer {
    outside_warp_enabled: bool,
    permissions: PermissionGrant,
    credentials: HashMap<String, ScopedCredential>,
    lifetime: Duration,
}

impl CredentialIssuer {
    pub fn new(outside_warp_enabled: bool, permissions: PermissionGrant) -> Self {
        Self {
            outside_warp_enabled,
            permissions,
            credentials: HashMap::new(),
            lifetime: Duration::from_secs(60),
        }
    }

    pub fn set_policy(&mut self, outside_warp_enabled: bool, permissions: PermissionGrant) {
        self.outside_warp_enabled = outside_warp_enabled;
        self.permissions = permissions;
    }

    pub fn issue(&mut self, request: &CredentialRequest) -> Result<ScopedCredential, ControlError> {
        if request.invocation_context == InvocationContext::InsideWarp {
            return Err(ControlError::new(
                ErrorCode::ExecutionContextNotAllowed,
                "inside-Warp local-control credentials are reserved for future verified terminal proof support",
            ));
        }
        if request.proof != ExecutionContextProof::None {
            return Err(ControlError::new(
                ErrorCode::ExecutionContextNotAllowed,
                "execution context proof is not accepted for outside-Warp foundation requests",
            ));
        }
        if !self.outside_warp_enabled {
            return Err(ControlError::new(
                ErrorCode::LocalControlDisabled,
                "outside-Warp local control is disabled",
            ));
        }

        let metadata = action_metadata(&request.action);
        if !metadata
            .allowed_invocation_contexts
            .contains(&request.invocation_context)
        {
            return Err(ControlError::new(
                ErrorCode::ExecutionContextNotAllowed,
                "action is not allowed from the requested invocation context",
            ));
        }
        if !self.permissions.allows(&metadata.permission) {
            return Err(ControlError::new(
                ErrorCode::InsufficientPermissions,
                "outside-Warp local control permission is disabled for this action category",
            ));
        }
        if metadata.requires_authenticated_user {
            return Err(ControlError::new(
                ErrorCode::AuthenticatedUserRequired,
                "authenticated-user local-control grants are not implemented in the foundation slice",
            ));
        }

        let credential = ScopedCredential {
            token: Uuid::new_v4().to_string(),
            action: request.action.clone(),
            permission: metadata.permission,
            invocation_context: request.invocation_context.clone(),
            expires_at_unix_ms: now_unix_ms() + self.lifetime.as_millis() as u64,
            requires_authenticated_user: metadata.requires_authenticated_user,
        };
        self.credentials
            .insert(credential.token.clone(), credential.clone());
        Ok(credential)
    }

    pub fn verify(
        &self,
        token: &str,
        action: &ControlAction,
        required_permission: &PermissionCategory,
    ) -> Result<ScopedCredential, ControlError> {
        let credential = self.credentials.get(token).ok_or_else(|| {
            ControlError::new(
                ErrorCode::UnauthorizedLocalClient,
                "missing or unknown local-control credential",
            )
        })?;
        if credential.expires_at_unix_ms <= now_unix_ms() {
            return Err(ControlError::new(
                ErrorCode::UnauthorizedLocalClient,
                "local-control credential has expired",
            ));
        }
        if &credential.action != action {
            return Err(ControlError::new(
                ErrorCode::InsufficientPermissions,
                "local-control credential was issued for a different action",
            ));
        }
        if &credential.permission != required_permission {
            return Err(ControlError::new(
                ErrorCode::InsufficientPermissions,
                "local-control credential lacks the required permission category",
            ));
        }
        Ok(credential.clone())
    }

    pub fn insert_for_tests(&mut self, credential: ScopedCredential) {
        self.credentials
            .insert(credential.token.clone(), credential);
    }
}

pub fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{CredentialRequest, PROTOCOL_VERSION};

    fn grant_all() -> PermissionGrant {
        PermissionGrant {
            metadata_read: true,
            underlying_data_read: true,
            app_state_mutation: true,
            metadata_configuration_mutation: true,
            underlying_data_mutation: true,
        }
    }

    #[test]
    fn rejects_inside_warp_credential_request() {
        let mut issuer = CredentialIssuer::new(true, grant_all());
        let request = CredentialRequest {
            protocol_version: PROTOCOL_VERSION,
            request_id: "r".to_owned(),
            action: ControlAction::AppPing,
            invocation_context: InvocationContext::InsideWarp,
            proof: ExecutionContextProof::VerifiedWarpTerminal {
                proof_id: "reserved".to_owned(),
            },
        };
        let error = issuer.issue(&request).unwrap_err();
        assert_eq!(error.code, ErrorCode::ExecutionContextNotAllowed);
    }

    #[test]
    fn disabled_outside_warp_control_rejects_credentials() {
        let mut issuer = CredentialIssuer::new(false, grant_all());
        let error = issuer
            .issue(&CredentialRequest::outside_warp(ControlAction::AppPing))
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::LocalControlDisabled);
    }

    #[test]
    fn disabled_granular_permission_rejects_credentials() {
        let mut issuer = CredentialIssuer::new(true, PermissionGrant::default());
        let error = issuer
            .issue(&CredentialRequest::outside_warp(ControlAction::TabCreate))
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::InsufficientPermissions);
    }

    #[test]
    fn verifies_valid_and_wrong_action_credentials() {
        let mut issuer = CredentialIssuer::new(true, grant_all());
        let credential = issuer
            .issue(&CredentialRequest::outside_warp(ControlAction::TabCreate))
            .unwrap();
        assert!(
            issuer
                .verify(
                    &credential.token,
                    &ControlAction::TabCreate,
                    &PermissionCategory::AppStateMutation,
                )
                .is_ok()
        );
        let error = issuer
            .verify(
                &credential.token,
                &ControlAction::AppPing,
                &PermissionCategory::MetadataRead,
            )
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::InsufficientPermissions);
    }

    #[test]
    fn rejects_expired_credentials() {
        let mut issuer = CredentialIssuer::new(true, grant_all());
        issuer.insert_for_tests(ScopedCredential {
            token: "expired".to_owned(),
            action: ControlAction::AppPing,
            permission: PermissionCategory::MetadataRead,
            invocation_context: InvocationContext::OutsideWarp,
            expires_at_unix_ms: 1,
            requires_authenticated_user: false,
        });
        let error = issuer
            .verify(
                "expired",
                &ControlAction::AppPing,
                &PermissionCategory::MetadataRead,
            )
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::UnauthorizedLocalClient);
    }
}
