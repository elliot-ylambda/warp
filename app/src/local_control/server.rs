use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use local_control::auth::ScopedCredential;
use local_control::discovery::{
    broker_token_path, default_registry_dir, write_broker_token, write_record, DiscoveryRecord,
};
use local_control::protocol::{
    CredentialRequest, CredentialResponse, ErrorCode, InstanceId, InvocationContext,
    RequestEnvelope, ResponseEnvelope, PROTOCOL_VERSION,
};
use warpui::{Entity, ModelContext, ModelSpawner, SingletonEntity};

use super::bridge::LocalControlBridge;
use super::permissions;
use crate::settings::LocalControlSettings;

#[derive(Clone)]
struct ServerState {
    bridge_spawner: ModelSpawner<LocalControlBridge>,
    credentials: Arc<Mutex<HashMap<String, ScopedCredential>>>,
    broker_token: String,
}

pub struct LocalControlServer {
    _runtime: Option<tokio::runtime::Runtime>,
}

impl LocalControlServer {
    pub fn new(
        bridge_spawner: ModelSpawner<LocalControlBridge>,
        ctx: &mut ModelContext<Self>,
    ) -> Self {
        let outside_enabled = *LocalControlSettings::as_ref(ctx).outside_warp_control_enabled;
        let runtime = if outside_enabled {
            Self::spawn_server(bridge_spawner)
                .inspect_err(|err| log::warn!("Failed to start local-control server: {err:#}"))
                .ok()
        } else {
            let record = DiscoveryRecord::disabled(
                InstanceId(uuid::Uuid::new_v4().to_string()),
                std::process::id(),
                warp_core::channel::ChannelState::channel().to_string(),
            );
            let path = default_registry_dir().join(format!("{}.json", record.instance_id.0));
            let _ = write_record(&path, &record);
            None
        };
        Self { _runtime: runtime }
    }

    fn spawn_server(
        bridge_spawner: ModelSpawner<LocalControlBridge>,
    ) -> anyhow::Result<tokio::runtime::Runtime> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_io()
            .enable_time()
            .build()?;
        runtime.spawn(async move {
            let broker_token = uuid::Uuid::new_v4().to_string();
            let state = ServerState {
                bridge_spawner,
                credentials: Arc::new(Mutex::new(HashMap::new())),
                broker_token,
            };
            let router = Router::new()
                .route("/v1/health", get(health))
                .route("/v1/control/credentials", post(credentials))
                .route("/v1/control", post(control))
                .with_state(state.clone());
            let listener =
                tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).await?;
            let addr = listener.local_addr()?;
            let instance_id = state
                .bridge_spawner
                .spawn(|bridge, _| bridge.instance_id())
                .await
                .map_err(|_| std::io::Error::other("local-control bridge dropped"))?;
            let record = DiscoveryRecord {
                instance_id: instance_id.clone(),
                pid: std::process::id(),
                channel: warp_core::channel::ChannelState::channel().to_string(),
                build_version: warp_core::channel::ChannelState::app_version().map(str::to_owned),
                protocol_version: PROTOCOL_VERSION,
                started_at_unix_millis: Utc::now().timestamp_millis(),
                outside_warp_control_enabled: true,
                endpoint: Some(format!("http://{addr}")),
                credential_broker: Some(format!("http://{addr}/v1/control/credentials")),
                actions: vec![
                    local_control::protocol::ControlActionKind::AppPing,
                    local_control::protocol::ControlActionKind::AppVersion,
                    local_control::protocol::ControlActionKind::TabCreate,
                ],
            };
            let path = default_registry_dir().join(format!("{}.json", instance_id.0));
            let _ = write_broker_token(&broker_token_path(&instance_id), &state.broker_token);
            let _ = write_record(&path, &record);
            axum::serve(listener, router).await
        });
        Ok(runtime)
    }
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "ok": true, "protocol_version": PROTOCOL_VERSION }))
}

async fn credentials(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<CredentialRequest>,
) -> (StatusCode, Json<ResponseEnvelope>) {
    let request_id = request.action.name().to_string();
    if request.protocol_version != PROTOCOL_VERSION {
        return (
            StatusCode::OK,
            Json(ResponseEnvelope::error(
                request_id,
                ErrorCode::UnsupportedAction,
                "unsupported local-control protocol version",
            )),
        );
    }
    let broker_token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    if broker_token != Some(state.broker_token.as_str()) {
        return (
            StatusCode::OK,
            Json(ResponseEnvelope::error(
                request_id,
                ErrorCode::UnauthorizedLocalClient,
                "missing or invalid local-control broker token",
            )),
        );
    }
    if request.invocation_context != InvocationContext::OutsideWarp {
        return (
            StatusCode::OK,
            Json(ResponseEnvelope::error(
                request_id,
                ErrorCode::ExecutionContextNotAllowed,
                "inside-Warp credential requests are reserved for future verified terminal proof support",
            )),
        );
    }
    let result = state
        .bridge_spawner
        .spawn(move |bridge, ctx| {
            permissions::issue_credential(
                ctx,
                bridge.instance_id().0,
                request.action,
                request.invocation_context,
            )
        })
        .await;
    match result {
        Ok(Ok(credential)) => {
            let expires_at_unix_millis = credential.expires_at.timestamp_millis();
            let credential_id = credential.id.clone();
            let credential_json = serde_json::to_string(&credential).unwrap_or_default();
            if let Ok(mut credentials) = state.credentials.lock() {
                credentials.insert(credential_id, credential);
            } else {
                return (
                    StatusCode::OK,
                    Json(ResponseEnvelope::error(
                        request_id,
                        ErrorCode::TransportError,
                        "credential store is unavailable",
                    )),
                );
            }
            (
                StatusCode::OK,
                Json(ResponseEnvelope::ok(
                    request_id,
                    None,
                    serde_json::to_value(CredentialResponse {
                        credential: credential_json,
                        expires_at_unix_millis,
                    })
                    .unwrap(),
                )),
            )
        }
        Ok(Err(code)) => (
            StatusCode::OK,
            Json(ResponseEnvelope::error(
                request_id,
                code,
                "credential request denied",
            )),
        ),
        Err(_) => (
            StatusCode::OK,
            Json(ResponseEnvelope::error(
                request_id,
                ErrorCode::TransportError,
                "local-control bridge dropped",
            )),
        ),
    }
}

async fn control(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<RequestEnvelope>,
) -> (StatusCode, Json<ResponseEnvelope>) {
    if request.protocol_version != PROTOCOL_VERSION {
        return (
            StatusCode::OK,
            Json(ResponseEnvelope::error(
                request.request_id,
                ErrorCode::UnsupportedAction,
                "unsupported local-control protocol version",
            )),
        );
    }
    let credential = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .and_then(|value| serde_json::from_str::<ScopedCredential>(value).ok());
    let Some(credential) = credential else {
        return (
            StatusCode::OK,
            Json(ResponseEnvelope::error(
                request.request_id,
                ErrorCode::UnauthorizedLocalClient,
                "missing or malformed local-control credential",
            )),
        );
    };
    let request_id = request.request_id.clone();
    let credential_id = credential.id.clone();
    let issued_credential = if let Ok(mut credentials) = state.credentials.lock() {
        credentials.retain(|_, credential| credential.expires_at > Utc::now());
        credentials.get(&credential_id).cloned()
    } else {
        return (
            StatusCode::OK,
            Json(ResponseEnvelope::error(
                request_id,
                ErrorCode::TransportError,
                "credential store is unavailable",
            )),
        );
    };
    if issued_credential.as_ref() != Some(&credential) {
        return (
            StatusCode::OK,
            Json(ResponseEnvelope::error(
                request_id,
                ErrorCode::UnauthorizedLocalClient,
                "credential was not issued by this Warp instance",
            )),
        );
    }
    let result = state
        .bridge_spawner
        .spawn(move |bridge, ctx| bridge.handle_request(request, credential, ctx))
        .await;
    match result {
        Ok(response) => (StatusCode::OK, Json(response)),
        Err(_) => (
            StatusCode::OK,
            Json(ResponseEnvelope::error(
                "bridge_dropped",
                ErrorCode::TransportError,
                "local-control bridge dropped",
            )),
        ),
    }
}

impl Entity for LocalControlServer {
    type Event = ();
}

impl SingletonEntity for LocalControlServer {}
