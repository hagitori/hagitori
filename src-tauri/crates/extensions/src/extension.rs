//! `MangaProvider` implementation via JavaScript extensions (QuickJS).

use std::sync::{Arc, Mutex, RwLock};

use async_trait::async_trait;
use tokio::sync::Semaphore;

use hagitori_core::entities::{Chapter, ExtensionMeta, Manga, MangaDetails, Pages};
use hagitori_core::error::{HagitoriError, Result};
use hagitori_core::provider::MangaProvider;

use crate::manifest::ExtensionManifest;
use crate::runtime::{JsRuntime, JsWorker};

const MAX_WORKERS: usize = 5;

pub struct JsExtension {
    meta: RwLock<ExtensionMeta>,
    source: String,
    script: RwLock<Arc<String>>,
    runtime: Arc<JsRuntime>,
    workers: Mutex<Vec<JsWorker>>,
    worker_semaphore: Arc<Semaphore>,
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
        // limit concurrent workers
        let _permit = self.worker_semaphore.acquire().await.map_err(|_| {
            HagitoriError::extension("worker semaphore closed")
        })?;

        // acquire or create a worker
        let worker = self.acquire_worker().await?;

        // call method on worker (async)
        let result = worker.call(function_name, args).await;

        // Auto-close browser after extension execution completes.
        // CF cookies are already propagated to the HTTP session store,
        // so the download engine can work independently with its own browser.
        {
            let bm = self.runtime.browser_manager();
            let mut guard = bm.lock().await;
            if guard.is_some() {
                tracing::info!(
                    "auto-closing browser after extension call '{}'",
                    function_name
                );
                *guard = None;
            }
        }

        // return worker to pool (even on failure   it may still be reusable)
        if result.is_ok() {
            self.release_worker(worker);
        }
        // on failure, the worker is dropped and the task ends

        result
    }

    async fn acquire_worker(&self) -> Result<JsWorker> {
        {
            let mut pool = self.workers.lock().map_err(|e| HagitoriError::extension(format!("mutex poisoned: {e}")))?;
            if let Some(worker) = pool.pop() {
                return Ok(worker);
            }
        }

        // pool empty   create new worker (async)
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

    fn release_worker(&self, worker: JsWorker) {
        let mut pool = self.workers.lock().expect("workers lock poisoned");
        pool.push(worker);
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
        if let Ok(mut pool) = self.workers.lock() {
            pool.clear();
        } else {
            tracing::error!("workers lock poisoned during set_lang   workers not cleared");
        }
    }
}

fn json_to_manga(val: &serde_json::Value, source: &str) -> Result<Manga> {
    let obj = val
        .as_object()
        .ok_or_else(|| HagitoriError::extension("getManga did not return an object"))?;

    let id = obj
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| HagitoriError::extension("manga: 'id' missing or not a string"))?;

    let name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| HagitoriError::extension("manga: 'name' missing or not a string"))?;

    let mut manga = Manga::new(id, name, source);

    if let Some(cover) = obj.get("cover").and_then(|v| v.as_str()) {
        manga = manga.with_cover(cover);
    }

    Ok(manga)
}

fn json_to_chapters(val: &serde_json::Value) -> Result<Vec<Chapter>> {
    let arr = val
        .as_array()
        .ok_or_else(|| HagitoriError::extension("getChapters did not return an array"))?;

    let mut chapters = Vec::with_capacity(arr.len());

    for (i, item) in arr.iter().enumerate() {
        let obj = item.as_object().ok_or_else(|| {
            HagitoriError::extension(format!("chapter [{i}] is not an object"))
        })?;

        let id = obj
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| HagitoriError::extension(format!("chapter [{i}]: 'id' missing")))?;

        let number = obj
            .get("number")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                HagitoriError::extension(format!("chapter [{i}]: 'number' missing"))
            })?;

        let name = obj
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                HagitoriError::extension(format!("chapter [{i}]: 'name' missing"))
            })?;

        let mut chapter = Chapter::new(id, number, name);

        if let Some(title) = obj.get("title").and_then(|v| v.as_str()) {
            chapter = chapter.with_title(title);
        }

        if let Some(date) = obj.get("date").and_then(|v| v.as_str()) {
            chapter = chapter.with_date(date);
        }

        if let Some(scanlator) = obj.get("scanlator").and_then(|v| v.as_str()) {
            chapter = chapter.with_scanlator(scanlator);
        }

        chapters.push(chapter);
    }

    Ok(chapters)
}

fn json_to_pages(val: &serde_json::Value) -> Result<Pages> {
    let obj = val
        .as_object()
        .ok_or_else(|| HagitoriError::extension("getPages did not return an object"))?;

    let chapter_id = obj
        .get("chapter_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| HagitoriError::extension("pages: 'chapter_id' missing"))?;

    let chapter_number = obj
        .get("chapter_number")
        .and_then(|v| v.as_str())
        .ok_or_else(|| HagitoriError::extension("pages: 'chapter_number' missing"))?;

    let manga_name = obj
        .get("manga_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| HagitoriError::extension("pages: 'manga_name' missing"))?;

    let pages_arr = obj
        .get("pages")
        .and_then(|v| v.as_array())
        .ok_or_else(|| HagitoriError::extension("pages: 'pages' missing or not an array"))?;

    let pages: Vec<String> = pages_arr
        .iter()
        .enumerate()
        .map(|(i, v)| {
            v.as_str()
                .map(|s| s.to_string())
                .ok_or_else(|| HagitoriError::extension(format!("pages[{i}] is not a string")))
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let mut result = Pages::new(chapter_id, chapter_number, manga_name, pages);

    if let Some(headers_obj) = obj.get("headers").and_then(|v| v.as_object()) {
        let headers: std::collections::HashMap<String, String> = headers_obj
            .iter()
            .map(|(k, v)| {
                let val = match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                (k.clone(), val)
            })
            .collect();
        if !headers.is_empty() {
            result = result.with_headers(headers);
        }
    }

    if let Some(ub) = obj.get("useBrowser").and_then(|v| v.as_bool()) {
        result.use_browser = ub;
    }

    Ok(result)
}

fn json_to_details(val: &serde_json::Value, source: &str) -> Result<MangaDetails> {
    let obj = val
        .as_object()
        .ok_or_else(|| HagitoriError::extension("getDetails did not return an object"))?;

    let id = obj
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| HagitoriError::extension("details: 'id' missing or not a string"))?;

    let name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| HagitoriError::extension("details: 'name' missing or not a string"))?;

    let mut details = MangaDetails::new(id, name, source);

    if let Some(cover) = obj.get("cover").and_then(|v| v.as_str()) {
        details = details.with_cover(cover);
    }

    if let Some(synopsis) = obj.get("synopsis").and_then(|v| v.as_str()) {
        details = details.with_synopsis(synopsis);
    }

    if let Some(author) = obj.get("author").and_then(|v| v.as_str()) {
        details = details.with_author(author);
    }

    if let Some(artist) = obj.get("artist").and_then(|v| v.as_str()) {
        details = details.with_artist(artist);
    }

    if let Some(status) = obj.get("status").and_then(|v| v.as_str()) {
        details = details.with_status(status);
    }

    if let Some(alt_arr) = obj.get("alt_titles").and_then(|v| v.as_array()) {
        let titles: Vec<String> = alt_arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        details = details.with_alt_titles(titles);
    }

    if let Some(tags_arr) = obj.get("tags").and_then(|v| v.as_array()) {
        let tags: Vec<String> = tags_arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        details = details.with_tags(tags);
    }

    Ok(details)
}
