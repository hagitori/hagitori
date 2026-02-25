use hagitori_config::ExtensionRegistry;
use tempfile::TempDir;

fn setup() -> (TempDir, ExtensionRegistry) {
    let tmp = TempDir::new().unwrap();
    let registry = ExtensionRegistry::new(tmp.path()).unwrap();
    (tmp, registry)
}

#[test]
fn register_catalog_and_get() {
    let (_tmp, registry) = setup();
    registry
        .register_catalog(
            "pt_br.sakura",
            "Sakura Mangás",
            2,
            "pt-br",
            "owner/hagitori-extensions",
            "main",
            "pt-br/sakuramangas",
        )
        .unwrap();

    let ext = registry.get("pt_br.sakura").unwrap().unwrap();
    assert_eq!(ext.extension_id, "pt_br.sakura", "extension ID should match");
    assert!(ext.auto_update, "auto_update should be enabled by default");
    assert_eq!(
        ext.source_repo.as_deref(),
        Some("owner/hagitori-extensions"),
        "source repo should match"
    );
}

#[test]
fn upsert_replaces() {
    let (_tmp, registry) = setup();
    registry
        .register_catalog("test.ext", "V1", 1, "en", "o/r", "main", "en/ext")
        .unwrap();
    registry
        .register_catalog("test.ext", "V2", 2, "en", "o/r2", "main", "en/ext2")
        .unwrap();

    let ext = registry.get("test.ext").unwrap().unwrap();
    assert_eq!(ext.version_id, 2, "version should be updated to 2");
    assert_eq!(ext.name, "V2", "name should be updated to V2");
    assert_eq!(ext.source_repo.as_deref(), Some("o/r2"), "repo should be updated");
}

#[test]
fn list_all() {
    let (_tmp, registry) = setup();
    registry
        .register_catalog("a.ext", "A", 1, "en", "o/r", "main", "en/a")
        .unwrap();
    registry
        .register_catalog("b.ext", "B", 1, "pt-br", "o/r", "main", "pt-br/b")
        .unwrap();

    let all = registry.list_all().unwrap();
    assert_eq!(all.len(), 2, "should list all registered extensions");
}

#[test]
fn remove() {
    let (_tmp, registry) = setup();
    registry
        .register_catalog("x.ext", "X", 1, "en", "o/r", "main", "en/x")
        .unwrap();
    assert!(registry.remove("x.ext").unwrap(), "removal should return true");
    assert!(registry.get("x.ext").unwrap().is_none(), "removed extension should not exist");
    assert!(!registry.remove("x.ext").unwrap(), "second removal should return false");
}

#[test]
fn set_auto_update() {
    let (_tmp, registry) = setup();
    registry
        .register_catalog("z.ext", "Z", 1, "en", "o/r", "main", "en/z")
        .unwrap();

    // catalog registrations default to auto_update = true
    let ext = registry.get("z.ext").unwrap().unwrap();
    assert!(ext.auto_update, "auto_update should be true by default");

    registry.set_auto_update("z.ext", false).unwrap();
    let ext = registry.get("z.ext").unwrap().unwrap();
    assert!(!ext.auto_update, "auto_update should be false after disabling");
}

#[test]
fn get_nonexistent() {
    let (_tmp, registry) = setup();
    assert!(registry.get("nonexistent").unwrap().is_none(), "nonexistent extension should return None");
}
