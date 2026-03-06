//! # hagitori-browser
//!
//! chromium browser automation for Cloudflare bypass, request interception,
//! and cookie extraction. Built on chromiumoxide.
//!
//! ## Capabilities
//! - **Intercept requests**   capture HTTP requests (GET/POST) matching URL patterns
//! - **Intercept responses**   capture HTTP responses with body, status, headers
//! - **Extract cookies**   get all cookies from a page
//! - **Cloudflare bypass**   solve JS challenges via headful browser navigation

pub mod error;
pub mod types;
pub use error::BrowserError;
pub use types::{InterceptedPageData, InterceptedRequest, InterceptedResponse};

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod chrome;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod cloudflare;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod config;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod intercept;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod manager;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod stealth;

#[cfg(any(target_os = "android", target_os = "ios"))]
mod mobile;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use chromiumoxide::Page;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use chrome::{build_matching_user_agent, detect_chrome_version, find_chrome};
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use cloudflare::{CloudflareBypassOptions, CloudflareBypassResult, is_cloudflare_challenge};
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use intercept::{download_image_with_page, intercept_all, intercept_requests, intercept_responses};
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use manager::{close_page_quietly, set_default_profile_dir, BrowserManager};
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use stealth::{build_config, build_stealth_config_with_options, BrowserOptions, LaunchConfig, StealthBrowserConfig};

#[cfg(any(target_os = "android", target_os = "ios"))]
pub use mobile::{
	BrowserManager, BrowserOptions, CloudflareBypassOptions, CloudflareBypassResult,
	LaunchConfig, Page, StealthBrowserConfig, build_config, build_matching_user_agent,
	build_stealth_config_with_options, close_page_quietly, detect_chrome_version,
	download_image_with_page, find_chrome, intercept_all, intercept_requests,
	intercept_responses, is_cloudflare_challenge, set_default_profile_dir,
};
