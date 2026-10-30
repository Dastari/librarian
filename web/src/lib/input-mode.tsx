/**
 * Input mode detection and spatial navigation.
 *
 * The app renders the same components for mouse, touch and remote-control users, but the
 * shell adapts: `data-input-mode` on <html> switches CSS (see tokens.css) and the spatial
 * navigation hook lets arrow keys move focus between any `[data-focusable]` element the
 * way a TV remote expects. Remotes and gamepads arrive as keyboard events (ArrowUp, Enter,
 * Backspace/Escape for back), plus the webOS/Tizen back key codes handled below.
 *
 * Remote behaviour follows TV home screens:
 * - `[data-nav-group]` (a poster row or grid) remembers its last focused item, so moving
 *   down and back up returns to the same card instead of the geometrically nearest one;
 *   `[data-nav-default]` inside it is where focus lands the first time.
 * - `[data-nav-section]` (a hero, a titled row) is revealed whole when focus enters it, so
 *   a row is never left half under the fold. Rows scroll sideways to keep the card in view.
 * - `[data-tv-autofocus]` receives focus when a page opens in TV mode (normally Play).
 * - `[data-nav-zone][data-nav-side]` (the rail, the letter rail) is entered and left only with
 *   Left/Right; other zones (the top bar) are entered vertically when content has nothing
 *   further that way.
 * - `[data-tv-skip]` controls (or regions) are pointer shortcuts a remote reaches another way.
 */
import { prefersReducedMotion } from "glass-ux/appearance";
import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";

export type InputMode = "pointer" | "touch" | "tv";

interface InputModeContextValue {
  mode: InputMode;
  /** Set when the user explicitly picks TV mode from the account menu. */
  forced: InputMode | null;
  setForced: (mode: InputMode | null) => void;
}

const STORAGE_KEY = "librarian.inputMode";
const InputModeContext = createContext<InputModeContextValue | null>(null);

function detectInitialMode(): InputMode {
  if (typeof window === "undefined") return "pointer";
  const ua = navigator.userAgent;
  if (/\b(SMART-TV|SmartTV|Tizen|Web0S|WebOS|BRAVIA|AppleTV|GoogleTV|Android TV|AFT[A-Z]?\b)/i.test(ua)) return "tv";
  if (window.matchMedia("(hover: none) and (pointer: coarse)").matches) return "touch";
  return "pointer";
}

function readForced(): InputMode | null {
  if (typeof window === "undefined") return null;
  const stored = window.localStorage.getItem(STORAGE_KEY);
  return stored === "tv" || stored === "pointer" || stored === "touch" ? stored : null;
}

export function InputModeProvider({ children }: { children: ReactNode }) {
  const [detected, setDetected] = useState<InputMode>(detectInitialMode);
  const [forced, setForcedState] = useState<InputMode | null>(readForced);

  useEffect(() => {
    // Promote to keyboard-driven behaviour when the user navigates with arrows for a while.
    let arrowPresses = 0;
    const onKey = (event: KeyboardEvent) => {
      if (typeof event.key === "string" && event.key.startsWith("Arrow")) {
        arrowPresses += 1;
        if (arrowPresses >= 6 && detected === "pointer") setDetected("tv");
      }
    };
    const onPointer = (event: PointerEvent) => {
      if (event.pointerType === "mouse" && detected === "tv" && arrowPresses < 6) setDetected("pointer");
    };
    window.addEventListener("keydown", onKey);
    window.addEventListener("pointerdown", onPointer);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("pointerdown", onPointer);
    };
  }, [detected]);

  const mode = forced ?? detected;

  useEffect(() => {
    document.documentElement.dataset.inputMode = mode;
  }, [mode]);

  const setForced = useCallback((next: InputMode | null) => {
    if (next) window.localStorage.setItem(STORAGE_KEY, next);
    else window.localStorage.removeItem(STORAGE_KEY);
    setForcedState(next);
  }, []);

  const value = useMemo(() => ({ mode, forced, setForced }), [mode, forced, setForced]);
  return <InputModeContext.Provider value={value}>{children}</InputModeContext.Provider>;
}

export function useInputMode(): InputModeContextValue {
  const context = useContext(InputModeContext);
  if (!context) throw new Error("useInputMode must be used within InputModeProvider");
  return context;
}

const FOCUSABLE_SELECTOR =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"]), [data-focusable]';

/** The focusable selector scoped to the routed content (each list entry needs the prefix). */
const IN_MAIN = FOCUSABLE_SELECTOR.split(",").map((selector) => `main ${selector.trim()}`).join(", ");

type Direction = "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight";

/** webOS (461) and Tizen (10009) report their remote's Back key only as a key code. */
const BACK_KEY_CODES = new Set([461, 10009]);
const OPEN_OVERLAY = '[role="dialog"], [role="alertdialog"], [role="menu"], [role="listbox"]';

function isVisible(element: HTMLElement): boolean {
  const rect = element.getBoundingClientRect();
  if (rect.width === 0 && rect.height === 0) return false;
  const style = window.getComputedStyle(element);
  return style.visibility !== "hidden" && style.display !== "none";
}

/** Last focused element per navigation group (poster row, grid). */
const groupMemory = new WeakMap<Element, HTMLElement>();

const isSide = (zone: Element | null) => Boolean(zone?.hasAttribute("data-nav-side"));

/** Last focused control in the routed content, for returning from the rail. */
let contentMemory: HTMLElement | null = null;

function rememberFocus(event: FocusEvent) {
  const target = event.target;
  if (!(target instanceof HTMLElement)) return;
  const group = target.closest("[data-nav-group]");
  if (group) groupMemory.set(group, target);
  if (target.closest("main")) contentMemory = target;
}

function inViewport(element: HTMLElement): boolean {
  const rect = element.getBoundingClientRect();
  return rect.bottom > 0 && rect.top < window.innerHeight && rect.right > 0 && rect.left < window.innerWidth;
}

/** Entering a group from outside resumes at its remembered item when that is still usable. */
function resumeInGroup(current: HTMLElement, candidate: HTMLElement): HTMLElement {
  const group = candidate.closest("[data-nav-group]");
  if (!group || group.contains(current)) return candidate;
  const remembered = groupMemory.get(group);
  if (remembered && remembered.isConnected && group.contains(remembered) && isVisible(remembered) && !remembered.matches(":disabled")) return remembered;
  // A group can name where a remote should land first (the next episode to watch).
  return group.querySelector<HTMLElement>("[data-nav-default]") ?? candidate;
}

/**
 * Picks the focus candidate in a direction the way TV focus engines do.
 *
 * Up/Down moves to the nearest row first: candidates are grouped by their navigation section
 * (a titled row, the hero), the section with the smallest edge gap wins, and inside it the
 * closest control by spatial-navigation distance (gap plus a penalty for sideways offset).
 * Controls outside any section compete by that distance directly. Left/Right stays within
 * the current row (controls overlapping it vertically) and does nothing at the row's end,
 * except to enter the rail.
 */
export function findNext(current: HTMLElement, direction: Direction, scope: ParentNode = document): HTMLElement | null {
  const origin = current.getBoundingClientRect();
  const vertical = direction === "ArrowUp" || direction === "ArrowDown";
  const zone = current.closest("[data-nav-zone]");
  const tv = document.documentElement.dataset.inputMode === "tv";
  type Scored = { element: HTMLElement; gap: number; distance: number; centre: number; overlaps: boolean; group: Element; zone: Element | null };
  const scored: Scored[] = [];

  for (const candidate of scope.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)) {
    if (candidate === current || candidate.closest("[aria-hidden='true'], [inert]") || !isVisible(candidate)) continue;
    const candidateZone = candidate.closest("[data-nav-zone]");
    // Side zones (the rail, the letter rail) are entered and left sideways only.
    if (vertical && candidateZone !== zone && (isSide(candidateZone) || isSide(zone))) continue;
    // Secondary overlay targets (a tile's play button) are reached by the remote's keys instead.
    if (tv && candidate.closest("[data-tv-skip]")) continue;
    // Focus lands on controls, not on scrollable regions that contain them.
    if (!candidate.matches("a[href], button, input, select, textarea") && candidate.querySelector(FOCUSABLE_SELECTOR)) continue;
    const rect = candidate.getBoundingClientRect();
    const cx = rect.left + rect.width / 2, cy = rect.top + rect.height / 2;
    const ox = origin.left + origin.width / 2, oy = origin.top + origin.height / 2;
    const ahead = direction === "ArrowDown" ? cy - oy : direction === "ArrowUp" ? oy - cy : direction === "ArrowRight" ? cx - ox : ox - cx;
    if (ahead <= 4) continue;
    const gap = Math.max(0, direction === "ArrowDown" ? rect.top - origin.bottom : direction === "ArrowUp" ? origin.top - rect.bottom : direction === "ArrowRight" ? rect.left - origin.right : origin.left - rect.right);
    const sideways = vertical ? Math.max(0, rect.left - origin.right, origin.left - rect.right) : Math.max(0, rect.top - origin.bottom, origin.top - rect.bottom);
    const distance = Math.hypot(gap, sideways) + sideways * (vertical ? 2 : 30);
    scored.push({
      element: candidate, gap, distance, zone: candidateZone,
      centre: vertical ? Math.abs(cx - ox) : Math.abs(cy - oy),
      overlaps: sideways === 0,
      group: candidate.closest("[data-nav-section]") ?? candidate,
    });
  }
  if (scored.length === 0) return null;
  const best = (pool: Scored[]) => pool.reduce((a, b) => (b.distance < a.distance || (b.distance === a.distance && b.centre < a.centre) ? b : a));

  if (!vertical) {
    const fromSide = isSide(zone);
    // Leaving a side zone returns to the control the person left, when it is still on screen
    // and lies that way.
    if (fromSide && contentMemory?.isConnected && isVisible(contentMemory) && inViewport(contentMemory)) {
      const rect = contentMemory.getBoundingClientRect();
      if (direction === "ArrowRight" ? rect.left >= origin.right : rect.right <= origin.left) return contentMemory;
    }
    const inRow = scored.filter((item) => item.overlaps && item.zone === zone);
    const otherZone = scored.filter((item) => item.zone !== zone && (fromSide ? !isSide(item.zone) : isSide(item.zone)));
    const pool = inRow.length > 0 ? inRow : otherZone;
    return pool.length > 0 ? resumeInGroup(current, best(pool).element) : null;
  }

  // Leave the current zone (content → top bar) only when nothing in it lies that way.
  const sameZone = scored.filter((item) => item.zone === zone);
  const pool = sameZone.length > 0 ? sameZone : scored;
  const groups = new Map<Element, Scored[]>();
  for (const item of pool) groups.set(item.group, [...(groups.get(item.group) ?? []), item]);
  let chosen: Scored[] | undefined;
  let chosenScore = Number.POSITIVE_INFINITY;
  for (const [group, items] of groups) {
    const section = group.hasAttribute("data-nav-section");
    const score = section ? Math.min(...items.map((item) => item.gap)) : Math.min(...items.map((item) => item.distance));
    if (score < chosenScore) {
      chosenScore = score;
      chosen = items;
    }
  }
  // Inside the section, the nearest row of it first (season tabs before the episodes below).
  const nearestGap = Math.min(...chosen!.map((item) => item.gap));
  const row = chosen!.filter((item) => item.gap <= nearestGap + Math.max(24, origin.height * 0.5));
  return resumeInGroup(current, best(row).element);
}

function scrollBehavior(): ScrollBehavior {
  return prefersReducedMotion() ? "auto" : "smooth";
}

/** Scroll a horizontal row so the element is inside it with a gutter's breathing room. */
function revealInRow(element: HTMLElement, behavior: ScrollBehavior) {
  const row = element.closest<HTMLElement>("[data-nav-group]");
  if (!row || row.scrollWidth <= row.clientWidth) return false;
  const rowRect = row.getBoundingClientRect();
  const rect = element.getBoundingClientRect();
  const padding = parseFloat(getComputedStyle(row).scrollPaddingInlineStart) || 0;
  let delta = 0;
  if (rect.left < rowRect.left + padding) delta = rect.left - rowRect.left - padding;
  else if (rect.right > rowRect.right - padding) delta = rect.right - rowRect.right + padding;
  if (delta !== 0) row.scrollBy({ left: delta, behavior });
  return true;
}

/**
 * Reveal a newly focused element. Its section scrolls fully into view when it fits, so a
 * remote never leaves a row cut by the fold; the first section scrolls to the very top so a
 * hero shows whole. Taller sections fall back to the element itself.
 */
export function revealFocused(element: HTMLElement) {
  const behavior = scrollBehavior();
  const inRow = revealInRow(element, behavior);
  const scroller = element.closest<HTMLElement>("#main-scroll");
  // The largest enclosing section that fits: a row together with the rows it belongs with.
  let section: HTMLElement | null = null;
  for (let candidate = element.closest<HTMLElement>("[data-nav-section]"); candidate && scroller?.contains(candidate); candidate = candidate.parentElement?.closest<HTMLElement>("[data-nav-section]") ?? null) {
    if (candidate.getBoundingClientRect().height > scroller.clientHeight) break;
    section = candidate;
  }
  if (section && scroller) {
    const first = scroller.querySelector("[data-nav-section]") === section;
    if (first) scroller.scrollTo({ top: 0, behavior });
    else section.scrollIntoView({ block: "nearest", inline: "nearest", behavior });
    return;
  }
  // Tall sections (a long episode list): leave a visible element alone, otherwise centre it so
  // the rows around it stay in view.
  if (scroller) {
    const view = scroller.getBoundingClientRect();
    const top = view.top + (parseFloat(getComputedStyle(scroller).getPropertyValue("--topbar-height")) || 0);
    const rect = element.getBoundingClientRect();
    if (rect.top >= top && rect.bottom <= view.bottom) return;
  }
  element.scrollIntoView({ block: "center", inline: inRow ? undefined : "nearest", behavior });
}

/** Composites that own every arrow key: two-dimensional or popup widgets. */
const OWNS_ALL = '[role="grid"], [role="tree"], [role="menu"], [role="listbox"], [aria-haspopup="tree"], [aria-haspopup="grid"]';
/** Popup triggers open with Up/Down only; sideways arrows keep moving between controls. */
const OWNS_VERTICAL = '[aria-haspopup="menu"], [aria-haspopup="listbox"]';
/** One-dimensional composites: they own the arrows along their axis only. */
const OWNS_AXIS = '[role="tablist"], [role="radiogroup"], .fa-segmented, .fa-swatch-picker';

/**
 * Whether the focused control's composite consumes this arrow (Base UI moves within it).
 * A horizontal tab strip or segmented control keeps Left/Right but lets Up/Down leave it,
 * so a remote is never trapped in a row of tabs.
 */
function compositeOwns(active: HTMLElement, direction: Direction): boolean {
  const role = active.getAttribute("role");
  if (role === "slider" || role === "menuitem" || role === "option" || role === "listbox" || role === "combobox" || role === "treeitem" || role === "gridcell") return true;
  if (active.closest(OWNS_ALL)) return true;
  if (active.matches(OWNS_VERTICAL)) return direction === "ArrowUp" || direction === "ArrowDown";
  const strip = active.closest<HTMLElement>(OWNS_AXIS);
  if (!strip) return false;
  const vertical = (strip.getAttribute("aria-orientation") ?? strip.querySelector("[aria-orientation]")?.getAttribute("aria-orientation")) === "vertical";
  const horizontalKey = direction === "ArrowLeft" || direction === "ArrowRight";
  return vertical ? !horizontalKey : horizontalKey;
}

/**
 * On a TV a single-line field must not trap the remote: Up/Down always leave it (they do
 * nothing in one line), and Left/Right leave once the caret is at that end of the text.
 */
function leavesField(active: HTMLElement, direction: Direction): boolean {
  if (!(active instanceof HTMLInputElement) || active.getAttribute("role") === "combobox") return false;
  if (direction === "ArrowUp" || direction === "ArrowDown") return true;
  const { selectionStart, selectionEnd, value } = active;
  if (selectionStart === null || selectionStart !== selectionEnd) return false;
  const start = getComputedStyle(active).direction === "rtl" ? "ArrowRight" : "ArrowLeft";
  return direction === start ? selectionStart === 0 : selectionStart === value.length;
}

function overlayOpen(): boolean {
  return Boolean(document.querySelector(OPEN_OVERLAY));
}

/**
 * Global arrow-key focus movement. Active in every mode so keyboard users on desktop get it
 * for free; text inputs, sliders and open menus keep their native arrow behaviour. In TV
 * mode Backspace and Escape act as the remote's Back key when nothing else consumes them.
 */
export function useSpatialNavigation(enabled = true, tv = false, onBack: () => void = () => window.history.back()): void {
  useEffect(() => {
    if (!enabled) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.altKey || event.ctrlKey || event.metaKey) return;
      const active = document.activeElement as HTMLElement | null;
      const editing = Boolean(active && (active.tagName === "INPUT" || active.tagName === "TEXTAREA" || active.tagName === "SELECT" || active.isContentEditable));
      const back = BACK_KEY_CODES.has(event.keyCode) || event.key === "GoBack" || event.key === "BrowserBack" || (tv && !editing && (event.key === "Backspace" || event.key === "Escape"));
      if (back) {
        if (overlayOpen()) return;
        event.preventDefault();
        onBack();
        return;
      }
      const direction = event.key as Direction;
      if (!["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"].includes(direction)) return;
      if (!active || active === document.body) {
        const first = document.querySelector<HTMLElement>("main [data-tv-autofocus]") ?? document.querySelector<HTMLElement>(`main [data-spatial-start], ${IN_MAIN}`);
        first?.focus();
        if (first) event.preventDefault();
        return;
      }
      if ((editing && !(tv && leavesField(active, direction))) || compositeOwns(active, direction)) return;
      if (active.closest("[data-spatial-ignore]")) return;
      const scope = active.closest<HTMLElement>('[role="dialog"], [role="alertdialog"]') ?? document;
      const next = findNext(active, direction, scope);
      if (!next) return;
      event.preventDefault();
      next.focus({ preventScroll: true });
      revealFocused(next);
    };
    // Base UI modal scopes stop bubbling key events. Handle ordinary modal
    // controls before that boundary; composites still take the exemptions above.
    // On a TV a ⋯ menu button opens with OK; Up/Down must move past it, so take those
    // arrows before Base UI's trigger turns them into "open the menu".
    const onModalKeyDown = (event: KeyboardEvent) => {
      const active = document.activeElement as HTMLElement | null;
      if (tv && active?.matches('[aria-haspopup="menu"]:not([role="menuitem"])') && (event.key === "ArrowUp" || event.key === "ArrowDown")) {
        const scope = active.closest<HTMLElement>('[role="dialog"], [role="alertdialog"]') ?? document;
        const next = findNext(active, event.key, scope);
        if (next) {
          event.preventDefault();
          event.stopPropagation();
          next.focus({ preventScroll: true });
          revealFocused(next);
        }
        return;
      }
      if (active?.closest('[role="dialog"], [role="alertdialog"]')) onKeyDown(event);
    };
    window.addEventListener("keydown", onModalKeyDown, true);
    window.addEventListener("keydown", onKeyDown);
    document.addEventListener("focusin", rememberFocus);
    return () => {
      window.removeEventListener("keydown", onModalKeyDown, true);
      window.removeEventListener("keydown", onKeyDown);
      document.removeEventListener("focusin", rememberFocus);
    };
  }, [enabled, tv, onBack]);
}

/** Identify a focused element well enough to find its replacement after a re-render. */
function focusKey(element: Element): string | null {
  const link = element.closest("a[href]");
  if (link) return `a[href="${CSS.escape(link.getAttribute("href") ?? "")}"]`;
  const label = element.getAttribute("aria-label");
  return label ? `[aria-label="${CSS.escape(label)}"]` : null;
}

/**
 * TV route focus. When a page opens, focus moves into it (its `[data-tv-autofocus]` control,
 * otherwise its first control) so the remote never starts from nowhere. Returning to a page
 * restores the card that was opened from it.
 */
export function useRouteFocus(pathname: string, active: boolean) {
  useEffect(() => {
    if (!active) return;
    const main = document.getElementById("main");
    if (!main) return;
    const remembered = routeFocus.get(pathname);
    // The previous page can stay mounted for a frame; only a newly focused control counts as
    // a choice the user made on this page.
    const previous = document.activeElement;
    let frame = 0;
    const started = performance.now();
    const settle = () => {
      const focused = document.activeElement;
      if (focused && focused !== previous && focused !== document.body && focused !== main && main.contains(focused)) return;
      const restore = remembered ? main.querySelector<HTMLElement>(remembered) : null;
      const elapsed = performance.now() - started;
      // Prefer the page's own start (Play), waiting for it while its data loads; otherwise
      // any control. A page that is still a skeleton keeps waiting a little longer.
      const start = main.querySelector<HTMLElement>("[data-tv-autofocus]");
      const target = restore
        ?? (start && start.getAttribute("aria-disabled") !== "true" ? start : null)
        ?? (elapsed > (start ? 3000 : 300) ? main.querySelector<HTMLElement>("[data-spatial-start], " + FOCUSABLE_SELECTOR) : null);
      if (target) {
        target.focus({ preventScroll: Boolean(restore) });
        if (!restore) revealFocused(target);
        return;
      }
      if (elapsed < 5000) frame = requestAnimationFrame(settle);
    };
    frame = requestAnimationFrame(settle);
    const record = (event: FocusEvent) => {
      if (event.target instanceof Element && main.contains(event.target)) {
        const key = focusKey(event.target);
        if (key) routeFocus.set(pathname, key);
      }
    };
    document.addEventListener("focusin", record);
    return () => {
      cancelAnimationFrame(frame);
      document.removeEventListener("focusin", record);
    };
  }, [pathname, active]);
}

const routeFocus = new Map<string, string>();
