/**
 * The Android TV app (android-tv/) and other constrained devices.
 *
 * The TV app is a WebView shell that loads this frontend from the server. It adds
 * `LibrarianTV/<version>` to the user agent and a `window.LibrarianTV` object (a Java bridge), and it
 * calls `window.librarianTv` for the remote's Back, Menu and media keys (docs/design.md Q137):
 *
 * - `back()` closes an open dialog or menu, leaves fullscreen, then goes back in the app. It returns
 *   `"exit"` on the home screen, where the shell closes the app, and `"unhandled"` when nothing here
 *   took the key (the shell then uses the WebView's own history).
 * - `key(name)` sends a key the WebView does not deliver itself (`MediaPlayPause`, `MediaTrackNext`…)
 *   to the focused element, as a keydown and keyup the players already listen for.
 * - `menu()` opens the focused item's actions (its context menu).
 * - `memoryPressure()` is called when Android asks the app to give memory back.
 *
 * Budget TVs (the user's has 2 GB and about 390 MB free) also get lighter defaults: see
 * `isConstrainedDevice` and its users.
 *
 * The page is laid out on a 10-foot canvas 1920 CSS px wide, whatever density the TV reports: a
 * 1080p TV at density 320 would otherwise give a 960 × 540 phone layout, blown up. The shell builds
 * its WebView at a density that does this (android-tv TvCanvas); `applyTvViewport` is the
 * fallback for a WebView that ignores that density. "Display size" (80–120 %) narrows or widens
 * the canvas; the shell keeps it, so both paths always agree.
 */

const TV_APP_AGENT = /\bLibrarianTV\/([0-9A-Za-z._-]+)/;
const OPEN_OVERLAY = '[role="dialog"], [role="alertdialog"], [role="menu"], [role="listbox"]';

/** The TV app's version, or null outside it. */
export function tvAppVersion(userAgent: string = typeof navigator === "undefined" ? "" : navigator.userAgent): string | null {
  return TV_APP_AGENT.exec(userAgent)?.[1] ?? null;
}

/** Whether this page runs inside the Librarian Android TV app. */
export function isTvApp(userAgent?: string): boolean {
  return tvAppVersion(userAgent) !== null;
}

/** `navigator.deviceMemory` (GB, rounded down to a power of two), where the browser reports it. */
export function deviceMemory(nav: Navigator | undefined = typeof navigator === "undefined" ? undefined : navigator): number | undefined {
  const value = (nav as (Navigator & { deviceMemory?: unknown }) | undefined)?.deviceMemory;
  return typeof value === "number" && value > 0 ? value : undefined;
}

/** 2 GB or less: the budget TVs and phones where WebGL and big layers run out of memory. */
export function isLowMemoryDevice(nav?: Navigator): boolean {
  const memory = deviceMemory(nav);
  return memory !== undefined && memory <= 2;
}

/** The TV app or a low-memory device: blur, animated backgrounds and large artwork default off. */
export function isConstrainedDevice(): boolean {
  return isTvApp() || isLowMemoryDevice();
}

/** What the TV app's Java bridge offers (`window.LibrarianTV`). */
export interface TvAppBridge {
  version(): string;
  /** This TV's display size in percent (shells from 1.0.0). */
  displaySize?(): number;
  /** Store a display size; the shell rebuilds its WebView at the new density and reloads. */
  setDisplaySize?(percent: number): void;
  /** Keep the TV from dimming or starting its screensaver while media plays. */
  setKeepScreenOn(on: boolean): void;
  /** Open the shell's server screen (change or forget the server). */
  openServerSettings(): void;
  /** The server address the shell loads. */
  serverUrl(): string;
}

export function tvAppBridge(): TvAppBridge | null {
  if (typeof window === "undefined") return null;
  const bridge = (window as Window & { LibrarianTV?: Partial<TvAppBridge> }).LibrarianTV;
  return bridge && typeof bridge.setKeepScreenOn === "function" ? (bridge as TvAppBridge) : null;
}

const keepAwake = new Set<symbol>();

/**
 * Hold the screen on while `on`; each caller passes its own token, so the audio dock and the
 * video player can both hold it. A no-op outside the TV app.
 */
export function holdScreenOn(token: symbol, on: boolean): void {
  const before = keepAwake.size > 0;
  if (on) keepAwake.add(token);
  else keepAwake.delete(token);
  const after = keepAwake.size > 0;
  if (before === after) return;
  try {
    tvAppBridge()?.setKeepScreenOn(after);
  } catch {
    // An older shell without the call: the TV's own screensaver rules apply.
  }
}

function focusTarget(): HTMLElement {
  const active = document.activeElement;
  return active instanceof HTMLElement && active !== document.documentElement ? active : document.body;
}

/** Send a key to the focused element as keydown then keyup. True when something handled it. */
export function sendKey(key: string, target: EventTarget = focusTarget()): boolean {
  const init: KeyboardEventInit = { key, bubbles: true, cancelable: true, composed: true };
  const down = new KeyboardEvent("keydown", init);
  target.dispatchEvent(down);
  target.dispatchEvent(new KeyboardEvent("keyup", init));
  return down.defaultPrevented;
}

export type BackResult = "handled" | "exit" | "unhandled";

export interface TvAppDeps {
  /** The current route's path. */
  pathname: () => string;
  /** Free what can be refetched (the Apollo cache). */
  onMemoryPressure?: () => void;
}

/** What the remote's Back does here; exported for tests. */
export function handleBack(deps: TvAppDeps): BackResult {
  if (document.querySelector(OPEN_OVERLAY)) {
    // Dialogs and menus close on Escape; aim it at the overlay so it is the one that closes.
    const overlay = document.querySelector<HTMLElement>(OPEN_OVERLAY);
    const target = overlay?.contains(document.activeElement) ? focusTarget() : (overlay ?? focusTarget());
    sendKey("Escape", target);
    return "handled";
  }
  if (document.fullscreenElement) {
    void document.exitFullscreen?.().catch(() => undefined);
    return "handled";
  }
  if (deps.pathname() === "/") return "exit";
  // The shell's remote focus (useSpatialNavigation) and the video player both go back on GoBack.
  return sendKey("GoBack") ? "handled" : "unhandled";
}

/** Open the focused item's actions, as a right-click would. */
export function openMenu(): boolean {
  if (sendKey("ContextMenu")) return true;
  const target = focusTarget();
  if (target === document.body) return false;
  const rect = target.getBoundingClientRect();
  const event = new MouseEvent("contextmenu", {
    bubbles: true,
    cancelable: true,
    clientX: rect.left + rect.width / 2,
    clientY: rect.top + rect.height / 2,
    button: 2,
  });
  target.dispatchEvent(event);
  return event.defaultPrevented;
}

/* ── The 10-foot canvas ──────────────────────────────────────────────────────────────────────── */

/** CSS width of the TV app's layout at 100 %: a 1080p desktop browser in full screen. */
export const TV_CANVAS_WIDTH = 1920;
/** "Display size" choices, in percent. Larger makes everything bigger (a narrower canvas). */
export const DISPLAY_SIZES = [80, 90, 100, 110, 120] as const;
export const DISPLAY_SIZE_KEY = "librarian.tvDisplaySize.v1";

function localStore(): Storage | null {
  try {
    return typeof window === "undefined" ? null : window.localStorage;
  } catch {
    return null;
  }
}

const validSize = (value: unknown): value is number => (DISPLAY_SIZES as readonly number[]).includes(value as number);

/** This TV's display size in percent: the shell's, else this browser's, else 100. */
export function readDisplaySize(store: Pick<Storage, "getItem"> | null = localStore()): number {
  try {
    const fromShell = tvAppBridge()?.displaySize?.();
    if (validSize(fromShell)) return fromShell;
  } catch {
    // An older shell: fall through to this browser's choice.
  }
  try {
    const stored = Number(store?.getItem(DISPLAY_SIZE_KEY));
    return validSize(stored) ? stored : 100;
  } catch {
    return 100;
  }
}

/** The canvas width for a display size: 1920 at 100 %, 1600 at 120 %. */
export function canvasWidth(displaySize: number): number {
  return Math.round((TV_CANVAS_WIDTH * 100) / displaySize);
}

/**
 * The viewport for a display size: a fixed CSS width with the scale that fits it to the screen.
 * `screenWidth` is the screen's CSS width before scaling (`screen.width`: 960 on a 1080p TV at
 * density 2, 1920 at density 1, 960 on a 4K TV at density 4), so the result is the same canvas on
 * any of them. Zoom stays off: a remote cannot pinch.
 */
export function tvViewportContent(displaySize: number, screenWidth: number): string {
  const width = canvasWidth(displaySize);
  if (!(screenWidth > 0)) return `width=${width}, user-scalable=no`;
  const scale = Number((screenWidth / width).toFixed(4));
  return `width=${width}, initial-scale=${scale}, minimum-scale=${scale}, maximum-scale=${scale}, user-scalable=no`;
}

/**
 * The fallback: when the screen is not already the canvas (the shell's density did not take, or
 * an older shell), point the page's viewport at it. Within 2 % the page is left as it is, so the
 * shell's density and this never both scale it. Returns whether the viewport was rewritten.
 */
export function applyTvViewport(displaySize: number = readDisplaySize()): boolean {
  document.documentElement.style.setProperty("--tv-display-size", String(displaySize / 100));
  const screenWidth = window.screen?.width ?? 0;
  const width = canvasWidth(displaySize);
  if (screenWidth > 0 && Math.abs(screenWidth - width) <= width * 0.02) return false;
  let meta = document.querySelector<HTMLMetaElement>('meta[name="viewport"]');
  if (!meta) {
    meta = document.createElement("meta");
    meta.name = "viewport";
    document.head.append(meta);
  }
  meta.content = tvViewportContent(displaySize, screenWidth);
  return true;
}

const displayListeners = new Set<() => void>();
let displaySnapshot: number | undefined;

/** Change this TV's display size. The shell applies it (and reloads); otherwise it applies here at once. */
export function setDisplaySize(percent: number): void {
  const size = validSize(percent) ? percent : 100;
  const shell = tvAppBridge();
  if (shell?.setDisplaySize) {
    displaySnapshot = size;
    for (const listener of displayListeners) listener();
    shell.setDisplaySize(size);
    return;
  }
  try {
    if (size === 100) localStore()?.removeItem(DISPLAY_SIZE_KEY);
    else localStore()?.setItem(DISPLAY_SIZE_KEY, String(size));
  } catch {
    // Storage is blocked: the size still applies until the app restarts.
  }
  displaySnapshot = size;
  if (isTvApp()) applyTvViewport(size);
  for (const listener of displayListeners) listener();
}

export const displaySizeStore = {
  subscribe(listener: () => void): () => void {
    displayListeners.add(listener);
    return () => displayListeners.delete(listener);
  },
  getSnapshot: (): number => (displaySnapshot ??= readDisplaySize()),
};

/** Install `window.librarianTv`, the `tv-app` marker on <html> and the 10-foot viewport. A no-op outside the TV app. */
export function installTvApp(deps: TvAppDeps): void {
  if (typeof window === "undefined" || !isTvApp()) return;
  document.documentElement.dataset.tvApp = "";
  applyTvViewport();
  (window as Window & { librarianTv?: unknown }).librarianTv = {
    back: () => handleBack(deps),
    key: (name: string) => sendKey(String(name)),
    menu: () => openMenu(),
    memoryPressure: () => deps.onMemoryPressure?.(),
  };
}
