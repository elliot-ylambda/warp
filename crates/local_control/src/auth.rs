use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::catalog::metadata_for;
use crate::protocol::{ControlActionKind, ErrorCode, InvocationContext, PermissionCategory};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopedCredential {
    pub id: String,
    pub instance_id: String,
    pub action: ControlActionKind,
    pub invocation_context: InvocationContext,
    pub grants: Vec<PermissionCategory>,
    pub authenticated_user: bool,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl ScopedCredential {
    pub fn issue(
        instance_id: impl Into<String>,
        action: ControlActionKind,
        invocation_context: InvocationContext,
        grants: Vec<PermissionCategory>,
        ttl: Duration,
    ) -> Self {
        let issued_at = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            instance_id: instance_id.into(),
            action,
            invocation_context,
            grants,
            authenticated_user: false,
            issued_at,
            expires_at: issued_at + ttl,
        }
    }

    pub fn verify(&self, action: ControlActionKind, now: DateTime<Utc>) -> Result<(), ErrorCode> {
        let metadata = metadata_for(action).ok_or(ErrorCode::UnsupportedAction)?;
        if self.action != action {
            return Err(ErrorCode::InsufficientPermissions);
        }
        if self.expires_at <= now {
            return Err(ErrorCode::UnauthorizedLocalClient);
        }
        if !metadata.allowed_contexts.contains(&self.invocation_context) {
            return Err(ErrorCode::ExecutionContextNotAllowed);
        }
        if metadata.requires_authenticated_user && !self.authenticated_user {
            return Err(ErrorCode::AuthenticatedUserRequired);
        }
        if !self.grants.contains(&metadata.permission_category) {
            return Err(ErrorCode::InsufficientPermissions);
        }
        Ok(())
    }
}

pub fn credential_request_allowed(context: InvocationContext) -> Result<(), ErrorCode> {
    match context {
        InvocationContext::OutsideWarp => Ok(()),
        InvocationContext::InsideWarp => Err(ErrorCode::ExecutionContextNotAllowed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::PermissionCategory;

    #[test]
    fn inside_warp_credential_requests_are_rejected() {
        assert_eq!(
            credential_request_allowed(InvocationContext::InsideWarp),
            Err(ErrorCode::ExecutionContextNotAllowed)
        );
    }

    #[test]
    fn verifies_valid_scoped_credential() {
        let credential = ScopedCredential::issue(
            "instance",
            ControlActionKind::TabCreate,
            InvocationContext::OutsideWarp,
            vec![PermissionCategory::MutateAppState],
            Duration::minutes(1),
        );
        assert_eq!(
            credential.verify(ControlActionKind::TabCreate, Utc::now()),
            Ok(())
        );
    }

    #[test]
    fn rejects_wrong_action_expired_and_insufficient_grants() {
        let credential = ScopedCredential::issue(
            "instance",
            ControlActionKind::TabCreate,
            InvocationContext::OutsideWarp,
            vec![PermissionCategory::ReadMetadata],
            Duration::minutes(1),
        );
        assert_eq!(
            credential.verify(ControlActionKind::AppPing, Utc::now()),
            Err(ErrorCode::InsufficientPermissions)
        );
        assert_eq!(
            credential.verify(ControlActionKind::TabCreate, Utc::now()),
            Err(ErrorCode::InsufficientPermissions)
        );

        let expired = ScopedCredential::issue(
            "instance",
            ControlActionKind::TabCreate,
            InvocationContext::OutsideWarp,
            vec![PermissionCategory::MutateAppState],
            Duration::seconds(-1),
        );
        assert_eq!(
            expired.verify(ControlActionKind::TabCreate, Utc::now()),
            Err(ErrorCode::UnauthorizedLocalClient)
        );
    }

    #[test]
    fn authenticated_user_required_for_stub_high_risk_actions() {
        let credential = ScopedCredential::issue(
            "instance",
            ControlActionKind::InputRun,
            InvocationContext::OutsideWarp,
            vec![PermissionCategory::MutateUnderlyingData],
            Duration::minutes(1),
        );
        assert_eq!(
            credential.verify(ControlActionKind::InputRun, Utc::now()),
            Err(ErrorCode::ExecutionContextNotAllowed)
        );
    }
}
