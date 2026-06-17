use crate::schema;

#[derive(cynic::Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManagedMcpStatus {
    #[cynic(rename = "ACTIVE")]
    Active,
    #[cynic(rename = "DRAFT")]
    Draft,
    #[cynic(rename = "ERROR")]
    Error,
}

#[derive(cynic::Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManagedMcpTransportKind {
    #[cynic(rename = "COMMAND")]
    Command,
    #[cynic(rename = "URL")]
    Url,
}

#[derive(cynic::QueryFragment, Debug, Clone)]
pub struct ManagedMcpServer {
    pub uid: cynic::Id,
    pub display_name: String,
    pub transport_kind: ManagedMcpTransportKind,
    pub status: ManagedMcpStatus,
    pub last_error: Option<String>,
}
