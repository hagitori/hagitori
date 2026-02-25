import { useMemo } from "react";
import { useNavigate } from "react-router-dom";
import { Puzzle, BookOpen, ChevronRight } from "lucide-react";
import { useTranslation } from "../hooks/useTranslation";
import { useLibraryStore, getCoverUrl } from "../stores/library-store";
import { EmptyState } from "../components/ui";

export default function Home() {
  const { t } = useTranslation();
  const navigate = useNavigate();

  const entries = useLibraryStore((s) => s.entries);
  const sourceNames = useLibraryStore((s) => s.sourceNames);

  const extensionPreviews = useMemo(() => {
    const sourceMap = new Map<string, number>();
    for (const entry of Object.values(entries)) {
      const count = sourceMap.get(entry.manga.source) || 0;
      sourceMap.set(entry.manga.source, count + 1);
    }
    return Array.from(sourceMap.entries()).map(([source, mangaCount]) => {
      const previews = Object.values(entries)
        .filter((e) => e.manga.source === source)
        .slice(0, 4);
      return { source, mangaCount, previews };
    });
  }, [entries]);

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

      {/* Extension grid */}
      {extensionPreviews.length === 0 ? (
        <div className="mt-8 flex-1">
          <EmptyState
            icon={
              <Puzzle
                size={28}
                strokeWidth={1.5}
                className="text-muted-foreground/50"
              />
            }
            title={t("library.empty")}
            description={t("library.emptyHint")}
          />
        </div>
      ) : (
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
          {extensionPreviews.map(({ source, mangaCount, previews }) => (
            <button
              key={source}
              type="button"
              onClick={() => navigate(`/library/${encodeURIComponent(source)}`)}
              className="group flex flex-col gap-3 rounded-xl border border-border bg-card p-4 text-left transition-colors duration-150 hover:border-primary/30 hover:bg-muted/50"
            >
              {/* Preview covers */}
              <div className="flex gap-1.5">
                {previews.map((entry) =>
                  entry.manga.cover ? (
                    <img
                      key={entry.manga.id}
                      src={getCoverUrl(entry.manga.cover)}
                      alt={entry.manga.name}
                      className="h-[72px] w-[50px] shrink-0 rounded-md border border-border object-cover"
                    />
                  ) : (
                    <div
                      key={entry.manga.id}
                      className="flex h-[72px] w-[50px] shrink-0 items-center justify-center rounded-md border border-border bg-muted"
                    >
                      <BookOpen
                        size={16}
                        strokeWidth={1.5}
                        className="text-muted-foreground/30"
                      />
                    </div>
                  ),
                )}
              </div>

              {/* Info */}
              <div className="flex items-center justify-between">
                <div className="min-w-0">
                  <h3 className="truncate text-sm font-semibold text-foreground">
                    {sourceNames[source] || source}
                  </h3>
                  <p className="text-xs text-muted-foreground">
                    {t("library.mangaCount", { count: mangaCount })}
                  </p>
                </div>
                <ChevronRight
                  size={16}
                  strokeWidth={2}
                  className="shrink-0 text-muted-foreground/40 transition-colors group-hover:text-primary"
                />
              </div>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
