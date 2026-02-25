import { useCallback } from "react";
import {
  translations,
  type TranslationKey,
} from "../i18n/translations";
import { useSettingsStore } from "../stores/settings-store";

export function useTranslation() {
  const language = useSettingsStore((s) => s.language);

  const t = useCallback(
    (key: TranslationKey, vars?: Record<string, string | number>): string => {
      const parts = key.split(".");
      let value: any = translations[language];

      for (const part of parts) {
        value = value?.[part];
      }

      if (typeof value !== "string") return key;

      if (vars) {
        return Object.entries(vars).reduce(
          (str, [k, v]) => str.replace(`{${k}}`, String(v)),
          value,
        );
      }

      return value;
    },
    [language],
  );

  return { t, language };
}
