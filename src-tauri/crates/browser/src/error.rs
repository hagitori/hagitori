use thiserror::Error;

#[derive(Debug, Error)]
pub enum BrowserError {
    #[error("chrome/chromium not found on the system")]
    ChromeNotFound,

    #[error("failed to create profile: {0}")]
    ProfileCreation(String),

    #[error("configuration error: {0}")]
    ConfigBuild(String),

    #[error("interaction error: {0}")]
    Interaction(String),

    #[error("invalid URL: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("{0}")]
    CloudflareTimeout(String),

    #[error("base64 decode failed: {0}")]
    Base64Decode(#[from] base64::DecodeError),

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    #[error(transparent)]
    Cdp(#[from] chromiumoxide::error::CdpError),

    #[error("unsupported platform: {0}")]
    UnsupportedPlatform(String),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
