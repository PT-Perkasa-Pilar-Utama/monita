import type { ReactElement, ReactNode } from "react";
import { createContext, useContext, useEffect, useState, useSyncExternalStore } from "react";

export type Theme = "dark" | "light" | "system";

const STORAGE_KEY = "monita-theme";
const DARK_QUERY = "(prefers-color-scheme: dark)";

function subscribe(onChange: () => void): () => void {
  const query = window.matchMedia(DARK_QUERY);
  query.addEventListener("change", onChange);
  return () => query.removeEventListener("change", onChange);
}
const prefersDark = (): boolean => window.matchMedia(DARK_QUERY).matches;

function readSaved(): Theme {
  try {
    const t = localStorage.getItem(STORAGE_KEY);
    return t === "dark" || t === "light" ? t : "system";
  } catch {
    return "system";
  }
}

type ThemeState = { theme: Theme; resolved: "dark" | "light"; setTheme: (t: Theme) => void };

const ThemeContext = createContext<ThemeState | null>(null);

/** The viewer's theme: their saved choice, or the OS setting. /static/theme.js applied it before paint. */
export function ThemeProvider({ children }: { children: ReactNode }): ReactElement {
  const [theme, setThemeState] = useState<Theme>(readSaved);
  const systemDark = useSyncExternalStore(subscribe, prefersDark, () => true);
  const resolved = theme === "system" ? (systemDark ? "dark" : "light") : theme;

  useEffect(() => {
    const root = document.documentElement;
    root.classList.remove("light", "dark");
    root.classList.add(resolved);
  }, [resolved]);

  const setTheme = (t: Theme): void => {
    try {
      if (t === "system") localStorage.removeItem(STORAGE_KEY);
      else localStorage.setItem(STORAGE_KEY, t);
    } catch {
      // A blocked storage only forgets the choice.
    }
    setThemeState(t);
  };

  return <ThemeContext.Provider value={{ theme, resolved, setTheme }}>{children}</ThemeContext.Provider>;
}

export function useTheme(): ThemeState {
  const t = useContext(ThemeContext);
  if (!t) throw new Error("useTheme outside ThemeProvider");
  return t;
}
