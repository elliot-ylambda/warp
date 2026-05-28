pub mod bridge;
pub mod permissions;
pub mod resolver;
pub mod handlers {
    pub mod layout;
    pub mod metadata;
}

use local_control::auth::CredentialIssuer;
use local_control::discovery::{default_registry_dir, write_record, DiscoveryRecord};
use warp_core::features::FeatureFlag;
use warpui::{AppContext, Entity, ModelContext, ModelSpawner, SingletonEntity};

pub struct LocalControlServer {
    pub bridge: bridge::LocalControlBridge,
    #[cfg(not(target_family = "wasm"))]
    instance_id: String,
    #[cfg(not(target_family = "wasm"))]
    endpoint: Option<String>,
    #[cfg(not(target_family = "wasm"))]
    _runtime: Option<tokio::runtime::Runtime>,
}

impl LocalControlServer {
    pub fn new(ctx: &mut ModelContext<Self>) -> Self {
        let permissions = permissions::outside_warp_permissions(ctx);
        let outside_warp_enabled = permissions::outside_warp_enabled(ctx);
        let mut server = Self {
            bridge: bridge::LocalControlBridge::new(CredentialIssuer::new(
                outside_warp_enabled,
                permissions,
            )),
            #[cfg(not(target_family = "wasm"))]
            instance_id: uuid::Uuid::new_v4().to_string(),
            #[cfg(not(target_family = "wasm"))]
            endpoint: None,
            #[cfg(not(target_family = "wasm"))]
            _runtime: None,
        };

        #[cfg(not(target_family = "wasm"))]
        server.start_loopback_server(ctx);
        #[cfg(not(target_family = "wasm"))]
        server.subscribe_to_settings(ctx);

        server
    }

    pub fn should_start(ctx: &AppContext) -> bool {
        let _ = ctx;
        FeatureFlag::WarpControlCli.is_enabled()
    }

    #[cfg(not(target_family = "wasm"))]
    fn start_loopback_server(&mut self, ctx: &mut ModelContext<Self>) {
        match spawn_loopback_server(ctx.spawner()) {
            Ok((runtime, endpoint)) => {
                self.endpoint = Some(endpoint);
                self._runtime = Some(runtime);
                self.write_discovery_record(ctx);
            }
            Err(error) => {
                log::warn!("Failed to start local-control server: {error:#}");
                self.write_discovery_record(ctx);
            }
        }
    }

    #[cfg(not(target_family = "wasm"))]
    fn subscribe_to_settings(&self, ctx: &mut ModelContext<Self>) {
        ctx.subscribe_to_model(
            &crate::settings::LocalControlSettings::handle(ctx),
            |server, _, ctx| {
                server.write_discovery_record(ctx);
            },
        );
    }

    #[cfg(not(target_family = "wasm"))]
    fn write_discovery_record(&self, ctx: &mut ModelContext<Self>) {
        let Some(dir) = default_registry_dir() else {
            return;
        };
        let outside_warp_control_enabled =
            FeatureFlag::WarpControlCli.is_enabled() && permissions::outside_warp_enabled(ctx);
        let record = DiscoveryRecord {
            instance_id: self.instance_id.clone(),
            pid: std::process::id(),
            channel: format!("{:?}", warp_core::channel::ChannelState::channel()),
            app_version: warp_core::channel::ChannelState::app_version()
                .unwrap_or("unknown")
                .to_owned(),
            protocol_version: local_control::PROTOCOL_VERSION,
            started_at_unix_ms: local_control::auth::now_unix_ms(),
            outside_warp_control_enabled,
            endpoint: outside_warp_control_enabled
                .then(|| self.endpoint.clone())
                .flatten(),
            implemented_actions: if outside_warp_control_enabled {
                enabled_implemented_actions()
            } else {
                Vec::new()
            },
        };
        if let Err(error) = write_record(&dir, &record) {
            log::warn!("Failed to write local-control discovery record: {error:#}");
        }
    }
}

impl Entity for LocalControlServer {
    type Event = ();
}

impl SingletonEntity for LocalControlServer {}

pub fn enabled_implemented_actions() -> Vec<String> {
    local_control::catalog::implemented_actions()
        .into_iter()
        .map(|metadata| metadata.action.as_str().to_owned())
        .collect()
}

#[cfg(not(target_family = "wasm"))]
fn spawn_loopback_server(
    spawner: ModelSpawner<LocalControlServer>,
) -> anyhow::Result<(tokio::runtime::Runtime, String)> {
    use std::net::{Ipv4Addr, SocketAddr, TcpListener};

    use axum::routing::post;
    use axum::Router;

    let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))?;
    listener.set_nonblocking(true)?;
    let local_addr = listener.local_addr()?;
    let endpoint = format!("http://{local_addr}");

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_io()
        .build()?;
    let router = Router::new()
        .route("/v1/control/credentials", post(http::issue_credential))
        .route("/v1/control", post(http::handle_control_request))
        .with_state(http::ServerState { spawner });

    runtime.spawn(async move {
        let listener = match tokio::net::TcpListener::from_std(listener) {
            Ok(listener) => listener,
            Err(error) => {
                log::warn!("Failed to create local-control listener: {error:#}");
                return;
            }
        };
        if let Err(error) = axum::serve(listener, router).await {
            log::warn!("Local-control server stopped with error: {error:#}");
        }
    });

    Ok((runtime, endpoint))
}

#[cfg(not(target_family = "wasm"))]
mod http {
    use axum::extract::{Json, State};
    use axum::http::{HeaderMap, StatusCode};
    use local_control::protocol::{
        ControlError, CredentialRequest, CredentialResponse, ErrorCode, RequestEnvelope,
        ResponseEnvelope, PROTOCOL_VERSION,
    };
    use warpui::ModelSpawner;

    use super::LocalControlServer;

    #[derive(Clone)]
    pub struct ServerState {
        pub spawner: ModelSpawner<LocalControlServer>,
    }

    pub async fn issue_credential(
        State(state): State<ServerState>,
        Json(request): Json<CredentialRequest>,
    ) -> (StatusCode, Json<CredentialResponse>) {
        let request_id = request.request_id.clone();
        match state
            .spawner
            .spawn(move |server, ctx| server.bridge.issue_credential(request, ctx))
            .await
        {
            Ok(response) => (StatusCode::OK, Json(response)),
            Err(_) => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(CredentialResponse {
                    protocol_version: PROTOCOL_VERSION,
                    request_id,
                    credential: None,
                    error: Some(ControlError::new(
                        ErrorCode::TransportError,
                        "local-control app bridge is unavailable",
                    )),
                }),
            ),
        }
    }

    pub async fn handle_control_request(
        State(state): State<ServerState>,
        headers: HeaderMap,
        Json(request): Json<RequestEnvelope>,
    ) -> (StatusCode, Json<ResponseEnvelope>) {
        let request_id = request.request_id.clone();
        let token = bearer_token(&headers);
        match state
            .spawner
            .spawn(move |server, ctx| server.bridge.handle_request(token, request, ctx))
            .await
        {
            Ok(response) => (StatusCode::OK, Json(response)),
            Err(_) => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ResponseEnvelope::error(
                    request_id,
                    ControlError::new(
                        ErrorCode::TransportError,
                        "local-control app bridge is unavailable",
                    ),
                )),
            ),
        }
    }

    fn bearer_token(headers: &HeaderMap) -> Option<String> {
        headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .map(str::to_owned)
    }
}
