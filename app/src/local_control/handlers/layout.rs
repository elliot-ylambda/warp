use local_control::protocol::{ControlError, ErrorCode, RequestEnvelope, ResponseEnvelope};
use serde_json::{json, Value};
use warpui::{ModelContext, TypedActionView};

use crate::workspace::{Workspace, WorkspaceAction};

pub fn tab_create(
    request: RequestEnvelope,
    ctx: &mut ModelContext<super::super::LocalControlServer>,
) -> ResponseEnvelope {
    if request.params != Value::Object(Default::default()) {
        return ResponseEnvelope::error(
            request.request_id,
            ControlError::new(
                ErrorCode::InvalidParams,
                "tab.create does not accept params in the foundation slice",
            ),
        );
    }
    if let Err(error) = super::super::resolver::validate_tab_create_target(&request.target) {
        return ResponseEnvelope::error(request.request_id, error);
    }
    let window_id = match super::super::resolver::resolve_active_window(ctx) {
        Ok(window_id) => window_id,
        Err(error) => return ResponseEnvelope::error(request.request_id, error),
    };

    let mut previous_tab_count = None;
    let mut current_tab_count = None;
    let mut active_tab_index = None;
    let mut dispatched = false;
    let Some(workspace_handles) = ctx.views_of_type::<Workspace>(window_id) else {
        return ResponseEnvelope::error(
            request.request_id,
            ControlError::new(
                ErrorCode::MissingTarget,
                "active window does not contain a workspace",
            ),
        );
    };

    if let Some(workspace_handle) = workspace_handles.into_iter().next() {
        workspace_handle.update(ctx, |workspace, ctx| {
            previous_tab_count = Some(workspace.tab_count());
            workspace.handle_action(
                &WorkspaceAction::AddTerminalTab {
                    hide_homepage: false,
                },
                ctx,
            );
            current_tab_count = Some(workspace.tab_count());
            active_tab_index = Some(workspace.active_tab_index());
            dispatched = true;
        });
    }

    if !dispatched {
        return ResponseEnvelope::error(
            request.request_id,
            ControlError::new(
                ErrorCode::MissingTarget,
                "active window does not contain a workspace",
            ),
        );
    }

    ResponseEnvelope::ok(
        request.request_id,
        None,
        json!({
            "action": "tab.create",
            "active_window_id": format!("{:?}", window_id),
            "previous_tab_count": previous_tab_count,
            "current_tab_count": current_tab_count,
            "active_tab_index": active_tab_index,
        }),
    )
}
