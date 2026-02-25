use std::path::Path;

use hagitori_core::error::{HagitoriError, Result};

/// removes a chapter directory and all its contents after successful archiving.
pub fn cleanup_chapter(chapter_dir: &Path) -> Result<()> {
    if !chapter_dir.exists() {
        return Ok(());
    }

    if !chapter_dir.is_dir() {
        return Err(HagitoriError::download(format!(
            "path is not a directory: {}",
            chapter_dir.display()
        )));
    }

    std::fs::remove_dir_all(chapter_dir).map_err(|e| {
        HagitoriError::download(format!(
            "failed to remove directory {}: {e}",
            chapter_dir.display()
        ))
    })?;

    tracing::info!("directory removed: {}", chapter_dir.display());
    Ok(())
}
