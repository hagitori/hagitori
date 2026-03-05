import { Download, Loader2, X } from "lucide-react";
import { useAppUpdater } from "../../hooks/useAppUpdater";
import { useTranslation } from "../../hooks/useTranslation";

/** Strip markdown badges, images, headings, and empty lines from release notes */
function cleanReleaseNotes(body: string): string {
  return body
    .split("\n")
    .filter((line) => {
      const trimmed = line.trim();
      if (!trimmed) return false;
      if (trimmed.startsWith("[![") || trimmed.startsWith("![")) return false;
      if (trimmed.startsWith("##")) return false;
      return true;
    })
    .join("\n")
    .trim();
}

export function UpdateModal() {
  const {
    updateAvailable,
    updateInfo,
    downloading,
    installing,
    progress,
    error,
    downloadAndInstall,
    dismiss,
  } = useAppUpdater();
  const { t } = useTranslation();

  if (!updateAvailable) return null;

  const isBusy = downloading || installing;

  return (
    <div className="fixed bottom-4 right-4 z-50 w-80 rounded-lg border border-[hsl(var(--info))]/20 bg-[hsl(var(--info))]/5 p-4 shadow-lg animate-in slide-in-from-right-full duration-200">
      <div className="flex items-start gap-3">
        {installing ? (
          <Loader2
            size={18}
            strokeWidth={2}
            className="mt-0.5 shrink-0 text-[hsl(var(--info))] animate-spin"
          />
        ) : (
          <Download
            size={18}
            strokeWidth={2}
            className="mt-0.5 shrink-0 text-[hsl(var(--info))]"
          />
        )}
        <div className="flex-1 min-w-0">
          <h3 className="text-sm font-semibold text-foreground">
            {installing
              ? t("updater.installing")
              : `${t("updater.title")} v${updateInfo?.version}`}
          </h3>

          {!installing && updateInfo?.body && cleanReleaseNotes(updateInfo.body) && (
            <p className="mt-1 text-xs text-muted-foreground line-clamp-3">
              {cleanReleaseNotes(updateInfo.body)}
            </p>
          )}

          {error && (
            <p className="mt-1 text-xs text-[hsl(var(--danger))]">{error}</p>
          )}

          {installing ? (
            <p className="mt-2 text-xs text-muted-foreground">
              {t("updater.installingHint")}
            </p>
          ) : downloading ? (
            <div className="mt-3">
              <div className="h-1.5 w-full rounded-full bg-muted overflow-hidden">
                <div
                  className="h-full rounded-full bg-[hsl(var(--info))] transition-all duration-300"
                  style={{ width: `${progress}%` }}
                />
              </div>
              <p className="mt-1 text-xs text-muted-foreground">
                {t("updater.downloading")} {progress}%
              </p>
            </div>
          ) : (
            <div className="mt-3 flex gap-2">
              <button
                type="button"
                onClick={downloadAndInstall}
                className="rounded-md bg-[hsl(var(--info))] px-3 py-1.5 text-xs font-medium text-white hover:opacity-90 transition-opacity"
              >
                {t("updater.updateNow")}
              </button>
              <button
                type="button"
                onClick={dismiss}
                className="rounded-md px-3 py-1.5 text-xs text-muted-foreground hover:text-foreground transition-colors"
              >
                {t("updater.later")}
              </button>
            </div>
          )}
        </div>

        {!isBusy && (
          <button
            type="button"
            onClick={dismiss}
            className="shrink-0 text-muted-foreground transition-colors hover:text-foreground"
          >
            <X size={14} strokeWidth={2} />
          </button>
        )}
      </div>
    </div>
  );
}
