import { useEffect } from "react";

import { currentAudioItem, playerStore } from "./store";
import { audioCommands, skipAudio } from "./usePlayer";

/**
 * The one place media keys are handled (design.md Q125). The standard `KeyboardEvent.key` values
 * arrive from keyboards, TV remotes and the Android TV shell; they work wherever focus is, without
 * reaching the player first. They go to the video player while it holds the screen (or is the only
 * thing playing), otherwise to the audio player.
 */
export const MEDIA_KEYS = ["MediaPlayPause", "MediaPlay", "MediaPause", "MediaStop", "MediaTrackNext", "MediaTrackPrevious", "MediaFastForward", "MediaRewind"] as const;
export type MediaKey = (typeof MEDIA_KEYS)[number];

/** What a player offers the media keys. Registered while it is mounted. */
export interface PlayerKeyTarget {
  toggle(): void;
  play(): void;
  pause(): void;
  stop(): void;
  next?(): void;
  previous?(): void;
  /** One scrub press towards the end (1) or start (-1); `repeat` for a held key. */
  scrub(direction: 1 | -1, repeat: boolean): void;
}

export const videoKeyTarget: { current: PlayerKeyTarget | null } = { current: null };
/** The audio seek bar's scrub, when one is on screen; otherwise the audio keys seek at once. */
export const audioScrub: { current: ((direction: 1 | -1, repeat: boolean) => void) | null } = { current: null };

const AUDIO_STEP = 10;

const audioKeyTarget: PlayerKeyTarget = {
  toggle: () => playerStore.updateTransport({ playing: !playerStore.getSnapshot().transport.playing }),
  play: () => playerStore.updateTransport({ playing: true }),
  pause: () => playerStore.updateTransport({ playing: false }),
  stop: () => playerStore.stopAudio(),
  next: () => skipAudio(1),
  previous: () => skipAudio(-1),
  scrub: (direction, repeat) => {
    if (audioScrub.current) audioScrub.current(direction, repeat);
    else audioCommands.seek?.(Math.max(0, playerStore.getSnapshot().transport.position + direction * AUDIO_STEP));
  },
};

/** Which player a media key goes to now, if any. */
export function mediaKeyTarget(): PlayerKeyTarget | null {
  const state = playerStore.getSnapshot();
  if (videoKeyTarget.current && state.video) return videoKeyTarget.current;
  return currentAudioItem(state) ? audioKeyTarget : null;
}

/** Act on a media key. Returns whether it was one this app handles. */
export function handleMediaKey(key: string, repeat = false): boolean {
  if (!(MEDIA_KEYS as readonly string[]).includes(key)) return false;
  const target = mediaKeyTarget();
  if (!target) return false;
  switch (key as MediaKey) {
    case "MediaPlayPause": target.toggle(); break;
    case "MediaPlay": target.play(); break;
    case "MediaPause": target.pause(); break;
    case "MediaStop": target.stop(); break;
    case "MediaTrackNext": target.next?.(); break;
    case "MediaTrackPrevious": target.previous?.(); break;
    case "MediaFastForward": target.scrub(1, repeat); break;
    case "MediaRewind": target.scrub(-1, repeat); break;
  }
  return true;
}

/** Listens for media keys on the whole window, before any control sees them. Mounted once. */
export function useGlobalMediaKeys(): void {
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.altKey || event.ctrlKey || event.metaKey) return;
      if (handleMediaKey(event.key, event.repeat)) {
        event.preventDefault();
        event.stopPropagation();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, []);
}
