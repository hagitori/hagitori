use serde::{Deserialize, Serialize};

/// full manga details, including synopsis, tags and publication status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MangaDetails {
    pub id: String,
    pub name: String,
    pub cover: Option<String>,
    #[serde(default)]
    pub source: String,
    pub synopsis: Option<String>,
    pub author: Option<String>,
    pub artist: Option<String>,
    #[serde(alias = "alt_titles", default)]
    pub alt_titles: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub status: Option<String>,
}

impl MangaDetails {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            cover: None,
            source: source.into(),
            synopsis: None,
            author: None,
            artist: None,
            alt_titles: Vec::new(),
            tags: Vec::new(),
            status: None,
        }
    }

    #[must_use]
    pub fn with_cover(mut self, url: impl Into<String>) -> Self {
        self.cover = Some(url.into());
        self
    }

    #[must_use]
    pub fn with_synopsis(mut self, synopsis: impl Into<String>) -> Self {
        self.synopsis = Some(synopsis.into());
        self
    }

    #[must_use]
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    #[must_use]
    pub fn with_artist(mut self, artist: impl Into<String>) -> Self {
        self.artist = Some(artist.into());
        self
    }

    #[must_use]
    pub fn with_alt_titles(mut self, titles: Vec<String>) -> Self {
        self.alt_titles = titles;
        self
    }

    #[must_use]
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    #[must_use]
    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }
}
