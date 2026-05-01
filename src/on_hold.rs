use reqwest::Method;
use serde::Deserialize;
use tracing::{debug, instrument};
use uuid::Uuid;

use crate::client::{decode_json_response, BlerifyClient};
use crate::error::BlerifyError;

#[derive(Debug, Clone, Deserialize)]
pub struct OnHoldResponse {
    pub status: String,
}

impl BlerifyClient {
    #[instrument(skip_all)]
    pub async fn on_hold(
        &self,
        credential_id: &str,
        correlation_id: Option<Uuid>,
    ) -> Result<OnHoldResponse, BlerifyError> {
        let path = format!(
            "{}/credentials/{}/onHold",
            self.project_base_path(),
            credential_id
        );

        debug!("onHold credential");

        let response = self
            .request(Method::PUT, &path, correlation_id)
            .await?
            .send()
            .await?;

        decode_json_response(response).await
    }
}
