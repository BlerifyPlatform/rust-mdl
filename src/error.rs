use thiserror::Error;

/// Errors surfaced by the Blerify Issuance API client.
#[derive(Debug, Error)]
pub enum BlerifyError {
    #[error("transport error: {0}")]
    Transport(#[from] reqwest::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("jwt error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("system time error: {0}")]
    Time(#[from] std::time::SystemTimeError),

    /// Non-2xx response from the Issuance API. `body` carries whatever the
    /// server returned (parsed JSON if possible, otherwise raw text).
    #[error("server returned {status}: {message}")]
    Server {
        status: u16,
        message: String,
        body: serde_json::Value,
    },

    #[error("{0}")]
    Custom(String),
}
