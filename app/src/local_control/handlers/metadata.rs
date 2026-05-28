use local_control::protocol::{ControlActionKind, InstanceId, ResponseEnvelope, PROTOCOL_VERSION};
use serde_json::json;

pub fn ping(request_id: String, instance_id: InstanceId) -> ResponseEnvelope {
    ResponseEnvelope::ok(
        request_id,
        Some(instance_id),
        json!({
            "action": ControlActionKind::AppPing.name(),
            "status": "ok",
            "protocol_version": PROTOCOL_VERSION,
        }),
    )
}

pub fn version(request_id: String, instance_id: InstanceId) -> ResponseEnvelope {
    ResponseEnvelope::ok(
        request_id,
        Some(instance_id),
        json!({
            "action": ControlActionKind::AppVersion.name(),
            "protocol_version": PROTOCOL_VERSION,
            "channel": warp_core::channel::ChannelState::channel().to_string(),
            "app_version": warp_core::channel::ChannelState::app_version(),
        }),
    )
}
