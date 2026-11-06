import { chapterTarget } from "./chapterNav";
import { useMemo } from "react";

import { currentAudioItem, playerStore, usePlayerState, type PlayItem } from "./store";

/** The imperative surface pages use to start playback. */
export function usePlayer() {
  const snapshot = usePlayerState();
  return useMemo(
    () => ({
      video: snapshot.video,
      audio: currentAudioItem(snapshot),
      queue: snapshot.queue,
      queueIndex: snapshot.index,
      transport: snapshot.transport,
      sheetOpen: snapshot.sheetOpen,
      /** Prepares a video item; the caller navigates to /watch/:mediaFileId. */
      playVideo: (item: PlayItem) => playerStore.setVideo(item),
      playAudio: (items: PlayItem[], index = 0) => playerStore.playQueue(items, index),
      enqueue: (items: PlayItem[]) => playerStore.enqueue(items),
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
  return direction === 1 ? index < queue.length - 1 : index > 0;
}
