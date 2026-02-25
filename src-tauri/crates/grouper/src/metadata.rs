use serde::Serialize;

/// ComicInfo.xml metadata embedded inside CBZ archives (ANANSI/ComicRack schema).
#[derive(Debug, Clone, Serialize)]
#[serde(rename = "ComicInfo")]
#[expect(non_snake_case, reason = "fields follow ComicInfo XML schema")]
pub struct ComicInfo {
    pub Title: String,
    pub Series: String,
    pub Number: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub Summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub Writer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub Penciller: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub Genre: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub Web: Option<String>,
    #[serde(rename = "LanguageISO", skip_serializing_if = "Option::is_none")]
    pub Iso639_1: Option<String>,
    pub Manga: String,
}
