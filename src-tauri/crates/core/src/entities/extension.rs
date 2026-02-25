use serde::{Deserialize, Serialize};

/// metadata describing an installed extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionMeta {
    pub id: String,
    pub name: String,
    pub lang: String,
    pub version: String,
    pub domains: Vec<String>,
    pub features: Vec<String>,
    #[serde(default)]
    pub supports_details: bool,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

impl ExtensionMeta {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        lang: impl Into<String>,
        version: impl Into<String>,
        domains: Vec<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            lang: lang.into(),
            version: version.into(),
            domains,
            features: Vec::new(),
            supports_details: false,
            languages: Vec::new(),
            icon: None,
        }
    }

    #[must_use]
    pub fn with_features(mut self, features: Vec<String>) -> Self {
        self.features = features;
        self
    }

    #[must_use]
    pub fn with_supports_details(mut self, supports_details: bool) -> Self {
        self.supports_details = supports_details;
        self
    }

    #[must_use]
    pub fn with_languages(mut self, languages: Vec<String>) -> Self {
        self.languages = languages;
        self
    }

    #[must_use]
    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn requires_browser(&self) -> bool {
        self.features.iter().any(|f| f == "browser")
    }

    pub fn requires_crypto(&self) -> bool {
        self.features.iter().any(|f| f == "crypto")
    }
}
