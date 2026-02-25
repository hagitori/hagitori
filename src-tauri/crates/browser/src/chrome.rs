use std::path::PathBuf;

#[cfg(target_os = "linux")]
const LINUX_CANDIDATES: &[&str] = &[
    "google-chrome-stable",
    "google-chrome",
    "chromium-browser",
    "chromium",
];

#[cfg(target_os = "windows")]
const WINDOWS_RELATIVE_PATHS: &[&str] = &[
    "Google\\Chrome\\Application\\chrome.exe",
    "Chromium\\Application\\chrome.exe",
];

pub fn find_chrome() -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        find_chrome_linux()
    }

    #[cfg(target_os = "windows")]
    {
        find_chrome_windows()
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        tracing::warn!("find_chrome() not implemented for this OS");
        None
    }
}

#[cfg(target_os = "linux")]
fn find_chrome_linux() -> Option<PathBuf> {
    for candidate in LINUX_CANDIDATES {
        match std::process::Command::new("which")
            .arg(candidate)
            .output()
        {
            Ok(output) if output.status.success() => {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    let path_buf = PathBuf::from(&path);
                    tracing::info!("chrome found: {}", path);
                    return Some(path_buf);
                }
            }
            _ => continue,
        }
    }

    tracing::warn!("chrome/chromium not found on the system (Linux)");
    None
}

#[cfg(target_os = "windows")]
fn find_chrome_windows() -> Option<PathBuf> {
    let env_vars = ["PROGRAMFILES", "PROGRAMFILES(X86)", "LOCALAPPDATA"];

    for env_var in &env_vars {
        if let Ok(base_dir) = std::env::var(env_var) {
            if base_dir.is_empty() {
                continue;
            }
            for rel_path in WINDOWS_RELATIVE_PATHS {
                let full_path = PathBuf::from(&base_dir).join(rel_path);
                if full_path.exists() {
                    tracing::info!("chrome found: {:?}", full_path);
                    return Some(full_path);
                }
            }
        }
    }

    tracing::warn!("chrome/chromium not found on the system (Windows)");
    None
}

/// detects the chrome version. on Windows reads from registry (chrome.exe --version
/// opens a visible window), on Linux/macOS runs `chrome --version`.
pub fn detect_chrome_version(chrome_path: &std::path::Path) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        detect_chrome_version_windows(chrome_path)
    }

    #[cfg(not(target_os = "windows"))]
    {
        detect_chrome_version_unix(chrome_path)
    }
}

#[cfg(target_os = "windows")]
fn detect_chrome_version_windows(_chrome_path: &std::path::Path) -> Option<String> {
    // try registry first (most reliable on Windows)
    let reg_keys = [
        r"HKLM\Software\Google\Chrome\BLBeacon",
        r"HKCU\Software\Google\Chrome\BLBeacon",
        r"HKLM\Software\Wow6432Node\Google\Chrome\BLBeacon",
    ];

    for key in &reg_keys {
        if let Ok(output) = std::process::Command::new("reg")
            .args(["query", key, "/v", "version"])
            .output()
            && output.status.success()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // output format: "version    REG_SZ    132.0.6834.83"
            if let Some(version) = stdout
                .lines()
                .find(|line| line.contains("version"))
                .and_then(|line| line.split_whitespace().last())
                .map(|v| v.to_string())
                && version.contains('.')
            {
                tracing::info!("detected Chrome version from registry: {}", version);
                return Some(version);
            }
        }
    }

    // fallback: use PowerShell to read file version from the exe
    if _chrome_path.exists() {
        let ps_script = format!(
            "(Get-Item '{}').VersionInfo.FileVersion",
            _chrome_path.display()
        );
        if let Ok(output) = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps_script])
            .output()
            && output.status.success()
        {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if version.contains('.') {
                tracing::info!("detected Chrome version from file metadata: {}", version);
                return Some(version);
            }
        }
    }

    tracing::debug!("could not detect Chrome version on Windows");
    None
}

#[cfg(not(target_os = "windows"))]
fn detect_chrome_version_unix(chrome_path: &std::path::Path) -> Option<String> {
    match std::process::Command::new(chrome_path)
        .arg("--version")
        .output()
    {
        Ok(output) if output.status.success() => {
            let version_str = String::from_utf8_lossy(&output.stdout);
            // output is like "Google Chrome 137.0.7151.68" or "Chromium 137.0.7151.68"
            let version = version_str
                .split_whitespace()
                .last()
                .map(|v| v.to_string());
            if let Some(ref v) = version {
                tracing::info!("detected Chrome version: {}", v);
            }
            version
        }
        Ok(output) => {
            tracing::debug!(
                "chrome --version failed (status={}): {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            );
            None
        }
        Err(e) => {
            tracing::debug!("failed to run chrome --version: {}", e);
            None
        }
    }
}

pub fn build_matching_user_agent(chrome_path: &std::path::Path) -> String {
    let version = detect_chrome_version(chrome_path)
        .map(|v| {
            // ensure we have a full version. If only major, add ".0.0.0"
            if v.contains('.') { v } else { format!("{}.0.0.0", v) }
        })
        .unwrap_or_else(|| "145.0.0.0".to_string());

    #[cfg(target_os = "windows")]
    let ua = format!(
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{} Safari/537.36",
        version
    );

    #[cfg(not(target_os = "windows"))]
    let ua = format!(
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{} Safari/537.36",
        version
    );

    tracing::info!("built matching user-agent: {}", ua);
    ua
}