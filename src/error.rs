use std::fmt;

#[derive(Debug)]
pub enum BlerifyError {
    Reqwest(reqwest::Error),
    Io(std::io::Error),
    Serde(serde_json::Error),
    Jwt(jsonwebtoken::errors::Error),
    Custom(String),
}

impl fmt::Display for BlerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlerifyError::Reqwest(e) => write!(f, "Request error: {}", e),
            BlerifyError::Io(e) => write!(f, "IO error: {}", e),
            BlerifyError::Serde(e) => write!(f, "Serialization error: {}", e),
            BlerifyError::Jwt(e) => write!(f, "JWT error: {}", e),
            BlerifyError::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

impl From<reqwest::Error> for BlerifyError {
    fn from(err: reqwest::Error) -> Self {
        BlerifyError::Reqwest(err)
    }
}

impl From<std::io::Error> for BlerifyError {
    fn from(err: std::io::Error) -> Self {
        BlerifyError::Io(err)
    }
}

impl From<serde_json::Error> for BlerifyError {
    fn from(err: serde_json::Error) -> Self {
        BlerifyError::Serde(err)
    }
}

impl From<jsonwebtoken::errors::Error> for BlerifyError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        BlerifyError::Jwt(err)
    }
}

// 👇 ESTE VA SEPARADO (IMPORTANTE)
impl From<std::time::SystemTimeError> for BlerifyError {
    fn from(err: std::time::SystemTimeError) -> Self {
        BlerifyError::Custom(format!("Time error: {}", err))
    }
}
