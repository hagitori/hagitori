import { useMemo } from "react";
import { useNavigate } from "react-router-dom";
import { BookOpen, Puzzle } from "lucide-react";
import { useTranslation } from "../hooks/useTranslation";
import { useLibraryStore, getCoverUrl } from "../stores/library-store";
import { EmptyState } from "../components/ui";
import { usePlatform } from "@/hooks/usePlatform";

export default function Home() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { isMobile } = usePlatform();

  const entries = useLibraryStore((s) => s.entries);
  const sourceNames = useLibraryStore((s) => s.sourceNames);

  const mangaList = useMemo(
    () =>
      Object.values(entries).sort((a, b) => {
        return b.updatedAt - a.updatedAt;
      }),
    [entries],
  );

  const sourceGroups = useMemo(() => {
    const bySource = new Map<
      string,
      {
        displayName: string;
        items: typeof mangaList;
      }
    >();

    for (const entry of mangaList) {
      const sourceId = entry.manga.source;
      const displayName = sourceNames[sourceId] || sourceId;
      const existing = bySource.get(sourceId);

      if (existing) {
        existing.items.push(entry);
      } else {
        bySource.set(sourceId, { displayName, items: [entry] });
      }
    }

    return Array.from(bySource.entries())
      .map(([sourceId, data]) => ({
        sourceId,
        displayName: data.displayName,
        items: data.items,
      }))
      .sort((a, b) => b.items.length - a.items.length);
  }, [mangaList, sourceNames]);

  const handleOpenManga = (mangaId: string) => {
    navigate(`/manga/${encodeURIComponent(mangaId)}`);
  };

  const handleOpenSource = (sourceId: string) => {
    navigate(`/library/${encodeURIComponent(sourceId)}`);
  };

  return (
    <div className="flex h-full flex-col">
      {/* Header */}
      <div className="mb-6">
        <h1 className="text-2xl font-bold tracking-tight text-foreground">
          {t("library.title")}
        </h1>
        <p className="mt-1 text-sm text-muted-foreground">
          {t("library.subtitle")}
        </p>
      </div>

      {/* Library list */}
      {mangaList.length === 0 ? (
        <div className="mt-8 flex-1">
          <EmptyState
            icon={
              <BookOpen
                size={28}
                strokeWidth={1.5}
                className="text-muted-foreground/50"
              />
            }
            title={t("library.empty")}
            description={t("library.emptyHint")}
          />
        </div>
      ) : isMobile ? (
        <div className="grid grid-cols-3 gap-3">
          {mangaList.map(({ manga, chapters, updatedAt }) => (
            <button
              key={manga.id}
              type="button"
              onClick={() => handleOpenManga(manga.id)}
              className="group rounded-xl border border-border bg-card p-2 text-left transition-colors duration-150 hover:border-primary/30 hover:bg-muted/50"
            >
              {/* Cover */}
              {manga.cover ? (
                <img
                  src={getCoverUrl(manga.cover)}
                  alt={manga.name}
                  className="aspect-[2/3] w-full rounded-lg border border-border object-cover"
                />
              ) : (
                <div className="flex aspect-[2/3] w-full items-center justify-center rounded-lg border border-border bg-muted">
                  <BookOpen
                    size={20}
                    strokeWidth={1.5}
                    className="text-muted-foreground/30"
                  />
                </div>
              )}

              {/* Info */}
              <div className="mt-2 min-w-0">
                <h3 className="line-clamp-2 text-xs font-semibold leading-tight text-foreground">
                  {manga.name}
                </h3>
                <p className="mt-1 truncate text-[10px] text-muted-foreground/70">
                  {sourceNames[manga.source] || manga.source}
                </p>
                <p className="mt-1 text-[10px] text-muted-foreground/55">
                  {t("library.chapterCount", { count: chapters.length })}
                </p>
                <p className="mt-1 text-[10px] text-muted-foreground/45">
                  {new Date(updatedAt).toLocaleDateString()}
                </p>
              </div>
            </button>
          ))}
        </div>
      ) : (
        <div className="grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-3">
          {sourceGroups.map(({ sourceId, displayName, items }) => {
            const preview = items.slice(0, 3);

            return (
              <button
                key={sourceId}
                type="button"
                onClick={() => handleOpenSource(sourceId)}
                className="group rounded-xl border border-border bg-card p-4 text-left transition-colors duration-150 hover:border-primary/30 hover:bg-muted/40"
              >
                <div className="mb-3 flex items-start justify-between gap-3">
                  <div className="min-w-0">
                    <h3 className="truncate text-sm font-semibold text-foreground">
                      {displayName}
                    </h3>
                    <p className="mt-1 text-xs text-muted-foreground">
                      {t("library.mangaCount", { count: items.length })}
                    </p>
                  </div>
                  <Puzzle
                    size={16}
                    strokeWidth={2}
                    className="shrink-0 text-muted-foreground/55 transition-colors group-hover:text-primary"
                  />
                </div>

                <div className="grid grid-cols-3 gap-2">
                  {preview.map(({ manga }) => (
                    <div key={manga.id} className="aspect-[2/3] overflow-hidden rounded-md border border-border bg-muted">
                      {manga.cover ? (
                        <img
                          src={getCoverUrl(manga.cover)}
                          alt={manga.name}
                          className="h-full w-full object-cover"
                        />
                      ) : (
                        <div className="flex h-full w-full items-center justify-center">
                          <BookOpen
                            size={16}
                            strokeWidth={1.5}
                            className="text-muted-foreground/30"
                          />
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
