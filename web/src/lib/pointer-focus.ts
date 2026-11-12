/**
 * One highlight at a time. Keyboard or remote focus and mouse hover both point at "the item", so
 * whichever was used last owns it (design.md Q144):
 *
 * - `data-last-input` on <html> is `keys` after a navigation key and `pointer` after the mouse
 *   really moves, so CSS can drop the other input's highlight.
 * - In TV mode, hovering a card moves focus to it, so the hovered card is the focused one and OK
 *   opens what the mouse is on. Cards that pass under a resting cursor while the keyboard scrolls a
 *   row do not take focus: that needs a real pointer move after the last key press.
 */

export type LastInput = "keys" | "pointer";

/** Keys that move or use focus. Modifier taps and typing into fields leave the owner alone. */
const NAVIGATION_KEYS = new Set([
  "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Tab", "Enter", " ", "Escape", "Backspace",
  "Home", "End", "PageUp", "PageDown", "GoBack", "ContextMenu", "Info",
]);

/** A card the mouse can hand focus to, and the element inside it that takes focus. */
const CARD = ".poster-card, .fa-dt-card, .tv-lift";

let installed = false;

function setLastInput(value: LastInput) {
  const root = document.documentElement;
  if (root.dataset.lastInput !== value) root.dataset.lastInput = value;
}

function isTv(): boolean {
  return document.documentElement.dataset.inputMode === "tv";
}

/** The element that takes focus for a card: its own link or button, or the card itself. */
export function focusTargetOf(card: Element): HTMLElement | null {
  if (card instanceof HTMLElement && card.matches("a[href], button, [tabindex]:not([tabindex='-1'])")) return card;
  const inner = card.querySelector<HTMLElement>(".poster-navigation, a[href], button:not([tabindex='-1']), [tabindex]:not([tabindex='-1'])");
  return inner && !inner.hasAttribute("disabled") ? inner : null;
}

function onKeyDown(event: KeyboardEvent) {
  if (NAVIGATION_KEYS.has(event.key) || event.key.startsWith("Media")) setLastInput("keys");
}

function onPointerMove(event: PointerEvent) {
  if (event.pointerType !== "mouse" && event.pointerType !== "pen") return;
  if (event.movementX === 0 && event.movementY === 0) return;
  setLastInput("pointer");
  handOver(event.target);
}

function onPointerOver(event: PointerEvent) {
  if (document.documentElement.dataset.lastInput !== "pointer") return;
  handOver(event.target);
}

/** In TV mode, the card under the mouse becomes the focused card. */
function handOver(target: EventTarget | null) {
  if (!isTv() || !(target instanceof Element)) return;
  const card = target.closest(CARD);
  if (!card) return;
  const focusTarget = focusTargetOf(card);
  if (!focusTarget || focusTarget === document.activeElement || card.contains(document.activeElement)) return;
  focusTarget.focus({ preventScroll: true });
}

/** Start tracking the last input; idempotent. The module does this when it loads in a browser. */
export function installPointerFocus(): void {
  if (installed || typeof window === "undefined") return;
  installed = true;
  setLastInput("pointer");
  window.addEventListener("keydown", onKeyDown, true);
  window.addEventListener("pointermove", onPointerMove, { capture: true, passive: true });
  window.addEventListener("pointerover", onPointerOver, { capture: true, passive: true });
}
