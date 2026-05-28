use anyhow::{Context, Result, bail};
use reqwest::blocking::Client as HttpClient;

use crate::protocol::{
    CredentialRequest, CredentialResponse, RequestEnvelope, ResponseEnvelope, ScopedCredential,
};

#[derive(Clone, Debug)]
pub struct LocalControlClient {
    endpoint: String,
    http: HttpClient,
}

impl LocalControlClient {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into().trim_end_matches('/').to_owned(),
            http: HttpClient::new(),
        }
    }

    pub fn request_credential(&self, request: &CredentialRequest) -> Result<ScopedCredential> {
        let response = self
            .http
            .post(format!("{}/v1/control/credentials", self.endpoint))
            .json(request)
            .send()
            .context("failed to request local-control credential")?
            .error_for_status()
            .context("local-control credential request returned an error status")?
            .json::<CredentialResponse>()
            .context("failed to decode local-control credential response")?;
        if let Some(error) = response.error {
            bail!("{}: {}", error.code.as_str(), error.message);
        }
        response
            .credential
            .context("local-control credential response did not include a credential")
    }

    pub fn send_request(
        &self,
        credential: &ScopedCredential,
        request: &RequestEnvelope,
    ) -> Result<ResponseEnvelope> {
        self.http
            .post(format!("{}/v1/control", self.endpoint))
            .bearer_auth(&credential.token)
            .json(request)
            .send()
            .context("failed to send local-control request")?
            .error_for_status()
            .context("local-control request returned an error status")?
            .json::<ResponseEnvelope>()
            .context("failed to decode local-control response")
    }
}
