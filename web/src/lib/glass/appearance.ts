import {
  BASE_APPEARANCE,
  applyAppearance,
  configureAppearance,
  defaultAppearance,
  definePalette,
  normalizeAppearance,
  type Appearance,
  type GlassRendering,
} from "glass-ux/appearance";

export const APPEARANCE_KEY = "librarian.appearance.v1";
export const LEGACY_THEME_KEY = "librarian.theme";
/**
 * Whether glass blurs what is behind it. A device setting, not a profile one: it is about this
 * device's graphics, so it is kept in this browser and applies to everyone who signs in on it.
 */
export const BLUR_KEY = "librarian.blur.v1";

export type ColorMode = "dark" | "light";

definePalette("cinema", "Cinema");

/**
 * Everything except the colour mode is fixed: the Cinema palette with full glass, comfortable
 * density, system motion and the library's corner radius, line weight and interface size.
 * Locks win over anything already stored, so older saved choices no longer apply.
 */
export const LOCKED_APPEARANCE = {
  style: "glass",
  background: BASE_APPEARANCE.background,
  fontMono: BASE_APPEARANCE.fontMono,
  palette: "cinema",
  glass: "full",
  density: "comfortable",
  motion: "system",
  radiusScale: 1,
  borderWidth: 1,
  uiScale: 1,
  accentHue: null,
  accentColor: null,
  accentColor2: null,
  canvasTint: null,
  backgroundImage: null,
  fontSans: "inter",
  fontDisplay: "outfit",
} satisfies Omit<Appearance, "mode">;

function storage(): Storage | null {
  try {
    return typeof window === "undefined" ? null : window.localStorage;
  } catch {
    return null;
  }
}

/** Blur is on unless this device has turned it off. */
export function readBlur(store: Pick<Storage, "getItem"> | null = storage()): boolean {
  try {
    return store?.getItem(BLUR_KEY) !== "off";
  } catch {
    return true;
  }
}

function configure(blur: boolean): void {
  const glassRendering: GlassRendering = blur ? "live" : "none";
  // Dark is the default whatever the system prefers; Light is the one alternative offered.
  configureAppearance({ storageKey: APPEARANCE_KEY, defaults: { mode: "dark" }, locked: LOCKED_APPEARANCE, glassRendering });
}

configure(readBlur());

const blurListeners = new Set<() => void>();
let blurSnapshot = readBlur();

/** Turn blur on or off for this device, at once and in every open tab. */
export function setBlur(on: boolean): void {
  try {
    if (on) storage()?.removeItem(BLUR_KEY);
    else storage()?.setItem(BLUR_KEY, "off");
  } catch {
    // Storage is blocked: the choice still applies until the page reloads.
  }
  applyBlur(on);
}

function applyBlur(on: boolean): void {
  if (on === blurSnapshot) return;
  blurSnapshot = on;
  configure(on);
  if (typeof document !== "undefined") {
    if (on) delete document.documentElement.dataset.glassRendering;
    else document.documentElement.dataset.glassRendering = "none";
  }
  for (const listener of blurListeners) listener();
}

export const blurStore = {
  subscribe(listener: () => void): () => void {
    blurListeners.add(listener);
    // Another tab changed it.
    const onStorage = (event: StorageEvent) => {
      if (event.key === BLUR_KEY || event.key === null) applyBlur(readBlur());
    };
    window.addEventListener("storage", onStorage);
    return () => {
      blurListeners.delete(listener);
      window.removeEventListener("storage", onStorage);
    };
  },
  getSnapshot: (): boolean => blurSnapshot,
};

/**
 * Keep only what can still be chosen. The colour mode survives (System, now retired, becomes
 * Dark); every other stored knob is dropped, and the one-time `librarian.theme` migration maps
 * Light to Light and everything else to Dark.
 */
export function migrateAppearance(store: Pick<Storage, "getItem" | "setItem" | "removeItem">): void {
  const stored = store.getItem(APPEARANCE_KEY);
  if (stored === null) {
    const legacy = store.getItem(LEGACY_THEME_KEY);
    if (legacy) store.setItem(APPEARANCE_KEY, JSON.stringify({ mode: legacy === "light" ? "light" : "dark" }));
    return;
  }
  let mode: ColorMode = "dark";
  try {
    const parsed: unknown = JSON.parse(stored);
    if (parsed && typeof parsed === "object" && (parsed as { mode?: unknown }).mode === "light") mode = "light";
  } catch {
    // Unreadable: start again from the default.
  }
  const clean = JSON.stringify({ mode });
  if (stored !== clean) store.setItem(APPEARANCE_KEY, clean);
}

/** Apply the configured runtime before React's first paint; the provider owns later updates. */
export function bootAppearance(): void {
  if (typeof window === "undefined") return;
  const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
  try {
    migrateAppearance(window.localStorage);
    const stored = window.localStorage.getItem(APPEARANCE_KEY);
    applyAppearance(document.documentElement, normalizeAppearance(JSON.parse(stored ?? "null")), prefersDark);
  } catch {
    applyAppearance(document.documentElement, defaultAppearance(), prefersDark);
  }
}
