use crate::protocol::{ControlActionKind, InvocationContext, PermissionCategory, SupportStatus};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionMetadata {
    pub kind: ControlActionKind,
    pub name: &'static str,
    pub status: SupportStatus,
    pub permission_category: PermissionCategory,
    pub requires_authenticated_user: bool,
    pub logged_out_safe: bool,
    pub allowed_contexts: &'static [InvocationContext],
}

const OUTSIDE_WARP_ONLY: &[InvocationContext] = &[InvocationContext::OutsideWarp];
const NO_CONTEXTS: &[InvocationContext] = &[];

pub const ACTIONS: &[ActionMetadata] = &[
    ActionMetadata {
        kind: ControlActionKind::InstanceList,
        name: "instance.list",
        status: SupportStatus::Implemented,
        permission_category: PermissionCategory::ReadMetadata,
        requires_authenticated_user: false,
        logged_out_safe: true,
        allowed_contexts: OUTSIDE_WARP_ONLY,
    },
    ActionMetadata {
        kind: ControlActionKind::AppPing,
        name: "app.ping",
        status: SupportStatus::Implemented,
        permission_category: PermissionCategory::ReadMetadata,
        requires_authenticated_user: false,
        logged_out_safe: true,
        allowed_contexts: OUTSIDE_WARP_ONLY,
    },
    ActionMetadata {
        kind: ControlActionKind::AppVersion,
        name: "app.version",
        status: SupportStatus::Implemented,
        permission_category: PermissionCategory::ReadMetadata,
        requires_authenticated_user: false,
        logged_out_safe: true,
        allowed_contexts: OUTSIDE_WARP_ONLY,
    },
    ActionMetadata {
        kind: ControlActionKind::TabCreate,
        name: "tab.create",
        status: SupportStatus::Implemented,
        permission_category: PermissionCategory::MutateAppState,
        requires_authenticated_user: false,
        logged_out_safe: true,
        allowed_contexts: OUTSIDE_WARP_ONLY,
    },
    ActionMetadata {
        kind: ControlActionKind::WindowList,
        name: "window.list",
        status: SupportStatus::Stub,
        permission_category: PermissionCategory::ReadMetadata,
        requires_authenticated_user: false,
        logged_out_safe: true,
        allowed_contexts: NO_CONTEXTS,
    },
    ActionMetadata {
        kind: ControlActionKind::TabList,
        name: "tab.list",
        status: SupportStatus::Stub,
        permission_category: PermissionCategory::ReadMetadata,
        requires_authenticated_user: false,
        logged_out_safe: true,
        allowed_contexts: NO_CONTEXTS,
    },
    ActionMetadata {
        kind: ControlActionKind::PaneList,
        name: "pane.list",
        status: SupportStatus::Stub,
        permission_category: PermissionCategory::ReadMetadata,
        requires_authenticated_user: false,
        logged_out_safe: true,
        allowed_contexts: NO_CONTEXTS,
    },
    ActionMetadata {
        kind: ControlActionKind::SessionList,
        name: "session.list",
        status: SupportStatus::Stub,
        permission_category: PermissionCategory::ReadMetadata,
        requires_authenticated_user: false,
        logged_out_safe: true,
        allowed_contexts: NO_CONTEXTS,
    },
    ActionMetadata {
        kind: ControlActionKind::InputRun,
        name: "input.run",
        status: SupportStatus::Stub,
        permission_category: PermissionCategory::MutateUnderlyingData,
        requires_authenticated_user: true,
        logged_out_safe: false,
        allowed_contexts: NO_CONTEXTS,
    },
    ActionMetadata {
        kind: ControlActionKind::DriveObjectCreate,
        name: "drive.object.create",
        status: SupportStatus::Stub,
        permission_category: PermissionCategory::MutateUnderlyingData,
        requires_authenticated_user: true,
        logged_out_safe: false,
        allowed_contexts: NO_CONTEXTS,
    },
];

pub fn metadata_for(kind: ControlActionKind) -> Option<&'static ActionMetadata> {
    ACTIONS.iter().find(|metadata| metadata.kind == kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_create_is_logged_out_safe_app_state_mutation() {
        let metadata = metadata_for(ControlActionKind::TabCreate).unwrap();
        assert_eq!(metadata.status, SupportStatus::Implemented);
        assert_eq!(
            metadata.permission_category,
            PermissionCategory::MutateAppState
        );
        assert!(metadata.logged_out_safe);
        assert!(!metadata.requires_authenticated_user);
        assert_eq!(metadata.allowed_contexts, [InvocationContext::OutsideWarp]);
    }

    #[test]
    fn future_high_risk_actions_are_stubbed_and_require_authenticated_user() {
        let metadata = metadata_for(ControlActionKind::InputRun).unwrap();
        assert_eq!(metadata.status, SupportStatus::Stub);
        assert_eq!(
            metadata.permission_category,
            PermissionCategory::MutateUnderlyingData
        );
        assert!(metadata.requires_authenticated_user);
        assert!(metadata.allowed_contexts.is_empty());
    }
}
