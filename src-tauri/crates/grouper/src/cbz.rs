//! creates CBZ/ZIP archives from chapter page directories.

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use zip::write::FileOptions;
use zip::CompressionMethod;

use hagitori_core::error::{HagitoriError, Result};

use crate::config::GroupFormat;
use crate::metadata::ComicInfo;

/// packages a directory of images into a CBZ or ZIP archive.
pub fn create_archive(
    chapter_dir: &Path,
    output_path: Option<&Path>,
    format: GroupFormat,
    metadata: Option<&ComicInfo>,
) -> Result<PathBuf> {
    if !chapter_dir.exists() || !chapter_dir.is_dir() {
        return Err(HagitoriError::download(format!(
            "directory not found: {}",
            chapter_dir.display()
        )));
    }

    let extension = match format {
        GroupFormat::Cbz => "cbz",
        GroupFormat::Zip => "zip",
    };

    let out = match output_path {
        Some(p) => p.to_path_buf(),
        None => {
            let dir_name = chapter_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("chapter");
            let parent = chapter_dir.parent().unwrap_or(chapter_dir);
            parent.join(format!("{}.{}", dir_name, extension))
        }
    };

    let mut entries: Vec<PathBuf> = std::fs::read_dir(chapter_dir)
        .map_err(|e| {
            HagitoriError::download(format!(
                "failed to read directory {}: {e}",
                chapter_dir.display()
            ))
        })?
        .filter_map(|e| match e {
            Ok(entry) => Some(entry),
            Err(err) => {
                tracing::warn!("skipping unreadable entry in {}: {err}", chapter_dir.display());
                None
            }
        })
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_image_file(p))
        .collect();

    // natural sort so unpadded filenames (1, 2, 10) appear in order.
    entries.sort_by(|a, b| natural_cmp(a.as_path(), b.as_path()));

    if entries.is_empty() {
        return Err(HagitoriError::download(format!(
            "no images found in {}",
            chapter_dir.display()
        )));
    }

    if let Some(parent) = out.parent()
        && !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| {
                HagitoriError::download(format!(
                    "failed to create directory {}: {e}",
                    parent.display()
                ))
            })?;
        }

    let file = File::create(&out).map_err(|e| {
        HagitoriError::download(format!("failed to create file {}: {e}", out.display()))
    })?;

    let mut zip = zip::ZipWriter::new(file);
    let options = FileOptions::<'_, ()>::default().compression_method(CompressionMethod::Stored);

    for (index, entry) in entries.iter().enumerate() {
        let filename = entry
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| format!("image_{:04}", index));

        zip.start_file(&filename, options).map_err(|e| {
            HagitoriError::download(format!("failed to add {} to archive: {e}", filename))
        })?;

        let mut f = File::open(entry).map_err(|e| {
            HagitoriError::download(format!("failed to open {}: {e}", entry.display()))
        })?;

        std::io::copy(&mut f, &mut zip).map_err(|e| {
            HagitoriError::download(format!("failed to write {} to archive: {e}", filename))
        })?;
    }

    if let Some(metadata) = metadata {
        let xml_body = quick_xml::se::to_string(metadata).map_err(|e| {
            HagitoriError::download(format!("failed to serialize ComicInfo.xml: {e}"))
        })?;
        let xml = format!("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n{xml_body}");

        zip.start_file("ComicInfo.xml", options).map_err(|e| {
            HagitoriError::download(format!("failed to add ComicInfo.xml to archive: {e}"))
        })?;

        zip.write_all(xml.as_bytes()).map_err(|e| {
            HagitoriError::download(format!("failed to write ComicInfo.xml to archive: {e}"))
        })?;
    }

    let finished_file = zip.finish().map_err(|e| {
        HagitoriError::download(format!("failed to finalize archive {}: {e}", out.display()))
    })?;

    // flush all data to disk before returning so a crash right after
    // doesn't leave a truncated archive.
    finished_file.sync_all().map_err(|e| {
        HagitoriError::download(format!("failed to sync archive {}: {e}", out.display()))
    })?;

    tracing::info!(
        "{} archive created: {} images -> {}",
        extension.to_uppercase(),
        entries.len(),
        out.display()
    );

    Ok(out)
}

/// numeric segments are compared as integers
/// so `page2.jpg` sorts before `page10.jpg`.
fn natural_cmp(a: &Path, b: &Path) -> std::cmp::Ordering {
    let a_name = a.file_name().map(|n| n.to_string_lossy()).unwrap_or_default();
    let b_name = b.file_name().map(|n| n.to_string_lossy()).unwrap_or_default();

    let mut a_chars = a_name.chars().peekable();
    let mut b_chars = b_name.chars().peekable();

    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(&ac), Some(&bc)) => {
                if ac.is_ascii_digit() && bc.is_ascii_digit() {
                    let an = take_number(&mut a_chars);
                    let bn = take_number(&mut b_chars);
                    match an.cmp(&bn) {
                        std::cmp::Ordering::Equal => continue,
                        other => return other,
                    }
                }
                match ac.to_ascii_lowercase().cmp(&bc.to_ascii_lowercase()) {
                    std::cmp::Ordering::Equal => {
                        a_chars.next();
                        b_chars.next();
                    }
                    other => return other,
                }
            }
        }
    }
}

fn take_number(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> u64 {
    let mut n: u64 = 0;
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            n = n.saturating_mul(10).saturating_add(c as u64 - '0' as u64);
            chars.next();
        } else {
            break;
        }
    }
    n
}

fn is_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_lowercase().as_str(),
                "jpg" | "jpeg" | "png" | "webp" | "gif" | "avif"
            )
        })
}
