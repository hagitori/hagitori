import { ptBr } from "./locales/pt-br";
import { en } from "./locales/en";

// ensures both locales have the same key structure at compile time.
type LocaleShape<T> = {
  [K in keyof T]: T[K] extends Record<string, unknown>
    ? LocaleShape<T[K]>
    : string;
};

export const translations = {
  "pt-br": ptBr satisfies LocaleShape<typeof en>,
  en: en satisfies LocaleShape<typeof ptBr>,
} as const;

export type Language = keyof typeof translations;
type TranslationKeys = typeof translations["en"];

type FlattenKeys<T, Prefix extends string = ""> = T extends Record<
  string,
  unknown
>
  ? {
      [K in keyof T]: K extends string
        ? T[K] extends Record<string, unknown>
          ? FlattenKeys<T[K], `${Prefix}${K}.`>
          : `${Prefix}${K}`
        : never;
    }[keyof T]
  : never;

export type TranslationKey = FlattenKeys<TranslationKeys>;

/**
 * detects the system language and maps it to a supported Language.
 * falls back to "en" if no match is found.
 */
export function detectSystemLanguage(): Language {
  const browserLang = navigator.language.toLowerCase();
  const supported = Object.keys(translations) as Language[];

  const exact = supported.find((lang) => browserLang === lang);
  if (exact) return exact;

  const prefix = browserLang.split("-")[0];
  const prefixMatch = supported.find(
    (lang) => lang === prefix || lang.startsWith(prefix + "-"),
  );
  if (prefixMatch) return prefixMatch;

  return "en";
}
