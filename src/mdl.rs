use crate::client::BlerifyClient;
use crate::error::BlerifyError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct DocumentResponse {
    pub id: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct CreateDocumentRequest {
    pub name: String,
    pub document_type: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateDocumentResponse {
    pub id: String,
    pub status: String,
}

impl BlerifyClient {
    pub async fn get_document(&self, document_id: &str) -> Result<DocumentResponse, BlerifyError> {
        let url = format!("{}/mdl/document/{}", self.base_url, document_id);

        let response = self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await?
            .error_for_status()?;

        let json = response.json::<DocumentResponse>().await?;

        Ok(json)
    }

    pub async fn create_document(
        &self,
        request: CreateDocumentRequest,
    ) -> Result<CreateDocumentResponse, BlerifyError> {
        let url = format!("{}/mdl/document", self.base_url);

        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&request)
            .send()
            .await?
            .error_for_status()?;

        let json = response.json::<CreateDocumentResponse>().await?;

        Ok(json)
    }
}
