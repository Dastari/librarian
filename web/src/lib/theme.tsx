import { AppearanceProvider, useAppearance } from "glass-ux/appearance";
import { createContext, useCallback, useContext, useEffect, useMemo, useState, useSyncExternalStore, type ReactNode } from "react";

import { blurStore, bootAppearance, LEGACY_THEME_KEY, setBlur, type ColorMode } from "@/lib/glass/appearance";

/**
 * The palette is always Cinema; the one choice left is Dark (the default) or Light. Everything
 * else about appearance is fixed in `lib/glass/appearance.ts`.
 */
export type ThemePreference = ColorMode;

export interface ThemeMeta {
  id: ColorMode;
  label: string;
}

export const THEMES: ThemeMeta[] = [
  { id: "dark", label: "Dark" },
  { id: "light", label: "Light" },
];

/** Cinema's canvas, for the browser and status bar colour (`theme-color`). */
export const THEME_COLORS: Record<ColorMode, string> = { dark: "#0e0e19", light: "#eaeaf6" };

interface ThemeContextValue {
  preference: ThemePreference;
  scheme: ColorMode;
  setPreference: (preference: ThemePreference) => void;
  /** Glass blurs what is behind it (device-wide, see BLUR_KEY). */
  blur: boolean;
  setBlur: (on: boolean) => void;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

function ThemeBridge({ children }: { children: ReactNode }) {
  const { resolvedMode, update } = useAppearance();
  const scheme: ColorMode = resolvedMode === "light" ? "light" : "dark";
  const blur = useSyncExternalStore(blurStore.subscribe, blurStore.getSnapshot, blurStore.getSnapshot);

  useEffect(() => {
    // Derived attributes for older selectors; Glass's own data-mode drives the styles.
    const root = document.documentElement;
    root.dataset.theme = scheme;
    root.classList.toggle("dark", scheme === "dark");
    root.classList.toggle("light", scheme === "light");
    root.style.colorScheme = scheme;
    for (const meta of document.querySelectorAll<HTMLMetaElement>('meta[name="theme-color"]')) meta.content = THEME_COLORS[scheme];
  }, [scheme]);

  const setPreference = useCallback((next: ThemePreference) => {
    update({ mode: next === "light" ? "light" : "dark" });
    // Retained for rollback and existing device tools, never read over the new preference.
    window.localStorage.setItem(LEGACY_THEME_KEY, next);
  }, [update]);
  const value = useMemo(() => ({ preference: scheme, scheme, setPreference, blur, setBlur }), [scheme, setPreference, blur]);
  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  useState(() => { bootAppearance(); return true; });
  return <AppearanceProvider><ThemeBridge>{children}</ThemeBridge></AppearanceProvider>;
}

export function useTheme(): ThemeContextValue {
  const context = useContext(ThemeContext);
  if (!context) throw new Error("useTheme must be used within ThemeProvider");
  return context;
}
