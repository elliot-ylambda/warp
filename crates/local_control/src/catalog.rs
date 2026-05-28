use crate::protocol::{ActionSupportStatus, ControlAction, InvocationContext, PermissionCategory};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionMetadata {
    pub action: ControlAction,
    pub support: ActionSupportStatus,
    pub permission: PermissionCategory,
    pub requires_authenticated_user: bool,
    pub allowed_invocation_contexts: &'static [InvocationContext],
    pub summary: &'static str,
}

const OUTSIDE_WARP: &[InvocationContext] = &[InvocationContext::OutsideWarp];

pub fn action_metadata(action: &ControlAction) -> ActionMetadata {
    match action {
        ControlAction::InstanceList => ActionMetadata {
            action: action.clone(),
            support: ActionSupportStatus::Implemented,
            permission: PermissionCategory::MetadataRead,
            requires_authenticated_user: false,
            allowed_invocation_contexts: OUTSIDE_WARP,
            summary: "List locally discoverable Warp instances.",
        },
        ControlAction::AppPing => ActionMetadata {
            action: action.clone(),
            support: ActionSupportStatus::Implemented,
            permission: PermissionCategory::MetadataRead,
            requires_authenticated_user: false,
            allowed_invocation_contexts: OUTSIDE_WARP,
            summary: "Check whether the selected Warp instance is reachable.",
        },
        ControlAction::AppVersion => ActionMetadata {
            action: action.clone(),
            support: ActionSupportStatus::Implemented,
            permission: PermissionCategory::MetadataRead,
            requires_authenticated_user: false,
            allowed_invocation_contexts: OUTSIDE_WARP,
            summary: "Read version metadata from the selected Warp instance.",
        },
        ControlAction::TabCreate => ActionMetadata {
            action: action.clone(),
            support: ActionSupportStatus::Implemented,
            permission: PermissionCategory::AppStateMutation,
            requires_authenticated_user: false,
            allowed_invocation_contexts: OUTSIDE_WARP,
            summary: "Create a terminal tab in the active window.",
        },
        ControlAction::WindowList
        | ControlAction::TabList
        | ControlAction::PaneList
        | ControlAction::SessionList => ActionMetadata {
            action: action.clone(),
            support: ActionSupportStatus::Planned,
            permission: PermissionCategory::MetadataRead,
            requires_authenticated_user: false,
            allowed_invocation_contexts: OUTSIDE_WARP,
            summary: "Planned metadata action not implemented in the foundation slice.",
        },
    }
}

pub fn implemented_actions() -> Vec<ActionMetadata> {
    [
        ControlAction::InstanceList,
        ControlAction::AppPing,
        ControlAction::AppVersion,
        ControlAction::TabCreate,
    ]
    .into_iter()
    .map(|action| action_metadata(&action))
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_create_is_logged_out_safe_app_state_mutation() {
        let metadata = action_metadata(&ControlAction::TabCreate);
        assert_eq!(metadata.support, ActionSupportStatus::Implemented);
        assert_eq!(metadata.permission, PermissionCategory::AppStateMutation);
        assert!(!metadata.requires_authenticated_user);
        assert_eq!(metadata.allowed_invocation_contexts, OUTSIDE_WARP);
    }

    #[test]
    fn future_actions_are_planned_not_implemented() {
        let metadata = action_metadata(&ControlAction::WindowList);
        assert_eq!(metadata.support, ActionSupportStatus::Planned);
        assert!(
            !implemented_actions()
                .iter()
                .any(|m| m.action == ControlAction::WindowList)
        );
    }
}
