import { applyAppearance, configureAppearance, defaultAppearance, definePalette, normalizeAppearance } from "glass-ux/appearance";

export const APPEARANCE_KEY = "librarian.appearance.v1";
export const LEGACY_THEME_KEY = "librarian.theme";

for (const [id, label] of [["cinema", "Cinema"], ["midnight", "Midnight"], ["ember", "Ember"], ["forest", "Forest"]] as const) {
  definePalette(id, label);
}

configureAppearance({
  storageKey: APPEARANCE_KEY,
  defaults: { palette: "cinema", mode: "dark", fontSans: "inter", fontDisplay: "outfit" },
});

export type LegacyTheme = "dark" | "light" | "midnight" | "ember" | "forest" | "system";

/** One-time preference migration. An existing Glass UX preference always wins. */
export function migrateAppearance(storage: Pick<Storage, "getItem" | "setItem">): void {
  if (storage.getItem(APPEARANCE_KEY) !== null) return;
  const old = storage.getItem(LEGACY_THEME_KEY);
  if (!old) return;
  const named = old === "midnight" || old === "ember" || old === "forest";
  const mode = old === "light" || old === "system" ? old : "dark";
  storage.setItem(APPEARANCE_KEY, JSON.stringify({ ...defaultAppearance(), palette: named ? old : "cinema", mode }));
}

/** Apply the configured runtime before React's first paint; the provider owns later updates. */
export function bootAppearance(): void {
  if (typeof window === "undefined") return;
  try {
    migrateAppearance(window.localStorage);
    const stored = window.localStorage.getItem(APPEARANCE_KEY);
    applyAppearance(document.documentElement, normalizeAppearance(JSON.parse(stored ?? "null")), window.matchMedia("(prefers-color-scheme: dark)").matches);
  } catch {
    applyAppearance(document.documentElement, defaultAppearance(), window.matchMedia("(prefers-color-scheme: dark)").matches);
  }
}
