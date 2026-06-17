use anyhow::{anyhow, Result};
use async_trait::async_trait;
use cynic::{MutationBuilder, QueryBuilder};
use uuid::Uuid;
use warp_graphql::managed_mcp::{
    ManagedMcpStatus as GraphqlManagedMcpStatus,
    ManagedMcpTransportKind as GraphqlManagedMcpTransportKind,
};
use warp_graphql::mutations::create_managed_mcp_proxy_session::{
    CreateManagedMcpProxySession, CreateManagedMcpProxySessionInput,
    CreateManagedMcpProxySessionResult, CreateManagedMcpProxySessionVariables,
};
use warp_graphql::queries::managed_mcp_server::{
    ManagedMcpServerInput, ManagedMcpServerQuery, ManagedMcpServerResult, ManagedMcpServerVariables,
};

use super::ServerApi;
use crate::server::graphql::{get_request_context, get_user_facing_error_message};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ManagedMcpServerForResolution {
    pub display_name: String,
    pub transport_kind: ManagedMcpTransportKind,
    pub status: ManagedMcpStatus,
    pub last_error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ManagedMcpTransportKind {
    Url,
    Command,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ManagedMcpStatus {
    Active,
    Draft,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ManagedMcpProxySession {
    pub mcp_config_json: String,
}

#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
pub(crate) trait ManagedMcpClient: Send + Sync {
    async fn get_managed_mcp_server(
        &self,
        uid: Uuid,
    ) -> Result<Option<ManagedMcpServerForResolution>>;

    async fn create_managed_mcp_proxy_session(&self, uid: Uuid) -> Result<ManagedMcpProxySession>;
}

#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
impl ManagedMcpClient for ServerApi {
    async fn get_managed_mcp_server(
        &self,
        uid: Uuid,
    ) -> Result<Option<ManagedMcpServerForResolution>> {
        let variables = ManagedMcpServerVariables {
            input: ManagedMcpServerInput {
                uid: cynic::Id::new(uid.to_string()),
            },
            request_context: get_request_context(),
        };
        let operation = ManagedMcpServerQuery::build(variables);
        let response = self.send_graphql_request(operation, None).await?;

        match response.managed_mcp_server {
            ManagedMcpServerResult::ManagedMcpServerOutput(output) => {
                let server = output.server;
                Ok(Some(ManagedMcpServerForResolution {
                    display_name: server.display_name,
                    transport_kind: match server.transport_kind {
                        GraphqlManagedMcpTransportKind::Url => ManagedMcpTransportKind::Url,
                        GraphqlManagedMcpTransportKind::Command => ManagedMcpTransportKind::Command,
                    },
                    status: match server.status {
                        GraphqlManagedMcpStatus::Active => ManagedMcpStatus::Active,
                        GraphqlManagedMcpStatus::Draft => ManagedMcpStatus::Draft,
                        GraphqlManagedMcpStatus::Error => ManagedMcpStatus::Error,
                    },
                    last_error: server.last_error,
                }))
            }
            ManagedMcpServerResult::UserFacingError(error) => {
                let message = get_user_facing_error_message(error);
                if is_absent_managed_mcp_message(&message) {
                    Ok(None)
                } else {
                    Err(anyhow!(message))
                }
            }
            ManagedMcpServerResult::Unknown => {
                Err(anyhow!("Unknown error while resolving managed MCP server"))
            }
        }
    }

    async fn create_managed_mcp_proxy_session(&self, uid: Uuid) -> Result<ManagedMcpProxySession> {
        let variables = CreateManagedMcpProxySessionVariables {
            input: CreateManagedMcpProxySessionInput {
                uid: cynic::Id::new(uid.to_string()),
            },
            request_context: get_request_context(),
        };
        let operation = CreateManagedMcpProxySession::build(variables);
        let response = self.send_graphql_request(operation, None).await?;

        match response.create_managed_mcp_proxy_session {
            CreateManagedMcpProxySessionResult::CreateManagedMcpProxySessionOutput(output) => {
                Ok(ManagedMcpProxySession {
                    mcp_config_json: output.mcp_config_json,
                })
            }
            CreateManagedMcpProxySessionResult::UserFacingError(error) => {
                Err(anyhow!(get_user_facing_error_message(error)))
            }
            CreateManagedMcpProxySessionResult::Unknown => Err(anyhow!(
                "Unknown error while creating managed MCP proxy session"
            )),
        }
    }
}

fn is_absent_managed_mcp_message(message: &str) -> bool {
    matches!(
        message.trim().to_ascii_lowercase().as_str(),
        "managed mcp server not found" | "managed mcp is not enabled"
    )
}
