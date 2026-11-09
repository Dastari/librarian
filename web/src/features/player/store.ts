/**
 * Player state lives outside React so playback survives navigation. Video plays on the
 * immersive /watch route; audio plays in the persistent dock. Both read from this store.
 *
 * The queue is the player's own ordered list, a virtual playlist. It holds audio (tracks,
 * chapters) and video (episodes, movies) in any mix, and `index` is the entry that is playing:
 * the dock plays it when it is audio, the video player when it is video, and advancing past it
 * switches between the two. Playing something fills the queue; Play next, Add to queue, moving and
 * removing then change it. When the last entry ends, playback continues with whatever the
 * continuation resolver finds (see `continuation.ts`).
 */
import { useSyncExternalStore } from "react";

import { session } from "@/lib/auth/session";

import { readQueue, writeQueue } from "./queuePersistence";

export type PlayEntityKind = "movie" | "episode" | "track" | "chapter";

export interface PlayItem {
  mediaFileId: string;
  title: string;
  subtitle?: string;
  artwork?: string;
  /** The widescreen picture a video shows before it plays. */
  backdrop?: string;
  entity: { kind: PlayEntityKind; id: string };
  /** Explicit parent identity for chapter sessions; never inferred from href. */
  audiobookId?: string;
  /** The album a track is on, so playback can carry on through the album and the artist's next. */
  albumId?: string;
  /** The show an episode is of, so playback can carry on through the show. */
  showId?: string;
  /** Names this entry of the queue, so the same media queued twice is two entries. Set by the store. */
  queueId?: string;
  /** Where "back" and the dock title should go. */
  href?: string;
  startPosition?: number;
  /**
   * Where in the file this entry ends, when it is only part of it (one chapter queued from a
   * playlist): playback moves to the next entry there instead of running on into the next chapter.
   */
  endPosition?: number;
  /** Open paused: a mini player that was paused when the page was refreshed. */
  startPaused?: boolean;
  duration?: number;
  /** Chapters that start in this file, by where in it. */
  chapters?: Array<{ start: number; title: string }>;
  /** The chapter already playing when this file begins, when a chapter spans files. */
  runningChapter?: string;
}

export const isVideoItem = (item: Pick<PlayItem, "entity">): boolean => item.entity.kind === "movie" || item.entity.kind === "episode";

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
  /** The video is in its floating window rather than on /watch. */
  videoMini: boolean;
  /** The queue: audio and video entries in play order. */
  queue: PlayItem[];
  /** The playing entry, or -1. */
  index: number;
  /** The queue's last entry has ended and the continuation resolver is looking for more. */
  continuing: boolean;
  /** The last entry (by `continuationKey`) that the resolver found nothing after. */
  exhausted: string | null;
  /** Changes for each selected queue entry, including entries sharing a file. */
  audioActivation: number;
  transport: AudioTransport;
  /** Expanded now-playing sheet on phones. */
  sheetOpen: boolean;
  /**
   * Counts audio a person started (a new queue: play an album, a track, a book, a playlist, or
   * Play next with nothing playing), not the queue moving on. On a TV each one opens now playing.
   */
  audioStarts: number;
}

const VOLUME_KEY = "librarian.player.volume";

const initialVolume = (() => {
  if (typeof window === "undefined") return 1;
  try {
    const stored = Number(window.localStorage.getItem(VOLUME_KEY));
    return Number.isFinite(stored) && stored > 0 ? Math.min(1, stored) : 1;
  } catch {
    return 1;
  }
})();

let state: PlayerState = {
  video: null,
  videoMini: false,
  queue: [],
  index: -1,
  continuing: false,
  exhausted: null,
  audioActivation: 0,
  transport: { playing: false, position: 0, duration: 0, buffered: 0, volume: initialVolume, muted: false, rate: 1, loading: false, error: null },
  sheetOpen: false,
  audioStarts: 0,
};

const listeners = new Set<() => void>();

/** Whose saved queue is kept up to date; null until the saved one has been looked at, so it is never overwritten unread. */
let persistFor: string | null = null;

/** Finds what plays after a queue's last entry; see `continuation.ts`. */
export type ContinuationResolver = (last: PlayItem) => Promise<PlayItem[]>;
let resolver: ContinuationResolver | null = null;

function set(patch: Partial<PlayerState> | ((previous: PlayerState) => Partial<PlayerState>)): void {
  const previous = state;
  const next = typeof patch === "function" ? patch(state) : patch;
  state = { ...state, ...next };
  if (persistFor && (state.queue !== previous.queue || state.index !== previous.index)) writeQueue(persistFor, state.queue, state.index);
  for (const listener of listeners) listener();
}

let entries = 0;
/** Gives each entry its own identity as it joins the queue (the same item may be queued twice). */
const tag = (items: readonly PlayItem[]): PlayItem[] => items.map((item) => ({ ...item, queueId: `q${(entries += 1)}` }));

const FRESH_TRANSPORT = { position: 0, duration: 0, buffered: 0, error: null };

/**
 * Starts the entry at `index`: audio loads in the dock, video goes to the video player. Either way
 * the other is stopped, which is what switches a mixed queue between them.
 */
function select(previous: PlayerState, queue: PlayItem[], index: number): Partial<PlayerState> {
  const item = queue[index]!;
  const video = isVideoItem(item);
  return {
    queue,
    index,
    exhausted: null,
    audioActivation: previous.audioActivation + 1,
    video: video ? item : null,
    videoMini: video ? previous.videoMini : false,
    sheetOpen: video ? false : previous.sheetOpen,
    transport: { ...previous.transport, ...FRESH_TRANSPORT, loading: !video, playing: !video },
  };
}

/** Names the end of a queue, so "nothing continues this" is only remembered until the queue changes. */
export const continuationKey = (queue: readonly PlayItem[]): string => {
  const last = queue[queue.length - 1];
  return last ? `${queue.length}:${last.entity.kind}:${last.entity.id}` : "";
};

export const playerStore = {
  subscribe(listener: () => void) {
    listeners.add(listener);
    return () => listeners.delete(listener);
  },
  getSnapshot: () => state,

  /** Hands a video to the video player without touching the queue (the mini player coming back after a refresh). */
  setVideo(item: PlayItem | null) {
    set({ video: item });
  },
  /** The video keeps playing in a floating window while the app is used around it. */
  setVideoMini(videoMini: boolean) {
    set({ videoMini });
  },

  /** Replaces the queue and starts at `index`. */
  playQueue(items: PlayItem[], index = 0) {
    if (!items.length || index < 0 || index >= items.length) {
      playerStore.stopAudio();
      return;
    }
    set((previous) => ({ ...select(previous, tag(items), index), audioStarts: previous.audioStarts + (isVideoItem(items[index]!) ? 0 : 1) }));
  },
  /** Puts `items` right after the playing entry. With nothing playing, plays them. */
  playNext(items: PlayItem[]) {
    if (!items.length) return;
    if (state.index < 0) return playerStore.playQueue(items, 0);
    const added = tag(items);
    set((previous) => ({ queue: [...previous.queue.slice(0, previous.index + 1), ...added, ...previous.queue.slice(previous.index + 1)], exhausted: null }));
  },
  /** Puts `items` at the end of the queue. With nothing playing, plays them. */
  enqueue(items: PlayItem[]) {
    if (!items.length) return;
    if (state.index < 0) return playerStore.playQueue(items, 0);
    const added = tag(items);
    set((previous) => ({ queue: [...previous.queue, ...added], exhausted: null }));
  },
  /** Moves an entry; the playing entry keeps playing wherever it ends up. */
  move(from: number, to: number) {
    if (from === to || from < 0 || to < 0 || from >= state.queue.length || to >= state.queue.length) return;
    set((previous) => {
      const queue = [...previous.queue];
      const [entry] = queue.splice(from, 1);
      queue.splice(to, 0, entry!);
      const index = previous.index === from ? to : previous.index > from && previous.index <= to ? previous.index - 1 : previous.index < from && previous.index >= to ? previous.index + 1 : previous.index;
      return { queue, index, exhausted: null };
    });
  },
  /** Drops everything after the playing entry. */
  clearUpcoming() {
    if (state.index < 0) return playerStore.stopAudio();
    if (state.index >= state.queue.length - 1) return;
    set((previous) => ({ queue: previous.queue.slice(0, previous.index + 1), exhausted: null }));
  },
  skip(direction: 1 | -1) {
    const next = state.index + direction;
    if (next < 0 || state.index < 0) return;
    if (next >= state.queue.length) {
      if (direction === 1) void playerStore.advance();
      return;
    }
    // A file reached by playing on starts at its beginning, wherever a chapter skip once left it.
    if (state.queue[next]!.chapters) set((previous) => ({ queue: previous.queue.map((item, index) => index === next ? { ...item, startPosition: 0 } : item) }));
    playerStore.jumpTo(next);
  },
  /**
   * The playing entry has ended: on to the next. At the end of the queue the continuation resolver
   * is asked what follows, which is added and played; nothing found means playback stops.
   * Resolves to whether something else started.
   */
  async advance(): Promise<boolean> {
    const at = state.index;
    if (at < 0) return false;
    if (at < state.queue.length - 1) {
      playerStore.skip(1);
      return true;
    }
    const last = state.queue[at]!;
    const key = continuationKey(state.queue);
    if (!isVideoItem(last)) set((previous) => ({ transport: { ...previous.transport, playing: false } }));
    if (!resolver || state.continuing || state.exhausted === key) return false;
    set({ continuing: true });
    let found: PlayItem[] = [];
    try {
      found = await resolver(last);
    } catch {
      found = [];
    }
    // The queue was changed while it was being looked up: leave the person's choice alone.
    if (state.queue[at] !== last || state.index !== at || state.queue.length !== at + 1) {
      set({ continuing: false });
      return false;
    }
    if (!found.length) {
      set({ continuing: false, exhausted: key });
      return false;
    }
    const added = tag(found);
    set((previous) => ({ continuing: false, ...select(previous, [...previous.queue, ...added], at + 1) }));
    return true;
  },
  /** Moves to another queue item and starts it at a chapter. */
  jumpToChapter(index: number, position: number) {
    if (index < 0 || index >= state.queue.length) return;
    set((previous) => ({ queue: previous.queue.map((item, at) => at === index ? { ...item, startPosition: position } : item) }));
    playerStore.jumpTo(index);
  },
  jumpTo(index: number) {
    if (index < 0 || index >= state.queue.length || index === state.index) return;
    set((previous) => select(previous, previous.queue, index));
  },
  removeAt(index: number) {
    if (index < 0 || index >= state.queue.length) return;
    set((previous) => {
      const queue = previous.queue.filter((_, i) => i !== index);
      if (previous.index !== index) return { queue, index: previous.index > index ? previous.index - 1 : previous.index, exhausted: null };
      const current = Math.min(previous.index, queue.length - 1);
      if (current < 0) return { queue, index: -1, exhausted: null, video: null, videoMini: false, sheetOpen: false, audioActivation: previous.audioActivation + 1, transport: { ...previous.transport, ...FRESH_TRANSPORT, loading: false, playing: false } };
      return select(previous, queue, current);
    });
  },
  stopAudio() {
    set({ queue: [], index: -1, continuing: false, exhausted: null, audioActivation: state.audioActivation + 1, sheetOpen: false, transport: { ...state.transport, playing: false, position: 0, duration: 0, buffered: 0, loading: false, error: null } });
  },
  /**
   * Brings back this person's saved queue after a reload, paused. A saved video entry has nothing to
   * show it, unless it is the mini player that came back too (`keepVideo`), so the queue resumes at
   * the next audio entry instead.
   */
  restore(userId: string, options: { keepVideo?: string | null } = {}): boolean {
    persistFor = null;
    const saved = readQueue(userId);
    persistFor = userId;
    if (!saved || state.queue.length) return false;
    let index = saved.index;
    while (index < saved.items.length && isVideoItem(saved.items[index]!) && saved.items[index]!.mediaFileId !== options.keepVideo) index += 1;
    if (index >= saved.items.length) return false;
    const kept = isVideoItem(saved.items[index]!);
    const queue = tag(saved.items);
    set((previous) => ({
      queue,
      index,
      exhausted: null,
      ...(kept ? {} : { audioActivation: previous.audioActivation + 1, transport: { ...previous.transport, ...FRESH_TRANSPORT, loading: false, playing: false } }),
    }));
    return true;
  },
  /** Stops saving the queue (logout or a change of person); `restore` starts it again. */
  forgetPerson() {
    persistFor = null;
  },
  setContinuationResolver(next: ContinuationResolver | null) {
    resolver = next;
  },
  updateTransport(patch: Partial<AudioTransport>) {
    if (patch.volume !== undefined) {
      try {
        window.localStorage.setItem(VOLUME_KEY, String(patch.volume));
      } catch {
        // A browser refusing storage only costs remembering the volume.
      }
    }
    set((previous) => ({ transport: { ...previous.transport, ...patch } }));
  },
  setSheetOpen(open: boolean) {
    set({ sheetOpen: open });
  },
};

// Playback identity is private state too. The live provider remounts on authority
// changes, while this store survives navigation and would otherwise retain it.
session.onChange(() => {
  playerStore.forgetPerson();
  playerStore.setVideo(null);
  playerStore.stopAudio();
});

export function usePlayerState(): PlayerState {
  return useSyncExternalStore(playerStore.subscribe, playerStore.getSnapshot, playerStore.getSnapshot);
}

/** The queue's playing entry, whatever it is. */
export function currentQueueItem(snapshot: PlayerState): PlayItem | null {
  return snapshot.index >= 0 ? (snapshot.queue[snapshot.index] ?? null) : null;
}

/** The playing entry when it is audio, which is what the dock shows; null while a video is the playing entry. */
export function currentAudioItem(snapshot: PlayerState): PlayItem | null {
  const item = currentQueueItem(snapshot);
  return item && !isVideoItem(item) ? item : null;
}

/** Whether going forward can find something: a next entry, or a continuation that has not come up empty. */
export function canAdvance(snapshot: PlayerState): boolean {
  if (snapshot.index < 0) return false;
  if (snapshot.index < snapshot.queue.length - 1) return true;
  const last = snapshot.queue[snapshot.index]!;
  return resolver !== null && !snapshot.continuing && last.entity.kind !== "movie" && snapshot.exhausted !== continuationKey(snapshot.queue);
}
