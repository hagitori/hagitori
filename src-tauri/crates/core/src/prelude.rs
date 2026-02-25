/// Convenience re-exports of all public hagitori-core types.
///
/// Intended for use inside test files:
/// ```
/// use hagitori_core::prelude::*;
/// ```
pub use crate::entities::{
    CatalogEntry, Chapter, DownloadProgress, DownloadStatus, ExtensionCatalog, ExtensionMeta,
    ExtensionSyncStatus, ExtensionUpdateInfo, InstalledExtension, Manga, MangaDetails, Pages,
};
pub use crate::error::{HagitoriError, Result};
pub use crate::provider::MangaProvider;
