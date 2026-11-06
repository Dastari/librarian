import { useSyncExternalStore } from "react";

/**
 * Whether a page has put artwork behind the shell. The rail and top bar sit on the canvas, so
 * they need the imagery treatment only while a picture is actually under them.
 */
let mounted = 0;
const listeners = new Set<() => void>();
const notify = () => { for (const listener of listeners) listener(); };

/** Register a backdrop for as long as it shows; returns the cleanup. */
export function registerShellArt(): () => void {
  mounted += 1;
  notify();
  return () => {
    mounted -= 1;
    notify();
  };
}

const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => listeners.delete(listener);
};

export function useHasShellArt(): boolean {
  return useSyncExternalStore(subscribe, () => mounted > 0, () => false);
}

/** The element every backdrop renders into, behind the whole shell. */
export const SHELL_ART_ID = "shell-art";
