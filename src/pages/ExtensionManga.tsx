import { useMemo } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { ArrowLeft, BookOpen, Trash2, ChevronRight } from "lucide-react";
import { useTranslation } from "../hooks/useTranslation";
import { useLibraryStore, getCoverUrl } from "../stores/library-store";
import { Button, Badge, EmptyState } from "../components/ui";

export default function ExtensionManga() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { source } = useParams<{ source: string }>();
  const decodedSource = decodeURIComponent(source || "");

  const entries = useLibraryStore((s) => s.entries);
  const removeFromLibrary = useLibraryStore((s) => s.removeFromLibrary);
  const sourceNames = useLibraryStore((s) => s.sourceNames);

  const displayName = sourceNames[decodedSource] || decodedSource;

  const mangaList = useMemo(
    () =>
      Object.values(entries).filter(
        (entry) => entry.manga.source === decodedSource,
      ),
    [entries, decodedSource],
  );

  const handleOpenManga = (mangaId: string) => {
    navigate(`/manga/${encodeURIComponent(mangaId)}`);
  };

  const handleRemove = async (e: React.MouseEvent, mangaId: string) => {
    e.stopPropagation();
    await removeFromLibrary(mangaId);
  };

  return (
    <div className="flex h-full flex-col">
      {/* Back button */}
      <button
        type="button"
        onClick={() => navigate("/")}
        className="mb-6 flex w-fit items-center gap-2 text-sm text-muted-foreground transition-colors hover:text-foreground"
      >
        <ArrowLeft size={16} strokeWidth={2} />
        {t("library.title")}
      </button>

      {/* Header */}
      <div className="mb-6">
        <h1 className="text-2xl font-bold tracking-tight text-foreground">
          {displayName}
        </h1>
        <p className="mt-1 text-sm text-muted-foreground">
          {t("library.mangaCount", { count: mangaList.length })}
        </p>
      </div>

      {/* Manga list */}
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
          />
        </div>
      ) : (
        <div className="flex flex-col gap-2">
          {mangaList.map(({ manga, chapters, updatedAt }) => (
            <button
              key={manga.id}
              type="button"
              onClick={() => handleOpenManga(manga.id)}
              className="group flex items-center gap-4 rounded-xl border border-border bg-card p-3 text-left transition-colors duration-150 hover:border-primary/30 hover:bg-muted/50"
            >
              {/* Cover */}
              {manga.cover ? (
                <img
                  src={getCoverUrl(manga.cover)}
                  alt={manga.name}
                  className="h-[80px] w-[56px] shrink-0 rounded-lg border border-border object-cover"
                />
              ) : (
                <div className="flex h-[80px] w-[56px] shrink-0 items-center justify-center rounded-lg border border-border bg-muted">
                  <BookOpen
                    size={20}
                    strokeWidth={1.5}
                    className="text-muted-foreground/30"
                  />
                </div>
              )}

              {/* Info */}
              <div className="min-w-0 flex-1">
                <h3 className="truncate text-sm font-semibold text-foreground">
                  {manga.name}
                </h3>
                <div className="mt-1.5 flex items-center gap-2">
                  <Badge variant="outline" size="sm">
                    {t("library.chapterCount", { count: chapters.length })}
                  </Badge>
                </div>
                <p className="mt-1 text-[11px] text-muted-foreground/50">
                  {t("library.lastUpdated")}{" "}
                  {new Date(updatedAt).toLocaleDateString()}
                </p>
              </div>

              {/* Actions */}
              <div className="flex items-center gap-1">
                <Button
                  variant="ghost"
                  size="sm"
                  icon={<Trash2 size={14} strokeWidth={2} />}
                  onClick={(e) => handleRemove(e, manga.id)}
                  className="text-muted-foreground/40 opacity-0 transition-opacity group-hover:opacity-100 hover:text-[hsl(var(--danger))]"
                />
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
