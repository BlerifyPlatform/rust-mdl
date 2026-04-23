use serde::Deserialize;
use crate::client::BlerifyClient;

#[derive(Debug, Deserialize)]
pub struct DocumentResponse {
    pub id: String,
    pub status: String,
}

impl BlerifyClient {
    pub async fn get_document(&self, document_id: &str) -> Result<DocumentResponse, reqwest::Error> {
        let url = format!("{}/mdl/document/{}", self.base_url, document_id);

        let response = self.client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await?
            .error_for_status()?;

        let json = response.json::<DocumentResponse>().await?;

        Ok(json)
    }
}