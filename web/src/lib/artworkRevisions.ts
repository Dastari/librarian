import { useSyncExternalStore } from "react";

/**
 * Pictures changed on this device, by entity, with when they changed. `artworkUrl` adds the
 * revision to their address so the browser loads the new picture instead of its cached copy.
 * Other devices see the change when their copy is next checked (the server asks for that every
 * few minutes). Kept in this browser for a week, the longest the server lets a copy be reused.
 */
const STORAGE_KEY = "librarian.artwork-revisions.v1";
const KEEP_MS = 7 * 24 * 60 * 60 * 1000;

type Revisions = Record<string, number>;

const listeners = new Set<() => void>();
let revisions: Revisions = load();
let version = 0;

function load(): Revisions {
  try {
    const raw = globalThis.localStorage?.getItem(STORAGE_KEY);
    const parsed = raw ? (JSON.parse(raw) as Revisions) : {};
    const now = Date.now();
    return Object.fromEntries(Object.entries(parsed).filter(([, at]) => typeof at === "number" && now - at < KEEP_MS));
  } catch {
    return {};
  }
}

function save() {
  try {
    globalThis.localStorage?.setItem(STORAGE_KEY, JSON.stringify(revisions));
  } catch {
    // Private windows and full storage: the revision still holds for this page's life.
  }
}

const key = (entity: string, id: string) => `${entity}/${id}`;

/** The revision of an entity's pictures, when one changed here. */
export function artworkRevision(entity: string, id: string): string | undefined {
  const at = revisions[key(entity, id)];
  return at ? at.toString(36) : undefined;
}

/** Note that an entity's pictures changed, so every address made from now on loads them again. */
export function bumpArtwork(entity: string, id: string) {
  revisions = { ...revisions, [key(entity, id)]: Date.now() };
  version += 1;
  save();
  for (const listener of listeners) listener();
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** Re-render when any picture changes here (for a picture no query data follows, like a logo). */
export function useArtworkRevisions(): number {
  return useSyncExternalStore(subscribe, () => version, () => version);
}
