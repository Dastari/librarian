/**
 * Input mode detection and spatial navigation.
 *
 * The app renders the same components for mouse, touch and remote-control users, but the
 * shell adapts: `data-input-mode` on <html> switches CSS (see tokens.css) and the spatial
 * navigation hook lets arrow keys move focus between any `[data-focusable]` element the
 * way a TV remote expects. Remotes and gamepads arrive as keyboard events (ArrowUp, Enter,
 * Backspace/Escape for back), so no platform-specific code is needed.
 */
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

type Direction = "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight";

function isVisible(element: HTMLElement): boolean {
  const rect = element.getBoundingClientRect();
  if (rect.width === 0 && rect.height === 0) return false;
  const style = window.getComputedStyle(element);
  return style.visibility !== "hidden" && style.display !== "none";
}

/**
 * Picks the best focus candidate in a direction: the closest element whose centre lies in the
 * requested half-plane, weighting the perpendicular offset so navigation feels like a grid.
 */
function findNext(current: HTMLElement, direction: Direction): HTMLElement | null {
  const origin = current.getBoundingClientRect();
  const ox = origin.left + origin.width / 2;
  const oy = origin.top + origin.height / 2;
  let best: HTMLElement | null = null;
  let bestScore = Number.POSITIVE_INFINITY;

  const candidates = document.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR);
  for (const candidate of candidates) {
    if (candidate === current || candidate.closest("[aria-hidden='true'], [inert]") || !isVisible(candidate)) continue;
    const rect = candidate.getBoundingClientRect();
    const cx = rect.left + rect.width / 2;
    const cy = rect.top + rect.height / 2;
    const dx = cx - ox;
    const dy = cy - oy;
    let primary: number;
    let secondary: number;
    switch (direction) {
      case "ArrowLeft":
        primary = -dx;
        secondary = Math.abs(dy);
        break;
      case "ArrowRight":
        primary = dx;
        secondary = Math.abs(dy);
        break;
      case "ArrowUp":
        primary = -dy;
        secondary = Math.abs(dx);
        break;
      case "ArrowDown":
        primary = dy;
        secondary = Math.abs(dx);
        break;
    }
    if (primary <= 4) continue;
    const score = primary + secondary * 2.5;
    if (score < bestScore) {
      bestScore = score;
      best = candidate;
    }
  }
  return best;
}

/**
 * Global arrow-key focus movement. Active in every mode so keyboard users on desktop get it
 * for free; text inputs, sliders and open menus keep their native arrow behaviour.
 */
export function useSpatialNavigation(enabled = true): void {
  useEffect(() => {
    if (!enabled) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.altKey || event.ctrlKey || event.metaKey) return;
      const direction = event.key as Direction;
      if (!["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"].includes(direction)) return;
      const active = document.activeElement as HTMLElement | null;
      if (!active || active === document.body) {
        const first = document.querySelector<HTMLElement>("[data-spatial-start], main " + FOCUSABLE_SELECTOR);
        first?.focus();
        if (first) event.preventDefault();
        return;
      }
      const tag = active.tagName;
      const role = active.getAttribute("role");
      if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || active.isContentEditable) return;
      if (role === "slider" || role === "menuitem" || role === "option" || role === "tab" || role === "listbox") return;
      if (active.closest("[data-spatial-ignore]")) return;
      const next = findNext(active, direction);
      if (!next) return;
      event.preventDefault();
      next.focus({ preventScroll: false });
      next.scrollIntoView({ block: "nearest", inline: "nearest", behavior: "smooth" });
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [enabled]);
}
