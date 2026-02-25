import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { Search as SearchIcon, ArrowRight, BookOpen } from "lucide-react";
import { useTranslation } from "../hooks/useTranslation";
import { getManga, getChapters, listExtensions } from "../lib/tauri";
import { useLibraryStore } from "../stores/library-store";
import {
  Button,
  Input,
  EmptyState,
  SkeletonMangaCard,
  SkeletonChapterRow,
} from "../components/ui";

export default function Search() {
  const { t } = useTranslation();
  const navigate = useNavigate();

  const addToLibrary = useLibraryStore((s) => s.addToLibrary);
  const setSourceName = useLibraryStore((s) => s.setSourceName);
  const setSourceSupportsDetails = useLibraryStore((s) => s.setSourceSupportsDetails);

  const [url, setUrl] = useState("");
  const [isSearching, setIsSearching] = useState(false);
  const [searchError, setSearchError] = useState<string | null>(null);

  const handleSearch = async () => {
    const trimmed = url.trim();
    if (!trimmed) return;

    setIsSearching(true);
    setSearchError(null);

    try {
      const manga = await getManga(trimmed);

      let chapters: import("../types").Chapter[] = [];
      try {
        chapters = await getChapters(manga.id, manga.source);
      } catch {
        chapters = [];
      }

      addToLibrary(manga, chapters);

      try {
        const extensions = await listExtensions();
        const ext = extensions.find((e) => e.id === manga.source);
        if (ext) {
          setSourceName(manga.source, ext.name);
          setSourceSupportsDetails(manga.source, ext.supportsDetails);
        }
      } catch {
      }

      navigate(`/manga/${encodeURIComponent(manga.id)}`);
    } catch (err) {
      const message =
        err instanceof Error
          ? err.message
          : typeof err === "string"
            ? err
            : t("home.errorNotFound");
      setSearchError(message);
    } finally {
      setIsSearching(false);
    }
  };

  return (
    <div className="flex h-full flex-col">
      {/* Header */}
      <div className="mb-6">
        <h1 className="text-2xl font-bold tracking-tight text-foreground">
          {t("search.title")}
        </h1>
        <p className="mt-1 text-sm text-muted-foreground">
          {t("search.subtitle")}
        </p>
      </div>

      {/* Search bar */}
      <div className="flex items-center gap-3">
        <div className="min-w-0 flex-1">
          <Input
            type="url"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleSearch()}
            placeholder={t("search.placeholder")}
            icon={<SearchIcon size={16} strokeWidth={2} />}
            className="h-12 text-base"
          />
        </div>
        <Button
          size="lg"
          className="shrink-0"
          onClick={handleSearch}
          disabled={!url.trim() || isSearching}
          loading={isSearching}
          iconRight={
            !isSearching ? (
              <ArrowRight size={16} strokeWidth={2} />
            ) : undefined
          }
        >
          {isSearching ? t("home.searching") : t("home.searchButton")}
        </Button>
      </div>

      {/* Search error */}
      {searchError && !isSearching && (
        <div className="mt-4 rounded-lg border border-[hsl(var(--danger))]/20 bg-[hsl(var(--danger))]/5 px-4 py-3 text-sm text-[hsl(var(--danger))]">
          {searchError}
        </div>
      )}

      {/* Loading skeletons */}
      {isSearching && (
        <div className="mt-6 flex flex-col gap-3">
          <SkeletonMangaCard />
          {[1, 2, 3, 4, 5].map((i) => (
            <SkeletonChapterRow key={i} />
          ))}
        </div>
      )}

      {/* Empty state */}
      {!isSearching && !searchError && (
        <div className="mt-8 flex-1">
          <EmptyState
            icon={
              <BookOpen
                size={28}
                strokeWidth={1.5}
                className="text-muted-foreground/50"
              />
            }
            title={t("search.hint")}
            description={t("home.searchPlaceholder")}
          />
        </div>
      )}
    </div>
  );
}
