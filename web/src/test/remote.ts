/**
 * Remote-control focus helpers for tests (design.md Q141).
 *
 * jsdom lays nothing out, and spatial navigation needs boxes, so `layoutColumn` gives every
 * element the arrows consider a box of its own, one per line in document order (side by side
 * where `rowOf` says so), the way a browser would show a simple page. Then `walkRemote` presses
 * an arrow until focus stops moving and returns what it stopped on, and `remoteStops` lists every
 * stop on the page. Both name a stop by `stopLabel`.
 *
 *     layoutColumn(document.body);
 *     expect(walkRemote(screen.getByRole("button", { name: "Play" }))).toEqual(["Play", "Sort by", "row:1", "row:2"]);
 */
import { fireEvent } from "@testing-library/react";

import { REMOTE_CANDIDATES, isRemoteStop, useSpatialNavigation } from "@/lib/input-mode";

export interface Box {
  x: number;
  y: number;
  w?: number;
  h?: number;
}

/** Give an element a fixed box. */
export function place(element: Element, { x, y, w = 160, h = 40 }: Box): void {
  const rect = { x, y, left: x, top: y, right: x + w, bottom: y + h, width: w, height: h, toJSON: () => ({}) } as DOMRect;
  (element as HTMLElement).getBoundingClientRect = () => rect;
}

/** Whether a browser would draw the element at all: no hidden or `display: none` ancestor. */
function rendered(element: Element): boolean {
  for (let node: Element | null = element; node; node = node.parentElement) {
    if (node.hasAttribute("hidden") || window.getComputedStyle(node).display === "none") return false;
  }
  return true;
}

export interface LayoutOptions {
  /**
   * Elements with the same key share a line, left to right in document order. The default puts
   * a one-dimensional composite (segmented, tabs, radio group, toolbar) on one line and every
   * other element on its own.
   */
  rowOf?: (element: HTMLElement) => string | null;
  /** The left edge of the column; the shell's rail sits left of it. */
  left?: number;
}

/**
 * Lay out every element the arrows consider under `root` as a column: one line each, 60px apart,
 * in document order. Elements under a hidden ancestor get an empty box, as in a browser. Returns
 * the laid-out elements.
 */
const COMPOSITE_ROW = '[role="radiogroup"], [role="tablist"], [role="toolbar"], .fa-segmented';
const composites = new WeakMap<Element, string>();

/** The default line: a composite's controls share one. */
export function compositeRow(element: HTMLElement): string | null {
  const composite = element.closest(COMPOSITE_ROW);
  if (!composite) return null;
  if (!composites.has(composite)) composites.set(composite, `composite-${Math.random()}`);
  return composites.get(composite)!;
}

export function layoutColumn(root: ParentNode = document.body, { rowOf = compositeRow, left = 200 }: LayoutOptions = {}): HTMLElement[] {
  // The page scroller and navigation's own scrolling are no-ops here.
  for (const method of ["scrollTo", "scrollBy"] as const) {
    if (!(method in Element.prototype)) Object.defineProperty(Element.prototype, method, { configurable: true, writable: true, value: () => {} });
  }
  const elements = Array.from(root.querySelectorAll<HTMLElement>(REMOTE_CANDIDATES));
  const lines = new Map<string, number>();
  let line = 0;
  let column = 0;
  let current: string | null = null;
  for (const element of elements) {
    if (!rendered(element)) {
      place(element, { x: 0, y: 0, w: 0, h: 0 });
      continue;
    }
    const key = rowOf?.(element) ?? null;
    if (key !== null && lines.has(key)) {
      column = key === current ? column + 1 : 0;
      place(element, { x: left + column * 180, y: lines.get(key)! * 60 });
      current = key;
      continue;
    }
    if (key !== null) lines.set(key, line);
    column = 0;
    current = key;
    place(element, { x: left, y: line * 60 });
    line += 1;
  }
  return elements;
}

/** What a stop is called in assertions: `row:<id>` for a table row, else its label or text. */
export function stopLabel(element: Element): string {
  const row = element.getAttribute("data-row-id");
  if (row !== null && element.closest(".fa-dt-grid")) return `row:${row}`;
  const label = element.getAttribute("aria-label");
  if (label) return label;
  const text = element.textContent?.replace(/\s+/g, " ").trim();
  return text || element.getAttribute("data-testid") || element.tagName.toLowerCase();
}

/** Every remote stop element under `root` that has a box, in document order. */
export function remoteStopElements(root: ParentNode = document.body): HTMLElement[] {
  return Array.from(root.querySelectorAll<HTMLElement>(REMOTE_CANDIDATES)).filter((element) => {
    const rect = element.getBoundingClientRect();
    return rect.width > 0 && rect.height > 0 && isRemoteStop(element);
  });
}

/** Every remote stop under `root` that has a box, in document order, by `stopLabel`. */
export function remoteStops(root: ParentNode = document.body): string[] {
  return remoteStopElements(root).map(stopLabel);
}

/** Mount the shell's remote navigation (TV mode, Back as a no-op step) beside a component under test. */
export function RemoteNavigation({ onBack = () => true }: { onBack?: () => boolean | void }): null {
  useSpatialNavigation(true, true, onBack);
  return null;
}

/**
 * Focus `start`, then press `key` until focus stops moving (or `limit` presses), and return the
 * label of every control focus rested on, `start` first.
 */
export function walkRemote(start: HTMLElement, key: "ArrowDown" | "ArrowUp" | "ArrowLeft" | "ArrowRight" = "ArrowDown", limit = 100): string[] {
  start.focus();
  const visited = [stopLabel(start)];
  let previous: Element | null = start;
  for (let press = 0; press < limit; press += 1) {
    fireEvent.keyDown(document.activeElement ?? document.body, { key, bubbles: true });
    const now = document.activeElement;
    if (!now || now === previous || now === document.body) break;
    visited.push(stopLabel(now));
    previous = now;
  }
  return visited;
}

/** Press the remote's Back (Escape, as a keyboard and the TV app send it) on whatever has focus. */
export function pressBack(key: "Escape" | "GoBack" | "Backspace" = "Escape"): boolean {
  return !fireEvent.keyDown(document.activeElement ?? document.body, { key, bubbles: true, cancelable: true });
}
