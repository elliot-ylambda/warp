use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

pub const PROTOCOL_VERSION: u16 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    LocalControlDisabled,
    UnauthorizedLocalClient,
    InsufficientPermissions,
    AuthenticatedUserRequired,
    AuthenticatedUserUnavailable,
    ExecutionContextNotAllowed,
    AmbiguousInstance,
    AmbiguousTarget,
    StaleTarget,
    InvalidSelector,
    UnsupportedAction,
    NotAllowlisted,
    InvalidParams,
    TargetStateConflict,
    MissingTarget,
    NoInstance,
    FeatureDisabled,
    TransportError,
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LocalControlDisabled => "local_control_disabled",
            Self::UnauthorizedLocalClient => "unauthorized_local_client",
            Self::InsufficientPermissions => "insufficient_permissions",
            Self::AuthenticatedUserRequired => "authenticated_user_required",
            Self::AuthenticatedUserUnavailable => "authenticated_user_unavailable",
            Self::ExecutionContextNotAllowed => "execution_context_not_allowed",
            Self::AmbiguousInstance => "ambiguous_instance",
            Self::AmbiguousTarget => "ambiguous_target",
            Self::StaleTarget => "stale_target",
            Self::InvalidSelector => "invalid_selector",
            Self::UnsupportedAction => "unsupported_action",
            Self::NotAllowlisted => "not_allowlisted",
            Self::InvalidParams => "invalid_params",
            Self::TargetStateConflict => "target_state_conflict",
            Self::MissingTarget => "missing_target",
            Self::NoInstance => "no_instance",
            Self::FeatureDisabled => "feature_disabled",
            Self::TransportError => "transport_error",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionCategory {
    MetadataRead,
    UnderlyingDataRead,
    AppStateMutation,
    MetadataConfigurationMutation,
    UnderlyingDataMutation,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvocationContext {
    OutsideWarp,
    InsideWarp,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ExecutionContextProof {
    None,
    VerifiedWarpTerminal { proof_id: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionSupportStatus {
    Implemented,
    Planned,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ControlAction {
    #[serde(rename = "instance.list")]
    InstanceList,
    #[serde(rename = "app.ping")]
    AppPing,
    #[serde(rename = "app.version")]
    AppVersion,
    #[serde(rename = "tab.create")]
    TabCreate,
    #[serde(rename = "window.list")]
    WindowList,
    #[serde(rename = "tab.list")]
    TabList,
    #[serde(rename = "pane.list")]
    PaneList,
    #[serde(rename = "session.list")]
    SessionList,
}

impl ControlAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InstanceList => "instance.list",
            Self::AppPing => "app.ping",
            Self::AppVersion => "app.version",
            Self::TabCreate => "tab.create",
            Self::WindowList => "window.list",
            Self::TabList => "tab.list",
            Self::PaneList => "pane.list",
            Self::SessionList => "session.list",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RequestEnvelope {
    pub protocol_version: u16,
    pub request_id: String,
    pub action: ControlAction,
    #[serde(default)]
    pub target: TargetSelector,
    #[serde(default)]
    pub params: Value,
}

impl RequestEnvelope {
    pub fn new(action: ControlAction) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            request_id: Uuid::new_v4().to_string(),
            action,
            target: TargetSelector::default(),
            params: Value::Object(Default::default()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResponseEnvelope {
    pub ok: bool,
    pub protocol_version: u16,
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_target: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ControlError>,
}

impl ResponseEnvelope {
    pub fn ok(request_id: impl Into<String>, instance_id: Option<String>, result: Value) -> Self {
        Self {
            ok: true,
            protocol_version: PROTOCOL_VERSION,
            request_id: request_id.into(),
            instance_id,
            resolved_target: None,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(request_id: impl Into<String>, error: ControlError) -> Self {
        Self {
            ok: false,
            protocol_version: PROTOCOL_VERSION,
            request_id: request_id.into(),
            instance_id: None,
            resolved_target: None,
            result: None,
            error: Some(error),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ControlError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selector: Option<Value>,
}

impl ControlError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            selector: None,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct TargetSelector {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window: Option<WindowSelector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab: Option<TabSelector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pane: Option<PaneSelector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionSelector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block: Option<BlockSelector>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum WindowSelector {
    Active,
    Id(String),
    Index(u32),
    Title(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum TabSelector {
    Active,
    Id(String),
    Index(u32),
    Title(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum PaneSelector {
    Active,
    Id(String),
    Index(u32),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum SessionSelector {
    Active,
    Id(String),
    Index(u32),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum BlockSelector {
    Id(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CredentialRequest {
    pub protocol_version: u16,
    pub request_id: String,
    pub action: ControlAction,
    pub invocation_context: InvocationContext,
    pub proof: ExecutionContextProof,
}

impl CredentialRequest {
    pub fn outside_warp(action: ControlAction) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            request_id: Uuid::new_v4().to_string(),
            action,
            invocation_context: InvocationContext::OutsideWarp,
            proof: ExecutionContextProof::None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CredentialResponse {
    pub protocol_version: u16,
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential: Option<ScopedCredential>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ControlError>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScopedCredential {
    pub token: String,
    pub action: ControlAction,
    pub permission: PermissionCategory,
    pub invocation_context: InvocationContext,
    pub expires_at_unix_ms: u64,
    pub requires_authenticated_user: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_serialize_to_stable_snake_case() {
        let json = serde_json::to_string(&ErrorCode::ExecutionContextNotAllowed).unwrap();
        assert_eq!(json, "\"execution_context_not_allowed\"");
        assert_eq!(ErrorCode::InvalidParams.as_str(), "invalid_params");
    }

    #[test]
    fn protocol_envelope_serializes_action_names_with_dots() {
        let envelope = RequestEnvelope::new(ControlAction::TabCreate);
        let value = serde_json::to_value(envelope).unwrap();
        assert_eq!(value["action"], "tab.create");
        assert_eq!(value["protocol_version"], PROTOCOL_VERSION);
    }
}
