//! parsing and validation of extension manifests (`package.json`).

use std::path::Path;

use hagitori_core::entities::ExtensionMeta;
use hagitori_core::error::{HagitoriError, Result};
use serde::{Deserialize, Serialize};

/// current extension API version.
/// increment on breaking changes to the JS API exposed to extensions.
pub const CURRENT_API_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HagitoriFields {
    /// API version the extension expects (safety net for compatibility).
    #[serde(rename = "apiVersion")]
    pub api_version: u32,

    #[serde(rename = "type")]
    pub extension_type: String,

    pub lang: String,

    pub domains: Vec<String>,

    /// required capabilities (e.g. ["browser", "crypto"]).
    #[serde(default)]
    pub capabilities: Vec<String>,

    #[serde(default, rename = "supportsDetails")]
    pub supports_details: bool,

    #[serde(default)]
    pub languages: Vec<String>,

    #[serde(default, rename = "displayName")]
    pub display_name: Option<String>,

    /// relative to the extension directory.
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    /// extension name (e.g. "hagitori.en.mangadex").
    pub name: String,

    pub version: u32,

    #[serde(default)]
    pub description: Option<String>,

    /// JS script entry point (e.g. "src/index.js").
    pub main: String,

    pub hagitori: HagitoriFields,
}

impl ExtensionManifest {
    pub fn from_dir(ext_dir: &Path) -> Result<Self> {
        let pkg_path = ext_dir.join("package.json");

        if !pkg_path.exists() {
            return Err(HagitoriError::extension(format!(
                "package.json not found in {}",
                ext_dir.display()
            )));
        }

        let content = std::fs::read_to_string(&pkg_path).map_err(|e| {
            HagitoriError::extension(format!(
                "failed to read package.json in {}: {e}",
                ext_dir.display()
            ))
        })?;

        let manifest: Self = serde_json::from_str(&content).map_err(|e| {
            HagitoriError::extension(format!(
                "invalid package.json in {}: {e}",
                ext_dir.display()
            ))
        })?;

        manifest.validate()?;
        manifest.validate_api_version()?;

        Ok(manifest)
    }

    /// format: `hagitori.{lang}.{name}` (e.g. `hagitori.en.mangadex`)
    pub fn id(&self) -> &str {
        &self.name
    }

    /// falls back to capitalizing the last part of the ID if `hagitori.displayName` is absent.
    pub fn display_name(&self) -> String {
        if let Some(ref dn) = self.hagitori.display_name
            && !dn.is_empty() {
                return dn.clone();
            }
        // fallback: last part of the ID (after last '.')
        let id = self.id();
        let last_part = id.rsplit('.').next().unwrap_or(id);
        let mut chars = last_part.chars();
        match chars.next() {
            None => last_part.to_string(),
            Some(first) => {
                let upper: String = first.to_uppercase().collect();
                format!("{upper}{}", chars.as_str())
            }
        }
    }

    /// formatted version
    pub fn version_string(&self) -> String {
        format!("0.1.{}", self.version.saturating_sub(1))
    }

    pub fn entry_point(&self) -> &str {
        &self.main
    }

    pub fn requires_browser(&self) -> bool {
        self.hagitori.capabilities.iter().any(|c| c == "browser")
    }

    pub fn requires_crypto(&self) -> bool {
        self.hagitori.capabilities.iter().any(|c| c == "crypto")
    }

    pub fn to_extension_meta(&self) -> ExtensionMeta {
        ExtensionMeta::new(
            self.id(),
            self.display_name(),
            &self.hagitori.lang,
            self.version_string(),
            self.hagitori.domains.clone(),
        )
        .with_features(self.hagitori.capabilities.clone())
        .with_supports_details(self.hagitori.supports_details)
        .with_languages(self.hagitori.languages.clone())
    }

    fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(HagitoriError::extension(
                "package.json: 'name' is required",
            ));
        }
        if self.main.is_empty() {
            return Err(HagitoriError::extension(
                "package.json: 'main' is required",
            ));
        }
        if self.version == 0 {
            return Err(HagitoriError::extension(
                "package.json: 'version' must be >= 1",
            ));
        }
        if self.hagitori.domains.is_empty() {
            return Err(HagitoriError::extension(
                "package.json: 'hagitori.domains' must contain at least one domain",
            ));
        }
        if self.hagitori.lang.is_empty() {
            return Err(HagitoriError::extension(
                "package.json: 'hagitori.lang' is required",
            ));
        }
        if self.hagitori.extension_type.is_empty() {
            return Err(HagitoriError::extension(
                "package.json: 'hagitori.type' is required",
            ));
        }
        Ok(())
    }

    fn validate_api_version(&self) -> Result<()> {
        if self.hagitori.api_version > CURRENT_API_VERSION {
            return Err(HagitoriError::extension(format!(
                "extension '{}' requires API v{}, but runtime supports up to v{CURRENT_API_VERSION}. update Hagitori.",
                self.name, self.hagitori.api_version
            )));
        }
        if self.hagitori.api_version == 0 {
            return Err(HagitoriError::extension(format!(
                "extension '{}': apiVersion must be >= 1",
                self.name
            )));
        }
        Ok(())
    }
}
