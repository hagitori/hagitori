import { create } from "zustand";
import { persist, createJSONStorage } from "zustand/middleware";
import { immer } from "zustand/middleware/immer";
import { convertFileSrc } from "@tauri-apps/api/core";
import { cacheCoverImage } from "@/lib/tauri";
import type { Manga, Chapter, MangaDetails } from "../types";

export interface LibraryEntry {
  manga: Manga;
  chapters: Chapter[];
  details?: MangaDetails;
  updatedAt: number; // last update timestamp
}

interface LibraryStore {
  entries: Record<string, LibraryEntry>;

  sourceNames: Record<string, string>;

  sourceSupportsDetails: Record<string, boolean>;

  extensionLangs: Record<string, string>;

  addToLibrary: (manga: Manga, chapters: Chapter[]) => void;

  updateChapters: (mangaId: string, chapters: Chapter[]) => void;

  removeFromLibrary: (mangaId: string) => void;

  setSourceName: (sourceId: string, displayName: string) => void;

  setSourceSupportsDetails: (sourceId: string, supports: boolean) => void;

  updateDetails: (mangaId: string, details: MangaDetails) => void;

  updateCover: (mangaId: string, coverDataUri: string) => void;

  setExtensionLang: (extensionId: string, lang: string) => void;
}

export const useLibraryStore = create<LibraryStore>()(
  persist(
    immer((set) => ({
      entries: {},
      sourceNames: {},
      sourceSupportsDetails: {},
      extensionLangs: {},

      addToLibrary: (manga, chapters) =>
        set((state) => {
          state.entries[manga.id] = {
            manga,
            chapters,
            updatedAt: Date.now(),
          };
        }),

      updateChapters: (mangaId, chapters) =>
        set((state) => {
          const existing = state.entries[mangaId];
          if (!existing) return;
          existing.chapters = chapters;
          existing.updatedAt = Date.now();
        }),

      removeFromLibrary: (mangaId) =>
        set((state) => {
          delete state.entries[mangaId];
        }),

      setSourceName: (sourceId, displayName) =>
        set((state) => {
          state.sourceNames[sourceId] = displayName;
        }),

      setSourceSupportsDetails: (sourceId, supports) =>
        set((state) => {
          state.sourceSupportsDetails[sourceId] = supports;
        }),

      updateDetails: (mangaId, details) =>
        set((state) => {
          const existing = state.entries[mangaId];
          if (!existing) return;
          existing.details = details;
        }),

      updateCover: (mangaId, coverDataUri) =>
        set((state) => {
          const existing = state.entries[mangaId];
          if (!existing) return;
          existing.manga.cover = coverDataUri;
          if (existing.details) {
            existing.details.cover = coverDataUri;
          }
        }),

      setExtensionLang: (extensionId, lang) =>
        set((state) => {
          state.extensionLangs[extensionId] = lang;
        }),
    })),
    {
      name: "hagitori-library",
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({ entries: state.entries, sourceNames: state.sourceNames, sourceSupportsDetails: state.sourceSupportsDetails, extensionLangs: state.extensionLangs }),
    },
  ),
);

/**
 * Returns a display URL for a stored cover value.
 * - data: URI -> returned as-is (legacy base64, still displayable)
 * - http(s) URL -> returned as-is (direct display)
 * - filesystem path -> converted to asset:// URL via convertFileSrc
 * - undefined -> undefined
 */
export function getCoverUrl(cover?: string): string | undefined {
  if (!cover) return undefined;
  if (cover.startsWith("data:")) return cover;   // legacy base64   still works for display
  if (cover.startsWith("http")) return cover;    // remote URL   display directly
  return convertFileSrc(cover);                   // disk path -> asset:// URL
}

/**
 * downloads cover from URL, writes to disk,
 * and updates the library store with the resulting disk path.
 * silently fails caller should not await unless needed.
 */
export async function cacheCoverForManga(mangaId: string, coverUrl: string): Promise<void> {
  try {
    const path = await cacheCoverImage(mangaId, coverUrl);
    useLibraryStore.getState().updateCover(mangaId, path);
  } catch {
    // keep original coverUrl, onError fallback in UI handles display
  }
}

/**
 * for library entries that still have a data: URI cover but have a
 * coverUrl (original remote URL), re-download to disk to eliminate base64 from localStorage.
 * called once at app startup.
 */
export async function migrateCoversToDisk(): Promise<void> {
  const entries = useLibraryStore.getState().entries;
  for (const [mangaId, entry] of Object.entries(entries)) {
    const cover = entry.manga.cover;
    // cover is a data: URI migrate to disk if coverUrl available
    if (cover && cover.startsWith("data:")) {
      const coverUrl = entry.manga.coverUrl;
      if (coverUrl) {
        await cacheCoverForManga(mangaId, coverUrl);
      }
    }
  }
}
