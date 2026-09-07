import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";

/** Selectable themes. `system` follows the OS between Cinema and Daylight. */
export type ThemeId = "dark" | "light" | "midnight" | "ember" | "forest";
export type ThemePreference = ThemeId | "system";

export interface ThemeMeta {
  id: ThemeId;
  label: string;
  scheme: "dark" | "light";
  /** Swatch colours for the theme picker. */
  swatch: [string, string, string];
}

export const THEMES: ThemeMeta[] = [
  { id: "dark", label: "Cinema", scheme: "dark", swatch: ["oklch(0.155 0.012 285)", "oklch(0.8 0.15 74)", "oklch(0.42 0.12 290)"] },
  { id: "midnight", label: "Midnight", scheme: "dark", swatch: ["oklch(0.14 0.03 255)", "oklch(0.82 0.11 210)", "oklch(0.38 0.12 260)"] },
  { id: "ember", label: "Ember", scheme: "dark", swatch: ["oklch(0.16 0.015 40)", "oklch(0.74 0.17 35)", "oklch(0.42 0.14 30)"] },
  { id: "forest", label: "Forest", scheme: "dark", swatch: ["oklch(0.15 0.025 160)", "oklch(0.82 0.15 160)", "oklch(0.4 0.12 160)"] },
  { id: "light", label: "Daylight", scheme: "light", swatch: ["oklch(0.975 0.004 286)", "oklch(0.62 0.15 60)", "oklch(0.86 0.06 290)"] },
];

interface ThemeContextValue {
  preference: ThemePreference;
  resolved: ThemeId;
  scheme: "dark" | "light";
  setPreference: (preference: ThemePreference) => void;
}

const STORAGE_KEY = "librarian.theme";
const ThemeContext = createContext<ThemeContextValue | null>(null);
const IDS = new Set<string>(THEMES.map((theme) => theme.id));

function readPreference(): ThemePreference {
  if (typeof window === "undefined") return "dark";
  const stored = window.localStorage.getItem(STORAGE_KEY);
  return stored === "system" || (stored && IDS.has(stored)) ? (stored as ThemePreference) : "dark";
}

function systemTheme(): ThemeId {
  if (typeof window === "undefined") return "dark";
  return window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [preference, setPreferenceState] = useState<ThemePreference>(readPreference);
  const [system, setSystem] = useState<ThemeId>(systemTheme);

  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: light)");
    const onChange = () => setSystem(media.matches ? "light" : "dark");
    media.addEventListener("change", onChange);
    return () => media.removeEventListener("change", onChange);
  }, []);

  const resolved: ThemeId = preference === "system" ? system : preference;
  const scheme = THEMES.find((theme) => theme.id === resolved)?.scheme ?? "dark";

  useEffect(() => {
    const root = document.documentElement;
    root.dataset.theme = resolved;
    root.classList.toggle("dark", scheme === "dark");
    root.classList.toggle("light", scheme === "light");
    root.style.colorScheme = scheme;
    const meta = document.querySelector<HTMLMetaElement>('meta[name="theme-color"]:not([media])');
    if (meta) meta.content = getComputedStyle(root).getPropertyValue("--background").trim() || meta.content;
  }, [resolved, scheme]);

  const setPreference = useCallback((next: ThemePreference) => {
    window.localStorage.setItem(STORAGE_KEY, next);
    setPreferenceState(next);
  }, []);

  const value = useMemo(() => ({ preference, resolved, scheme, setPreference }), [preference, resolved, scheme, setPreference]);
  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export function useTheme(): ThemeContextValue {
  const context = useContext(ThemeContext);
  if (!context) throw new Error("useTheme must be used within ThemeProvider");
  return context;
}
