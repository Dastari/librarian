import { useSyncExternalStore } from "react";

/** Which file's dialog is open. One dialog serves every row, chip and page that names a file. */
let current: string | null = null;
const listeners = new Set<() => void>();
const set = (id: string | null) => {
  current = id;
  for (const listener of listeners) listener();
};

export const openFileDialog = (id: string) => set(id);
export const closeFileDialog = () => set(null);

const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => listeners.delete(listener);
};

export function useOpenFileId(): string | null {
  return useSyncExternalStore(subscribe, () => current, () => null);
}
