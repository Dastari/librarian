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
 * - What is a stop, the Back order and focus restoration are design.md Q141–Q143 and the
 *   style guide's "Remote focus" section: `isRemoteStop` is the one definition of a stop.
 */
import { prefersReducedMotion } from "glass-ux/appearance";
import { DataTableRemoteContext } from "glass-ux/data-table";
import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";

import { backHandled, isEditing, isRemoteBackKey } from "@/lib/remote-back";
import { isTvApp } from "@/lib/tv-app";

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
  if (isTvApp(ua)) return "tv";
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
  // TV comes from a TV's browser or a person's choice, never from arrow keys: a desktop keyboard
  // user browsing rows with arrows would otherwise be switched to the TV scale mid-page.
  const [detected] = useState<InputMode>(detectInitialMode);
  const [forced, setForcedState] = useState<InputMode | null>(readForced);

  // The Android TV app is always driven by a remote, whatever this browser stored before.
  const mode = isTvApp() ? "tv" : (forced ?? detected);

  useEffect(() => {
    document.documentElement.dataset.inputMode = mode;
  }, [mode]);

  const setForced = useCallback((next: InputMode | null) => {
    if (next) window.localStorage.setItem(STORAGE_KEY, next);
    else window.localStorage.removeItem(STORAGE_KEY);
    setForcedState(next);
  }, []);

  const value = useMemo(() => ({ mode, forced, setForced }), [mode, forced, setForced]);
  // A remote never stops on column headers, so tables put their sort menu in the toolbar (Q141).
  return (
    <InputModeContext.Provider value={value}>
      <DataTableRemoteContext.Provider value={mode === "tv"}>{children}</DataTableRemoteContext.Provider>
    </InputModeContext.Provider>
  );
}

export function useInputMode(): InputModeContextValue {
  const context = useContext(InputModeContext);
  if (!context) throw new Error("useInputMode must be used within InputModeProvider");
  return context;
}

/** Whether the app is in TV mode; false outside the provider (components used on their own). */
export function useIsTvMode(): boolean {
  return useContext(InputModeContext)?.mode === "tv";
}

const FOCUSABLE_SELECTOR =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"]), [data-focusable]';

/** The focusable selector scoped to the routed content (each list entry needs the prefix). */
const IN_MAIN = FOCUSABLE_SELECTOR.split(",").map((selector) => `main ${selector.trim()}`).join(", ");

/** A Glass data table's grid, and the lines in it (rows, cards, detail panels) the table moves between. */
const TABLE_GRID = ".fa-dt-grid";
const TABLE_LINE = "[data-item][data-index]";
/** Every element the arrows consider: the focusable ones, plus a table's lines (only its active one is tabbable). */
const CANDIDATES = `${FOCUSABLE_SELECTOR}, ${TABLE_GRID} ${TABLE_LINE}`;
/**
 * Never stops, whatever they contain: a table's header (sort buttons, column menus, resize
 * handles, select-all; sorting is in the toolbar's sort menu on a TV) and resize separators.
 */
const NEVER_STOPS = '.fa-dt-head, [role="columnheader"], [role="separator"], [role="tooltip"]';

const isTvDocument = () => typeof document !== "undefined" && document.documentElement.dataset.inputMode === "tv";

/**
 * Whether an element is a remote stop, apart from where it is on screen (design.md Q141). Not a
 * stop: anything hidden, inert or `aria-hidden`; disabled controls; pointer-only shortcuts
 * (`data-nav-skip`, and `data-tv-skip` on a TV); a data table's header; anything inside a table
 * line except the line itself (a row or card is one stop; its checkbox and ⋯ are the pointer's,
 * its actions are in its menu), though a detail panel's own controls are; and a region that only
 * contains controls (focus lands on the controls).
 */
export function isRemoteStop(element: HTMLElement, tv = isTvDocument()): boolean {
  if (element.closest("[aria-hidden='true'], [inert], [hidden], [data-nav-skip]")) return false;
  if (tv && element.closest("[data-tv-skip]")) return false;
  if (element.matches(":disabled") || element.getAttribute("aria-disabled") === "true") return false;
  if (element.closest(NEVER_STOPS)) return false;
  const line = element.matches(TABLE_LINE) && Boolean(element.closest(TABLE_GRID));
  if (!line && element.closest(TABLE_GRID) && !element.closest('[data-item="detail"]')) return false;
  if (!line && !element.matches("a[href], button, input, select, textarea") && element.querySelector(FOCUSABLE_SELECTOR)) return false;
  return true;
}

/** The first stop matching a selector, in document order (no geometry: a page may not be laid out yet). */
function firstStop(root: ParentNode, selector: string): HTMLElement | null {
  for (const element of root.querySelectorAll<HTMLElement>(selector)) if (isRemoteStop(element)) return element;
  return null;
}

type Direction = "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight";

const OPEN_OVERLAY = '[role="dialog"], [role="alertdialog"], [role="menu"], [role="listbox"]';
/**
 * Arrows stay inside a modal dialog. A non-modal one (the now playing drawer above the dock)
 * does not hold them, so the dock's controls under it stay reachable.
 */
const ARROW_SCOPE = '[role="dialog"]:not([aria-modal="false"]), [role="alertdialog"]';

function isVisible(element: HTMLElement): boolean {
  const rect = element.getBoundingClientRect();
  // A collapsed panel keeps its width but has no height: its controls are not stops either.
  if (rect.width === 0 || rect.height === 0) return false;
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

  for (const candidate of scope.querySelectorAll<HTMLElement>(CANDIDATES)) {
    // Pointer-only controls (a row's scroll arrows, a tile's play button) are never arrow-key
    // stops: the arrows move to the next card, and the row scrolls to show it.
    if (candidate === current || !isRemoteStop(candidate, tv) || !isVisible(candidate)) continue;
    const candidateZone = candidate.closest("[data-nav-zone]");
    // Side zones (the rail, the letter rail) are entered and left sideways only.
    if (vertical && candidateZone !== zone && (isSide(candidateZone) || isSide(zone))) continue;
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
  const scroller = element.closest<HTMLElement>("[data-page-scroller]");
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
    const rect = element.getBoundingClientRect();
    if (rect.top >= view.top && rect.bottom <= view.bottom) return;
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
  // A seek bar takes Left/Right for scrubbing; Up and Down leave it (design.md Q125).
  if (role === "slider" && active.hasAttribute("data-owns-horizontal")) return direction === "ArrowLeft" || direction === "ArrowRight";
  if (role === "slider" || role === "menuitem" || role === "option" || role === "listbox" || role === "combobox" || role === "treeitem" || role === "gridcell") return true;
  // A data table moves between its lines with Up/Down (and across a grid of cards, where it takes
  // Left/Right itself); sideways keys it doesn't use leave it. Its edges are `leavesTable`'s.
  // Controls inside a line or a detail panel don't belong to its keys: the arrows are ours there.
  if (active.closest(TABLE_GRID)) return active.matches(TABLE_LINE) && (direction === "ArrowUp" || direction === "ArrowDown");
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

/**
 * A tag field is a combobox for its suggestions, but with its list closed it is one line of chips:
 * on a TV Up/Down leave it instead of owning the remote.
 */
function leavesClosedTagField(active: HTMLElement, direction: string): boolean {
  return (direction === "ArrowUp" || direction === "ArrowDown") && active instanceof HTMLInputElement && active.getAttribute("aria-expanded") !== "true" && Boolean(active.closest(".fa-tag-input"));
}

/** Glass NumberField's input (Base UI steps its value with Up/Down and stops the event). */
const NUMBER_INPUT = '.fa-number-input, [aria-roledescription="Number field"]';
const ADJUSTING = "data-tv-adjusting";

function isNumberInput(active: HTMLElement | null): active is HTMLInputElement {
  return active instanceof HTMLInputElement && active.matches(NUMBER_INPUT);
}

/**
 * On a TV a number field does not own Up/Down, or a remote walking down a form would change
 * values instead of moving to the next row (its −/+ steppers are pointer-only). OK (Enter)
 * puts the focused field into an explicit adjusting state, marked `data-tv-adjusting`, where
 * Up/Down step the value as usual; OK or Back, or leaving the field, ends it. Returns true
 * when the key was taken.
 */
function handleTvNumberKey(event: KeyboardEvent, active: HTMLInputElement): boolean {
  const adjusting = active.hasAttribute(ADJUSTING);
  const take = () => {
    event.preventDefault();
    event.stopPropagation();
  };
  if (event.key === "Enter") {
    take();
    if (adjusting) active.removeAttribute(ADJUSTING);
    else active.setAttribute(ADJUSTING, "");
    return true;
  }
  if (adjusting && (event.key === "Escape" || isRemoteBackKey(event))) {
    take();
    active.removeAttribute(ADJUSTING);
    return true;
  }
  if (adjusting || (event.key !== "ArrowUp" && event.key !== "ArrowDown")) return false;
  take();
  const scope = active.closest<HTMLElement>('[role="dialog"], [role="alertdialog"]') ?? document;
  const next = findNext(active, event.key, scope);
  if (next) {
    next.focus({ preventScroll: true });
    revealFocused(next);
  }
  return true;
}

/** The topmost open overlay: portals append to the body, so the last one in document order. */
function topOverlay(): HTMLElement | null {
  const open = document.querySelectorAll<HTMLElement>(OPEN_OVERLAY);
  return open.item(open.length - 1) ?? null;
}

const isArrow = (key: string): key is Direction => key === "ArrowUp" || key === "ArrowDown" || key === "ArrowLeft" || key === "ArrowRight";

/**
 * Whether an arrow leaves a data table from this line: Up from its first row (or the first row
 * of cards), Down from its last once everything is loaded, and Left/Right from the end of a row
 * of cards. The table itself would stay put there (it clamps), so the remote would be stuck.
 */
function leavesTable(line: HTMLElement, direction: Direction): boolean {
  const grid = line.closest<HTMLElement>(TABLE_GRID);
  if (!grid) return false;
  const rect = line.getBoundingClientRect();
  const index = Number(line.dataset.index);
  const others = Array.from(grid.querySelectorAll<HTMLElement>(TABLE_LINE)).filter((other) => other !== line);
  const sameRow = (other: DOMRect) => other.top < rect.bottom - 4 && other.bottom > rect.top + 4;
  if (direction === "ArrowUp") return index === 0 || others.some((other) => other.dataset.index === "0" && sameRow(other.getBoundingClientRect()));
  if (direction === "ArrowDown") {
    if (grid.getAttribute("aria-busy") === "true") return false;
    const later = others.filter((other) => Number(other.dataset.index) > index);
    if (later.some((other) => other.getBoundingClientRect().top >= rect.bottom - 4)) return false;
    // `aria-rowcount` is the header plus every row, or -1 while the total is unknown.
    const count = Number(grid.getAttribute("aria-rowcount") ?? -1);
    const loaded = Math.max(index, ...later.map((other) => Number(other.dataset.index)));
    return count <= 0 || loaded + 2 >= count;
  }
  // Sideways only matters across a grid of cards; a table row leaves sideways by the bubbling path.
  if (!line.classList.contains("fa-dt-card")) return false;
  return !others.some((other) => {
    const box = other.getBoundingClientRect();
    return sameRow(box) && (direction === "ArrowLeft" ? box.right <= rect.left + 4 : box.left >= rect.right - 4);
  });
}

/** A search field: Back clears its text before it goes back (design.md Q142). */
const SEARCH_FIELD = 'input[type="search"], input[role="searchbox"], input[enterkeyhint="search"], [role="search"] input, input[data-back-clears]';

/** Empty a field the way typing would, so a controlled React input hears it. */
function clearField(input: HTMLInputElement) {
  const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set;
  setter?.call(input, "");
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

/** The last control focused outside any overlay: where focus returns when an overlay leaves it nowhere. */
let lastOutside: HTMLElement | null = null;
/** Set while Back is re-sent to an overlay as Escape, so that Escape is not taken as a second Back. */
let resending = false;

/** A visible, connected stop to put focus back on. */
function usable(element: HTMLElement | null): element is HTMLElement {
  return Boolean(element?.isConnected) && isVisible(element!) && !element!.matches(":disabled");
}

/**
 * Put focus somewhere sensible when it has fallen to the page body (an overlay closed and its
 * opener is gone, or the opener was a menu item): the control that opened it, the content control
 * last used, the page's start, then its first stop (design.md Q143).
 */
export function restoreFocus(): HTMLElement | null {
  const main = document.getElementById("main") ?? document.querySelector("main");
  const opener = lastOutside && !lastOutside.matches(TABLE_LINE) ? lastOutside.closest<HTMLElement>(`${TABLE_GRID} ${TABLE_LINE}`) ?? lastOutside : lastOutside;
  const start = main?.querySelector<HTMLElement>("[data-tv-autofocus]") ?? null;
  const target = [opener, contentMemory, start].find(usable) ?? (main ? firstStop(main, "[data-spatial-start], " + CANDIDATES) : null);
  if (!target) return null;
  target.focus({ preventScroll: true });
  revealFocused(target);
  return target;
}

function overlayOpen(): boolean {
  return Boolean(document.querySelector(OPEN_OVERLAY));
}

/**
 * Global arrow-key focus movement. Active in every mode so keyboard users on desktop get it
 * for free; text inputs, sliders and open menus keep their native arrow behaviour.
 *
 * Back (design.md Q142), when nothing nearer took it (`backHandled`; a closing tooltip doesn't
 * count): close the topmost overlay, then leave an edit state (a search field's text), then
 * `onBack`, one navigation step. `onBack` returns false when there is nowhere to go (Home), and
 * the key is left unhandled so the TV app can exit. In TV mode Escape and Backspace (outside a
 * text field) are Back too; the remote keys always are.
 */
export function useSpatialNavigation(enabled = true, tv = false, onBack: () => boolean | void = () => window.history.back()): void {
  useEffect(() => {
    if (!enabled) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.altKey || event.ctrlKey || event.metaKey) return;
      const active = document.activeElement as HTMLElement | null;
      const editing = isEditing(active);
      const back = isRemoteBackKey(event) || (tv && (event.key === "Escape" || (event.key === "Backspace" && !editing)));
      if (back) {
        if (resending || backHandled(event)) return;
        // 1. The topmost overlay. Dialogs and menus close on Escape themselves; another Back key,
        // or focus outside the overlay (the now playing drawer, with focus on the page), is aimed
        // at it as Escape.
        const overlay = topOverlay();
        if (overlay) {
          const inside = Boolean(active && overlay.contains(active));
          if (event.key !== "Escape" || !inside) {
            resending = true;
            try {
              (inside ? active! : overlay).dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true, composed: true }));
            } finally {
              resending = false;
            }
          }
          event.preventDefault();
          return;
        }
        // 2. An edit state: a search field's text goes before the page does.
        if (active instanceof HTMLInputElement && active.matches(SEARCH_FIELD) && active.value !== "") {
          event.preventDefault();
          clearField(active);
          return;
        }
        // 3. One navigation step, unless there is none (Home).
        if (onBack() === false) return;
        event.preventDefault();
        return;
      }
      if (event.defaultPrevented) return;
      const direction = event.key as Direction;
      if (!isArrow(direction)) return;
      if (!active || active === document.body) {
        const first = document.querySelector<HTMLElement>("main [data-tv-autofocus]") ?? firstStop(document, `main [data-spatial-start], ${IN_MAIN}`);
        first?.focus();
        if (first) event.preventDefault();
        return;
      }
      if ((editing && !(tv && leavesField(active, direction))) || compositeOwns(active, direction)) return;
      if (active.closest("[data-spatial-ignore]")) return;
      const scope = active.closest<HTMLElement>(ARROW_SCOPE) ?? document;
      const next = findNext(active, direction, scope);
      if (!next) return;
      event.preventDefault();
      next.focus({ preventScroll: true });
      revealFocused(next);
    };
    /** Move on in a direction before a composite (Base UI) turns the key into its own. */
    const moveFirst = (event: KeyboardEvent, active: HTMLElement): boolean => {
      const scope = active.closest<HTMLElement>(ARROW_SCOPE) ?? document;
      const next = findNext(active, event.key as Direction, scope);
      if (!next) return false;
      event.preventDefault();
      event.stopPropagation();
      next.focus({ preventScroll: true });
      revealFocused(next);
      return true;
    };
    // Base UI modal scopes stop bubbling key events. Handle ordinary modal
    // controls before that boundary; composites still take the exemptions above.
    // On a TV a ⋯ menu button opens with OK; Up/Down must move past it, so take those
    // arrows before Base UI's trigger turns them into "open the menu".
    const onModalKeyDown = (event: KeyboardEvent) => {
      const active = document.activeElement as HTMLElement | null;
      const plain = !event.defaultPrevented && !event.altKey && !event.ctrlKey && !event.metaKey;
      if (tv && isNumberInput(active) && plain && !event.shiftKey && handleTvNumberKey(event, active)) return;
      // Base UI's combobox takes Up/Down to open its list before a bubbling handler could run.
      if (tv && active && plain && leavesClosedTagField(active, event.key)) {
        moveFirst(event, active);
        return;
      }
      if (tv && active?.matches('[aria-haspopup="menu"]:not([role="menuitem"])') && (event.key === "ArrowUp" || event.key === "ArrowDown")) {
        moveFirst(event, active);
        return;
      }
      // A data table clamps at its edges; the remote leaves it there instead of being stuck.
      if (active && plain && !event.shiftKey && isArrow(event.key) && active.matches(`${TABLE_GRID} ${TABLE_LINE}`) && leavesTable(active, event.key)) {
        moveFirst(event, active);
        return;
      }
      if (active?.closest(ARROW_SCOPE)) onKeyDown(event);
    };
    const onFocusIn = (event: FocusEvent) => {
      rememberFocus(event);
      const target = event.target;
      if (!(target instanceof HTMLElement)) return;
      if (!target.closest(OPEN_OVERLAY)) lastOutside = target;
      // On a TV a row's own controls (its ⋯, its checkbox) are the pointer's: focus handed back to
      // one (a menu closing) goes to its row, the stop.
      if (tv && target.closest(TABLE_GRID) && !isRemoteStop(target, true)) {
        const line = target.closest<HTMLElement>(TABLE_LINE);
        if (line) requestAnimationFrame(() => {
          if (document.activeElement === target && !overlayOpen()) line.focus({ preventScroll: true });
        });
      }
    };
    // Focus is never left on the page body on a TV (Q143): when it falls there, an overlay
    // closed and nothing took it back, so it returns to the opener.
    let fallTimer: ReturnType<typeof setTimeout> | undefined;
    const onFocusOut = (event: FocusEvent) => {
      if (event.target instanceof HTMLElement) event.target.removeAttribute(ADJUSTING);
      if (!tv || event.relatedTarget) return;
      clearTimeout(fallTimer);
      fallTimer = setTimeout(() => {
        const now = document.activeElement;
        if (now && now !== document.body && now !== document.documentElement) return;
        if (overlayOpen()) return;
        restoreFocus();
      }, 120);
    };
    window.addEventListener("keydown", onModalKeyDown, true);
    window.addEventListener("keydown", onKeyDown);
    document.addEventListener("focusin", onFocusIn);
    document.addEventListener("focusout", onFocusOut);
    return () => {
      clearTimeout(fallTimer);
      document.removeEventListener("focusout", onFocusOut);
      window.removeEventListener("keydown", onModalKeyDown, true);
      window.removeEventListener("keydown", onKeyDown);
      document.removeEventListener("focusin", onFocusIn);
    };
  }, [enabled, tv, onBack]);
}

/**
 * Identify a focused element well enough to find it again after the page re-renders or is
 * opened again: its link, table row, label, id or, for a button, its text.
 */
function focusKey(element: Element): string | null {
  const link = element.closest("a[href]");
  if (link) return `a[href="${CSS.escape(link.getAttribute("href") ?? "")}"]`;
  const row = element.closest("[data-row-id]");
  if (row) return `[data-row-id="${CSS.escape(row.getAttribute("data-row-id") ?? "")}"]`;
  const label = element.getAttribute("aria-label");
  if (label) return `[aria-label="${CSS.escape(label)}"]`;
  if (element.id) return `#${CSS.escape(element.id)}`;
  const text = element.matches("button") ? element.textContent?.trim() : "";
  return text ? `button${TEXT_MARK}${text}` : null;
}

const TEXT_MARK = "\u0000";

function findByKey(root: HTMLElement, key: string): HTMLElement | null {
  const mark = key.indexOf(TEXT_MARK);
  if (mark < 0) return root.querySelector<HTMLElement>(key);
  const text = key.slice(mark + 1);
  return Array.from(root.querySelectorAll<HTMLElement>(key.slice(0, mark))).find((element) => element.textContent?.trim() === text) ?? null;
}

/**
 * TV route focus. When a page opens, focus moves into it (its `[data-tv-autofocus]` control,
 * otherwise its first stop) so the remote never starts from nowhere. Returning to a page (Back,
 * or closing the player) restores the control that was used last on it, the card that was opened
 * from it, when it is still there (design.md Q143).
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
      const found = remembered ? findByKey(main, remembered) : null;
      const restore = found && isRemoteStop(found) ? found : found?.closest<HTMLElement>(`${TABLE_GRID} ${TABLE_LINE}`) ?? null;
      const elapsed = performance.now() - started;
      // Prefer the page's own start (Play), waiting for it while its data loads; otherwise
      // any control. A page that is still a skeleton keeps waiting a little longer. A remembered
      // card that loads with the page is worth the same wait.
      const start = main.querySelector<HTMLElement>("[data-tv-autofocus]");
      const waitForRestore = Boolean(remembered) && !restore && elapsed < 1500;
      const target = restore
        ?? (waitForRestore ? null : (start && start.getAttribute("aria-disabled") !== "true" ? start : null)
          ?? (elapsed > (start ? 3000 : 300) ? firstStop(main, "[data-spatial-start], " + CANDIDATES) : null));
      if (target) {
        // A restored control keeps the scroll position the router brought back.
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
