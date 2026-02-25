use async_trait::async_trait;

use crate::entities::{Chapter, ExtensionMeta, Manga, MangaDetails, Pages};
use crate::error::Result;

#[async_trait]
pub trait MangaProvider: Send + Sync {
    fn meta(&self) -> ExtensionMeta;
    async fn get_manga(&self, url: &str) -> Result<Manga>;
    async fn get_chapters(&self, manga_id: &str) -> Result<Vec<Chapter>>;
    async fn get_pages(&self, chapter: &Chapter) -> Result<Pages>;

    async fn get_details(&self, _manga_id: &str) -> Result<MangaDetails> {
        Err(crate::error::HagitoriError::extension(
            "getDetails not implemented for this extension",
        ))
    }

    fn set_lang(&self, _lang: &str) {}
}
