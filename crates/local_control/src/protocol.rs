use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTOCOL_VERSION: u16 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InstanceId(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WindowId(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TabId(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PaneId(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionId(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BlockId(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DriveObjectId(pub String);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvocationContext {
    OutsideWarp,
    InsideWarp,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExecutionContextProof {
    None,
    VerifiedWarpTerminal { handle: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionCategory {
    ReadMetadata,
    ReadUnderlyingData,
    MutateAppState,
    MutateMetadata,
    MutateUnderlyingData,
}

impl PermissionCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReadMetadata => "read_metadata",
            Self::ReadUnderlyingData => "read_underlying_data",
            Self::MutateAppState => "mutate_app_state",
            Self::MutateMetadata => "mutate_metadata",
            Self::MutateUnderlyingData => "mutate_underlying_data",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportStatus {
    Implemented,
    Stub,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlActionKind {
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
    #[serde(rename = "input.run")]
    InputRun,
    #[serde(rename = "drive.object.create")]
    DriveObjectCreate,
}

impl ControlActionKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::InstanceList => "instance.list",
            Self::AppPing => "app.ping",
            Self::AppVersion => "app.version",
            Self::TabCreate => "tab.create",
            Self::WindowList => "window.list",
            Self::TabList => "tab.list",
            Self::PaneList => "pane.list",
            Self::SessionList => "session.list",
            Self::InputRun => "input.run",
            Self::DriveObjectCreate => "drive.object.create",
        }
    }
}

impl fmt::Display for ControlActionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WindowSelector {
    #[default]
    Active,
    Id(WindowId),
    Index(u32),
    Title(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TabSelector {
    Active,
    Id(TabId),
    Index(u32),
    Title(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaneSelector {
    Active,
    Id(PaneId),
    Index(u32),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionSelector {
    Active,
    Id(SessionId),
    Index(u32),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockSelector {
    Id(BlockId),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileSelector {
    pub path: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriveObjectSelector {
    Id(DriveObjectId),
    Lookup {
        object_type: String,
        name_or_path: String,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetSelector {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<WindowSelector>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab: Option<TabSelector>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pane: Option<PaneSelector>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionSelector>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block: Option<BlockSelector>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<FileSelector>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drive_object: Option<DriveObjectSelector>,
}

impl TargetSelector {
    pub fn only_default_or_active_window(&self) -> bool {
        let window_supported = matches!(self.window, None | Some(WindowSelector::Active));
        window_supported
            && self.tab.is_none()
            && self.pane.is_none()
            && self.session.is_none()
            && self.block.is_none()
            && self.file.is_none()
            && self.drive_object.is_none()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RequestEnvelope {
    pub protocol_version: u16,
    pub request_id: String,
    pub action: ControlActionKind,
    #[serde(default)]
    pub target: TargetSelector,
    #[serde(default)]
    pub params: Value,
}

impl RequestEnvelope {
    pub fn new(request_id: String, action: ControlActionKind) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            request_id,
            action,
            target: TargetSelector::default(),
            params: Value::Object(Default::default()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResponseEnvelope {
    pub ok: bool,
    pub protocol_version: u16,
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<InstanceId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_target: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorBody>,
}

impl ResponseEnvelope {
    pub fn ok(
        request_id: impl Into<String>,
        instance_id: Option<InstanceId>,
        result: Value,
    ) -> Self {
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

    pub fn error(
        request_id: impl Into<String>,
        code: ErrorCode,
        message: impl Into<String>,
    ) -> Self {
        Self {
            ok: false,
            protocol_version: PROTOCOL_VERSION,
            request_id: request_id.into(),
            instance_id: None,
            resolved_target: None,
            result: None,
            error: Some(ErrorBody {
                code,
                message: message.into(),
                details: None,
            }),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ErrorBody {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    #[error("local_control_disabled")]
    LocalControlDisabled,
    #[error("unauthorized_local_client")]
    UnauthorizedLocalClient,
    #[error("insufficient_permissions")]
    InsufficientPermissions,
    #[error("authenticated_user_required")]
    AuthenticatedUserRequired,
    #[error("authenticated_user_unavailable")]
    AuthenticatedUserUnavailable,
    #[error("execution_context_not_allowed")]
    ExecutionContextNotAllowed,
    #[error("ambiguous_instance")]
    AmbiguousInstance,
    #[error("ambiguous_target")]
    AmbiguousTarget,
    #[error("stale_target")]
    StaleTarget,
    #[error("invalid_selector")]
    InvalidSelector,
    #[error("unsupported_action")]
    UnsupportedAction,
    #[error("not_allowlisted")]
    NotAllowlisted,
    #[error("invalid_params")]
    InvalidParams,
    #[error("target_state_conflict")]
    TargetStateConflict,
    #[error("missing_target")]
    MissingTarget,
    #[error("no_instance")]
    NoInstance,
    #[error("feature_disabled")]
    FeatureDisabled,
    #[error("transport_error")]
    TransportError,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialRequest {
    pub protocol_version: u16,
    pub action: ControlActionKind,
    pub invocation_context: InvocationContext,
    pub proof: ExecutionContextProof,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialResponse {
    pub credential: String,
    pub expires_at_unix_millis: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_are_stable_snake_case() {
        assert_eq!(
            serde_json::to_string(&ErrorCode::ExecutionContextNotAllowed).unwrap(),
            "\"execution_context_not_allowed\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::LocalControlDisabled).unwrap(),
            "\"local_control_disabled\""
        );
    }

    #[test]
    fn request_envelope_serializes_protocol_shape() {
        let request = RequestEnvelope::new("req-1".to_owned(), ControlActionKind::TabCreate);
        let value = serde_json::to_value(request).unwrap();
        assert_eq!(value["protocol_version"], PROTOCOL_VERSION);
        assert_eq!(value["action"], "tab.create");
    }

    #[test]
    fn tab_create_foundation_target_accepts_only_default_or_active_window() {
        assert!(TargetSelector::default().only_default_or_active_window());
        assert!(
            TargetSelector {
                window: Some(WindowSelector::Active),
                ..TargetSelector::default()
            }
            .only_default_or_active_window()
        );
        assert!(
            !TargetSelector {
                window: Some(WindowSelector::Index(0)),
                ..TargetSelector::default()
            }
            .only_default_or_active_window()
        );
    }
}
