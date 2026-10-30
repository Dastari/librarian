import { AppearanceProvider, useAppearance } from "glass-ux/appearance";
import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";

import { bootAppearance, LEGACY_THEME_KEY } from "@/lib/glass/appearance";

/** Selectable themes. `system` follows the OS between Cinema and Daylight. */
export type ThemeId = "dark" | "light" | "midnight" | "ember" | "forest";
export type ThemePreference = ThemeId | "system";

export interface ThemeMeta {
  id: ThemeId;
  label: string;
  scheme: "dark" | "light";
}

export const THEMES: ThemeMeta[] = [
  { id: "dark", label: "Cinema", scheme: "dark" },
  { id: "midnight", label: "Midnight", scheme: "dark" },
  { id: "ember", label: "Ember", scheme: "dark" },
  { id: "forest", label: "Forest", scheme: "dark" },
  { id: "light", label: "Daylight", scheme: "light" },
];

interface ThemeContextValue {
  preference: ThemePreference;
  resolved: ThemeId;
  scheme: "dark" | "light";
  setPreference: (preference: ThemePreference) => void;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

/** Compatibility facade for unmigrated consumers. Glass UX owns all appearance state. */
function LegacyThemeBridge({ children }: { children: ReactNode }) {
  const { appearance, resolvedMode, update } = useAppearance();
  const named = appearance.palette === "midnight" || appearance.palette === "ember" || appearance.palette === "forest" ? appearance.palette : null;
  const preference: ThemePreference = appearance.mode === "system" ? "system" : named ?? (appearance.mode === "light" ? "light" : "dark");
  const resolved: ThemeId = resolvedMode === "light" ? "light" : named ?? "dark";

  useEffect(() => {
    // Derived legacy attributes are read by old route/tests; they do not drive Glass UX.
    const root = document.documentElement;
    root.dataset.theme = resolved;
    root.classList.toggle("dark", resolvedMode === "dark");
    root.classList.toggle("light", resolvedMode === "light");
    root.style.colorScheme = resolvedMode;
  }, [resolved, resolvedMode]);

  const setPreference = useCallback((next: ThemePreference) => {
    const palette = next === "midnight" || next === "ember" || next === "forest" ? next : "cinema";
    update({ palette, mode: next === "system" ? "system" : next === "light" ? "light" : "dark" });
    // Retained for rollback and existing device tools, never read over the new preference.
    window.localStorage.setItem(LEGACY_THEME_KEY, next);
  }, [update]);
  const value = useMemo(() => ({ preference, resolved, scheme: resolvedMode, setPreference }), [preference, resolved, resolvedMode, setPreference]);
  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  useState(() => { bootAppearance(); return true; });
  return <AppearanceProvider><LegacyThemeBridge>{children}</LegacyThemeBridge></AppearanceProvider>;
}

export function useTheme(): ThemeContextValue {
  const context = useContext(ThemeContext);
  if (!context) throw new Error("useTheme must be used within ThemeProvider");
  return context;
}
