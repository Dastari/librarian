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
 * - `keyEvent(name, "keydown" | "keyup", repeat)` forwards a remote button the WebView does not
 *   deliver itself to the focused element as a standard KeyboardEvent (design.md Q137 lists the names): `MediaPlayPause`,
 *   `MediaPlay`, `MediaPause`, `MediaStop`, `MediaTrackNext`, `MediaTrackPrevious`, `MediaRewind`
 *   and `MediaFastForward` (repeating while held), `Info`, `Guide`, `PageUp`, `PageDown`.
 * - `menu()` opens the focused item's actions (its context menu).
 * - `memoryPressure()` is called when Android asks the app to give memory back.
 *
 * Budget TVs (the user's has 2 GB and about 390 MB free) also get lighter defaults: see
 * `isConstrainedDevice` and its users.
 *
 * The shell lays the page out on a 10-foot canvas by rewriting its viewport meta (1920 CSS px wide
 * on a 1080p TV, which reports density 320 and would otherwise give a 960 × 540 phone layout). The
 * web app never touches the viewport meta itself, so nothing undoes that.
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
  /** This TV's display size in percent. */
  displaySize?(): number;
  /** Store a display size; the shell rewrites the viewport at once. */
  setDisplaySize?(percent: number): void;
  /** JSON `{step, max, muted, fixed}` for the TV's media volume. */
  volume?(): string;
  /** Set the TV's media volume, 0 to 1 (the shell rounds to a whole step). */
  setVolume?(level: number): void;
  /** Mute or unmute the TV; false when it could not. */
  setMuted?(muted: boolean): boolean;
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
  const handled = dispatchKey(key, "keydown", false, target);
  dispatchKey(key, "keyup", false, target);
  return handled;
}

/**
 * One standard KeyboardEvent at the focused element, as the shell forwards a remote button: a held
 * button gives repeated keydowns with `repeat: true`, then one keyup, as a keyboard does.
 */
export function dispatchKey(key: string, type: "keydown" | "keyup", repeat = false, target: EventTarget = focusTarget()): boolean {
  const event = new KeyboardEvent(type, { key, repeat, bubbles: true, cancelable: true, composed: true });
  target.dispatchEvent(event);
  return event.defaultPrevented;
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

/* ── Display size ───────────────────────────────────────────────────────────────────────────── */

/**
 * "Display size" choices, in percent. The shell sets the page's viewport to
 * `panel width × 100 / size` CSS px (1920 at 100 % on a 1080p TV, 1600 at 120 %), so larger makes
 * everything bigger. The shell alone writes the viewport and keeps the size; nothing here does.
 */
export const DISPLAY_SIZES = [80, 90, 100, 110, 120] as const;

const validSize = (value: unknown): value is number => (DISPLAY_SIZES as readonly number[]).includes(value as number);

/** This TV's display size in percent (100 outside the TV app). */
export function readDisplaySize(): number {
  try {
    const size = tvAppBridge()?.displaySize?.();
    return validSize(size) ? size : 100;
  } catch {
    return 100;
  }
}

const displayListeners = new Set<() => void>();
let displaySnapshot: number | undefined;

/** Change this TV's display size; the shell rewrites the viewport at once, without a reload. */
export function setDisplaySize(percent: number): void {
  const size = validSize(percent) ? percent : 100;
  try {
    tvAppBridge()?.setDisplaySize?.(size);
  } catch {
    return;
  }
  displaySnapshot = size;
  for (const listener of displayListeners) listener();
}

export const displaySizeStore = {
  subscribe(listener: () => void): () => void {
    displayListeners.add(listener);
    return () => displayListeners.delete(listener);
  },
  getSnapshot: (): number => (displaySnapshot ??= readDisplaySize()),
};

/* ── The TV's volume ────────────────────────────────────────────────────────────────────────── */

/** The TV's media volume as the shell reports it (`LibrarianTV.volume()` and the event below). */
export interface TvVolume {
  /** 0 to 1, always a whole step (`step / max`). */
  level: number;
  step: number;
  max: number;
  muted: boolean;
  /** The app cannot set the level (fixed volume, or a sound system over HDMI that ignores it). */
  fixed: boolean;
}

/** Sent on `window` by the shell whenever the TV's volume changes, from anywhere. */
export const TV_VOLUME_EVENT = "librarian-tv-volume";

function parseVolume(value: unknown): TvVolume | null {
  let data: unknown = value;
  if (typeof value === "string") {
    try {
      data = JSON.parse(value);
    } catch {
      return null;
    }
  }
  if (!data || typeof data !== "object") return null;
  const { step, max, muted, fixed } = data as Record<string, unknown>;
  if (typeof step !== "number" || typeof max !== "number" || !(max > 0)) return null;
  const clamped = Math.min(max, Math.max(0, Math.round(step)));
  return { level: clamped / max, step: clamped, max, muted: muted === true, fixed: fixed === true };
}

/** The nearest whole step for a level, as a level. */
export function quantizeVolume(level: number, max: number): number {
  return Math.round(Math.min(1, Math.max(0, level)) * max) / max;
}

let volumeSnapshot: TvVolume | null | undefined;

/** Tests only: forget the cached volume so the next read asks the bridge. */
export function resetTvVolumeForTests(): void {
  volumeSnapshot = undefined;
}
const volumeListeners = new Set<() => void>();

function readTvVolume(): TvVolume | null {
  try {
    const bridge = tvAppBridge();
    return bridge?.volume ? parseVolume(bridge.volume()) : null;
  } catch {
    return null;
  }
}

function publishVolume(next: TvVolume | null): void {
  volumeSnapshot = next;
  for (const listener of volumeListeners) listener();
}

function onVolumeEvent(event: Event): void {
  const next = parseVolume((event as CustomEvent).detail);
  if (next) publishVolume(next);
}

export const tvVolumeStore = {
  subscribe(listener: () => void): () => void {
    if (volumeListeners.size === 0) window.addEventListener(TV_VOLUME_EVENT, onVolumeEvent);
    volumeListeners.add(listener);
    return () => {
      volumeListeners.delete(listener);
      if (volumeListeners.size === 0) window.removeEventListener(TV_VOLUME_EVENT, onVolumeEvent);
    };
  },
  /** Null outside the TV app (or a shell without volume): players keep their own volume then. */
  getSnapshot: (): TvVolume | null => (volumeSnapshot === undefined ? (volumeSnapshot = readTvVolume()) : volumeSnapshot),
};

/** Set the TV's volume (0–1). Shown at once at the nearest step; the shell's event confirms it. */
export function setTvVolume(level: number): void {
  const current = tvVolumeStore.getSnapshot();
  const bridge = tvAppBridge();
  if (!current || !bridge?.setVolume) return;
  const step = Math.round(quantizeVolume(level, current.max) * current.max);
  bridge.setVolume(step / current.max);
  publishVolume({ ...current, step, level: step / current.max, muted: step === 0 ? current.muted : false });
}

/** Mute or unmute the TV. False when the TV would not (a sound system that ignores it). */
export function setTvMuted(muted: boolean): boolean {
  const current = tvVolumeStore.getSnapshot();
  const bridge = tvAppBridge();
  if (!current || !bridge?.setMuted) return false;
  const done = bridge.setMuted(muted) !== false;
  if (done) publishVolume({ ...current, muted });
  return done;
}

const mediaVolume = () => Object.getOwnPropertyDescriptor(HTMLMediaElement.prototype, "volume")!;
const mediaMuted = () => Object.getOwnPropertyDescriptor(HTMLMediaElement.prototype, "muted")!;

/**
 * Let a media element's volume and mute drive the TV's (Q137). The element itself stays at full
 * volume, unmuted, so the two levels never multiply. Its `volume` and `muted` then read and write
 * the TV's, and a change made on the remote fires `volumechange`, so a player bound to the element
 * (Glass's controller) shows it live. Mute falls back to the element when the TV will not mute.
 * Returns the undo.
 */
export function bindTvVolume(media: HTMLMediaElement): () => void {
  const volume = mediaVolume();
  const muted = mediaMuted();
  volume.set!.call(media, 1);
  muted.set!.call(media, false);
  const fire = () => media.dispatchEvent(new Event("volumechange"));
  Object.defineProperty(media, "volume", {
    configurable: true,
    get: () => {
      const tv = tvVolumeStore.getSnapshot();
      return tv && !tv.fixed ? tv.level : 1;
    },
    set: (level: number) => {
      if (tvVolumeStore.getSnapshot()?.fixed) return;
      setTvVolume(level);
      fire();
    },
  });
  Object.defineProperty(media, "muted", {
    configurable: true,
    get: () => Boolean(tvVolumeStore.getSnapshot()?.muted) || (muted.get!.call(media) as boolean),
    set: (value: boolean) => {
      const own = muted.get!.call(media) as boolean;
      if (setTvMuted(value)) {
        if (own) muted.set!.call(media, false);
      } else {
        muted.set!.call(media, value);
      }
      fire();
    },
  });
  const unsubscribe = tvVolumeStore.subscribe(fire);
  return () => {
    unsubscribe();
    delete (media as { volume?: number }).volume;
    delete (media as { muted?: boolean }).muted;
  };
}

/** Install `window.librarianTv` and the `tv-app` marker on <html>. A no-op outside the TV app. */
export function installTvApp(deps: TvAppDeps): void {
  if (typeof window === "undefined" || !isTvApp()) return;
  document.documentElement.dataset.tvApp = "";
  (window as Window & { librarianTv?: unknown }).librarianTv = {
    back: () => handleBack(deps),
    key: (name: string) => sendKey(String(name)),
    keyEvent: (name: string, type: string, repeat?: boolean) => dispatchKey(String(name), type === "keyup" ? "keyup" : "keydown", repeat === true),
    menu: () => openMenu(),
    memoryPressure: () => deps.onMemoryPressure?.(),
  };
}
