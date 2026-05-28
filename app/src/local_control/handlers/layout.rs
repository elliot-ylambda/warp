use local_control::protocol::{
    ControlActionKind, ErrorCode, InstanceId, RequestEnvelope, ResponseEnvelope,
};
use serde_json::json;
use warpui::{ModelContext, TypedActionView as _};

use crate::local_control::bridge::LocalControlBridge;
use crate::local_control::resolver;
use crate::workspace::view::Workspace;
use crate::workspace::WorkspaceAction;

pub fn tab_create(
    request: RequestEnvelope,
    instance_id: InstanceId,
    ctx: &mut ModelContext<LocalControlBridge>,
) -> ResponseEnvelope {
    if let Err(code) = resolver::validate_tab_create_target(&request.target) {
        return ResponseEnvelope::error(
            request.request_id,
            code,
            "unsupported target selector for tab.create foundation slice",
        );
    }
    if request
        .params
        .as_object()
        .is_some_and(|params| !params.is_empty())
    {
        return ResponseEnvelope::error(
            request.request_id,
            ErrorCode::InvalidParams,
            "tab.create does not accept params in the foundation slice",
        );
    }

    let Some(window_id) = ctx.windows().active_window() else {
        return ResponseEnvelope::error(
            request.request_id,
            ErrorCode::MissingTarget,
            "no active Warp window found",
        );
    };
    let Some(workspace_handle) = ctx
        .views_of_type::<Workspace>(window_id)
        .and_then(|views| views.into_iter().next())
    else {
        return ResponseEnvelope::error(
            request.request_id,
            ErrorCode::MissingTarget,
            "active window has no workspace",
        );
    };

    let (previous_tab_count, current_tab_count, active_tab_index) =
        workspace_handle.update(ctx, |workspace, ctx| {
            let (previous_tab_count, _) = workspace.local_control_tab_snapshot();
            workspace.handle_action(
                &WorkspaceAction::AddTerminalTab {
                    hide_homepage: false,
                },
                ctx,
            );
            let (current_tab_count, active_tab_index) = workspace.local_control_tab_snapshot();
            (previous_tab_count, current_tab_count, active_tab_index)
        });

    ResponseEnvelope::ok(
        request.request_id,
        Some(instance_id),
        json!({
            "action": ControlActionKind::TabCreate.name(),
            "active_window_id": format!("{window_id:?}"),
            "previous_tab_count": previous_tab_count,
            "current_tab_count": current_tab_count,
            "active_tab_index": active_tab_index,
        }),
    )
}
