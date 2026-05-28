use local_control::auth::CredentialIssuer;
use local_control::catalog::action_metadata;
use local_control::protocol::{
    ControlAction, ControlError, CredentialRequest, CredentialResponse, ErrorCode, RequestEnvelope,
    ResponseEnvelope, PROTOCOL_VERSION,
};
use serde_json::json;
use warpui::ModelContext;

use super::handlers::{layout, metadata};
use super::permissions;

pub struct LocalControlBridge {
    credential_issuer: CredentialIssuer,
}

impl LocalControlBridge {
    pub fn new(credential_issuer: CredentialIssuer) -> Self {
        Self { credential_issuer }
    }

    pub fn issue_credential(
        &mut self,
        request: CredentialRequest,
        ctx: &mut ModelContext<super::LocalControlServer>,
    ) -> CredentialResponse {
        self.refresh_policy(ctx);
        match self.credential_issuer.issue(&request) {
            Ok(credential) => CredentialResponse {
                protocol_version: PROTOCOL_VERSION,
                request_id: request.request_id,
                credential: Some(credential),
                error: None,
            },
            Err(error) => CredentialResponse {
                protocol_version: PROTOCOL_VERSION,
                request_id: request.request_id,
                credential: None,
                error: Some(error),
            },
        }
    }

    pub fn handle_request(
        &mut self,
        token: Option<String>,
        request: RequestEnvelope,
        ctx: &mut ModelContext<super::LocalControlServer>,
    ) -> ResponseEnvelope {
        self.refresh_policy(ctx);
        let metadata = action_metadata(&request.action);
        let credential = match token
            .ok_or_else(|| {
                ControlError::new(
                    ErrorCode::UnauthorizedLocalClient,
                    "missing bearer credential",
                )
            })
            .and_then(|token| {
                self.credential_issuer
                    .verify(&token, &request.action, &metadata.permission)
            })
            .and_then(|credential| {
                permissions::verify_request_policy(&request.action, &credential, ctx)?;
                Ok(credential)
            }) {
            Ok(credential) => credential,
            Err(error) => return ResponseEnvelope::error(request.request_id, error),
        };
        drop(credential);

        match request.action {
            ControlAction::AppPing => metadata::ping(request, ctx),
            ControlAction::AppVersion => metadata::version(request, ctx),
            ControlAction::TabCreate => layout::tab_create(request, ctx),
            ControlAction::InstanceList => ResponseEnvelope::ok(
                request.request_id,
                None,
                json!({ "actions": super::enabled_implemented_actions() }),
            ),
            ControlAction::WindowList
            | ControlAction::TabList
            | ControlAction::PaneList
            | ControlAction::SessionList => ResponseEnvelope::error(
                request.request_id,
                ControlError::new(
                    ErrorCode::UnsupportedAction,
                    "action is planned but not implemented in the foundation slice",
                ),
            ),
        }
    }

    fn refresh_policy(&mut self, ctx: &mut ModelContext<super::LocalControlServer>) {
        self.credential_issuer.set_policy(
            permissions::outside_warp_enabled(ctx),
            permissions::outside_warp_permissions(ctx),
        );
    }
}
