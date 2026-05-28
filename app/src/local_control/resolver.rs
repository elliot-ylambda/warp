use local_control::protocol::{ErrorCode, TargetSelector};

pub fn validate_tab_create_target(target: &TargetSelector) -> Result<(), ErrorCode> {
    local_control::selectors::validate_tab_create_target(target)
}
