import { FolderOpen } from "lucide-react";
import { useTranslation } from "../hooks/useTranslation";
import { useSettingsStore } from "../stores/settings-store";
import { setConfig } from "../lib/tauri";
import { open } from "@tauri-apps/plugin-dialog";
import { Button, Select, Toggle } from "../components/ui";
import { cn } from "../lib/utils";

export default function Settings() {
  const { t } = useTranslation();

  const downloadPath = useSettingsStore((s) => s.downloadPath);
  const setDownloadPath = useSettingsStore((s) => s.setDownloadPath);
  const groupFormat = useSettingsStore((s) => s.groupFormat);
  const setGroupFormat = useSettingsStore((s) => s.setGroupFormat);
  const imageFormat = useSettingsStore((s) => s.imageFormat);
  const setImageFormat = useSettingsStore((s) => s.setImageFormat);
  const language = useSettingsStore((s) => s.language);
  const setLanguage = useSettingsStore((s) => s.setLanguage);
  const maxConcurrentPages = useSettingsStore((s) => s.maxConcurrentPages);
  const setMaxConcurrentPages = useSettingsStore((s) => s.setMaxConcurrentPages);
  const autoUpdateExtensions = useSettingsStore((s) => s.autoUpdateExtensions);
  const setAutoUpdateExtensions = useSettingsStore((s) => s.setAutoUpdateExtensions);

  const handleChooseDir = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: t("settings.downloadDir"),
      });
      if (selected) {
        setDownloadPath(selected);
        await setConfig("download_path", selected);
      }
    } catch {
      // cancelled or error
    }
  };

  const selects = [
    {
      id: "format",
      label: t("settings.groupFormat"),
      description: t("settings.download"),
      value: groupFormat,
      options: [
        { value: "cbz", label: "CBZ" },
        { value: "zip", label: "ZIP" },
        { value: "folder", label: t("settings.folder") },
      ],
      onChange: (v: string) => {
        setGroupFormat(v as "cbz" | "zip" | "folder");
        setConfig("group_format", v);
      },
    },
    {
      id: "imageFormat",
      label: t("settings.imageFormat"),
      description: t("settings.postProcessing"),
      value: imageFormat,
      options: [
        { value: "original", label: t("settings.originalFormat") },
        { value: "png", label: "PNG" },
        { value: "jpeg", label: "JPEG" },
        { value: "webp", label: "WEBP" },
      ],
      onChange: (v: string) => {
        setImageFormat(v as "original" | "png" | "jpeg" | "webp");
        setConfig("image_format", v);
      },
    },
    {
      id: "language",
      label: t("settings.language"),
      description: t("settings.appearance"),
      value: language,
      options: ["pt-br", "en"] as const,
      onChange: (v: string) => {
        setLanguage(v as "pt-br" | "en");
        setConfig("language", v);
      },
    },
  ];

  return (
    <div className="flex h-full flex-col">
      {/* Header */}
      <div className="mb-8">
        <h1 className="text-2xl font-bold tracking-tight text-foreground">
          {t("settings.title")}
        </h1>
        <p className="mt-1 text-sm text-muted-foreground">
          {t("settings.title")}
        </p>
      </div>

      <div className="flex-1 overflow-y-auto">
        {/* Download path */}
        <div className="mb-8">
          <h2 className="mb-3 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            {t("settings.downloadDir")}
          </h2>
          <div className="flex items-center gap-3">
            <div className="flex flex-1 items-center gap-3 rounded-lg border border-border bg-muted px-4 py-2.5">
              <FolderOpen
                size={16}
                strokeWidth={2}
                className="shrink-0 text-muted-foreground"
              />
              <span className="truncate text-sm text-foreground">
                {downloadPath || "—"}
              </span>
            </div>
            <Button
              variant="outline"
              size="sm"
              onClick={handleChooseDir}
            >
              {t("settings.chooseDir")}
            </Button>
          </div>
        </div>

        {/* Selects */}
        <div className="mb-8">
          <h2 className="mb-3 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            {t("settings.appearance")}
          </h2>
          <div className="flex flex-col gap-1">
            {selects.map((select) => (
              <div
                key={select.id}
                className="flex items-center justify-between rounded-lg px-3 py-3 transition-colors hover:bg-muted/50"
              >
                <div>
                  <p className="text-sm font-medium text-foreground">
                    {select.label}
                  </p>
                  <p className="mt-0.5 text-xs text-muted-foreground/60">
                    {select.description}
                  </p>
                </div>
                <Select
                  value={select.value}
                  options={[...select.options]}
                  onChange={select.onChange}
                  className="h-8 w-auto"
                />
              </div>
            ))}
          </div>
        </div>

        {/* Extensions */}
        <div className="mb-8">
          <h2 className="mb-3 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            {t("settings.extensions")}
          </h2>
          <Toggle
            checked={autoUpdateExtensions}
            onChange={(checked) => setAutoUpdateExtensions(checked)}
            label={t("settings.autoUpdateExtensions")}
            description={t("settings.autoUpdateExtensionsDesc")}
          />
        </div>

        {/* Performance */}
        <div className="mb-8">
          <h2 className="mb-3 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            {t("settings.performance")}
          </h2>
          <div className="flex items-center justify-between rounded-lg px-3 py-3 transition-colors hover:bg-muted/50">
            <div>
              <p className="text-sm font-medium text-foreground">
                {t("settings.maxConcurrentPages")}
              </p>
              <p className="mt-0.5 text-xs text-muted-foreground/60">
                {t("settings.maxConcurrentPagesDesc")}
              </p>
            </div>
            <div className="inline-flex items-center rounded-lg border border-border bg-muted p-0.5">
              {[1, 2, 3, 4, 5].map((n) => (
                <button
                  key={n}
                  type="button"
                  onClick={() => setMaxConcurrentPages(n)}
                  className={cn(
                    "h-7 w-8 rounded-md text-xs font-semibold transition-colors duration-150",
                    maxConcurrentPages === n
                      ? "bg-primary text-primary-foreground shadow-sm"
                      : "text-muted-foreground hover:bg-muted/80 hover:text-foreground",
                  )}
                >
                  {n}
                </button>
              ))}
            </div>
          </div>
        </div>

      </div>
    </div>
  );
}
