/**
 * What is playing, in the browser tab's title, as Plex does: "▶ Stargate Atlantis · S05 · E03 · The
 * Seer" while it plays, "⏸ …" while paused, and the page's own title once playback stops.
 *
 * Media Session needs nothing here: the Glass video player publishes its own, and the audio dock
 * sets metadata and playback state for audio.
 */
import { useEffect, useState, useSyncExternalStore } from "react";

import { currentChapterTitle } from "./chapterNav";
import { currentAudioItem, playerStore, type PlayerState, type PlayItem } from "./store";

export const PLAYING_MARK = "▶";
export const PAUSED_MARK = "⏸";

/** "Title · Subtitle", with the subtitle's own wide gaps (`S05 · E03  The Seer`) as separators. */
function describe(item: PlayItem, detail?: string): string {
  const parts = [item.title, ...(detail ?? item.subtitle ?? "").split(/\s{2,}/)].map((part) => part.trim()).filter(Boolean);
  return parts.filter((part, index) => parts.indexOf(part) === index).join(" · ");
}

/**
 * The tab title for this moment, or null when nothing is loaded. A video wins while it plays or
 * while no audio is playing; an audiobook names its current chapter.
 */
export function nowPlayingTitle(state: PlayerState, videoPlaying: boolean): string | null {
  const audio = currentAudioItem(state);
  const audioPlaying = Boolean(audio) && state.transport.playing;
  if (state.video && (videoPlaying || !audioPlaying)) {
    return `${videoPlaying ? PLAYING_MARK : PAUSED_MARK} ${describe(state.video)}`;
  }
  if (audio) {
    const chapter = audio.chapters ? currentChapterTitle(audio, state.transport.position) : undefined;
    return `${audioPlaying ? PLAYING_MARK : PAUSED_MARK} ${describe(audio, chapter && chapter !== audio.title ? chapter : undefined)}`;
  }
  return null;
}

/** Whether any video element on the page is playing, from its media events (they don't bubble, so capture). */
function useVideoPlaying(): boolean {
  const [playing, setPlaying] = useState(false);
  useEffect(() => {
    const update = (event: Event) => {
      if (!(event.target instanceof HTMLVideoElement)) return;
      setPlaying([...document.querySelectorAll("video")].some((video) => !video.paused && !video.ended));
    };
    const events = ["play", "playing", "pause", "ended", "emptied"] as const;
    for (const name of events) document.addEventListener(name, update, true);
    return () => {
      for (const name of events) document.removeEventListener(name, update, true);
    };
  }, []);
  return playing;
}

/**
 * Mount once. The label only changes with the item, the chapter or play/pause, so position
 * ticks never touch the title. A route's own title is kept: one set while something plays
 * becomes the title to return to.
 */
export function useNowPlayingTitle(): void {
  const videoPlaying = useVideoPlaying();
  const label = useSyncExternalStore(
    playerStore.subscribe,
    () => nowPlayingTitle(playerStore.getSnapshot(), videoPlaying),
    () => null,
  );

  useEffect(() => {
    if (!label) return;
    let base = document.title;
    document.title = label;
    const element = document.querySelector("title");
    const observer = element
      ? new MutationObserver(() => {
          if (document.title === label) return;
          base = document.title;
          document.title = label;
        })
      : null;
    if (element) observer?.observe(element, { childList: true, characterData: true, subtree: true });
    return () => {
      observer?.disconnect();
      if (document.title === label) document.title = base;
    };
  }, [label]);
}

/** Renders nothing; mounts the hook where it outlives every route. */
export function NowPlayingTitle(): null {
  useNowPlayingTitle();
  return null;
}
