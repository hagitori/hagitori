//! stealth browser configuration

use std::path::{Path, PathBuf};
use std::time::Duration;

use chromiumoxide::browser::BrowserConfig;
use tempfile::TempDir;

use crate::error::BrowserError;

const LAUNCH_TIMEOUT: Duration = Duration::from_secs(30);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// alias kept for backward compatibility with manager.rs
pub type StealthBrowserConfig = LaunchConfig;

// ---------------------------------------------------------------------------
// browser options
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BrowserOptions {
    pub headless: bool,
    pub user_agent: Option<String>,
    pub window_width: u32,
    pub window_height: u32,
    pub user_data_dir: Option<std::path::PathBuf>,
    pub extra_args: Vec<String>,
    pub extra_headers: std::collections::HashMap<String, String>,
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
            extra_headers: std::collections::HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// launch config
// ---------------------------------------------------------------------------

pub struct LaunchConfig {
    pub browser_config: BrowserConfig,
    pub user_data_dir_path: PathBuf,
    _temp_dir: Option<TempDir>,
}

impl LaunchConfig {
    pub fn user_data_dir(&self) -> &std::path::Path {
        self.user_data_dir_path.as_path()
    }
}

// ---------------------------------------------------------------------------
// config builder
// ---------------------------------------------------------------------------

/// builds a browser config with .hide().
pub fn build_config(
    chrome_path: &Path,
    options: &BrowserOptions,
) -> Result<LaunchConfig, BrowserError> {
    let (user_data_dir, temp_dir) = resolve_user_data_dir(&options.user_data_dir)?;

    let mut builder = BrowserConfig::builder()
        .chrome_executable(chrome_path)
        .user_data_dir(&user_data_dir)
        .window_size(options.window_width, options.window_height)
        .hide()
        .launch_timeout(LAUNCH_TIMEOUT)
        .request_timeout(REQUEST_TIMEOUT);

    for extra in &options.extra_args {
        builder = builder.arg(extra.as_str());
    }

    if options.headless {
        builder = builder.new_headless_mode();
    } else {
        builder = builder.with_head();
    }

    let config = builder.build().map_err(BrowserError::ConfigBuild)?;

    Ok(LaunchConfig {
        browser_config: config,
        user_data_dir_path: user_data_dir,
        _temp_dir: temp_dir,
    })
}

/// alias kept for backward compatibility with manager.rs
pub fn build_stealth_config_with_options(
    chrome_path: &Path,
    options: &BrowserOptions,
) -> Result<LaunchConfig, BrowserError> {
    build_config(chrome_path, options)
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn resolve_user_data_dir(
    configured: &Option<PathBuf>,
) -> Result<(PathBuf, Option<TempDir>), BrowserError> {
    match configured {
        Some(path) => {
            if !path.exists() {
                std::fs::create_dir_all(path).map_err(|e| {
                    BrowserError::ProfileCreation(format!(
                        "failed to create profile directory {}: {}",
                        path.display(),
                        e
                    ))
                })?;
            }
            cleanup_profile_locks(path);
            Ok((path.clone(), None))
        }
        None => {
            let temp = TempDir::new().map_err(|e| {
                BrowserError::ProfileCreation(format!(
                    "failed to create temp directory: {}",
                    e
                ))
            })?;
            Ok((temp.path().to_path_buf(), Some(temp)))
        }
    }
}

fn cleanup_profile_locks(profile_dir: &std::path::Path) {
    for lock_name in ["SingletonLock", "SingletonCookie", "SingletonSocket"] {
        let lock_path = profile_dir.join(lock_name);
        if lock_path.exists()
            && let Err(e) = std::fs::remove_file(&lock_path)
        {
            tracing::debug!(
                "failed to remove stale profile lock {}: {}",
                lock_path.display(),
                e
            );
        }
    }
}
