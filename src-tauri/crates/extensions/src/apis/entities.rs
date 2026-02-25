//! QuickJS entity constructors for Manga, Chapter, Pages, and MangaDetails.

use rquickjs::{Ctx, Value};

/// registers global constructors: Manga(), Chapter(), Pages().
/// defined as real JS functions to support `new Manga(...)`.
pub fn register<'js>(ctx: &Ctx<'js>) -> rquickjs::Result<()> {
    ctx.eval::<Value, _>(r#"
        function Manga(data) {
            this.id = data.id;
            this.name = data.name;
            this.cover = data.cover ?? null;
            this.source = "";
        }

        function Chapter(data) {
            this.id = data.id;
            this.number = data.number;
            this.name = data.name;
            this.title = data.title ?? null;
            this.date = data.date ?? null;
            this.scanlator = data.scanlator ?? null;
        }

        function Pages(data) {
            this.chapter_id = data.id;
            this.chapter_number = data.number;
            this.manga_name = data.name;
            this.pages = data.urls;
            this.headers = data.headers ?? null;
            this.useBrowser = data.useBrowser ?? false;
        }
    "#)?;

    Ok(())
}
