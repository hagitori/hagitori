//! Native rquickjs entity classes for Manga, Chapter, and Pages.

use rquickjs::class::Trace;
use rquickjs::{Class, Ctx, Object};

// ─── Manga ──────────────────────────────────────────────────────────────────

#[derive(Trace, rquickjs::JsLifetime, serde::Serialize)]
#[rquickjs::class(rename = "Manga")]
pub struct JsManga {
    #[qjs(skip_trace)]
    pub id: String,
    #[qjs(skip_trace)]
    pub name: String,
    #[qjs(skip_trace)]
    pub cover: Option<String>,
    #[qjs(skip_trace)]
    pub source: String,
}

#[rquickjs::methods]
#[allow(clippy::needless_pass_by_value)]
impl JsManga {
    #[qjs(constructor)]
    pub fn new(data: Object<'_>) -> rquickjs::Result<Self> {
        Ok(Self {
            id: data.get("id")?,
            name: data.get("name")?,
            cover: data.get("cover").unwrap_or(None),
            source: String::new(),
        })
    }
}

// ─── Chapter ────────────────────────────────────────────────────────────────

#[derive(Trace, rquickjs::JsLifetime, serde::Serialize)]
#[rquickjs::class(rename = "Chapter")]
pub struct JsChapter {
    #[qjs(skip_trace)]
    pub id: String,
    #[qjs(skip_trace)]
    pub number: String,
    #[qjs(skip_trace)]
    pub name: String,
    #[qjs(skip_trace)]
    pub title: Option<String>,
    #[qjs(skip_trace)]
    pub date: Option<String>,
    #[qjs(skip_trace)]
    pub scanlator: Option<String>,
}

#[rquickjs::methods]
#[allow(clippy::needless_pass_by_value)]
impl JsChapter {
    #[qjs(constructor)]
    pub fn new(data: Object<'_>) -> rquickjs::Result<Self> {
        Ok(Self {
            id: data.get("id")?,
            number: data.get("number")?,
            name: data.get("name")?,
            title: data.get("title").unwrap_or(None),
            date: data.get("date").unwrap_or(None),
            scanlator: data.get("scanlator").unwrap_or(None),
        })
    }
}

// ─── Pages ──────────────────────────────────────────────────────────────────

#[derive(Trace, rquickjs::JsLifetime, serde::Serialize)]
#[rquickjs::class(rename = "Pages")]
#[serde(rename_all = "camelCase")]
pub struct JsPages {
    #[qjs(skip_trace)]
    pub chapter_id: String,
    #[qjs(skip_trace)]
    pub chapter_number: String,
    #[qjs(skip_trace)]
    pub manga_name: String,
    #[qjs(skip_trace)]
    pub pages: Vec<String>,
    #[qjs(skip_trace)]
    pub headers: Option<std::collections::HashMap<String, String>>,
    #[qjs(skip_trace)]
    pub use_browser: bool,
}

#[rquickjs::methods]
#[allow(clippy::needless_pass_by_value)]
impl JsPages {
    #[qjs(constructor)]
    pub fn new(data: Object<'_>) -> rquickjs::Result<Self> {
        Ok(Self {
            chapter_id: data.get("id")?,
            chapter_number: data.get("number")?,
            manga_name: data.get("name")?,
            pages: data.get("urls")?,
            headers: data.get("headers").unwrap_or(None),
            use_browser: data.get("useBrowser").unwrap_or(false),
        })
    }
}

// ─── Registration ───────────────────────────────────────────────────────────

pub fn register(ctx: &Ctx<'_>) -> rquickjs::Result<()> {
    let globals = ctx.globals();
    Class::<JsManga>::define(&globals)?;
    Class::<JsChapter>::define(&globals)?;
    Class::<JsPages>::define(&globals)?;
    Ok(())
}
