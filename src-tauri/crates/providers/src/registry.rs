//! central manga provider registry with domain index.

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use url::Url;

use hagitori_browser::BrowserManager;
use hagitori_core::entities::ExtensionMeta;
use hagitori_core::error::{HagitoriError, Result};
use hagitori_core::provider::MangaProvider;
use hagitori_extensions::{ExtensionLoader, JsRuntime};
use hagitori_http::HttpClient;

/// providers are stored as `Arc<dyn MangaProvider>` so callers can clone
/// a handle and release the registry lock before awaiting async operations.
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn MangaProvider>>,
    domain_index: HashMap<String, String>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            domain_index: HashMap::new(),
        }
    }

    pub fn register(&mut self, provider: Box<dyn MangaProvider>) {
        let provider: Arc<dyn MangaProvider> = Arc::from(provider);
        let meta = provider.meta();

        // remove stale domains from a previous registration of the same provider.
        if self.providers.contains_key(&meta.id) {
            self.domain_index.retain(|_, pid| pid != &meta.id);
        }

        for domain in &meta.domains {
            let normalized = Self::normalize_domain(domain);

            // warn when a domain already belongs to a different provider.
            if let Some(existing) = self.domain_index.get(normalized.as_ref())
                && existing != &meta.id
            {
                tracing::warn!(
                    "domain '{}' already registered for provider '{}', overwriting with '{}'",
                    normalized,
                    existing,
                    meta.id
                );
            }

            tracing::debug!(
                "domain '{}' registered for provider '{}'",
                normalized,
                meta.id
            );
            self.domain_index
                .insert(normalized.into_owned(), meta.id.clone());
        }

        tracing::info!(
            "provider registered: {} ({})   {} domain(s)",
            meta.name,
            meta.id,
            meta.domains.len()
        );

        self.providers.insert(meta.id.clone(), provider);
    }

    pub fn get_provider(&self, id: &str) -> Result<Arc<dyn MangaProvider>> {
        self.providers
            .get(id)
            .cloned()
            .ok_or_else(|| HagitoriError::extension(format!("provider '{}' not found", id)))
    }

    pub fn find_provider_by_url(&self, url: &str) -> Result<Arc<dyn MangaProvider>> {
        let parsed = Url::parse(url)
            .map_err(|e| HagitoriError::extension(format!("invalid URL '{}': {e}", url)))?;

        let host = parsed
            .host_str()
            .ok_or_else(|| HagitoriError::extension(format!("URL has no host: '{}'", url)))?;

        let normalized = Self::normalize_domain(host);

        let provider_id = self.domain_index.get(normalized.as_ref()).ok_or_else(|| {
            HagitoriError::extension(format!("no provider found for domain '{}'", normalized))
        })?;

        self.providers.get(provider_id).cloned().ok_or_else(|| {
            HagitoriError::extension(format!(
                "provider '{}' registered but not found (internal inconsistency)",
                provider_id
            ))
        })
    }

    pub fn list(&self) -> Vec<ExtensionMeta> {
        self.providers.values().map(|p| p.meta()).collect()
    }

    pub fn remove(&mut self, id: &str) -> Option<Arc<dyn MangaProvider>> {
        let provider = self.providers.remove(id)?;

        self.domain_index.retain(|_, pid| pid != id);

        tracing::info!("provider removed: {}", id);
        Some(provider)
    }

    pub fn load_extensions(
        &mut self,
        extensions_dir: &Path,
        http_client: Arc<HttpClient>,
        browser_manager: Arc<tokio::sync::Mutex<Option<Arc<BrowserManager>>>>,
    ) -> Result<usize> {
        let runtime = JsRuntime::with_shared_browser_manager(http_client, browser_manager);

        let runtime = Arc::new(runtime);
        let loader = ExtensionLoader::new(extensions_dir.to_path_buf(), runtime);

        let (extensions, load_errors) = loader.load_all();

        if !load_errors.is_empty() {
            for e in &load_errors {
                tracing::warn!("extension load error: {e}");
            }
        }

        let count = extensions.len();

        for ext in extensions {
            self.register(Box::new(ext));
        }

        tracing::info!(
            "{} extension(s) loaded from {}",
            count,
            extensions_dir.display()
        );

        Ok(count)
    }

    pub fn set_extension_lang(&self, provider_id: &str, lang: &str) -> Result<()> {
        self.get_provider(provider_id)?.set_lang(lang);
        Ok(())
    }

    fn normalize_domain(domain: &str) -> Cow<'_, str> {
        let has_upper = domain.bytes().any(|b| b.is_ascii_uppercase());
        if has_upper {
            let mut lower = domain.to_ascii_lowercase();
            if lower.starts_with("www.") {
                lower.drain(..4);
            }
            Cow::Owned(lower)
        } else {
            match domain.strip_prefix("www.") {
                Some(stripped) => Cow::Borrowed(stripped),
                None => Cow::Borrowed(domain),
            }
        }
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
