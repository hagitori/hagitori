//! chromium instance manager with stealth and Cloudflare bypass.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use chromiumoxide::{Browser, Page};
use futures::StreamExt;

use crate::chrome::{find_chrome, build_matching_user_agent};
use crate::cloudflare::{bypass_cloudflare, CloudflareBypassOptions, CloudflareBypassResult};
use crate::error::BrowserError;
use crate::stealth::{
    build_stealth_config_with_options,
    BrowserOptions, StealthBrowserConfig,
};

#[cfg(target_os = "windows")]
const FALLBACK_STEALTH_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36";

#[cfg(not(target_os = "windows"))]
const FALLBACK_STEALTH_UA: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36";

static DEFAULT_PROFILE_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn set_default_profile_dir(path: PathBuf) {
    let _ = DEFAULT_PROFILE_DIR.set(path);
}

fn get_default_profile_dir() -> Option<PathBuf> {
    DEFAULT_PROFILE_DIR.get().cloned()
}

pub struct BrowserManager {
    browser: Browser,
    _stealth_config: StealthBrowserConfig,
    _handler_handle: tokio::task::JoinHandle<()>,
    custom_user_agent: Option<String>,
    /// UA string that matches the real Chrome version (auto-detected at launch)
    detected_user_agent: String,
    extra_headers: HashMap<String, String>,
    is_headless: bool,
}

impl BrowserManager {
    pub async fn launch() -> Result<Self, BrowserError> {
        let options = BrowserOptions { user_data_dir: get_default_profile_dir(), ..Default::default() };
        Self::launch_with_options(options).await
    }

    pub async fn launch_with_options(mut options: BrowserOptions) -> Result<Self, BrowserError> {
        let chrome = find_chrome()
            .ok_or(BrowserError::ChromeNotFound)?;

        if options.user_data_dir.is_none() {
            options.user_data_dir = get_default_profile_dir();
        }
        if options.user_data_dir.is_none() {
            let fallback = std::env::temp_dir().join("hagitori_browser_profile");
            tokio::fs::create_dir_all(&fallback)
                .await
                .map_err(|e| BrowserError::ProfileCreation(format!("failed to create fallback persistent profile {}: {e}", fallback.display())))?;
            options.user_data_dir = Some(fallback);
        }

        let ua = options.user_agent.clone();
        let headers = options.extra_headers.clone();

        // auto-detect the real Chrome version and build a matching UA
        let detected_ua = build_matching_user_agent(&chrome);

        let stealth_config = build_stealth_config_with_options(&chrome, &options)?;

        tracing::info!(
            "launching browser {} (window: {}x{}, detected UA: {})",
            if options.headless { "headless" } else { "headful" },
            options.window_width,
            options.window_height,
            detected_ua,
        );

        let (browser, mut handler) =
            Browser::launch(stealth_config.browser_config.clone()).await?;

        let handler_handle = tokio::spawn(async move {
            while let Some(event) = handler.next().await {
                if let Err(e) = event {
                    tracing::warn!("browser handler error: {:?}", e);
                    break;
                }
            }
        });

        let is_headless = options.headless;
        Ok(Self {
            browser,
            _stealth_config: stealth_config,
            _handler_handle: handler_handle,
            custom_user_agent: ua,
            detected_user_agent: detected_ua,
            extra_headers: headers,
            is_headless,
        })
    }

    /// returns whether the browser was launched in headless mode.
    pub fn is_headless(&self) -> bool {
        self.is_headless
    }

    pub async fn new_stealth_page(&self, _user_agent: Option<&str>) -> Result<Page, BrowserError> {
        let page = self.browser.new_page("about:blank").await?;

        if !self.extra_headers.is_empty() {
            use chromiumoxide::cdp::browser_protocol::network::SetExtraHttpHeadersParams;
            let headers_map: HashMap<String, String> = self.extra_headers.clone();
            let params = SetExtraHttpHeadersParams::new(
                chromiumoxide::cdp::browser_protocol::network::Headers::new(
                    headers_map.into_iter().map(|(k, v)| (k, serde_json::Value::String(v))).collect::<serde_json::Map<String, serde_json::Value>>()
                )
            );
            page.execute(params).await?;
            tracing::debug!("extra headers applied: {:?}", self.extra_headers.keys().collect::<Vec<_>>());
        }

        tracing::debug!(
            "new stealth page created (mode={}, UA: {})",
            if self.is_headless { "headless" } else { "headful" },
            self.custom_user_agent.as_deref().unwrap_or(&self.detected_user_agent)
        );
        Ok(page)
    }

    pub async fn bypass_cloudflare(
        &self,
        url: &str,
    ) -> Result<CloudflareBypassResult, BrowserError> {
        self.bypass_cloudflare_with_options(url, &CloudflareBypassOptions::default()).await
    }

    pub async fn bypass_cloudflare_with_options(
        &self,
        url: &str,
        options: &CloudflareBypassOptions,
    ) -> Result<CloudflareBypassResult, BrowserError> {
        let page = self.new_stealth_page(None).await?;
        let result = bypass_cloudflare(&page, url, options).await;
        close_page_quietly(page, "bypass_cloudflare").await;
        result
    }

    pub async fn evaluate_js(
        &self,
        url: &str,
        js: &str,
    ) -> Result<serde_json::Value, BrowserError> {
        let page = self.new_stealth_page(None).await?;
        page.goto(url).await?;

        let result = page.evaluate(js).await?;
        let value: serde_json::Value = result.into_value()?;
        close_page_quietly(page, "evaluate_js").await;
        Ok(value)
    }

    pub async fn navigate(
        &self,
        url: &str,
        wait_ms: Option<u64>,
        wait_for_selector: Option<&str>,
    ) -> Result<String, BrowserError> {
        let page = self.new_stealth_page(None).await?;
        page.goto(url).await?;

        if let Some(selector) = wait_for_selector {
            let timeout = std::time::Duration::from_millis(wait_ms.unwrap_or(10_000));
            let start = std::time::Instant::now();
            loop {
                if start.elapsed() >= timeout {
                    close_page_quietly(page, "navigate/timeout").await;
                    return Err(BrowserError::Interaction(format!(
                        "timeout waiting for selector '{}' on {}",
                        selector, url
                    )));
                }
                let js = format!(
                    "document.querySelector('{}') !== null",
                    selector.replace('\'', "\\'")
                );
                let found: bool = page
                    .evaluate(js)
                    .await?
                    .into_value()
                    .unwrap_or(false);
                if found {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
        } else {
            let wait = wait_ms.unwrap_or(2000);
            tokio::time::sleep(std::time::Duration::from_millis(wait)).await;
        }

        let html = page.content().await?;
        close_page_quietly(page, "navigate").await;
        Ok(html)
    }

    pub async fn get_cookies(
        &self,
        url: &str,
    ) -> Result<HashMap<String, String>, BrowserError> {
        let page = self.new_stealth_page(None).await?;
        page.goto(url).await?;

        let browser_cookies = page.get_cookies().await?;
        let cookies: HashMap<String, String> = browser_cookies
            .into_iter()
            .map(|c| (c.name, c.value))
            .collect();

        close_page_quietly(page, "get_cookies").await;
        Ok(cookies)
    }

    pub fn default_user_agent() -> &'static str {
        FALLBACK_STEALTH_UA
    }

    /// returns the auto-detected UA string that matches the real Chrome version.
    pub fn detected_user_agent(&self) -> &str {
        &self.detected_user_agent
    }

    /// alias for `new_stealth_page` used by intercept.rs
    pub async fn new_page(&self, user_agent: Option<&str>) -> Result<Page, BrowserError> {
        self.new_stealth_page(user_agent).await
    }
}

impl Drop for BrowserManager {
    fn drop(&mut self) {
        self._handler_handle.abort();
        tracing::debug!("BrowserManager dropped   handler task aborted");
    }
}

/// closes a page quietly, logging any error instead of propagating it.
pub async fn close_page_quietly(page: Page, context: &str) {
    if let Err(e) = page.close().await {
        tracing::debug!("failed to close page ({}): {:?}", context, e);
    }
}
