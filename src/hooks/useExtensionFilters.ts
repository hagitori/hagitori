import { useState, useMemo } from "react";
import type { ExtensionMeta, ExtensionUpdateInfo } from "@/types";

interface UseExtensionFiltersParams {
  extensions: ExtensionMeta[];
  updates: ExtensionUpdateInfo[];
  search: string;
}

export function useExtensionFilters({ extensions, updates, search }: UseExtensionFiltersParams) {
  const [selectedLangs, setSelectedLangs] = useState<Set<string>>(new Set());

  const filteredExtensions = extensions.filter((ext) =>
    ext.name.toLowerCase().includes(search.toLowerCase()),
  );

  const availableLangs = useMemo(() => {
    const langs = new Set<string>();
    for (const ext of updates) {
      if (ext.lang) langs.add(ext.lang);
    }
    return Array.from(langs).sort();
  }, [updates]);

  const toggleLang = (lang: string) => {
    setSelectedLangs((prev) => {
      const next = new Set(prev);
      if (next.has(lang)) {
        next.delete(lang);
      } else {
        next.add(lang);
      }
      return next;
    });
  };

  const clearLangs = () => setSelectedLangs(new Set());

  const filteredUpdates = updates.filter((ext) => {
    const matchesSearch = ext.name.toLowerCase().includes(search.toLowerCase());
    const matchesLang = selectedLangs.size === 0 || selectedLangs.has(ext.lang);
    return matchesSearch && matchesLang;
  });

  const updatesAvailableCount = updates.filter(
    (u) => u.status === "update_available",
  ).length;

  return {
    filteredExtensions,
    filteredUpdates,
    availableLangs,
    selectedLangs,
    toggleLang,
    clearLangs,
    updatesAvailableCount,
  };
}
