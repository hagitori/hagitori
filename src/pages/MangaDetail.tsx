import { useState, useMemo, useEffect, useRef } from "react";
import { useNavigate, useParams } from "react-router-dom";
import {
  ArrowLeft,
} from "lucide-react";
import { useTranslation } from "@/hooks/useTranslation";
import { useLibraryStore, getCoverUrl } from "@/stores/library-store";
import { useDownloadStore } from "@/stores/download-store";
import { getChapters, getDetails, downloadChapters, getManga, listExtensions } from "@/lib/tauri";
import { Badge } from "@/components/ui";
import { MangaHeader } from "@/components/manga/MangaHeader";
import { ChapterList } from "@/components/manga/ChapterList";
import type { Chapter, MangaDetails } from "@/types";

interface SelectableChapter extends Chapter {
  selected: boolean;
}

export default function MangaDetail() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { id } = useParams<{ id: string }>();
  const mangaId = decodeURIComponent(id || "");

  const entry = useLibraryStore((s) => s.entries[mangaId]);
  const updateChapters = useLibraryStore((s) => s.updateChapters);
  const updateDetails = useLibraryStore((s) => s.updateDetails);
  const setSourceSupportsDetails = useLibraryStore((s) => s.setSourceSupportsDetails);
  const supportsDetails = useLibraryStore(
    (s) => s.sourceSupportsDetails[entry?.manga?.source ?? ""] ?? false,
  );

  const manga = entry?.manga ?? null;
  const storedChapters = entry?.chapters ?? [];
  const storedDetails = entry?.details ?? null;

  const [chapters, setChapters] = useState<SelectableChapter[]>([]);
  const [sortAsc, setSortAsc] = useState(false);
  const [isUpdating, setIsUpdating] = useState(false);
  const [updateError, setUpdateError] = useState<string | null>(null);
  const [details, setDetails] = useState<MangaDetails | null>(storedDetails);
  const [detailsLoading, setDetailsLoading] = useState(false);
  const [scanlatorFilter, setScanlatorFilter] = useState<string>("all");

  const extensionCheckedRef = useRef<string>("");
  const detailsFetchedRef = useRef<string>("");

  const rawCover = details?.cover || manga?.cover || undefined;
  const coverSrc = getCoverUrl(rawCover);

  // update supportsDetails based on current extension (may have changed)
  useEffect(() => {
    const source = entry?.manga?.source;
    if (!source || extensionCheckedRef.current === source) return;
    extensionCheckedRef.current = source;
    listExtensions()
      .then((exts) => {
        const ext = exts.find((e) => e.id === source);
        if (ext) setSourceSupportsDetails(source, ext.supportsDetails);
      })
      .catch(() => {});
  }, [entry?.manga?.source]);

  useEffect(() => {
    if (!mangaId || !supportsDetails) return;
    if (storedDetails) {
      if (!details) setDetails(storedDetails);
      detailsFetchedRef.current = mangaId;
      return;
    }
    if (detailsFetchedRef.current === mangaId) return;
    detailsFetchedRef.current = mangaId;
    setDetailsLoading(true);
    getDetails(mangaId, entry?.manga?.source ?? "")
      .then((d) => {
        setDetails(d);
        updateDetails(mangaId, d);
      })
      .catch(() => {})
      .finally(() => setDetailsLoading(false));
  }, [mangaId, supportsDetails]);

  useEffect(() => {
    if (storedChapters.length > 0 && chapters.length === 0) {
      setChapters(storedChapters.map((ch) => ({ ...ch, selected: false })));
    }
  }, [storedChapters]);

  const selectedCount = useMemo(
    () => chapters.filter((ch) => ch.selected).length,
    [chapters],
  );

  const scanlators = useMemo(() => {
    const set = new Set<string>();
    for (const ch of chapters) {
      if (ch.scanlator) set.add(ch.scanlator);
    }
    return Array.from(set).sort();
  }, [chapters]);

  const sortedChapters = useMemo(() => {
    const filtered = scanlatorFilter === "all"
      ? [...chapters]
      : chapters.filter((ch) => ch.scanlator === scanlatorFilter);
    return filtered.sort((a, b) => {
      const na = parseFloat(a.number) || 0;
      const nb = parseFloat(b.number) || 0;
      return sortAsc ? na - nb : nb - na;
    });
  }, [chapters, sortAsc, scanlatorFilter]);

  const handleUpdate = async () => {
    if (!manga) return;
    setIsUpdating(true);
    setUpdateError(null);

    try {
      // first update chapters (may open browser for getManga)
      let newChapters = await getChapters(manga.id, manga.source);

      if (newChapters.length === 0 && manga.url) {
        await getManga(manga.url);
        newChapters = await getChapters(manga.id, manga.source);
      }

      if (newChapters.length === 0 && chapters.length > 0) {
        setUpdateError(t("manga.updateEmpty"));
        return;
      }

      await updateChapters(manga.id, newChapters);
      setChapters(newChapters.map((ch) => ({ ...ch, selected: false })));

      // then update details (sequential avoids 2 simultaneous browsers)
      if (supportsDetails) {
        try {
          const d = await getDetails(manga.id, manga.source);
          setDetails(d);
          await updateDetails(manga.id, d);
        } catch {
        }
      }
    } catch (err) {
      setUpdateError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsUpdating(false);
    }
  };

  const toggleChapter = (chId: string) => {
    setChapters((prev) =>
      prev.map((ch) =>
        ch.id === chId ? { ...ch, selected: !ch.selected } : ch,
      ),
    );
  };

  const selectAll = () => {
    const visibleIds = new Set(sortedChapters.map((ch) => ch.id));
    const allVisibleSelected = sortedChapters.every((ch) => ch.selected);
    if (allVisibleSelected) {
      setChapters((prev) => prev.map((ch) => ({ ...ch, selected: false })));
    } else {
      setChapters((prev) =>
        prev.map((ch) =>
          visibleIds.has(ch.id) ? { ...ch, selected: true } : ch,
        ),
      );
    }
  };

  const invertSelection = () => {
    const visibleIds = new Set(sortedChapters.map((ch) => ch.id));
    setChapters((prev) =>
      prev.map((ch) =>
        visibleIds.has(ch.id) ? { ...ch, selected: !ch.selected } : ch,
      ),
    );
  };

  const handleDownload = async () => {
    const selected = chapters.filter((ch) => ch.selected);
    if (selected.length === 0 || !manga) return;

    const { addToQueue, saveRetryInfo } = useDownloadStore.getState();
    for (const ch of selected) {
      addToQueue({
        mangaName: manga.name,
        chapterNumber: ch.number,
        currentPage: 0,
        totalPages: 0,
        status: "queued",
      });
      saveRetryInfo({
        chapterId: ch.id,
        mangaName: manga.name,
        chapterNumber: ch.number,
        chapterName: ch.name,
        source: manga.source,
        retryCount: 0,
      });
    }

    navigate("/downloads");

    try {
      await downloadChapters(selected, manga.source, manga.name);
    } catch (err) {
      console.error("Download failed:", err);
    }
  };

  if (!manga) {
    return (
      <div className="flex h-full items-center justify-center">
        <p className="text-sm text-muted-foreground">
          {t("common.error")}
        </p>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col overflow-hidden">
      {/* Back button */}
      <button
        type="button"
        onClick={() => navigate(-1)}
        className="mb-4 flex w-fit items-center gap-2 text-sm text-muted-foreground transition-colors hover:text-foreground shrink-0"
      >
        <ArrowLeft size={16} strokeWidth={2} />
        {t("common.back")}
      </button>

      {/* Manga info   fixed */}
      <MangaHeader
        manga={manga}
        details={details}
        coverSrc={coverSrc}
        isUpdating={isUpdating}
        onUpdate={handleUpdate}
      />

      {/* Synopsis, alt titles, tags fixed with max height */}
      {!detailsLoading && details && (
        <div className="mb-4 max-h-[25vh] overflow-y-auto shrink-0 space-y-4">
          {/* Synopsis */}
          <div>
            <h3 className="mb-1.5 text-xs font-semibold uppercase tracking-wider text-muted-foreground/70">
              {t("manga.synopsis")}
            </h3>
            <p className="text-sm leading-relaxed text-foreground/80">
              {details.synopsis || t("manga.noSynopsis")}
            </p>
          </div>

          {/* Author & Artist */}
          {(details.author || details.artist) && (
            <div className="flex flex-wrap gap-x-6 gap-y-2">
              {details.author && (
                <div>
                  <h3 className="mb-0.5 text-xs font-semibold uppercase tracking-wider text-muted-foreground/70">
                    {t("manga.author")}
                  </h3>
                  <p className="text-sm text-foreground/80">{details.author}</p>
                </div>
              )}
              {details.artist && (
                <div>
                  <h3 className="mb-0.5 text-xs font-semibold uppercase tracking-wider text-muted-foreground/70">
                    {t("manga.artist")}
                  </h3>
                  <p className="text-sm text-foreground/80">{details.artist}</p>
                </div>
              )}
            </div>
          )}

          {/* Alt titles */}
          {details.altTitles.length > 0 && (
            <div>
              <h3 className="mb-1.5 text-xs font-semibold uppercase tracking-wider text-muted-foreground/70">
                {t("manga.altTitles")}
              </h3>
              <div className="flex flex-wrap gap-1.5">
                {details.altTitles.map((title) => (
                  <Badge key={title} variant="default" size="sm">
                    {title}
                  </Badge>
                ))}
              </div>
            </div>
          )}

          {/* Tags */}
          {details.tags.length > 0 && (
            <div>
              <h3 className="mb-1.5 text-xs font-semibold uppercase tracking-wider text-muted-foreground/70">
                {t("manga.tags")}
              </h3>
              <div className="flex flex-wrap gap-1.5">
                {details.tags.map((tag) => (
                  <Badge key={tag} variant="outline" size="sm">
                    {tag}
                  </Badge>
                ))}
              </div>
            </div>
          )}
        </div>
      )}

      {/* Update error */}
      {updateError && (
        <div className="mb-3 shrink-0 rounded-lg border border-[hsl(var(--danger))]/20 bg-[hsl(var(--danger))]/5 px-4 py-3 text-sm text-[hsl(var(--danger))]">
          {updateError}
        </div>
      )}

      {/* Chapter list */}
      <ChapterList
        sortedChapters={sortedChapters}
        chapters={chapters}
        scanlators={scanlators}
        scanlatorFilter={scanlatorFilter}
        sortAsc={sortAsc}
        isUpdating={isUpdating}
        selectedCount={selectedCount}
        onToggleChapter={toggleChapter}
        onSelectAll={selectAll}
        onInvertSelection={invertSelection}
        onToggleSort={() => setSortAsc(!sortAsc)}
        onScanlatorFilterChange={setScanlatorFilter}
        onDownload={handleDownload}
      />
    </div>
  );
}
