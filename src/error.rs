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
    #[error("server returned {status}: {}", format_server_detail(.message, .body))]
    Server {
        status: u16,
        message: String,
        body: serde_json::Value,
    },

    #[error("{0}")]
    Custom(String),
}

/// Render the server-error detail, falling back to the raw body when the
/// envelope's `message` field is missing or empty.
fn format_server_detail(message: &str, body: &serde_json::Value) -> String {
    if !message.is_empty() {
        return message.to_string();
    }
    if body.is_null() {
        return "<empty body>".to_string();
    }
    body.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_error_displays_message_when_present() {
        let err = BlerifyError::Server {
            status: 400,
            message: "bad payload".into(),
            body: serde_json::json!({"message": "bad payload", "code": "X"}),
        };
        assert_eq!(err.to_string(), "server returned 400: bad payload");
    }

    #[test]
    fn server_error_falls_back_to_body_when_message_empty() {
        let err = BlerifyError::Server {
            status: 500,
            message: String::new(),
            body: serde_json::json!({"error": "internal", "code": 42}),
        };
        let s = err.to_string();
        assert!(s.starts_with("server returned 500: "));
        assert!(s.contains("\"error\":\"internal\""));
        assert!(s.contains("\"code\":42"));
    }

    #[test]
    fn server_error_handles_null_body() {
        let err = BlerifyError::Server {
            status: 502,
            message: String::new(),
            body: serde_json::Value::Null,
        };
        assert_eq!(err.to_string(), "server returned 502: <empty body>");
    }
}
