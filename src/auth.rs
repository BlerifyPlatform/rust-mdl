use std::fs;
use serde::{Deserialize, Serialize};
use jsonwebtoken::{encode, Header, EncodingKey};
use std::time::{SystemTime, UNIX_EPOCH};
use reqwest::Client;

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


pub async fn get_access_token() -> Result<String, reqwest::Error> {
    let data = fs::read_to_string("credentials.json")
        .expect("Failed to read credentials file");

    let creds: Credentials = serde_json::from_str(&data)
        .expect("Failed to parse JSON");

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

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
        &EncodingKey::from_rsa_pem(creds.private_key.as_bytes())
            .expect("Error con private key"),
    )
    .expect("Error generando JWT");

    let client = Client::new();

    let url = std::env::var("AUTH_URL")
    .unwrap_or("https://api.demo.blerify.com/auth/v2/protocol/openid-connect/token".to_string());

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
        .await?;

    let text = response.text().await?;

    Ok(text)
}