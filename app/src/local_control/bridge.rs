use local_control::protocol::{ControlActionKind, InstanceId, RequestEnvelope, ResponseEnvelope};
use warpui::{Entity, ModelContext, SingletonEntity};

use crate::local_control::handlers::{layout, metadata};
use crate::local_control::permissions;

pub struct LocalControlBridge {
    instance_id: InstanceId,
}

impl LocalControlBridge {
    pub fn new(_: &mut ModelContext<Self>) -> Self {
        Self {
            instance_id: InstanceId(uuid::Uuid::new_v4().to_string()),
        }
    }

    pub fn instance_id(&self) -> InstanceId {
        self.instance_id.clone()
    }

    pub fn handle_request(
        &mut self,
        request: RequestEnvelope,
        credential: local_control::auth::ScopedCredential,
        ctx: &mut ModelContext<Self>,
    ) -> ResponseEnvelope {
        if let Err(code) = permissions::verify_request(ctx, &credential, request.action) {
            return ResponseEnvelope::error(
                request.request_id,
                code,
                "local-control request denied",
            );
        }
        match request.action {
            ControlActionKind::AppPing => {
                metadata::ping(request.request_id, self.instance_id.clone())
            }
            ControlActionKind::AppVersion => {
                metadata::version(request.request_id, self.instance_id.clone())
            }
            ControlActionKind::TabCreate => {
                layout::tab_create(request, self.instance_id.clone(), ctx)
            }
            _ => ResponseEnvelope::error(
                request.request_id,
                local_control::protocol::ErrorCode::UnsupportedAction,
                "action is not implemented in the foundation slice",
            ),
        }
    }
}

impl Entity for LocalControlBridge {
    type Event = ();
}

impl SingletonEntity for LocalControlBridge {}
