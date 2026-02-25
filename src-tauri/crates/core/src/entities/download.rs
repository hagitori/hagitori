use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DownloadStatus {
    Queued,
    Downloading,
    Processing,
    Completed,
    Failed(String),
}

impl DownloadStatus {
    /// returns `true` if the download has finished (completed or failed).
    pub const fn is_finished(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed(_))
    }

    pub const fn is_active(&self) -> bool {
        matches!(self, Self::Downloading | Self::Processing)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub manga_name: String,
    pub chapter_number: String,
    pub current_page: u32,
    pub total_pages: u32,
    pub status: DownloadStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub save_path: Option<String>,
}

impl DownloadProgress {
    pub fn new(
        manga_name: impl Into<String>,
        chapter_number: impl Into<String>,
        current_page: u32,
        total_pages: u32,
        status: DownloadStatus,
    ) -> Self {
        Self {
            manga_name: manga_name.into(),
            chapter_number: chapter_number.into(),
            current_page,
            total_pages,
            status,
            save_path: None,
        }
    }

    pub fn completed_with_path(
        manga_name: impl Into<String>,
        chapter_number: impl Into<String>,
        total_pages: u32,
        save_path: impl Into<String>,
    ) -> Self {
        Self {
            manga_name: manga_name.into(),
            chapter_number: chapter_number.into(),
            current_page: total_pages,
            total_pages,
            status: DownloadStatus::Completed,
            save_path: Some(save_path.into()),
        }
    }
}
