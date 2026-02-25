import {
  ArrowUpDown,
  CheckCheck,
  ArrowLeftRight,
  Download,
  BookOpen,
  Users,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { Button, EmptyState, SkeletonChapterRow } from "@/components/ui";
import { useTranslation } from "@/hooks/useTranslation";
import type { Chapter } from "@/types";

interface SelectableChapter extends Chapter {
  selected: boolean;
}

export interface ChapterListProps {
  sortedChapters: SelectableChapter[];
  chapters: SelectableChapter[];
  scanlators: string[];
  scanlatorFilter: string;
  sortAsc: boolean;
  isUpdating: boolean;
  selectedCount: number;
  onToggleChapter: (chId: string) => void;
  onSelectAll: () => void;
  onInvertSelection: () => void;
  onToggleSort: () => void;
  onScanlatorFilterChange: (value: string) => void;
  onDownload: () => void;
}

export function ChapterList({
  sortedChapters,
  chapters,
  scanlators,
  scanlatorFilter,
  sortAsc,
  isUpdating,
  selectedCount,
  onToggleChapter,
  onSelectAll,
  onInvertSelection,
  onToggleSort,
  onScanlatorFilterChange,
  onDownload,
}: ChapterListProps) {
  const { t } = useTranslation();
  return (
    <>
      {/* Chapter list header */}
      <div className="mb-3 flex items-center justify-between shrink-0">
        <h2 className="text-sm font-semibold text-foreground">
          {t("manga.chapters")} ({sortedChapters.length})
        </h2>
        <div className="flex items-center gap-1">
          {scanlators.length > 1 && (
            <div className="relative">
              <Users size={14} strokeWidth={2} className="absolute left-2 top-1/2 -translate-y-1/2 text-muted-foreground pointer-events-none" />
              <select
                value={scanlatorFilter}
                onChange={(e) => onScanlatorFilterChange(e.target.value)}
                className="h-8 appearance-none rounded-md border border-border bg-background pl-7 pr-6 text-xs text-foreground outline-none transition-colors hover:bg-muted focus:ring-1 focus:ring-ring cursor-pointer"
              >
                <option value="all">{t("manga.allScanlators")}</option>
                {scanlators.map((s) => (
                  <option key={s} value={s}>{s}</option>
                ))}
              </select>
            </div>
          )}
          <Button
            variant="ghost"
            size="sm"
            icon={<ArrowUpDown size={14} strokeWidth={2} />}
            onClick={onToggleSort}
          >
            {sortAsc ? "Asc" : "Desc"}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            icon={<CheckCheck size={14} strokeWidth={2} />}
            onClick={onSelectAll}
          >
            {t("manga.selectAll")}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            icon={<ArrowLeftRight size={14} strokeWidth={2} />}
            onClick={onInvertSelection}
          >
            {t("manga.invertSelection")}
          </Button>
        </div>
      </div>

      {/* Chapter list scrollable container */}
      <div className="flex-1 overflow-y-auto min-h-0">
        {isUpdating && (
          <div className="flex flex-col gap-1">
            {[1, 2, 3, 4, 5].map((i) => (
              <SkeletonChapterRow key={i} />
            ))}
          </div>
        )}

        {!isUpdating && chapters.length === 0 && (
          <EmptyState
            icon={
              <BookOpen
                size={28}
                strokeWidth={1.5}
                className="text-muted-foreground/50"
              />
            }
            title={t("manga.noChapters")}
          />
        )}

        {!isUpdating && (
          <div className="flex flex-col gap-0.5">
            {sortedChapters.map((chapter) => (
              <button
                key={chapter.id}
                type="button"
                onClick={() => onToggleChapter(chapter.id)}
                className={cn(
                  "flex items-center gap-3 rounded-lg px-3 py-2.5 text-left transition-colors duration-100",
                  chapter.selected ? "bg-primary/10" : "hover:bg-muted",
                )}
              >
                {/* Custom checkbox */}
                <div
                  className={cn(
                    "flex h-4 w-4 shrink-0 items-center justify-center rounded border transition-colors duration-100",
                    chapter.selected
                      ? "border-primary bg-primary"
                      : "border-border bg-transparent",
                  )}
                >
                  {chapter.selected && (
                    <svg
                      width="10"
                      height="10"
                      viewBox="0 0 10 10"
                      fill="none"
                      className="text-primary-foreground"
                    >
                      <path
                        d="M8 3L4.2 7L2 5"
                        stroke="currentColor"
                        strokeWidth="1.5"
                        strokeLinecap="round"
                        strokeLinejoin="round"
                      />
                    </svg>
                  )}
                </div>

                <div className="min-w-0 flex-1">
                  <span
                    className={cn(
                      "text-sm",
                      chapter.selected
                        ? "font-medium text-primary"
                        : "text-foreground",
                    )}
                  >
                    {t("manga.chapter")} {chapter.number}
                  </span>
                  {chapter.title && (
                    <span className="ml-2 text-xs text-muted-foreground/60">
                      {chapter.title}
                    </span>
                  )}
                  {chapter.scanlator && (
                    <span className="ml-2 text-[11px] text-muted-foreground/40">
                      [{chapter.scanlator}]
                    </span>
                  )}
                </div>
                {chapter.date && (
                  <span className="shrink-0 text-[11px] text-muted-foreground/40">
                    {chapter.date}
                  </span>
                )}
              </button>
            ))}
          </div>
        )}
      </div>

      {/* Download button */}
      <div className="mt-3 border-t border-border pt-3 shrink-0">
        <Button
          size="lg"
          fullWidth
          icon={<Download size={16} strokeWidth={2} />}
          onClick={onDownload}
          disabled={selectedCount === 0}
        >
          {t("manga.downloadSelected", { count: String(selectedCount) })}
        </Button>
      </div>
    </>
  );
}
