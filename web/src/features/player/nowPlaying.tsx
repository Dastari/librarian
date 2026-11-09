import { useNavigate, useRouterState } from "@tanstack/react-router";
import { Rail } from "glass-ux/shell";
import { useEffect } from "react";

import { Artwork } from "@/components/ui";
import { useIsTvMode } from "@/lib/input-mode";

import { currentQueueItem, playerStore, usePlayerState } from "./store";

/**
 * Reaching what is playing from anywhere with a remote (design.md Q125): a "Now playing" item at
 * the top of the TV rail, and a shortcut (the remote's Info key, or N on a keyboard). Both open
 * the video player for a video and the now playing drawer for audio, and put focus there; leaving
 * puts focus back where it was.
 */
export const NOW_PLAYING_KEYS = new Set(["Info", "n", "N"]);

/** Set before opening the drawer to have it focus Play/Pause instead of its close button. */
export const drawerFocus: { current: "transport" | null } = { current: null };

/** Where focus was before now playing took it, so Up or Back can return there. */
export const nowPlayingReturn: { current: HTMLElement | null } = { current: null };

const WATCH_PATH = /^\/watch\//;

/** Focus the dock's first transport control (Play/Pause). */
export function focusDock(): boolean {
  const start = document.querySelector<HTMLElement>("[data-dock-start]");
  if (!start) return false;
  start.focus({ preventScroll: true });
  return true;
}

export function openNowPlaying(navigate: ReturnType<typeof useNavigate>, pathname: string): boolean {
  const state = playerStore.getSnapshot();
  const opener = document.activeElement instanceof HTMLElement && document.activeElement !== document.body ? document.activeElement : null;
  if (state.video) {
    if (WATCH_PATH.test(pathname) && !state.videoMini) {
      document.querySelector<HTMLElement>(".video-host-full video")?.focus({ preventScroll: true });
    } else {
      void navigate({ to: "/watch/$mediaFileId", params: { mediaFileId: state.video.mediaFileId } });
    }
    return true;
  }
  if (!currentQueueItem(state)) return false;
  nowPlayingReturn.current = opener && !opener.closest("[data-dock]") ? opener : nowPlayingReturn.current;
  drawerFocus.current = "transport";
  playerStore.setSheetOpen(true);
  // A drawer that opens focuses its own close button (a phone's sheet likewise); one already open
  // leaves focus where it was, so take it to the dock's controls.
  requestAnimationFrame(() => {
    if (!document.activeElement?.closest('[role="dialog"], [data-dock]')) focusDock();
  });
  return true;
}

function typing(target: EventTarget | null): boolean {
  return target instanceof HTMLElement && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.tagName === "SELECT" || target.isContentEditable);
}

/** The Info key (or N) opens now playing from anywhere. Mounted once, in the shell. */
export function useNowPlayingShortcut(): void {
  const navigate = useNavigate();
  const pathname = useRouterState({ select: (router) => router.location.pathname });
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.altKey || event.ctrlKey || event.metaKey || !NOW_PLAYING_KEYS.has(event.key) || typing(event.target)) return;
      if (event.target instanceof Element && event.target.closest('[role="menu"], [role="listbox"]')) return;
      if (openNowPlaying(navigate, pathname)) event.preventDefault();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [navigate, pathname]);
}

/** The rail's "Now playing" entry: on a TV only, while anything is playing or paused. */
export function NowPlayingRailItem() {
  const tv = useIsTvMode();
  const state = usePlayerState();
  const item = state.video ?? currentQueueItem(state);
  if (!tv || !item) return null;
  return <NowPlayingRailEntry title={item.title} artwork={item.artwork ?? item.backdrop} playing={state.video ? true : state.transport.playing} open={state.sheetOpen} />;
}

function NowPlayingRailEntry({ title, artwork, playing, open }: { title: string; artwork?: string; playing: boolean; open: boolean }) {
  const navigate = useNavigate();
  const pathname = useRouterState({ select: (router) => router.location.pathname });
  return (
    <Rail.Item
      className="nav-focus"
      label={`Now playing: ${title}`}
      icon={<Artwork src={artwork} alt="" aspect="square" className={playing ? "rounded-sm" : "rounded-sm opacity-70"} />}
      active={WATCH_PATH.test(pathname) || open}
      onClick={() => openNowPlaying(navigate, pathname)}
      data-focusable
      data-now-playing-rail
    />
  );
}
