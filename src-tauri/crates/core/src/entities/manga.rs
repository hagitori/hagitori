use serde::{Deserialize, Serialize};

/// summarized manga representation (used in listings and search results).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Manga {
    pub id: String,
    pub name: String,
    pub cover: Option<String>,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub url: Option<String>,
}

impl Manga {
    pub fn new(id: impl Into<String>, name: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            cover: None,
            source: source.into(),
            url: None,
        }
    }

    #[must_use]
    pub fn with_cover(mut self, cover: impl Into<String>) -> Self {
        self.cover = Some(cover.into());
        self
    }
}
