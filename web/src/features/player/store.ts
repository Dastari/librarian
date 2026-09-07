/**
 * Player state lives outside React so playback survives navigation. Video plays on the
 * immersive /watch route; audio plays in the persistent dock. Both read from this store.
 */
import { useSyncExternalStore } from "react";

export type PlayEntityKind = "movie" | "episode" | "track" | "chapter";

export interface PlayItem {
  mediaFileId: string;
  title: string;
  subtitle?: string;
  artwork?: string;
  entity: { kind: PlayEntityKind; id: string };
  /** Where "back" and the dock title should go. */
  href?: string;
  startPosition?: number;
  duration?: number;
}

export interface AudioTransport {
  playing: boolean;
  position: number;
  duration: number;
  buffered: number;
  volume: number;
  muted: boolean;
  rate: number;
  loading: boolean;
  error: string | null;
}

export interface PlayerState {
  video: PlayItem | null;
  queue: PlayItem[];
  index: number;
  transport: AudioTransport;
  /** Expanded now-playing sheet on phones. */
  sheetOpen: boolean;
}

const VOLUME_KEY = "librarian.player.volume";

const initialVolume = (() => {
  if (typeof window === "undefined") return 1;
  const stored = Number(window.localStorage.getItem(VOLUME_KEY));
  return Number.isFinite(stored) && stored > 0 ? Math.min(1, stored) : 1;
})();

let state: PlayerState = {
  video: null,
  queue: [],
  index: -1,
  transport: { playing: false, position: 0, duration: 0, buffered: 0, volume: initialVolume, muted: false, rate: 1, loading: false, error: null },
  sheetOpen: false,
};

const listeners = new Set<() => void>();

function set(patch: Partial<PlayerState> | ((previous: PlayerState) => Partial<PlayerState>)): void {
  const next = typeof patch === "function" ? patch(state) : patch;
  state = { ...state, ...next };
  for (const listener of listeners) listener();
}

export const playerStore = {
  subscribe(listener: () => void) {
    listeners.add(listener);
    return () => listeners.delete(listener);
  },
  getSnapshot: () => state,

  setVideo(item: PlayItem | null) {
    set({ video: item });
  },

  /** Replaces the audio queue and starts at `index`. */
  playQueue(items: PlayItem[], index = 0) {
    set({ queue: items, index, transport: { ...state.transport, position: 0, duration: 0, buffered: 0, loading: true, error: null, playing: true } });
  },
  enqueue(items: PlayItem[]) {
    set((previous) => ({ queue: [...previous.queue, ...items], index: previous.index < 0 ? 0 : previous.index }));
  },
  skip(direction: 1 | -1) {
    const next = state.index + direction;
    if (next < 0 || next >= state.queue.length) return;
    set({ index: next, transport: { ...state.transport, position: 0, duration: 0, buffered: 0, loading: true, playing: true } });
  },
  jumpTo(index: number) {
    if (index < 0 || index >= state.queue.length) return;
    set({ index, transport: { ...state.transport, position: 0, loading: true, playing: true } });
  },
  removeAt(index: number) {
    set((previous) => {
      const queue = previous.queue.filter((_, i) => i !== index);
      const current = previous.index > index ? previous.index - 1 : Math.min(previous.index, queue.length - 1);
      return { queue, index: current };
    });
  },
  stopAudio() {
    set({ queue: [], index: -1, sheetOpen: false, transport: { ...state.transport, playing: false, position: 0, duration: 0, buffered: 0, loading: false, error: null } });
  },
  updateTransport(patch: Partial<AudioTransport>) {
    if (patch.volume !== undefined) window.localStorage.setItem(VOLUME_KEY, String(patch.volume));
    set((previous) => ({ transport: { ...previous.transport, ...patch } }));
  },
  setSheetOpen(open: boolean) {
    set({ sheetOpen: open });
  },
};

export function usePlayerState(): PlayerState {
  return useSyncExternalStore(playerStore.subscribe, playerStore.getSnapshot, playerStore.getSnapshot);
}

export function currentAudioItem(snapshot: PlayerState): PlayItem | null {
  return snapshot.index >= 0 ? (snapshot.queue[snapshot.index] ?? null) : null;
}
