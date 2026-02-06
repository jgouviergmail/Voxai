import { createSignal, createRoot } from "solid-js";
import en from "./translations/en";
import fr from "./translations/fr";
import zh from "./translations/zh";

type Translations = Record<string, string>;
const all: Record<string, Translations> = { en, fr, zh };

function createI18n() {
  const [locale, setLocale] = createSignal("en");

  /**
   * Translate a key. MUST be called in a reactive context (JSX, createMemo, createEffect).
   * Do NOT call in module-level constants.
   */
  function t(key: string): string {
    const dict = all[locale()] ?? all["en"];
    return dict[key] ?? all["en"][key] ?? key;
  }

  return { locale, setLocale, t };
}

export const i18n = createRoot(createI18n);
