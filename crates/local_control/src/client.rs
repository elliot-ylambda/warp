use serde_json::json;
use uuid::Uuid;

use crate::discovery::{DiscoveryRecord, default_registry_dir, read_broker_token, read_records};
use crate::protocol::{
    ControlActionKind, CredentialRequest, CredentialResponse, ErrorCode, ExecutionContextProof,
    InvocationContext, PROTOCOL_VERSION, RequestEnvelope, ResponseEnvelope,
};
use crate::selection::{InstanceSelector, select_instance};

#[derive(Clone, Debug)]
pub struct Client {
    registry_dir: std::path::PathBuf,
}

impl Default for Client {
    fn default() -> Self {
        Self {
            registry_dir: default_registry_dir(),
        }
    }
}

impl Client {
    pub fn with_registry_dir(registry_dir: std::path::PathBuf) -> Self {
        Self { registry_dir }
    }

    pub fn instances(&self) -> Result<Vec<DiscoveryRecord>, ErrorCode> {
        read_records(&self.registry_dir)
    }

    pub fn send(
        &self,
        selector: &InstanceSelector,
        action: ControlActionKind,
    ) -> Result<ResponseEnvelope, ErrorCode> {
        let records = self.instances()?;
        let record = select_instance(&records, selector)?;
        let endpoint = record.endpoint.as_ref().ok_or(ErrorCode::NoInstance)?;
        let credential_broker = record
            .credential_broker
            .as_ref()
            .ok_or(ErrorCode::NoInstance)?;
        let broker_token = read_broker_token(&record.instance_id)?;
        let http_client = reqwest::blocking::Client::new();
        let credential_response = http_client
            .post(credential_broker)
            .bearer_auth(broker_token)
            .json(&CredentialRequest {
                protocol_version: PROTOCOL_VERSION,
                action,
                invocation_context: InvocationContext::OutsideWarp,
                proof: ExecutionContextProof::None,
            })
            .send()
            .map_err(|_| ErrorCode::TransportError)?
            .json::<ResponseEnvelope>()
            .map_err(|_| ErrorCode::TransportError)?;
        if !credential_response.ok {
            return Ok(credential_response);
        }
        let credential = credential_response
            .result
            .ok_or(ErrorCode::TransportError)
            .and_then(|value| {
                serde_json::from_value::<CredentialResponse>(value)
                    .map_err(|_| ErrorCode::TransportError)
            })?
            .credential;
        let request = RequestEnvelope::new(Uuid::new_v4().to_string(), action);
        let response = http_client
            .post(format!("{endpoint}/v1/control"))
            .bearer_auth(credential)
            .json(&request)
            .send()
            .map_err(|_| ErrorCode::TransportError)?;
        response
            .json::<ResponseEnvelope>()
            .map_err(|_| ErrorCode::TransportError)
    }

    pub fn no_instance_error(request_id: impl Into<String>) -> ResponseEnvelope {
        ResponseEnvelope::error(
            request_id,
            ErrorCode::NoInstance,
            "no running Warp instance found for local control",
        )
    }

    pub fn render_instances_json(records: &[DiscoveryRecord]) -> serde_json::Value {
        json!({ "instances": records })
    }
}
