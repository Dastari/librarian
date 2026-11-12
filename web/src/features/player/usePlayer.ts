import { chapterTarget } from "./chapterNav";
import { useMemo } from "react";

import { canAdvance, currentAudioItem, currentQueueItem, playerStore, usePlayerState, type PlayItem } from "./store";

/** The imperative surface pages use to start playback. */
export function usePlayer() {
  const snapshot = usePlayerState();
  return useMemo(
    () => ({
      video: snapshot.video,
      audio: currentAudioItem(snapshot),
      /** The playing queue entry, audio or video. */
      current: currentQueueItem(snapshot),
      queue: snapshot.queue,
      queueIndex: snapshot.index,
      transport: snapshot.transport,
      sheetOpen: snapshot.sheetOpen,
      /** Starts a video, with what follows it in the queue; the caller navigates to /watch/:mediaFileId. */
      playVideo: (item: PlayItem, following: PlayItem[] = []) => playerStore.playQueue([item, ...following], 0),
      /** Replaces the queue with `items` and starts at `index`; audio, video or a mix. */
      play: (items: PlayItem[], index = 0) => playerStore.playQueue(items, index),
      playAudio: (items: PlayItem[], index = 0) => playerStore.playQueue(items, index),
      playNext: (items: PlayItem[]) => playerStore.playNext(items),
      enqueue: (items: PlayItem[]) => playerStore.enqueue(items),
      move: (from: number, to: number) => playerStore.move(from, to),
      clearUpcoming: () => playerStore.clearUpcoming(),
      next: () => playerStore.skip(1),
      previous: () => playerStore.skip(-1),
      jumpTo: (index: number) => playerStore.jumpTo(index),
      removeAt: (index: number) => playerStore.removeAt(index),
      stopAudio: () => playerStore.stopAudio(),
      setPlaying: (playing: boolean) => playerStore.updateTransport({ playing }),
      togglePlaying: () => playerStore.updateTransport({ playing: !snapshot.transport.playing }),
      seek: (position: number) => audioCommands.seek?.(position),
      retry: () => audioCommands.retry?.(),
      setVolume: (volume: number) => playerStore.updateTransport({ volume, muted: volume === 0 ? true : false }),
      setMuted: (muted: boolean) => playerStore.updateTransport({ muted }),
      setRate: (rate: number) => playerStore.updateTransport({ rate }),
      openSheet: () => playerStore.setSheetOpen(true),
      closeSheet: () => playerStore.setSheetOpen(false),
    }),
    [snapshot],
  );
}

/** Imperative hooks the audio engine registers so seeking works from anywhere. */
export const audioCommands: { seek?: (position: number) => void; retry?: () => void } = {};

/** Next or Previous: between chapters where the queue has them, otherwise between items. */
export function skipAudio(direction: 1 | -1) {
  const { queue, index, transport } = playerStore.getSnapshot();
  const target = chapterTarget(queue, index, transport.position, direction);
  if (!target) {
    playerStore.skip(direction);
    return;
  }
  if (target.index === index) audioCommands.seek?.(target.start);
  else playerStore.jumpToChapter(target.index, target.start);
}

export function canSkipAudio(queue: readonly PlayItem[], index: number, position: number, direction: 1 | -1) {
  if (chapterTarget(queue, index, position, direction)) return true;
  // Going forward from the last entry is allowed while something may continue it.
  return direction === 1 ? index < queue.length - 1 || canAdvance(playerStore.getSnapshot()) : index > 0;
}
