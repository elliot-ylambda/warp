use crate::error::UserFacingError;
use crate::request_context::RequestContext;
use crate::schema;

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    graphql_type = "RootMutation",
    variables = "CreateManagedMcpProxySessionVariables"
)]
pub struct CreateManagedMcpProxySession {
    #[arguments(input: $input, requestContext: $request_context)]
    pub create_managed_mcp_proxy_session: CreateManagedMcpProxySessionResult,
}

crate::client::define_operation! {
    create_managed_mcp_proxy_session(CreateManagedMcpProxySessionVariables) -> CreateManagedMcpProxySession;
}

#[derive(cynic::QueryVariables, Debug)]
pub struct CreateManagedMcpProxySessionVariables {
    pub input: CreateManagedMcpProxySessionInput,
    pub request_context: RequestContext,
}

#[derive(cynic::InputObject, Debug)]
pub struct CreateManagedMcpProxySessionInput {
    pub uid: cynic::Id,
}

#[derive(cynic::InlineFragments, Debug)]
pub enum CreateManagedMcpProxySessionResult {
    CreateManagedMcpProxySessionOutput(CreateManagedMcpProxySessionOutput),
    UserFacingError(UserFacingError),
    #[cynic(fallback)]
    Unknown,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct CreateManagedMcpProxySessionOutput {
    pub mcp_config_json: String,
}
