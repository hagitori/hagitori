import { create } from "zustand";
import { immer } from "zustand/middleware/immer";
import { convertFileSrc } from "@tauri-apps/api/core";
import {
  libraryList,
  libraryAdd,
  libraryRemove,
  libraryUpdateChapters,
  libraryUpdateDetails,
  librarySetSourceMeta,
  libraryGetSourceMeta,
  librarySetExtensionLang,
  libraryGetExtensionLangs,
} from "@/lib/tauri";
import type { Manga, Chapter, MangaDetails, LibraryEntry } from "../types";

export type { LibraryEntry };

interface LibraryStore {
  entries: Record<string, LibraryEntry>;
  sourceNames: Record<string, string>;
  sourceSupportsDetails: Record<string, boolean>;
  extensionLangs: Record<string, string>;
  isLoaded: boolean;

  loadLibrary: () => Promise<void>;
  addToLibrary: (manga: Manga, chapters: Chapter[]) => Promise<void>;
  updateChapters: (mangaId: string, chapters: Chapter[]) => Promise<void>;
  removeFromLibrary: (mangaId: string) => Promise<void>;
  setSourceName: (sourceId: string, displayName: string) => Promise<void>;
  setSourceSupportsDetails: (sourceId: string, supports: boolean) => Promise<void>;
  updateDetails: (mangaId: string, details: MangaDetails) => Promise<void>;
  setExtensionLang: (extensionId: string, lang: string) => Promise<void>;
}

export const useLibraryStore = create<LibraryStore>()(
  immer((set) => ({
    entries: {},
    sourceNames: {},
    sourceSupportsDetails: {},
    extensionLangs: {},
    isLoaded: false,

    loadLibrary: async () => {
      const [entries, sourceMeta, extensionLangs] = await Promise.all([
        libraryList(),
        libraryGetSourceMeta(),
        libraryGetExtensionLangs(),
      ]);

      const sourceNames: Record<string, string> = {};
      const sourceSupportsDetails: Record<string, boolean> = {};
      for (const [id, meta] of Object.entries(sourceMeta)) {
        if (meta.displayName) sourceNames[id] = meta.displayName;
        sourceSupportsDetails[id] = meta.supportsDetails;
      }

      set((state) => {
        state.entries = {};
        for (const entry of entries) {
          state.entries[entry.manga.id] = entry;
        }
        state.sourceNames = sourceNames;
        state.sourceSupportsDetails = sourceSupportsDetails;
        state.extensionLangs = extensionLangs;
        state.isLoaded = true;
      });
    },

    addToLibrary: async (manga, chapters) => {
      await libraryAdd(manga, chapters);
      set((state) => {
        state.entries[manga.id] = { manga, chapters, updatedAt: Date.now() };
      });
    },

    updateChapters: async (mangaId, chapters) => {
      await libraryUpdateChapters(mangaId, chapters);
      set((state) => {
        const existing = state.entries[mangaId];
        if (!existing) return;
        existing.chapters = chapters;
        existing.updatedAt = Date.now();
      });
    },

    removeFromLibrary: async (mangaId) => {
      await libraryRemove(mangaId);
      set((state) => {
        delete state.entries[mangaId];
      });
    },

    setSourceName: async (sourceId, displayName) => {
      await librarySetSourceMeta(sourceId, displayName);
      set((state) => {
        state.sourceNames[sourceId] = displayName;
      });
    },

    setSourceSupportsDetails: async (sourceId, supports) => {
      await librarySetSourceMeta(sourceId, undefined, supports);
      set((state) => {
        state.sourceSupportsDetails[sourceId] = supports;
      });
    },

    updateDetails: async (mangaId, details) => {
      await libraryUpdateDetails(mangaId, details);
      set((state) => {
        const existing = state.entries[mangaId];
        if (!existing) return;
        existing.details = details;
      });
    },

    setExtensionLang: async (extensionId, lang) => {
      await librarySetExtensionLang(extensionId, lang);
      set((state) => {
        state.extensionLangs[extensionId] = lang;
      });
    },
  })),
);

/**
 * Returns a display URL for a stored cover value.
 * - http(s) URL -> returned as-is (direct display)
 * - filesystem path -> converted to asset:// URL via convertFileSrc
 */
export function getCoverUrl(cover?: string): string | undefined {
  if (!cover) return undefined;
  if (cover.startsWith("http")) return cover;
  return convertFileSrc(cover);
}

/**
 * migrate existing localStorage data to SQLite.
 * reads from the old zustand persist key and imports each entry.
 */
export async function migrateFromLocalStorage(): Promise<void> {
  const raw = localStorage.getItem("hagitori-library");
  if (!raw) return;

  try {
    const data = JSON.parse(raw);
    const entries = data?.state?.entries;
    if (!entries || typeof entries !== "object") return;

    for (const [, entry] of Object.entries<any>(entries)) {
      if (!entry?.manga?.id) continue;
      await libraryAdd(entry.manga, entry.chapters ?? []);
      if (entry.details) {
        await libraryUpdateDetails(entry.manga.id, entry.details);
      }
    }

    const sourceNames = data?.state?.sourceNames;
    if (sourceNames && typeof sourceNames === "object") {
      for (const [id, name] of Object.entries<string>(sourceNames)) {
        await librarySetSourceMeta(id, name);
      }
    }

    const sourceSupportsDetails = data?.state?.sourceSupportsDetails;
    if (sourceSupportsDetails && typeof sourceSupportsDetails === "object") {
      for (const [id, supports] of Object.entries<boolean>(sourceSupportsDetails)) {
        await librarySetSourceMeta(id, undefined, supports);
      }
    }

    const extensionLangs = data?.state?.extensionLangs;
    if (extensionLangs && typeof extensionLangs === "object") {
      for (const [id, lang] of Object.entries<string>(extensionLangs)) {
        await librarySetExtensionLang(id, lang);
      }
    }

    localStorage.removeItem("hagitori-library");
    console.info("[migration] library migrated from localStorage to SQLite");
  } catch (err) {
    console.warn("[migration] failed to migrate localStorage:", err);
  }
}
