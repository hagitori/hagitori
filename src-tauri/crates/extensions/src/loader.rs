//! discovers and loads JS extensions from the filesystem.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use base64::{engine::general_purpose::STANDARD, Engine as _};

use hagitori_core::error::{HagitoriError, Result};
use hagitori_core::provider::MangaProvider;

use crate::extension::JsExtension;
use crate::manifest::ExtensionManifest;
use crate::runtime::JsRuntime;

fn load_icon(ext_dir: &Path, manifest: &ExtensionManifest) -> Option<String> {
    // try path declared in package.json, then fallback to icon.png
    let icon_paths = [
        manifest.hagitori.icon.as_deref().map(|p| ext_dir.join(p)),
        Some(ext_dir.join("icon.png")),
    ];

    for icon_path in icon_paths.into_iter().flatten() {
        if icon_path.exists()
            && let Ok(bytes) = std::fs::read(&icon_path) {
                let b64 = STANDARD.encode(&bytes);
                return Some(format!("data:image/png;base64,{b64}"));
            }
    }
    None
}

pub struct ExtensionLoader {
    extensions_dir: PathBuf,
    runtime: Arc<JsRuntime>,
}

/// checks whether a directory contains a valid Hagitori extension `package.json`.
fn is_extension_dir(dir: &Path) -> bool {
    dir.join("package.json").exists()
}

impl ExtensionLoader {
    pub fn new(extensions_dir: PathBuf, runtime: Arc<JsRuntime>) -> Self {
        Self {
            extensions_dir,
            runtime,
        }
    }

    pub fn load_all(&self) -> (Vec<JsExtension>, Vec<HagitoriError>) {
        if !self.extensions_dir.exists() {
            tracing::warn!(
                "extensions directory not found: {}",
                self.extensions_dir.display()
            );
            return (Vec::new(), Vec::new());
        }

        let mut extensions = Vec::new();
        let mut errors = Vec::new();

        self.scan_dir_recursive(&self.extensions_dir, &mut extensions, &mut errors, 0);

        tracing::info!(
            "extensions loaded: {} success, {} errors",
            extensions.len(),
            errors.len()
        );

        (extensions, errors)
    }

    fn scan_dir_recursive(
        &self,
        dir: &Path,
        extensions: &mut Vec<JsExtension>,
        errors: &mut Vec<HagitoriError>,
        depth: usize,
    ) {
        const MAX_DEPTH: usize = 4;
        if depth > MAX_DEPTH {
            return;
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("error reading directory {}: {e}", dir.display());
                return;
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!("error reading directory entry: {e}");
                    continue;
                }
            };

            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            if let Some(name) = path.file_name().and_then(|n| n.to_str())
                && name.starts_with('.')
            {
                continue;
            }

            // this directory has package.json? -> it's an extension
            if is_extension_dir(&path) {
                match self.load_extension(&path) {
                    Ok(ext) => {
                        tracing::info!(
                            "extension loaded: {} ({}) from {}",
                            ext.meta().name,
                            ext.meta().id,
                            path.display()
                        );
                        extensions.push(ext);
                    }
                    Err(e) => {
                        tracing::warn!(
                            "failed to load extension at {}: {e}",
                            path.display()
                        );
                        errors.push(e);
                    }
                }
                continue;
            }

            // no package.json   recurse into subdirectories
            self.scan_dir_recursive(&path, extensions, errors, depth + 1);
        }
    }

    pub fn load_extension(&self, ext_dir: &Path) -> Result<JsExtension> {
        let manifest = ExtensionManifest::from_dir(ext_dir)?;

        // resolve entry point
        let script_path = ext_dir.join(manifest.entry_point());
        if !script_path.exists() {
            return Err(HagitoriError::extension(format!(
                "entry point '{}' not found in {}",
                manifest.entry_point(),
                ext_dir.display()
            )));
        }

        let script = std::fs::read_to_string(&script_path).map_err(|e| {
            HagitoriError::extension(format!(
                "failed to read '{}': {e}",
                script_path.display()
            ))
        })?;

        if script.trim().is_empty() {
            return Err(HagitoriError::extension(format!(
                "script '{}' is empty",
                script_path.display()
            )));
        }

        tracing::debug!(
            "extension loaded: {} v{}   domains: {:?}",
            manifest.id(),
            manifest.version_string(),
            manifest.hagitori.domains,
        );

        let icon = load_icon(ext_dir, &manifest);

        Ok(JsExtension::new(&manifest, script, self.runtime.clone(), icon))
    }
}
