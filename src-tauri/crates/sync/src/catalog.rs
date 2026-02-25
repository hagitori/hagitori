use std::borrow::Cow;
use std::sync::Arc;


use hagitori_core::entities::catalog::ExtensionCatalog;
use hagitori_core::error::{HagitoriError, Result};
use hagitori_http::HttpClient;

const CATALOG_FILENAMES: &[&str] = &["catalog.min.json", "catalog.json"];

pub struct CatalogFetcher {
    http: Arc<HttpClient>,
    catalog_url: String,
}

impl CatalogFetcher {
    pub fn new(http: Arc<HttpClient>, catalog_url: impl Into<String>) -> Self {
        Self {
            http,
            catalog_url: catalog_url.into(),
        }
    }

    pub fn catalog_url(&self) -> &str {
        &self.catalog_url
    }

    /// base URL for extension files (catalog URL without `/catalog.json`).
    pub fn raw_base_url(&self) -> Cow<'_, str> {
        for name in CATALOG_FILENAMES {
            if let Some(base) = self.catalog_url.strip_suffix(&format!("/{name}")) {
                return Cow::Borrowed(base);
            }
            // only strip suffix without `/` if it's the entire URL or preceded by `/`
            if let Some(base) = self.catalog_url.strip_suffix(name)
                && (base.is_empty() || base.ends_with('/'))
            {
                return Cow::Borrowed(base.trim_end_matches('/'));
            }
        }
        Cow::Borrowed(&self.catalog_url)
    }

    pub fn http(&self) -> &Arc<HttpClient> {
        &self.http
    }

    pub async fn fetch(&self) -> Result<ExtensionCatalog> {
        tracing::info!("fetching extension catalog: {}", self.catalog_url);

        let text = self.http.get_text(&self.catalog_url, None).await.map_err(|e| {
            HagitoriError::extension(format!(
                "failed to fetch catalog from {}: {e}",
                self.catalog_url
            ))
        })?;

        let catalog: ExtensionCatalog = serde_json::from_str(&text).map_err(|e| {
            HagitoriError::extension(format!(
                "failed to parse catalog.json: {e}"
            ))
        })?;

        // validate schema version
        if catalog.version != 1 {
            return Err(HagitoriError::extension(format!(
                "unsupported catalog version: {} (expected: 1)",
                catalog.version
            )));
        }

        // warn if empty
        if catalog.extensions.is_empty() {
            tracing::warn!("extension catalog is empty");
        }

        // validate paths (security)
        for entry in &catalog.extensions {
            validate_catalog_path(&entry.path)?;
            validate_catalog_path(&entry.entry)?;
            for req in &entry.requires {
                validate_catalog_path(req)?;
            }
            if let Some(icon) = &entry.icon {
                validate_catalog_path(icon)?;
            }
            for filename in entry.files.keys() {
                validate_catalog_path(filename)?;
            }
        }

        tracing::info!(
            "catalog loaded: {} extension(s), updated at {}",
            catalog.extensions.len(),
            catalog.updated_at
        );

        Ok(catalog)
    }
}

/// security: rejects absolute paths, traversal (`..`), and null characters.
pub fn validate_catalog_path(path: &str) -> Result<()> {
    if path.is_empty() {
        return Err(HagitoriError::extension(
            "empty path in catalog".to_string(),
        ));
    }

    if path.starts_with('/') || path.starts_with('\\') || path.contains(':') {
        return Err(HagitoriError::extension(format!(
            "absolute path not allowed in catalog: '{}'",
            path
        )));
    }

    if path.contains("..") {
        return Err(HagitoriError::extension(format!(
            "path traversal not allowed in catalog: '{}'",
            path
        )));
    }

    if path.contains('\0') {
        return Err(HagitoriError::extension(format!(
            "null character not allowed in catalog path: '{}'",
            path
        )));
    }

    Ok(())
}
