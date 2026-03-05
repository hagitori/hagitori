import {
  CheckCircle,
  XCircle,
  Clock,
  Loader2,
  Trash2,
  Package,
  FolderOpen,
  Ban,
  RotateCcw,
  X,
} from "lucide-react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { cancelDownload, downloadChapters } from "@/lib/tauri";
import { useTranslation } from "@/hooks/useTranslation";
import { useDownloadStore } from "@/stores/download-store";
import { Button, Card, Badge, EmptyState, ProgressBar, useToast } from "@/components/ui";
import type { Chapter } from "@/types";

export default function Downloads() {
  const { t } = useTranslation();
  const toast = useToast();
  const queue = useDownloadStore((s) => s.queue);
  const clearCompleted = useDownloadStore((s) => s.clearCompleted);
  const clearFailed = useDownloadStore((s) => s.clearFailed);
  const removeFromQueue = useDownloadStore((s) => s.removeFromQueue);
  const retryMap = useDownloadStore((s) => s.retryMap);
  const resetToQueued = useDownloadStore((s) => s.resetToQueued);

  const hasCompleted = queue.some((i) => i.status === "completed");
  const hasFailed = queue.some(
    (i) => typeof i.status === "object" && "failed" in i.status,
  );
  const hasRetryableFailed = queue.some(
    (i) =>
      typeof i.status === "object" &&
      "failed" in i.status &&
      Object.values(retryMap).some((r) => r.chapterNumber === i.chapterNumber),
  );
  const hasActive = queue.some(
    (i) => i.status === "queued" || i.status === "downloading" || i.status === "processing",
  );

  const handleRetry = async (chapterNumber: string) => {
    // find retry info by chapter number
    const info = Object.values(retryMap).find(
      (r) => r.chapterNumber === chapterNumber,
    );
    if (!info) return;

    // reset queue item status to "queued"
    resetToQueued(info.chapterId);

    // reconstruct a minimal Chapter and call backend
    const chapter: Chapter = {
      id: info.chapterId,
      number: info.chapterNumber,
      name: info.chapterName,
    };

    try {
      await downloadChapters([chapter], info.source, info.mangaName, info.mangaId);
    } catch (err) {
      console.error("Retry failed:", err);
      toast.error(t("downloads.retryFailed"));
    }
  };

  const handleRetryAll = async () => {
    const failedItems = queue.filter(
      (i) => typeof i.status === "object" && "failed" in i.status,
    );

    const retryInfos = failedItems
      .map((item) =>
        Object.values(retryMap).find(
          (r) => r.chapterNumber === item.chapterNumber,
        ),
      )
      .filter((info): info is NonNullable<typeof info> => info != null);

    if (retryInfos.length === 0) return;

    const groups = new Map<string, typeof retryInfos>();
    for (const info of retryInfos) {
      const key = `${info.source}::${info.mangaName}`;
      const group = groups.get(key) ?? [];
      group.push(info);
      groups.set(key, group);
    }

    for (const [, group] of groups) {
      group.sort((a, b) => parseFloat(a.chapterNumber) - parseFloat(b.chapterNumber));
      for (const info of group) {
        resetToQueued(info.chapterId);
      }
    }

    for (const [, group] of groups) {
      const chapters: Chapter[] = group.map((info) => ({
        id: info.chapterId,
        number: info.chapterNumber,
        name: info.chapterName,
      }));

      try {
        await downloadChapters(chapters, group[0].source, group[0].mangaName, group[0].mangaId);
      } catch (err) {
        console.error("Retry all failed:", err);
        toast.error(t("downloads.retryFailed"));
      }
    }
  };

  return (
    <div className="flex h-full flex-col">
      {/* Header */}
      <div className="mb-6 flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-foreground">
            {t("downloads.title")}
          </h1>
          <p className="mt-1 text-sm text-muted-foreground">
            {queue.length > 0
              ? t("downloads.count").replace("{count}", String(queue.length))
              : t("downloads.empty")}
          </p>
        </div>
        <div className="flex items-center gap-2">
          {hasActive && (
            <Button
              variant="outline"
              size="sm"
              icon={<Ban size={14} />}
              onClick={cancelDownload}
            >
              {t("downloads.cancel")}
            </Button>
          )}
          {hasCompleted && (
            <Button
              variant="outline"
              size="sm"
              icon={<Trash2 size={14} />}
              onClick={clearCompleted}
            >
              {t("downloads.clearCompleted")}
            </Button>
          )}
          {hasFailed && (
            <Button
              variant="outline"
              size="sm"
              icon={<XCircle size={14} />}
              onClick={clearFailed}
            >
              {t("downloads.clearFailed")}
            </Button>
          )}
          {hasRetryableFailed && !hasActive && (
            <Button
              variant="outline"
              size="sm"
              icon={<RotateCcw size={14} />}
              onClick={handleRetryAll}
            >
              {t("downloads.retryAll")}
            </Button>
          )}
        </div>
      </div>

      {/* Queue */}
      <div className="flex-1 overflow-y-auto">
        {queue.length === 0 ? (
          <EmptyState
            icon={
              <Package
                size={28}
                strokeWidth={1.5}
                className="text-muted-foreground/50"
              />
            }
            title={t("downloads.empty")}
          />
        ) : (
          <div className="flex flex-col gap-2">
            {queue.map((item, i) => {
              const isFailed =
                typeof item.status === "object" && "failed" in item.status;
              const failMsg = isFailed
                ? (item.status as { failed: string }).failed
                : null;

              return (
                <Card
                  key={`${item.mangaName}-${item.chapterNumber}-${i}`}
                >
                  <div className="flex items-center gap-4">
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-2">
                        <h3 className="truncate text-sm font-semibold text-foreground">
                          {item.mangaName}
                        </h3>
                        <Badge variant="outline" size="sm">
                          {t("manga.chapter")} {item.chapterNumber}
                        </Badge>
                      </div>

                      {item.totalPages > 0 && (
                        <div className="mt-2">
                          <ProgressBar
                            value={item.currentPage}
                            max={item.totalPages}
                            variant={
                              isFailed
                                ? "danger"
                                : item.status === "completed"
                                  ? "success"
                                  : "primary"
                            }
                          />
                        </div>
                      )}

                      <div className="mt-1.5 flex items-center gap-1.5">
                        {item.status === "queued" && (
                          <Badge
                            variant="default"
                            size="sm"
                            className="flex items-center gap-1"
                          >
                            <Clock size={10} /> {t("downloads.queued")}
                          </Badge>
                        )}
                        {item.status === "downloading" && (
                          <Badge
                            variant="info"
                            size="sm"
                            className="flex items-center gap-1"
                          >
                            <Loader2 size={10} className="animate-spin" />{" "}
                            {t("downloads.downloading")}
                            {item.totalPages > 0 &&
                              ` ${item.currentPage}/${item.totalPages}`}
                          </Badge>
                        )}
                        {item.status === "processing" && (
                          <Badge
                            variant="warning"
                            size="sm"
                            className="flex items-center gap-1"
                          >
                            <Loader2 size={10} className="animate-spin" />{" "}
                            {t("downloads.processing")}
                          </Badge>
                        )}
                        {item.status === "completed" && (
                          <Badge
                            variant="success"
                            size="sm"
                            className="flex items-center gap-1"
                          >
                            <CheckCircle size={10} />{" "}
                            {t("downloads.completed")}
                          </Badge>
                        )}
                        {isFailed && (
                          <Badge
                            variant="danger"
                            size="sm"
                            className="flex items-center gap-1"
                          >
                            <XCircle size={10} /> {failMsg}
                          </Badge>
                        )}
                      </div>
                    </div>

                    {item.status === "completed" && item.savePath && (
                      <button
                        type="button"
                        title={t("downloads.openFolder")}
                        className="flex-shrink-0 rounded-md p-1.5 text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
                        onClick={() => revealItemInDir(item.savePath!)}
                      >
                        <FolderOpen size={16} />
                      </button>
                    )}
                    {isFailed && !hasActive && Object.values(retryMap).some(
                      (r) => r.chapterNumber === item.chapterNumber,
                    ) && (
                      <button
                        type="button"
                        title={t("downloads.retry")}
                        className="flex-shrink-0 rounded-md p-1.5 text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
                        onClick={() => handleRetry(item.chapterNumber)}
                      >
                        <RotateCcw size={16} />
                      </button>
                    )}
                    {isFailed && (
                      <button
                        type="button"
                        title={t("downloads.dismiss")}
                        className="flex-shrink-0 rounded-md p-1.5 text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
                        onClick={() => removeFromQueue(item.chapterNumber)}
                      >
                        <X size={16} />
                      </button>
                    )}
                  </div>
                </Card>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
