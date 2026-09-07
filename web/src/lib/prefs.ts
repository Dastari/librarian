import { useCallback, useSyncExternalStore } from "react";

/**
 * Tiny persisted-preference store (localStorage) for UI state that should survive reloads:
 * view modes, sidebar state, volume, player quality. Cross-component and cross-tab reactive.
 */
const PREFIX = "librarian.pref.";
const listeners = new Map<string, Set<() => void>>();

function emit(key: string): void {
  for (const listener of listeners.get(key) ?? []) listener();
}

if (typeof window !== "undefined") {
  window.addEventListener("storage", (event) => {
    if (event.key?.startsWith(PREFIX)) emit(event.key.slice(PREFIX.length));
  });
}

export function readPref<T>(key: string, fallback: T): T {
  if (typeof window === "undefined") return fallback;
  const raw = window.localStorage.getItem(PREFIX + key);
  if (raw === null) return fallback;
  try {
    return JSON.parse(raw) as T;
  } catch {
    return fallback;
  }
}

export function writePref<T>(key: string, value: T): void {
  window.localStorage.setItem(PREFIX + key, JSON.stringify(value));
  emit(key);
}

export function usePref<T>(key: string, fallback: T): [T, (value: T | ((previous: T) => T)) => void] {
  const subscribe = useCallback(
    (listener: () => void) => {
      const set = listeners.get(key) ?? new Set();
      set.add(listener);
      listeners.set(key, set);
      return () => {
        set.delete(listener);
      };
    },
    [key],
  );
  const getSnapshot = useCallback(() => window.localStorage.getItem(PREFIX + key), [key]);
  const raw = useSyncExternalStore(subscribe, getSnapshot, () => null);
  let value: T = fallback;
  if (raw !== null) {
    try {
      value = JSON.parse(raw) as T;
    } catch {
      value = fallback;
    }
  }
  const setValue = useCallback(
    (next: T | ((previous: T) => T)) => {
      const previous = readPref(key, fallback);
      writePref(key, typeof next === "function" ? (next as (previous: T) => T)(previous) : next);
    },
    [key, fallback],
  );
  return [value, setValue];
}
