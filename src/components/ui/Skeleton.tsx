import { cn } from "@/lib/utils";

interface SkeletonProps {
  className?: string;
  rounded?: "sm" | "md" | "lg" | "full" | "xl" | "2xl";
}

/**
 * Skeleton shimmer placeholder for loading states.
 * Uses the shimmer animation from styles.css.
 */
function Skeleton({ className, rounded = "md" }: SkeletonProps) {
  const roundedMap = {
    sm: "rounded-sm",
    md: "rounded",
    lg: "rounded-lg",
    xl: "rounded-xl",
    "2xl": "rounded-2xl",
    full: "rounded-full",
  };

  return (
    <div
      className={cn(
        "relative overflow-hidden bg-muted",
        roundedMap[rounded],
        className,
      )}
    >
      <div className="absolute inset-0 bg-gradient-to-r from-transparent via-foreground/5 to-transparent animate-shimmer" />
    </div>
  );
}

/* Pre-built skeleton patterns */

function SkeletonCard() {
  return (
    <div className="flex items-center gap-4 rounded-xl border border-border bg-card p-4">
      <Skeleton className="h-10 w-10" rounded="lg" />
      <div className="flex-1 space-y-2">
        <Skeleton className="h-4 w-28" />
        <Skeleton className="h-3 w-40" />
      </div>
    </div>
  );
}

function SkeletonMangaCard() {
  return (
    <div className="flex items-center gap-4 rounded-xl border border-border bg-card p-4">
      <Skeleton className="h-16 w-12" rounded="lg" />
      <div className="flex-1 space-y-2">
        <Skeleton className="h-4 w-40" />
        <Skeleton className="h-3 w-24" />
      </div>
    </div>
  );
}

function SkeletonChapterRow() {
  return (
    <div className="flex items-center gap-3 rounded-lg px-3 py-2.5">
      <Skeleton className="h-4 w-4" rounded="sm" />
      <Skeleton className="h-4 w-32" />
    </div>
  );
}

export {
  Skeleton,
  SkeletonCard,
  SkeletonMangaCard,
  SkeletonChapterRow,
  type SkeletonProps,
};
