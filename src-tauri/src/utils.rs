use hagitori_core::entities::{Chapter, MangaDetails};
use hagitori_grouper::ComicInfo;

/// converts `Result<T, E: Display>` to `Result<T, String>` for tauri commands.
pub(crate) trait CommandResult<T> {
    fn cmd(self) -> Result<T, String>;
}

impl<T, E: std::fmt::Display> CommandResult<T> for Result<T, E> {
    fn cmd(self) -> Result<T, String> {
        self.map_err(|e| e.to_string())
    }
}

pub fn infer_iso639_1(source: &str) -> Option<String> {
    let lang = source
        .split(&['/', '-', '_'][..])
        .find(|part| !part.is_empty())?
        .to_lowercase();

    let normalized = match lang.as_str() {
        "ptbr" | "ptbrasil" => "pt-br",
        "ja" | "jp" => "ja",
        _ if lang.chars().count() >= 2 => {
            let end = lang.char_indices().nth(2).map_or(lang.len(), |(i, _)| i);
            &lang[..end]
        }
        _ => return None,
    };

    Some(normalized.to_string())
}

fn chapter_title(chapter: &Chapter) -> String {
    chapter
        .title
        .as_ref()
        .filter(|v| !v.trim().is_empty())
        .cloned()
        .or_else(|| {
            if chapter.name.trim().is_empty() {
                None
            } else {
                Some(chapter.name.clone())
            }
        })
        .unwrap_or_else(|| format!("Cap. {}", chapter.number))
}

pub fn build_comic_info(
    details: &MangaDetails,
    chapter: &Chapter,
    web: Option<String>,
    iso639_1: Option<String>,
) -> ComicInfo {
    let genre = if details.tags.is_empty() {
        None
    } else {
        Some(details.tags.join(", "))
    };

    ComicInfo {
        Title: chapter_title(chapter),
        Series: details.name.clone(),
        Number: chapter.number.clone(),
        Summary: details.synopsis.clone(),
        Writer: details.author.clone(),
        Penciller: details.artist.clone(),
        Genre: genre,
        Web: web,
        Iso639_1: iso639_1,
        Manga: "Yes".to_string(),
    }
}

/// serializes a ComicInfo to XML string with declaration header.
pub fn serialize_comic_info_xml(info: &ComicInfo) -> Option<String> {
    let body = quick_xml::se::to_string(info).ok()?;
    Some(format!("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n{body}"))
}
