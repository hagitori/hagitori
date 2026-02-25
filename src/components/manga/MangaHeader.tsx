import {
  BookOpen,
  RefreshCw,
} from "lucide-react";
import { Button, Badge } from "@/components/ui";
import { useTranslation } from "@/hooks/useTranslation";
import type { Manga, MangaDetails } from "@/types";

export interface MangaHeaderProps {
  manga: Manga;
  details: MangaDetails | null;
  coverSrc: string | undefined;
  isUpdating: boolean;
  onCoverError: () => void;
  onUpdate: () => void;
}

export function MangaHeader({
  manga,
  details,
  coverSrc,
  isUpdating,
  onCoverError,
  onUpdate,
}: MangaHeaderProps) {
  const { t } = useTranslation();
  return (
    <div className="mb-4 flex gap-5 shrink-0">
      {/* Cover */}
      {coverSrc ? (
        <img
          src={coverSrc}
          alt={manga.name}
          className="h-[180px] w-[130px] shrink-0 rounded-xl border border-border object-cover"
          onError={onCoverError}
        />
      ) : (
        <div className="flex h-[180px] w-[130px] shrink-0 items-center justify-center rounded-xl border border-border bg-muted">
          <BookOpen
            size={32}
            strokeWidth={1.5}
            className="text-muted-foreground/30"
          />
        </div>
      )}

      {/* Details */}
      <div className="flex min-w-0 flex-col justify-center">
        <h1 className="text-xl font-bold tracking-tight text-foreground text-balance">
          {manga.name}
        </h1>
        <p className="mt-0.5 text-xs text-muted-foreground/60">
          {manga.source}
        </p>
        <div className="mt-3 flex flex-wrap items-center gap-1.5">
          {details?.status && (
            <Badge variant="primary" size="md">
              {details.status}
            </Badge>
          )}
          <Badge variant="default" size="md">
            {manga.source}
          </Badge>
        </div>
        {/* update button */}
        <div className="mt-3">
          <Button
            variant="outline"
            size="sm"
            icon={
              <RefreshCw
                size={14}
                strokeWidth={2}
                className={isUpdating ? "animate-spin" : ""}
              />
            }
            onClick={onUpdate}
            disabled={isUpdating}
          >
            {isUpdating ? t("common.loading") : t("manga.update")}
          </Button>
        </div>
      </div>
    </div>
  );
}
