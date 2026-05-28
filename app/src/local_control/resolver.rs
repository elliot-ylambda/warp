use local_control::protocol::{ControlError, ErrorCode, TargetSelector};
use warpui::AppContext;

pub fn resolve_active_window(ctx: &AppContext) -> Result<warpui::WindowId, ControlError> {
    ctx.windows().active_window().ok_or_else(|| {
        ControlError::new(
            ErrorCode::MissingTarget,
            "tab.create requires an active Warp window",
        )
    })
}

pub fn validate_tab_create_target(target: &TargetSelector) -> Result<(), ControlError> {
    local_control::selectors::validate_foundation_tab_create_target(target)
}
