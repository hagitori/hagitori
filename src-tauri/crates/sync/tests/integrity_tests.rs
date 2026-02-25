use hagitori_sync::integrity::{sha256_hex, SizeLimits};

#[test]
fn sha256_hex_known_value() {
    assert_eq!(
        sha256_hex(b"hello"),
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
    );
}

#[test]
fn size_limits_validation() {
    // JS files
    assert!(SizeLimits::validate_file("index.js", 1000).is_ok());
    assert!(SizeLimits::validate_file("index.js", 3 * 1024 * 1024).is_err());

    // icons
    assert!(SizeLimits::validate_file("icon.png", 100 * 1024).is_ok());
    assert!(SizeLimits::validate_file("icon.png", 600 * 1024).is_err());

    // total
    assert!(SizeLimits::validate_total(1024 * 1024, "test").is_ok());
    assert!(SizeLimits::validate_total(6 * 1024 * 1024, "test").is_err());
}
