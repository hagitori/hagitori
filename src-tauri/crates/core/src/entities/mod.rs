//! domain entities: manga, chapter, download progress, catalog and extension metadata.

pub mod catalog;
mod chapter;
mod details;
mod download;
mod extension;
mod manga;
mod pages;

pub use catalog::{
    CatalogEntry, ExtensionCatalog, ExtensionSyncStatus, ExtensionUpdateInfo,
    InstalledExtension,
};
pub use chapter::Chapter;
pub use details::MangaDetails;
pub use download::{DownloadProgress, DownloadStatus};
pub use extension::ExtensionMeta;
pub use manga::Manga;
pub use pages::Pages;
