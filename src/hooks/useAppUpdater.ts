import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { useState, useEffect, useRef } from "react";

export interface UpdateInfo {
  version: string;
  body: string;
  date?: string;
}

export function useAppUpdater() {
  const [updateAvailable, setUpdateAvailable] = useState(false);
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [downloading, setDownloading] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [progress, setProgress] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const updateRef = useRef<Update | null>(null);

  useEffect(() => {
    checkForUpdates();
  }, []);

  async function checkForUpdates() {
    try {
      const update = await check();
      if (update) {
        setUpdateAvailable(true);
        setUpdateInfo({
          version: update.version,
          body: update.body ?? "",
          date: update.date ?? undefined,
        });
        updateRef.current = update;
      }
    } catch (err) {
      console.warn("[updater] failed to check:", err);
    }
  }

  async function downloadAndInstall() {
    if (!updateRef.current) return;
    setDownloading(true);
    setError(null);

    try {
      let totalBytes = 0;
      let downloadedBytes = 0;

      await updateRef.current.download((event) => {
        switch (event.event) {
          case "Started":
            totalBytes = event.data.contentLength ?? 0;
            break;
          case "Progress":
            downloadedBytes += event.data.chunkLength;
            if (totalBytes > 0) {
              setProgress(Math.round((downloadedBytes / totalBytes) * 100));
            }
            break;
          case "Finished":
            setProgress(100);
            break;
        }
      });

      setDownloading(false);
      setInstalling(true);

      await updateRef.current.install();
      await relaunch();
    } catch (err) {
      setError(String(err));
      setDownloading(false);
      setInstalling(false);
    }
  }

  return {
    updateAvailable,
    updateInfo,
    downloading,
    installing,
    progress,
    error,
    downloadAndInstall,
    dismiss: () => setUpdateAvailable(false),
  };
}
