import { useEffect } from "react";
import { useSettingsStore } from "../stores/settings-store";
import { getConfig, getDownloadPath } from "../lib/tauri";
import type { Language } from "../i18n/translations";
import { translations } from "../i18n/translations";

/**
 * hook that syncs the settings-store with the backend ConfigManager.
 * on mount, loads all keys from the backend and populates the store.
 */
export function useConfig() {
  const store = useSettingsStore();

  // load full config from backend on mount
  useEffect(() => {
    (async () => {
      try {
        const [config, downloadPath] = await Promise.all([
          getConfig(),
          getDownloadPath(),
        ]);

        if (downloadPath) store.setDownloadPath(downloadPath);

        if (config.group_format) {
          store.setGroupFormat(
            config.group_format as "cbz" | "zip" | "folder",
          );
        }
        if (config.image_format) {
          store.setImageFormat(
            config.image_format as "original" | "png" | "jpeg" | "webp",
          );
        }

        const persisted = localStorage.getItem("hagitori-settings");
        if (!persisted && config.language) {
          const lang = config.language.toLowerCase().replace("_", "-");
          const supported = Object.keys(translations) as Language[];
          const match = supported.find((l) => l === lang) ?? "en";
          store.setLanguage(match);
        }

        // additional fields managed by the backend
        const maxPages = config.max_concurrent_pages;
        if (maxPages) {
          const parsed = parseInt(maxPages, 10);
          if (!isNaN(parsed) && parsed > 0) {
            useSettingsStore.setState({ maxConcurrentPages: parsed });
          }
        }

        useSettingsStore.setState({
          autoUpdateExtensions: config.auto_update_extensions === "true",
        });
      } catch (err) {
        console.error("Failed to load config:", err);
      }
    })();
    // store actions are stable (zustand), safe to omit from deps
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
}
