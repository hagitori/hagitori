use hagitori_browser::{
    find_chrome, build_config, BrowserOptions,
    CloudflareBypassResult, is_cloudflare_challenge,
    detect_chrome_version, build_matching_user_agent,
    InterceptedRequest,
};
use std::collections::HashMap;
use std::path::PathBuf;

#[test]
fn find_chrome_returns_valid_path_if_found() {
    if let Some(path) = find_chrome() {
        assert!(!path.to_str().unwrap().is_empty());
        assert!(path.exists(), "chrome found but path doesn't exist: {:?}", path);
    }
}

#[test]
fn build_config_creates_and_drops_temp_dir() {
    let fake = PathBuf::from("/tmp/fake-chrome");
    let opts = BrowserOptions::default();
    let config = build_config(&fake, &opts).unwrap();
    let path = config.user_data_dir().to_path_buf();
    assert!(path.exists(), "temp dir should exist while config is alive");
    drop(config);
    assert!(!path.exists(), "temp dir should be removed on drop");
}

#[test]
fn bypass_result_cf_clearance() {
    let result = CloudflareBypassResult {
        cookies: HashMap::from([("cf_clearance".into(), "token123".into())]),
        user_agent: "Chrome/131".into(),
    };
    assert!(result.has_cf_clearance());
}

#[test]
fn bypass_result_cookies_formatting() {
    // multiple cookies
    let result = CloudflareBypassResult {
        cookies: HashMap::from([
            ("cf_clearance".into(), "abc".into()),
            ("__cf_bm".into(), "xyz".into()),
        ]),
        user_agent: "UA".into(),
    };
    let header = result.cookies_as_header();
    assert!(header.contains("cf_clearance=abc"));
    assert!(header.contains("__cf_bm=xyz"));

    // empty cookies
    let empty = CloudflareBypassResult {
        cookies: HashMap::new(),
        user_agent: "UA".into(),
    };
    assert!(empty.cookies_as_header().is_empty());

    // single cookie   no trailing semicolon
    let single = CloudflareBypassResult {
        cookies: HashMap::from([("one".into(), "1".into())]),
        user_agent: "UA".into(),
    };
    assert_eq!(single.cookies_as_header(), "one=1");
}

#[test]
fn cloudflare_challenge_detection() {
    // positive detections
    assert!(is_cloudflare_challenge("Just a moment..."));
    assert!(is_cloudflare_challenge("Just a moment"));
    assert!(is_cloudflare_challenge("Checking your browser before accessing"));
    assert!(is_cloudflare_challenge("Attention Required! | Cloudflare"));

    // negative normal titles
    assert!(!is_cloudflare_challenge("MangaDex"));
    assert!(!is_cloudflare_challenge("Chapter 1 - My Manga"));
    assert!(!is_cloudflare_challenge(""));
}

#[test]
fn intercepted_request_serde_roundtrip() {
    let req = InterceptedRequest {
        url: "https://api.manga.com/chapters".into(),
        method: "GET".into(),
        post_body: None,
        headers: HashMap::from([("Accept".into(), "application/json".into())]),
        resource_type: Some("XHR".into()),
    };

    let json = serde_json::to_string(&req).unwrap();
    let deser: InterceptedRequest = serde_json::from_str(&json).unwrap();

    assert_eq!(deser.url, req.url);
    assert_eq!(deser.method, req.method);
    assert_eq!(deser.resource_type, req.resource_type);
    assert_eq!(deser.headers.get("Accept").unwrap(), "application/json");
}

#[test]
fn default_user_agent_is_valid() {
    use hagitori_browser::BrowserManager;

    let ua = BrowserManager::default_user_agent();
    assert!(!ua.is_empty());
    assert!(ua.contains("Chrome/"));
    assert!(!ua.contains("Headless"));
    assert!(ua.contains("Linux") || ua.contains("Windows"));
}

#[test]
fn detect_chrome_version_from_installed() {
    if let Some(chrome_path) = find_chrome() {
        let version = detect_chrome_version(&chrome_path);
        if let Some(ref v) = version {
            assert!(v.contains('.'), "version should have dots: {v}");
            let major: u32 = v.split('.').next().unwrap().parse().unwrap();
            assert!(major >= 90, "Chrome version should be >= 90, got {major}");
        }
    }
}

#[test]
fn build_matching_ua_has_real_version() {
    if let Some(chrome_path) = find_chrome() {
        let ua = build_matching_user_agent(&chrome_path);
        assert!(ua.contains("Chrome/"));
        if let Some(version) = detect_chrome_version(&chrome_path) {
            let major = version.split('.').next().unwrap();
            assert!(ua.contains(&format!("Chrome/{major}")));
        }
    }
}
