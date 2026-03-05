export interface Manga {
  id: string;
  name: string;
  cover?: string;
  source: string;
  url?: string;
}

export interface Chapter {
  id: string;
  number: string;
  name: string;
  title?: string;
  date?: string;
  scanlator?: string;
}

export interface DownloadProgress {
  mangaName: string;
  chapterNumber: string;
  currentPage: number;
  totalPages: number;
  status: DownloadStatus;
  savePath?: string;
}

export type DownloadStatus =
  | "queued"
  | "downloading"
  | "processing"
  | "completed"
  | { failed: string };

export interface ExtensionMeta {
  id: string;
  name: string;
  lang: string;
  version: string;
  domains: string[];
  features: string[];
  supportsDetails: boolean;
  languages: string[];
  icon?: string;
}

export interface MangaDetails {
  id: string;
  name: string;
  cover?: string;
  source: string;
  synopsis?: string;
  author?: string;
  artist?: string;
  altTitles: string[];
  tags: string[];
  status?: string;
}

// ---------------------------------------------------------------------------
// sync / catalog
// ---------------------------------------------------------------------------

export interface LibraryEntry {
  manga: Manga;
  chapters: Chapter[];
  details?: MangaDetails;
  updatedAt: number;
}

export interface SourceMeta {
  displayName?: string;
  supportsDetails: boolean;
}

export interface ExtensionCatalog {
  version: number;
  updatedAt: string;
  repo: string;
  branch: string;
  extensions: CatalogEntry[];
}

export interface CatalogEntry {
  id: string;
  name: string;
  lang: string;
  versionId: number;
  path: string;
  entry: string;
  requires: string[];
  icon?: string;
  domains: string[];
  features: string[];
  supportsDetails: boolean;
  languages: string[];
  files: Record<string, string>;
  minAppVersion?: string;
}

export type ExtensionSyncStatus =
  | "not_installed"
  | "up_to_date"
  | "update_available"
  | "local_newer"
  | "orphaned";

export interface ExtensionUpdateInfo {
  id: string;
  name: string;
  lang: string;
  localVersionId?: number;
  remoteVersionId: number;
  status: ExtensionSyncStatus;
  domains: string[];
  features: string[];
  iconUrl?: string;
}

export interface InstalledExtension {
  extensionId: string;
  name: string;
  versionId: number;
  lang: string;
  sourceRepo?: string;
  sourceBranch?: string;
  sourcePath?: string;
  installedAt: string;
  updatedAt?: string;
  autoUpdate: boolean;
}
