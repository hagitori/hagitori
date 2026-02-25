use std::path::Path;

use hagitori_core::error::{HagitoriError, Result};

pub(crate) fn resolve_output_filename(
    page_number: usize,
    source_ext: &str,
    target_format: &str,
) -> (String, String) {
    let ext = if target_format == "original" {
        source_ext.to_string()
    } else {
        normalize_extension(target_format)
    };
    (format!("{:04}.{}", page_number, ext), ext)
}

fn normalize_extension(format: &str) -> String {
    match format.to_lowercase().as_str() {
        "jpeg" | "jpg" => "jpg".to_string(),
        "png" => "png".to_string(),
        "webp" => "webp".to_string(),
        "avif" => "avif".to_string(),
        other => other.to_string(),
    }
}

fn to_image_format(ext: &str) -> Option<image::ImageFormat> {
    match ext.to_lowercase().as_str() {
        "jpg" | "jpeg" => Some(image::ImageFormat::Jpeg),
        "png" => Some(image::ImageFormat::Png),
        "webp" => Some(image::ImageFormat::WebP),
        "avif" => Some(image::ImageFormat::Avif),
        _ => None,
    }
}

/// saves image bytes to disk, converting format if target differs from source.
pub(crate) async fn save_image(
    bytes: &[u8],
    output_path: &Path,
    target_ext: &str,
    page_number: usize,
) -> Result<()> {
    let source_format = image::guess_format(bytes).ok();
    let target_format = to_image_format(target_ext);

    // skip conversion when target matches source or target is unknown
    let needs_conversion = match (source_format, target_format) {
        (Some(src), Some(tgt)) => src != tgt,
        _ => false,
    };

    if needs_conversion {
        let target_fmt = target_format.expect("checked above");
        let owned_bytes = bytes.to_vec();
        let path = output_path.to_path_buf();
        let ext_label = target_ext.to_string();

        // run CPU-intensive image decoding/encoding off the async runtime
        tokio::task::spawn_blocking(move || {
            let img = image::load_from_memory(&owned_bytes).map_err(|e| {
                HagitoriError::download(format!(
                    "failed to decode page {page_number} for conversion: {e}"
                ))
            })?;
            img.save_with_format(&path, target_fmt).map_err(|e| {
                HagitoriError::download(format!(
                    "failed to convert page {page_number} to {ext_label}: {e}",
                ))
            })
        })
        .await
        .map_err(|e| {
            HagitoriError::download(format!(
                "image conversion task panicked for page {page_number}: {e}"
            ))
        })??;
    } else {
        tokio::fs::write(output_path, bytes).await.map_err(|e| {
            HagitoriError::download(format!(
                "failed to save page {page_number} to {}: {e}",
                output_path.display()
            ))
        })?;
    }

    Ok(())
}

pub(crate) fn extract_extension(url: &str) -> &str {
    url.rsplit('/')
        .next()
        .and_then(|segment| {
            let segment = segment.split('?').next().unwrap_or(segment);
            segment.rsplit('.').next()
        })
        .and_then(|ext| match ext.to_ascii_lowercase().as_str() {
            "jpg" | "jpeg" => Some("jpg"),
            "png" => Some("png"),
            "webp" => Some("webp"),
            "gif" => Some("gif"),
            "avif" => Some("avif"),
            _ => None,
        })
        .unwrap_or("jpg")
}
