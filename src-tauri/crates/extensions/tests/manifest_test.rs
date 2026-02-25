use hagitori_extensions::ExtensionManifest;
use tempfile::TempDir;

/// creates a temp directory with a package.json and returns (TempDir, manifest).
/// TempDir must be kept alive for the duration of the test.
fn setup_manifest(overrides: &str) -> (TempDir, ExtensionManifest) {
    let json = make_package_json(overrides);
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("package.json"), &json).unwrap();
    std::fs::write(tmp.path().join("index.js"), "class Extension {}").unwrap();
    let manifest = ExtensionManifest::from_dir(tmp.path()).unwrap();
    (tmp, manifest)
}

/// creates a temp directory with a package.json and tries to parse it, returning the Result.
fn try_manifest(json: &str) -> Result<ExtensionManifest, hagitori_core::error::HagitoriError> {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("package.json"), json).unwrap();
    std::fs::write(tmp.path().join("index.js"), "class Extension {}").unwrap();
    ExtensionManifest::from_dir(tmp.path())
}

fn make_package_json(overrides: &str) -> String {
    let base = r#"{
        "name": "hagitori.pt-br.testsite",
        "version": 1,
        "main": "index.js",
        "hagitori": {
            "apiVersion": 1,
            "type": "source",
            "lang": "pt-br",
            "domains": ["testsite.com"]
        }
    }"#;

    if overrides.is_empty() {
        return base.to_string();
    }

    // Merge simples: substitui campos no JSON base
    let mut val: serde_json::Value = serde_json::from_str(base).unwrap();
    let overrides_val: serde_json::Value = serde_json::from_str(overrides).unwrap();

    if let (Some(base_obj), Some(over_obj)) = (val.as_object_mut(), overrides_val.as_object()) {
        for (k, v) in over_obj {
            if k == "hagitori" {
                if let (Some(base_h), Some(over_h)) = (
                    base_obj.get_mut("hagitori").and_then(|h| h.as_object_mut()),
                    v.as_object(),
                ) {
                    for (hk, hv) in over_h {
                        base_h.insert(hk.clone(), hv.clone());
                    }
                }
            } else {
                base_obj.insert(k.clone(), v.clone());
            }
        }
    }

    serde_json::to_string(&val).unwrap()
}

#[test]
fn manifest_parses_package_json() {
    let (_tmp, manifest) = setup_manifest(r#"{"hagitori": {"capabilities": ["browser"]}}"#);
    assert_eq!(manifest.id(), "hagitori.pt-br.testsite");
    assert_eq!(manifest.name, "hagitori.pt-br.testsite");
    assert_eq!(manifest.version, 1);
    assert_eq!(manifest.version_string(), "0.1.0");
    assert_eq!(manifest.hagitori.lang, "pt-br");
    assert_eq!(manifest.hagitori.domains, vec!["testsite.com"]);
    assert!(manifest.requires_browser());
    assert!(!manifest.requires_crypto());
}

#[test]
fn manifest_parses_multiple_domains_and_languages() {
    let (_tmp, manifest) = setup_manifest(r#"{
        "name": "hagitori.multi.reader",
        "hagitori": {
            "lang": "multi",
            "domains": ["reader.com", "api.reader.com"],
            "capabilities": ["browser", "crypto"],
            "languages": ["en", "pt-br", "es"]
        }
    }"#);

    assert_eq!(manifest.hagitori.domains, vec!["reader.com", "api.reader.com"]);
    assert_eq!(manifest.hagitori.languages, vec!["en", "pt-br", "es"]);
    assert!(manifest.requires_browser());
    assert!(manifest.requires_crypto());
}

#[test]
fn manifest_converts_to_extension_meta() {
    let (_tmp, manifest) = setup_manifest(r#"{
        "name": "hagitori.en.test",
        "version": 2,
        "hagitori": {
            "lang": "en",
            "domains": ["test.com"],
            "capabilities": ["crypto"]
        }
    }"#);

    assert_eq!(manifest.version_string(), "0.1.1");
    let meta = manifest.to_extension_meta();
    assert_eq!(meta.id, "hagitori.en.test");
    assert!(meta.requires_crypto());
}

#[test]
fn manifest_validation_rejects_invalid_inputs() {
    let cases = [
        // empty name
        r#"{"name": "", "version": 1, "main": "index.js", "hagitori": {"apiVersion": 1, "type": "source", "lang": "en", "domains": ["test.com"]}}"#,
        // empty domains
        r#"{"name": "hagitori.en.test", "version": 1, "main": "index.js", "hagitori": {"apiVersion": 1, "type": "source", "lang": "en", "domains": []}}"#,
        // empty main
        r#"{"name": "hagitori.en.test", "version": 1, "main": "", "hagitori": {"apiVersion": 1, "type": "source", "lang": "en", "domains": ["test.com"]}}"#,
        // version 0
        r#"{"name": "hagitori.en.test", "version": 0, "main": "index.js", "hagitori": {"apiVersion": 1, "type": "source", "lang": "en", "domains": ["test.com"]}}"#,
    ];

    for json in &cases {
        assert!(try_manifest(json).is_err(), "should reject: {json}");
    }
}

#[test]
fn manifest_version_generates_correct_semver() {
    let build = |vid: u32| -> (TempDir, ExtensionManifest) {
        setup_manifest(&format!(r#"{{
            "name": "hagitori.en.test",
            "version": {vid},
            "hagitori": {{
                "lang": "en",
                "domains": ["test.com"]
            }}
        }}"#))
    };

    let (_t1, m1) = build(1);
    let (_t2, m2) = build(2);
    let (_t3, m3) = build(5);
    let (_t4, m4) = build(10);
    assert_eq!(m1.version_string(), "0.1.0");
    assert_eq!(m2.version_string(), "0.1.1");
    assert_eq!(m3.version_string(), "0.1.4");
    assert_eq!(m4.version_string(), "0.1.9");
}
