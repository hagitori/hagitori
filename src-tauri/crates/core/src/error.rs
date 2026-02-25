//! central error types and `Result` alias for Hagitori.

use thiserror::Error;

/// unified error enum covering HTTP, browser, extensions, downloads, I/O and config.
///
/// # Examples
///
/// ```
/// use hagitori_core::error::HagitoriError;
///
/// let err = HagitoriError::http("connection timeout");
/// assert!(err.to_string().contains("timeout"));
/// ```
#[derive(Debug, Error)]
pub enum HagitoriError {
    #[error("HTTP error: {0}")]
    Http(String),

    #[error("browser error: {0}")]
    Browser(String),

    #[error("extension error: {0}")]
    Extension(String),

    #[error("download error: {0}")]
    Download(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("config error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, HagitoriError>;

impl serde::Serialize for HagitoriError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl HagitoriError {
    pub fn http(msg: impl std::fmt::Display) -> Self {
        Self::Http(msg.to_string())
    }

    pub fn browser(msg: impl std::fmt::Display) -> Self {
        Self::Browser(msg.to_string())
    }

    pub fn extension(msg: impl std::fmt::Display) -> Self {
        Self::Extension(msg.to_string())
    }

    pub fn download(msg: impl std::fmt::Display) -> Self {
        Self::Download(msg.to_string())
    }

    pub fn config(msg: impl std::fmt::Display) -> Self {
        Self::Config(msg.to_string())
    }
}


