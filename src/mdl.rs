use serde::Deserialize;
use crate::client::BlerifyClient;

#[derive(Debug, Deserialize)]
pub struct DocumentResponse {
    pub id: String,
    pub status: String,
}

impl BlerifyClient {
    pub async fn get_document(&self) -> Result<DocumentResponse, reqwest::Error> {
        let url = format!("{}/mdl/document", self.base_url);

        let response = self.client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await?;

        let json = response.json::<DocumentResponse>().await?;

        Ok(json)
    }
}