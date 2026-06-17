use crate::error::UserFacingError;
use crate::managed_mcp::ManagedMcpServer;
use crate::request_context::RequestContext;
use crate::schema;

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "RootQuery", variables = "ManagedMcpServerVariables")]
pub struct ManagedMcpServerQuery {
    #[arguments(input: $input, requestContext: $request_context)]
    pub managed_mcp_server: ManagedMcpServerResult,
}

crate::client::define_operation! {
    managed_mcp_server(ManagedMcpServerVariables) -> ManagedMcpServerQuery;
}

#[derive(cynic::QueryVariables, Debug)]
pub struct ManagedMcpServerVariables {
    pub input: ManagedMcpServerInput,
    pub request_context: RequestContext,
}

#[derive(cynic::InputObject, Debug)]
pub struct ManagedMcpServerInput {
    pub uid: cynic::Id,
}

#[derive(cynic::InlineFragments, Debug)]
pub enum ManagedMcpServerResult {
    ManagedMcpServerOutput(ManagedMcpServerOutput),
    UserFacingError(UserFacingError),
    #[cynic(fallback)]
    Unknown,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct ManagedMcpServerOutput {
    pub server: ManagedMcpServer,
}
