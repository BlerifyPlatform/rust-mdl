use crate::error::BlerifyError;
use jsonwebtoken::{encode, EncodingKey, Header};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Deserialize)]
struct Credentials {
    client_id: String,
    organization_id: String,
    private_key: String,
    iam_audience: String,
}

#[derive(Debug, Serialize)]
struct Claims {
    iss: String,
    sub: String,
    aud: String,
    exp: usize,
    iat: usize,
    jti: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
}

pub async fn get_access_token() -> Result<String, BlerifyError> {
    let data = fs::read_to_string("credentials.json")?;
    let creds: Credentials = serde_json::from_str(&data)?;

    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    let claims = Claims {
        iss: creds.client_id.clone(),
        sub: creds.client_id.clone(),
        aud: creds.iam_audience.clone(),
        exp: (now + 3600) as usize,
        iat: now as usize,
        jti: format!("{}", now),
    };

    let jwt = encode(
        &Header::new(jsonwebtoken::Algorithm::RS256),
        &claims,
        &EncodingKey::from_rsa_pem(creds.private_key.as_bytes())?,
    )?;

    let client = Client::new();

    let url = std::env::var("AUTH_URL").unwrap_or(
        "https://api.demo.blerify.com/auth/v2/protocol/openid-connect/token".to_string(),
    );

    let params = [
        ("client_id", creds.client_id.as_str()),
        ("organization_id", creds.organization_id.as_str()),
        ("client_assertion", jwt.as_str()),
    ];

    let response = client
        .post(url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&params)
        .send()
        .await?
        .error_for_status()?;

    let json = response.json::<TokenResponse>().await?;

    Ok(json.access_token)
}
