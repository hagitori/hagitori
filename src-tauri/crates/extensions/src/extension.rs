//! `MangaProvider` implementation via JavaScript extensions (QuickJS).

use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicU32, Ordering};

use async_trait::async_trait;
use tokio::sync::{Mutex, Semaphore};

use hagitori_core::entities::{Chapter, ExtensionMeta, Manga, MangaDetails, Pages};
use hagitori_core::error::{HagitoriError, Result};
use hagitori_core::provider::MangaProvider;

use crate::manifest::ExtensionManifest;
use crate::runtime::{JsRuntime, JsWorker};

const MAX_WORKERS: usize = 5;
const MAX_CONSECUTIVE_FAILURES: u32 = 5;

pub struct JsExtension {
    meta: RwLock<ExtensionMeta>,
    source: String,
    script: RwLock<Arc<String>>,
    runtime: Arc<JsRuntime>,
    workers: Mutex<Vec<JsWorker>>,
    worker_semaphore: Arc<Semaphore>,
    consecutive_failures: AtomicU32,
}

impl JsExtension {
    pub fn new(manifest: &ExtensionManifest, script: String, runtime: Arc<JsRuntime>, icon: Option<String>) -> Self {
        let mut meta = manifest.to_extension_meta();
        meta.icon = icon;
        let effective_lang = Self::resolve_lang(&meta.lang, &meta.languages);
        let full_script = Self::build_script(&effective_lang, &meta.id, &script);
        Self {
            meta: RwLock::new(meta),
            source: script,
            script: RwLock::new(Arc::new(full_script)),
            runtime,
            workers: Mutex::new(Vec::new()),
            worker_semaphore: Arc::new(Semaphore::new(MAX_WORKERS)),
            consecutive_failures: AtomicU32::new(0),
        }
    }

    fn resolve_lang(lang: &str, languages: &[String]) -> String {
        if lang == "multi" && !languages.is_empty() {
            if languages.iter().any(|l| l == "en") {
                return "en".to_string();
            }
            return languages[0].clone();
        }
        lang.to_string()
    }

    fn build_script(lang: &str, id: &str, source: &str) -> String {
        let lang_escaped = lang.replace('\\', "\\\\").replace('"', "\\\"");
        let id_escaped = id.replace('\\', "\\\\").replace('"', "\\\"");
        let preamble = format!(
            "var __lang__ = \"{lang_escaped}\";\nvar __id__ = \"{id_escaped}\";\n"
        );
        format!("{preamble}{source}")
    }

    async fn call_js_function(
        &self,
        function_name: &str,
        args: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        // reject calls after too many consecutive failures
        let failures = self.consecutive_failures.load(Ordering::Relaxed);
        if failures >= MAX_CONSECUTIVE_FAILURES {
            let id = self.meta.read().map(|m| m.id.clone()).unwrap_or_default();
            return Err(HagitoriError::extension(format!(
                "extension '{id}' disabled after {failures} consecutive failures"
            )));
        }

        // limit concurrent workers
        let _permit = self.worker_semaphore.acquire().await.map_err(|_| {
            HagitoriError::extension("worker semaphore closed")
        })?;

        // acquire or create a worker
        let worker = self.acquire_worker().await?;

        // call method on worker (async)
        let result = worker.call(function_name, args).await;

        // browser is not closed here, it persists so that subsequent calls
        // can reuse the same browser session with its CF cookies intact.
        // the browser is closed by the download engine after the entire download sequence finishes.

        if result.is_ok() {
            self.consecutive_failures.store(0, Ordering::Relaxed);
            self.release_worker(worker).await;
        } else {
            let count = self.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;
            tracing::warn!(
                "worker discarded after failure ({count}/{MAX_CONSECUTIVE_FAILURES})"
            );
        }

        result
    }

    async fn acquire_worker(&self) -> Result<JsWorker> {
        {
            let mut pool = self.workers.lock().await;
            if let Some(worker) = pool.pop() {
                return Ok(worker);
            }
        }

        // pool empty create new worker (async)
        let (script, has_browser, has_crypto) = {
            let meta = self.meta.read().map_err(|e| HagitoriError::extension(format!("meta lock poisoned: {e}")))?;
            let script = self.script.read().map_err(|e| HagitoriError::extension(format!("script lock poisoned: {e}")))?.clone();
            let has_browser = meta.features.iter().any(|f| f == "browser");
            let has_crypto = meta.features.iter().any(|f| f == "crypto");
            (script, has_browser, has_crypto)
        };
        JsWorker::spawn(
            script,
            self.runtime.to_runtime_data(),
            has_browser,
            has_crypto,
        )
        .await
    }

    async fn release_worker(&self, worker: JsWorker) {
        self.workers.lock().await.push(worker);
    }
}

#[async_trait]
impl MangaProvider for JsExtension {
    fn meta(&self) -> ExtensionMeta {
        self.meta.read().expect("meta lock poisoned").clone()
    }

    async fn get_manga(&self, url: &str) -> Result<Manga> {
        let result = self
            .call_js_function("getManga", vec![serde_json::Value::String(url.to_string())])
            .await?;

        let id = self.meta.read().expect("meta lock poisoned").id.clone();
        json_to_manga(&result, &id)
    }

    async fn get_chapters(&self, manga_id: &str) -> Result<Vec<Chapter>> {
        let result = self
            .call_js_function(
                "getChapters",
                vec![serde_json::Value::String(manga_id.to_string())],
            )
            .await?;

        json_to_chapters(&result)
    }

    async fn get_pages(&self, chapter: &Chapter) -> Result<Pages> {
        let chapter_json = serde_json::to_value(chapter).map_err(|e| {
            HagitoriError::extension(format!("failed to serialize chapter: {e}"))
        })?;

        let result = self
            .call_js_function("getPages", vec![chapter_json])
            .await?;

        json_to_pages(&result)
    }

    async fn get_details(&self, manga_id: &str) -> Result<MangaDetails> {
        let result = self
            .call_js_function(
                "getDetails",
                vec![serde_json::Value::String(manga_id.to_string())],
            )
            .await?;

        let id = self.meta.read().expect("meta lock poisoned").id.clone();
        json_to_details(&result, &id)
    }

    fn set_lang(&self, lang: &str) {
        let new_script = {
            let mut meta = self.meta.write().expect("meta lock poisoned");
            meta.lang = lang.to_string();
            Arc::new(Self::build_script(lang, &meta.id, &self.source))
        };
        *self.script.write().expect("script lock poisoned") = new_script;
        // invalidate all workers so they get recreated with the new script/lang
        self.workers.blocking_lock().clear();
        self.consecutive_failures.store(0, Ordering::Relaxed);
    }
}

fn json_to_manga(val: &serde_json::Value, source: &str) -> Result<Manga> {
    let mut manga: Manga = serde_json::from_value(val.clone()).map_err(|e| {
        HagitoriError::extension(format!("getManga: failed to deserialize: {e}"))
    })?;
    manga.source = source.to_string();
    Ok(manga)
}

fn json_to_chapters(val: &serde_json::Value) -> Result<Vec<Chapter>> {
    serde_json::from_value(val.clone()).map_err(|e| {
        HagitoriError::extension(format!("getChapters: failed to deserialize: {e}"))
    })
}

fn json_to_pages(val: &serde_json::Value) -> Result<Pages> {
    serde_json::from_value(val.clone()).map_err(|e| {
        HagitoriError::extension(format!("getPages: failed to deserialize: {e}"))
    })
}

fn json_to_details(val: &serde_json::Value, source: &str) -> Result<MangaDetails> {
    let mut details: MangaDetails = serde_json::from_value(val.clone()).map_err(|e| {
        HagitoriError::extension(format!("getDetails: failed to deserialize: {e}"))
    })?;
    details.source = source.to_string();
    Ok(details)
}
