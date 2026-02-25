import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useDownloadStore } from "../stores/download-store";
import type { DownloadProgress } from "../types";

/**
 * listens to the `download-progress` event emitted by the Rust backend
 * and updates the download-store automatically.
 *
 * should be mounted once in Layout or App.
 */
export function useDownloadProgress() {
  const addToQueue = useDownloadStore((s) => s.addToQueue);
  const updateProgress = useDownloadStore((s) => s.updateProgress);

  useEffect(() => {
    const unlisten = listen<DownloadProgress>(
      "download-progress",
      (event) => {
        const progress = event.payload;

        // check if this chapter already exists in the queue
        const queue = useDownloadStore.getState().queue;
        const exists = queue.some(
          (item) =>
            item.chapterNumber === progress.chapterNumber &&
            item.mangaName === progress.mangaName,
        );

        if (exists) {
          updateProgress(progress.chapterNumber, progress);
        } else {
          addToQueue(progress);
        }
      },
    );

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [addToQueue, updateProgress]);
}
