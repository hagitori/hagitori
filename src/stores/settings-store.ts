import { create } from "zustand";
import { persist } from "zustand/middleware";
import { detectSystemLanguage } from "../i18n/translations";
import type { Language } from "../i18n/translations";
import { getConfig, setConfig } from "../lib/tauri";

interface SettingsStore {
  downloadPath: string;
  groupFormat: "cbz" | "zip" | "folder";
  imageFormat: "original" | "png" | "jpeg" | "webp";
  language: Language;
  maxConcurrentPages: number;
  autoUpdateExtensions: boolean;
  setDownloadPath: (path: string) => void;
  setGroupFormat: (format: "cbz" | "zip" | "folder") => void;
  setImageFormat: (format: "original" | "png" | "jpeg" | "webp") => void;
  setLanguage: (language: Language) => void;
  setMaxConcurrentPages: (value: number) => Promise<void>;
  setAutoUpdateExtensions: (enabled: boolean) => Promise<void>;
  loadMaxConcurrentPages: () => Promise<void>;
  loadAutoUpdateExtensions: () => Promise<void>;
}

export const useSettingsStore = create<SettingsStore>()(
  persist(
    (set) => ({
      downloadPath: "",
      groupFormat: "cbz",
      imageFormat: "original",
      language: detectSystemLanguage(),
      maxConcurrentPages: 3,
      autoUpdateExtensions: false,
      setDownloadPath: (path) => set({ downloadPath: path }),
      setGroupFormat: (format) => set({ groupFormat: format }),
      setImageFormat: (format) => set({ imageFormat: format }),
      setLanguage: (language) => set({ language }),
      setMaxConcurrentPages: async (value) => {
        await setConfig("max_concurrent_pages", String(value));
        set({ maxConcurrentPages: value });
      },
      setAutoUpdateExtensions: async (enabled) => {
        await setConfig("auto_update_extensions", String(enabled));
        set({ autoUpdateExtensions: enabled });
      },
      loadMaxConcurrentPages: async () => {
        const config = await getConfig();
        const val = config["max_concurrent_pages"];
        if (val) {
          const parsed = parseInt(val, 10);
          if (!isNaN(parsed) && parsed > 0) {
            set({ maxConcurrentPages: parsed });
          }
        }
      },
      loadAutoUpdateExtensions: async () => {
        const config = await getConfig();
        const val = config["auto_update_extensions"];
        set({ autoUpdateExtensions: val === "true" });
      },
    }),
    {
      name: "hagitori-settings",
      partialize: (state) => ({ language: state.language }),
    },
  ),
);