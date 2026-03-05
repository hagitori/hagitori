import { invoke } from "@tauri-apps/api/core";
import type {
  Manga,
  Chapter,
  ExtensionMeta,
  MangaDetails,
  ExtensionCatalog,
  ExtensionUpdateInfo,
  InstalledExtension,
  LibraryEntry,
  SourceMeta,
} from "../types";

// ---------------------------------------------------------------------------
// Manga
// ---------------------------------------------------------------------------

export async function getManga(url: string): Promise<Manga> {
  return invoke("get_manga", { url });
}

export async function getChapters(mangaId: string, source: string): Promise<Chapter[]> {
  return invoke("get_chapters", { mangaId, source });
}

export async function downloadChapters(chapters: Chapter[], source: string, mangaName: string): Promise<void> {
  return invoke("download_chapters", { chapters, source, mangaName });
}

export async function cancelDownload(): Promise<void> {
  return invoke("cancel_download");
}

export async function getDetails(mangaId: string, source: string): Promise<MangaDetails> {
  return invoke("get_details", { mangaId, source });
}

export async function setExtensionLang(
  extensionId: string,
  lang: string,
): Promise<void> {
  return invoke("set_extension_lang", { extensionId, lang });
}

// ---------------------------------------------------------------------------
// Library
// ---------------------------------------------------------------------------

export async function libraryList(): Promise<LibraryEntry[]> {
  return invoke("library_list");
}

export async function libraryGet(mangaId: string): Promise<LibraryEntry | null> {
  return invoke("library_get", { mangaId });
}

export async function libraryAdd(manga: Manga, chapters: Chapter[]): Promise<void> {
  return invoke("library_add", { manga, chapters });
}

export async function libraryRemove(mangaId: string): Promise<void> {
  return invoke("library_remove", { mangaId });
}

export async function libraryUpdateChapters(mangaId: string, chapters: Chapter[]): Promise<void> {
  return invoke("library_update_chapters", { mangaId, chapters });
}

export async function libraryUpdateDetails(mangaId: string, details: MangaDetails): Promise<void> {
  return invoke("library_update_details", { mangaId, details });
}

export async function libraryUpdateCover(mangaId: string, cover: string): Promise<void> {
  return invoke("library_update_cover", { mangaId, cover });
}

export async function librarySetSourceMeta(
  sourceId: string,
  name?: string,
  supportsDetails?: boolean,
): Promise<void> {
  return invoke("library_set_source_meta", { sourceId, name: name ?? null, supportsDetails: supportsDetails ?? null });
}

export async function libraryGetSourceMeta(): Promise<Record<string, SourceMeta>> {
  return invoke("library_get_source_meta");
}

export async function librarySetExtensionLang(extensionId: string, lang: string): Promise<void> {
  return invoke("library_set_extension_lang", { extensionId, lang });
}

export async function libraryGetExtensionLangs(): Promise<Record<string, string>> {
  return invoke("library_get_extension_langs");
}

// ---------------------------------------------------------------------------
// extensions
// ---------------------------------------------------------------------------

export async function listExtensions(): Promise<ExtensionMeta[]> {
  return invoke("list_extensions");
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

export async function getConfig(): Promise<Record<string, string>> {
  return invoke("get_config");
}

export async function setConfig(key: string, value: string): Promise<void> {
  return invoke("set_config", { entry: { key, value } });
}

export async function getDownloadPath(): Promise<string> {
  return invoke("get_download_path");
}

// ---------------------------------------------------------------------------
// sync / catalog
// ---------------------------------------------------------------------------

export async function fetchCatalog(): Promise<ExtensionCatalog> {
  return invoke("fetch_catalog");
}

export async function checkExtensionUpdates(catalog?: ExtensionCatalog): Promise<ExtensionUpdateInfo[]> {
  return invoke("check_extension_updates", { catalog: catalog ?? null });
}

export async function installCatalogExtension(
  extensionId: string,
  catalog: ExtensionCatalog,
): Promise<void> {
  return invoke("install_catalog_extension", { extensionId, catalog });
}

export async function updateCatalogExtension(
  extensionId: string,
  catalog: ExtensionCatalog,
): Promise<void> {
  return invoke("update_catalog_extension", { extensionId, catalog });
}

export async function removeCatalogExtension(
  extensionId: string,
): Promise<void> {
  return invoke("remove_catalog_extension", { extensionId });
}

export async function listInstalledExtensions(): Promise<InstalledExtension[]> {
  return invoke("list_installed_extensions");
}

export async function setExtensionAutoUpdate(
  extensionId: string,
  enabled: boolean,
): Promise<void> {
  return invoke("set_extension_auto_update", { extensionId, enabled });
}

export async function setCatalogUrl(url: string): Promise<void> {
  return invoke("set_catalog_url", { url });
}

// ---------------------------------------------------------------------------
// Auto-update
// ---------------------------------------------------------------------------

export interface AutoUpdatedEntry {
  id: string;
  name: string;
  fromVersion: number;
  toVersion: number;
}

export interface AutoUpdateFailure {
  id: string;
  name: string;
  error: string;
}

export interface AutoUpdateResult {
  updated: AutoUpdatedEntry[];
  failed: AutoUpdateFailure[];
  skipped: number;
  upToDate: number;
}

export async function autoUpdateExtensions(): Promise<AutoUpdateResult> {
  return invoke("auto_update_extensions");
}
