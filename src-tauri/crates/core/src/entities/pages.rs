use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// set of page/image URLs for a chapter, with headers and download metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Pages {
    pub chapter_id: String,
    pub chapter_number: String,
    pub manga_name: String,
    pub pages: Vec<String>,
    pub headers: Option<HashMap<String, String>>,
    #[serde(default)]
    pub use_browser: bool,
    pub scanlator: Option<String>,
}

impl Pages {
    pub fn new(
        chapter_id: impl Into<String>,
        chapter_number: impl Into<String>,
        manga_name: impl Into<String>,
        pages: Vec<String>,
    ) -> Self {
        Self {
            chapter_id: chapter_id.into(),
            chapter_number: chapter_number.into(),
            manga_name: manga_name.into(),
            pages,
            headers: None,
            use_browser: false,
            scanlator: None,
        }
    }

    #[must_use]
    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = Some(headers);
        self
    }

    #[must_use]
    pub fn with_scanlator(mut self, scanlator: impl Into<String>) -> Self {
        self.scanlator = Some(scanlator.into());
        self
    }

    pub fn total_pages(&self) -> usize {
        self.pages.len()
    }
}
