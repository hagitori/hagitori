import { create } from "zustand";
import type { DownloadProgress } from "../types";

interface RetryInfo {
  chapterId: string;
  mangaId: string;
  mangaName: string;
  chapterNumber: string;
  chapterName: string;
  source: string;
  retryCount: number;
  lastError?: string;
}

interface DownloadStore {
  queue: DownloadProgress[];
  retryMap: Record<string, RetryInfo>;
  addToQueue: (item: DownloadProgress) => void;
  updateProgress: (chapterNumber: string, progress: Partial<DownloadProgress>) => void;
  clearCompleted: () => void;
  clearFailed: () => void;
  removeFromQueue: (chapterNumber: string) => void;
  saveRetryInfo: (info: RetryInfo) => void;
  deleteRetryInfo: (chapterId: string) => void;
  resetToQueued: (chapterId: string) => void;
}

export const useDownloadStore = create<DownloadStore>((set) => ({
  queue: [],
  retryMap: {},
  addToQueue: (item) => set((state) => ({ queue: [...state.queue, item] })),
  updateProgress: (chapterNumber, progress) =>
    set((state) => {
      const newQueue = state.queue.map((item) =>
        item.chapterNumber === chapterNumber ? { ...item, ...progress } : item,
      );
      const updatedItem = newQueue.find((i) => i.chapterNumber === chapterNumber);
      const newRetryMap = { ...state.retryMap };
      if (updatedItem?.status === "completed") {
        delete newRetryMap[chapterNumber];
      }
      return { queue: newQueue, retryMap: newRetryMap };
    }),
  clearCompleted: () =>
    set((state) => {
      const completedNumbers = state.queue
        .filter((i) => i.status === "completed")
        .map((i) => i.chapterNumber);
      const newRetryMap = { ...state.retryMap };
      for (const num of completedNumbers) {
        delete newRetryMap[num];
      }
      return {
        queue: state.queue.filter((i) => i.status !== "completed"),
        retryMap: newRetryMap,
      };
    }),
  clearFailed: () =>
    set((state) => {
      const isFailed = (s: DownloadProgress["status"]) =>
        typeof s === "object" && "failed" in s;
      const failedNumbers = state.queue
        .filter((i) => isFailed(i.status))
        .map((i) => i.chapterNumber);
      const newRetryMap = { ...state.retryMap };
      for (const num of failedNumbers) {
        // remove retry info for failed items
        for (const [key, info] of Object.entries(newRetryMap)) {
          if (info.chapterNumber === num) delete newRetryMap[key];
        }
      }
      return {
        queue: state.queue.filter((i) => !isFailed(i.status)),
        retryMap: newRetryMap,
      };
    }),
  removeFromQueue: (chapterNumber) =>
    set((state) => {
      const newRetryMap = { ...state.retryMap };
      for (const [key, info] of Object.entries(newRetryMap)) {
        if (info.chapterNumber === chapterNumber) delete newRetryMap[key];
      }
      return {
        queue: state.queue.filter((i) => i.chapterNumber !== chapterNumber),
        retryMap: newRetryMap,
      };
    }),
  saveRetryInfo: (info) =>
    set((state) => ({
      retryMap: { ...state.retryMap, [info.chapterId]: info },
    })),
  deleteRetryInfo: (chapterId) =>
    set((state) => {
      const newRetryMap = { ...state.retryMap };
      delete newRetryMap[chapterId];
      return { retryMap: newRetryMap };
    }),
  resetToQueued: (chapterId) =>
    set((state) => {
      const info = state.retryMap[chapterId];
      if (!info) return state;
      // reset the failed item to "queued" if it exists, otherwise add new
      const existingIdx = state.queue.findIndex(
        (i) => i.chapterNumber === info.chapterNumber,
      );
      if (existingIdx !== -1) {
        const newQueue = [...state.queue];
        newQueue[existingIdx] = {
          ...newQueue[existingIdx],
          status: "queued" as const,
          currentPage: 0,
          totalPages: 0,
        };
        return {
          queue: newQueue,
          retryMap: {
            ...state.retryMap,
            [info.chapterId]: { ...info, retryCount: info.retryCount + 1 },
          },
        };
      }
      return {
        queue: [
          ...state.queue,
          {
            mangaName: info.mangaName,
            chapterNumber: info.chapterNumber,
            currentPage: 0,
            totalPages: 0,
            status: "queued" as const,
          },
        ],
        retryMap: {
          ...state.retryMap,
          [info.chapterId]: { ...info, retryCount: info.retryCount + 1 },
        },
      };
    }),
}));
