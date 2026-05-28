use local_control::protocol::{RequestEnvelope, ResponseEnvelope};
use serde_json::json;
use warp_core::channel::ChannelState;
use warpui::ModelContext;

pub fn ping(
    request: RequestEnvelope,
    _ctx: &mut ModelContext<super::super::LocalControlServer>,
) -> ResponseEnvelope {
    ResponseEnvelope::ok(request.request_id, None, json!({ "status": "ok" }))
}

pub fn version(
    request: RequestEnvelope,
    _ctx: &mut ModelContext<super::super::LocalControlServer>,
) -> ResponseEnvelope {
    ResponseEnvelope::ok(
        request.request_id,
        None,
        json!({
            "app_version": ChannelState::app_version(),
            "channel": format!("{:?}", ChannelState::channel()),
            "protocol_version": local_control::PROTOCOL_VERSION,
        }),
    )
}
