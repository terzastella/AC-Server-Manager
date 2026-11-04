import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from "react";
import { api } from "../api";
import { it } from "./it";
import { en } from "./en";

export type Lang = "it" | "en";
export type TKey = keyof typeof it;
export type TParams = Record<string, string | number>;

const dicts: Record<Lang, Record<TKey, string>> = { it, en };

type LanguageContextValue = {
  lang: Lang;
  setLang: (lang: Lang) => void;
  t: (key: TKey, params?: TParams) => string;
};

const LanguageContext = createContext<LanguageContextValue | null>(null);

/**
 * Lingua dell'interfaccia, salvata in `settings.json` (condiviso fra manager e
 * launcher): scelta a mano, nessun rilevamento automatico.
 */
export function LanguageProvider({ children }: { children: React.ReactNode }) {
  const [lang, setLangState] = useState<Lang>("it");

  useEffect(() => {
    let cancelled = false;
    api
      .getSettings()
      .then((settings) => {
        const stored = (settings as { language?: string }).language;
        if (!cancelled && (stored === "it" || stored === "en")) setLangState(stored);
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    document.documentElement.lang = lang;
  }, [lang]);

  const setLang = useCallback((next: Lang) => {
    setLangState(next);
    api.setLanguage(next).catch(() => undefined);
  }, []);

  const t = useCallback(
    (key: TKey, params?: TParams) => {
      const template = dicts[lang][key] ?? dicts.it[key] ?? key;
      if (!params) return template;
      return template.replace(/\{(\w+)\}/g, (match, name: string) =>
        params[name] === undefined ? match : String(params[name]),
      );
    },
    [lang],
  );

  const value = useMemo(() => ({ lang, setLang, t }), [lang, setLang, t]);
  return <LanguageContext.Provider value={value}>{children}</LanguageContext.Provider>;
}

export function useT(): LanguageContextValue {
  const value = useContext(LanguageContext);
  if (!value) throw new Error("useT fuori dal LanguageProvider");
  return value;
}
