/**
 * The remote's Back key, shared by everything that reacts to it (design.md Q142).
 *
 * Back arrives as `Escape` (a keyboard, the Android TV app aiming at an overlay), `GoBack` (the
 * Android TV app), `BrowserBack`, or only as a key code on webOS (461) and Tizen (10009). In TV
 * mode Backspace outside a text field is Back too.
 *
 * Whether something already took a Back press can't be read from `defaultPrevented`: a focused
 * icon button's tooltip prevents Escape's default as it closes (Glass's tooltip host, then Base
 * UI's dismiss) and still lets it carry on, so every "already handled?" check would ignore Back
 * whenever a tooltip happened to be showing. `backHandled` asks the real question instead: did a
 * control, a page or a player take it? Document-level listeners are the dismiss layers (tooltips
 * here; dialogs and menus are open overlays, which the Back order checks first), so their
 * `preventDefault` does not count.
 */

/** webOS (461) and Tizen (10009) report their remote's Back key only as a key code. */
export const BACK_KEY_CODES: ReadonlySet<number> = new Set([461, 10009]);

type KeyLike = Pick<KeyboardEvent, "key" | "keyCode">;

/** Keys that are always Back, wherever focus is. */
export function isRemoteBackKey(event: KeyLike): boolean {
  return event.key === "GoBack" || event.key === "BrowserBack" || BACK_KEY_CODES.has(event.keyCode);
}

/**
 * Whether this key is Back. `Escape` always is; `Backspace` only on a TV and outside a text
 * field, where it would otherwise delete a character.
 */
export function isBackKey(event: KeyLike, options: { tv?: boolean; editing?: boolean } = {}): boolean {
  if (event.key === "Escape" || isRemoteBackKey(event)) return true;
  return event.key === "Backspace" && Boolean(options.tv) && !options.editing;
}

/** Whether focus is in something that takes typing. */
export function isEditing(element: Element | null): boolean {
  if (!(element instanceof HTMLElement)) return false;
  return element.tagName === "INPUT" || element.tagName === "TEXTAREA" || element.tagName === "SELECT" || element.isContentEditable;
}

const taken = new WeakSet<Event>();
/** Back presses whose `preventDefault` could not be wrapped; `defaultPrevented` stands in. */
const unwatched = new WeakSet<Event>();
const WATCHED = new Set(["Escape", "Backspace", "GoBack", "BrowserBack"]);

/** Seen first, on the window's capture phase: note who prevents this Back press's default. */
function watch(event: KeyboardEvent) {
  if (!WATCHED.has(event.key) && !BACK_KEY_CODES.has(event.keyCode)) return;
  const prevent = event.preventDefault;
  try {
    Object.defineProperty(event, "preventDefault", {
      configurable: true,
      value: function preventDefault(this: KeyboardEvent) {
        if (event.currentTarget !== document) taken.add(event);
        prevent.call(event);
      },
    });
  } catch {
    // Something already fixed `preventDefault` on this event (a synthetic event from a test
    // tool, an extension). Fall back to the plain flag for it.
    unwatched.add(event);
  }
}

let watching = false;

/** Start watching Back presses; idempotent. The module does this when it loads in a browser. */
export function watchBackKeys(): void {
  if (watching || typeof window === "undefined") return;
  watching = true;
  window.addEventListener("keydown", watch, true);
}

watchBackKeys();

/**
 * Whether a control, a page or a player already took this Back press. A tooltip closing on it
 * does not count, so Back still reaches the next step (design.md Q142).
 */
export function backHandled(event: Event | { nativeEvent: Event }): boolean {
  const native = "nativeEvent" in event ? event.nativeEvent : event;
  return taken.has(native) || (unwatched.has(native) && native.defaultPrevented);
}

