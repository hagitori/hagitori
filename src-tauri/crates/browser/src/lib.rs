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

pub mod chrome;
pub mod cloudflare;
pub mod config;
pub mod error;
pub mod intercept;
pub mod manager;
pub mod stealth;
pub mod types;

pub use chromiumoxide::Page;
pub use chrome::{build_matching_user_agent, detect_chrome_version, find_chrome};
pub use cloudflare::{CloudflareBypassOptions, CloudflareBypassResult, is_cloudflare_challenge};
pub use stealth::{build_config, build_stealth_config_with_options, BrowserOptions, LaunchConfig, StealthBrowserConfig};
pub use error::BrowserError;
pub use intercept::{download_image_with_page, intercept_all, intercept_requests, intercept_responses};
pub use manager::{close_page_quietly, set_default_profile_dir, BrowserManager};
pub use types::{InterceptedPageData, InterceptedRequest, InterceptedResponse};
