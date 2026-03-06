use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::BrowserError;
use crate::types::{InterceptedPageData, InterceptedRequest, InterceptedResponse};

const MOBILE_UA: &str = "Mozilla/5.0 (Linux; Android 14) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Mobile Safari/537.36";

#[derive(Debug)]
pub struct Page;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BrowserOptions {
    pub headless: bool,
    pub user_agent: Option<String>,
    pub window_width: u32,
    pub window_height: u32,
    pub user_data_dir: Option<PathBuf>,
    pub extra_args: Vec<String>,
    pub extra_headers: HashMap<String, String>,
}

impl Default for BrowserOptions {
    fn default() -> Self {
        Self {
            headless: true,
            user_agent: None,
            window_width: 1280,
            window_height: 720,
            user_data_dir: None,
            extra_args: Vec::new(),
            extra_headers: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LaunchConfig {
    pub user_data_dir_path: PathBuf,
}

impl LaunchConfig {
    pub fn user_data_dir(&self) -> &Path {
        self.user_data_dir_path.as_path()
    }
}

pub type StealthBrowserConfig = LaunchConfig;

#[derive(Debug, Clone, Default)]
pub struct CloudflareBypassOptions {
    pub auto_click: bool,
}

#[derive(Debug, Clone, Default)]
pub struct CloudflareBypassResult {
    pub cookies: HashMap<String, String>,
    pub user_agent: String,
}

impl CloudflareBypassResult {
    pub fn has_cf_clearance(&self) -> bool {
        self.cookies.contains_key("cf_clearance")
    }

    pub fn cookies_as_header(&self) -> String {
        self.cookies
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("; ")
    }
}

#[derive(Debug, Default)]
pub struct BrowserManager;

impl BrowserManager {
    pub async fn launch() -> Result<Self, BrowserError> {
        Err(BrowserError::UnsupportedPlatform(
            "browser automation is not available on Android/iOS yet".to_string(),
        ))
    }

    pub async fn launch_with_options(_options: BrowserOptions) -> Result<Self, BrowserError> {
        Err(BrowserError::UnsupportedPlatform(
            "browser automation is not available on Android/iOS yet".to_string(),
        ))
    }

    pub fn is_headless(&self) -> bool {
        true
    }

    pub async fn new_page(&self, _user_agent: Option<&str>) -> Result<Page, BrowserError> {
        Err(BrowserError::UnsupportedPlatform(
            "browser pages are not available on Android/iOS yet".to_string(),
        ))
    }

    pub async fn get_cookies(&self, _url: &str) -> Result<HashMap<String, String>, BrowserError> {
        Err(BrowserError::UnsupportedPlatform(
            "cookie extraction via browser is not available on Android/iOS yet".to_string(),
        ))
    }

    pub async fn bypass_cloudflare(
        &self,
        _url: &str,
    ) -> Result<CloudflareBypassResult, BrowserError> {
        Err(BrowserError::UnsupportedPlatform(
            "cloudflare bypass is not available on Android/iOS yet".to_string(),
        ))
    }

    pub async fn bypass_cloudflare_with_options(
        &self,
        _url: &str,
        _options: &CloudflareBypassOptions,
    ) -> Result<CloudflareBypassResult, BrowserError> {
        Err(BrowserError::UnsupportedPlatform(
            "cloudflare bypass is not available on Android/iOS yet".to_string(),
        ))
    }

    pub fn default_user_agent() -> &'static str {
        MOBILE_UA
    }

    pub fn detected_user_agent(&self) -> &str {
        MOBILE_UA
    }
}

pub fn set_default_profile_dir(_path: PathBuf) {}

pub fn find_chrome() -> Option<PathBuf> {
    None
}

pub fn detect_chrome_version(_chrome_path: &Path) -> Option<String> {
    None
}

pub fn build_matching_user_agent(_chrome_path: &Path) -> String {
    MOBILE_UA.to_string()
}

pub fn is_cloudflare_challenge(title: &str) -> bool {
    [
        "Just a moment",
        "Checking your browser",
        "Attention Required",
        "Please Wait",
        "Um momento",
    ]
    .iter()
    .any(|challenge| title.contains(challenge))
}

pub fn build_config(
    _chrome_path: &Path,
    _options: &BrowserOptions,
) -> Result<LaunchConfig, BrowserError> {
    Err(BrowserError::UnsupportedPlatform(
        "desktop browser config is not available on Android/iOS".to_string(),
    ))
}

pub fn build_stealth_config_with_options(
    _chrome_path: &Path,
    _options: &BrowserOptions,
) -> Result<LaunchConfig, BrowserError> {
    Err(BrowserError::UnsupportedPlatform(
        "desktop stealth config is not available on Android/iOS".to_string(),
    ))
}

pub async fn close_page_quietly(_page: Page, _context: &str) {}

pub async fn download_image_with_page(
    _page: &Page,
    _url: &str,
    _timeout_seconds: u64,
) -> Result<Vec<u8>, BrowserError> {
    Err(BrowserError::UnsupportedPlatform(
        "browser-based image download is not available on Android/iOS yet".to_string(),
    ))
}

pub async fn intercept_requests(
    _browser: &BrowserManager,
    _url: &str,
    _patterns: &[&str],
    _timeout_seconds: u64,
) -> Result<Vec<InterceptedRequest>, BrowserError> {
    Err(BrowserError::UnsupportedPlatform(
        "request interception is not available on Android/iOS yet".to_string(),
    ))
}

pub async fn intercept_responses(
    _browser: &BrowserManager,
    _url: &str,
    _patterns: &[&str],
    _timeout_seconds: u64,
) -> Result<Vec<InterceptedResponse>, BrowserError> {
    Err(BrowserError::UnsupportedPlatform(
        "response interception is not available on Android/iOS yet".to_string(),
    ))
}

pub async fn intercept_all(
    _browser: &BrowserManager,
    _url: &str,
    _request_patterns: &[&str],
    _response_patterns: &[&str],
    _timeout_seconds: u64,
) -> Result<InterceptedPageData, BrowserError> {
    Err(BrowserError::UnsupportedPlatform(
        "request/response interception is not available on Android/iOS yet".to_string(),
    ))
}
