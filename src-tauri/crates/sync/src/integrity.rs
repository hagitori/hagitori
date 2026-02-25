use sha2::{Digest, Sha256};

use hagitori_core::error::{HagitoriError, Result};

pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    hex_encode(&result)
}

pub struct SizeLimits;

impl SizeLimits {
    /// max size for JavaScript files (2 MB).
    pub const JS_MAX_BYTES: usize = 2 * 1024 * 1024;
    /// max size for icon files (500 KB).
    pub const ICON_MAX_BYTES: usize = 500 * 1024;
    /// max total size per extension (5 MB).
    pub const TOTAL_MAX_BYTES: usize = 5 * 1024 * 1024;

    pub fn validate_file(filename: &str, size: usize) -> Result<()> {
        // S-10: treat all common image formats as icons (500 KB limit).
        let lower = filename.to_ascii_lowercase();
        let is_icon = [".png", ".ico", ".jpg", ".jpeg", ".svg", ".webp", ".gif"]
            .iter()
            .any(|ext| lower.ends_with(ext));

        let limit = if is_icon {
            Self::ICON_MAX_BYTES
        } else {
            Self::JS_MAX_BYTES
        };

        if size > limit {
            return Err(HagitoriError::extension(format!(
                "file '{}' exceeds size limit ({} bytes > {} bytes)",
                filename, size, limit
            )));
        }

        Ok(())
    }

    pub fn validate_total(total: usize, extension_id: &str) -> Result<()> {
        if total > Self::TOTAL_MAX_BYTES {
            return Err(HagitoriError::extension(format!(
                "extension '{}' exceeds max total size ({} bytes > {} bytes)",
                extension_id, total, Self::TOTAL_MAX_BYTES
            )));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}
