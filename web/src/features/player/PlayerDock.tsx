import { useNavigate } from "@tanstack/react-router";
import { Button } from "glass-ux/button";
import { ListRow } from "glass-ux/list-row";
import { Notice } from "glass-ux/notice";
import { IconChevronUp, IconList, IconClose } from "glass-ux/icons";
import { useEffect, useRef, useState, type FocusEvent as ReactFocusEvent, type KeyboardEvent as ReactKeyboardEvent } from "react";

import { Artwork } from "@/components/ui";
import { useIsDesktop } from "@/hooks/useMediaQuery";
import { useIsTvMode } from "@/lib/input-mode";
import { holdScreenOn } from "@/lib/tv-app";
import { cn } from "@/lib/utils";

import { PlayPauseButton, SeekBar, SkipButton, VolumeControl } from "./controls";
import { NowPlayingDrawer, NowPlayingSheet } from "./NowPlayingSheet";
import { drawerFocus, nowPlayingReturn } from "./nowPlaying";
import { CastButton } from "./CastButton";
import { currentChapterTitle, seekSpan } from "./chapterNav";
import { audioCommands, canSkipAudio, skipAudio } from "./usePlayer";
import { currentAudioItem, playerStore, usePlayerState } from "./store";
import { useMediaSource } from "./useMediaSource";
import { useProgressSync } from "./useProgressSync";
import { useTvVolume } from "./useTvVolume";

/**
 * Persistent audio player. Holds the single <audio> element for the whole app, mirrors its
 * state into the player store and renders the dock (desktop) or the mini bar (phones).
 */
export function PlayerDock() {
  const state = usePlayerState();
  const item = currentAudioItem(state);
  const audioRef = useRef<HTMLAudioElement>(null);
  const tvVolume = useTvVolume() !== null;
  const navigate = useNavigate();
  const activationKey = state.audioActivation;
  const progress = useProgressSync(item, activationKey);
  const media = useMediaSource(item && !progress.loading ? item.mediaFileId : null, audioRef, progress.resumePosition ?? item?.startPosition, { activationKey });
  const { transport } = state;

  const { report, flush } = progress;
  const { isTransitioning, retry } = media;

  useEffect(() => {
    if (!item) return;
    const retryAudio = () => {
      if (playerStore.getSnapshot().audioActivation !== activationKey) return;
      playerStore.updateTransport({ error: null, loading: true, playing: true });
      retry();
    };
    audioCommands.retry = retryAudio;
    return () => { if (audioCommands.retry === retryAudio) delete audioCommands.retry; };
  }, [item, activationKey, retry]);

  useEffect(() => {
    const audio = audioRef.current;
    if (!audio || !item || !media.source) return;
    const active = () => playerStore.getSnapshot().audioActivation === activationKey && !isTransitioning();
    const duration = () => Math.max(media.duration ?? 0, Number.isFinite(audio.duration) ? audio.duration : 0);
    const checkpoint = (playing: boolean) => report(audio.currentTime, duration(), playing);
    const seek = (position: number) => {
      if (!active() || !Number.isFinite(position)) return;
      audio.currentTime = Math.max(0, duration() > 0 ? Math.min(position, duration()) : position);
      playerStore.updateTransport({ position: audio.currentTime });
      checkpoint(!audio.paused);
    };
    audioCommands.seek = seek;
    const end = item.endPosition;
    let finished = false;
    // The entry's own end is its end: a lone chapter queued from a playlist stops here and the queue moves on.
    const onTime = () => {
      if (!active()) return;
      playerStore.updateTransport({ position: audio.currentTime, buffered: audio.buffered.length ? audio.buffered.end(audio.buffered.length - 1) : 0 });
      checkpoint(!audio.paused);
      if (end !== undefined && !finished && audio.currentTime >= end) {
        finished = true;
        audio.pause();
        onEnded();
      }
    };
    const onDuration = () => { if (active()) playerStore.updateTransport({ duration: duration() }); };
    const onPlay = () => {
      if (!active()) return;
      playerStore.updateTransport({ playing: true, loading: false, error: null });
      checkpoint(true);
    };
    const onPause = () => {
      if (!active() || !audio.paused) return;
      playerStore.updateTransport({ playing: false });
      checkpoint(false);
      void flush(true);
    };
    const onWaiting = () => { if (active()) playerStore.updateTransport({ loading: true }); };
    const onReady = () => {
      if (!active()) return;
      playerStore.updateTransport({ loading: false, duration: duration() });
      if (!audio.paused) onPlay();
    };
    const onEnded = () => {
      if (!active()) return;
      checkpoint(false);
      void flush(true);
      // Next entry, audio or video; past the last, whatever the queue continues with, else it stops.
      void playerStore.advance();
    };
    audio.addEventListener("timeupdate", onTime);
    audio.addEventListener("durationchange", onDuration);
    audio.addEventListener("loadedmetadata", onReady);
    audio.addEventListener("play", onPlay);
    audio.addEventListener("pause", onPause);
    audio.addEventListener("waiting", onWaiting);
    audio.addEventListener("canplay", onReady);
    audio.addEventListener("playing", onReady);
    audio.addEventListener("ended", onEnded);
    return () => {
      if (audioCommands.seek === seek) delete audioCommands.seek;
      audio.removeEventListener("timeupdate", onTime);
      audio.removeEventListener("durationchange", onDuration);
      audio.removeEventListener("loadedmetadata", onReady);
      audio.removeEventListener("play", onPlay);
      audio.removeEventListener("pause", onPause);
      audio.removeEventListener("waiting", onWaiting);
      audio.removeEventListener("canplay", onReady);
      audio.removeEventListener("playing", onReady);
      audio.removeEventListener("ended", onEnded);
    };
  }, [item, activationKey, media.source, media.duration, isTransitioning, report, flush]);

  useEffect(() => {
    if (!item) return;
    if (media.error) {
      playerStore.updateTransport({ error: media.error, loading: false, playing: false });
    } else if (media.loading) {
      playerStore.updateTransport({ loading: true });
    }
  }, [item, media.error, media.loading]);

  // Playing is the requested state while a source is transitioning. A load()
  // pause or an obsolete play() rejection must not cancel the next item's intent.
  useEffect(() => {
    const audio = audioRef.current;
    if (!audio || !item || !media.source || media.error) return;
    let cancelled = false;
    if (transport.playing && audio.paused) {
      void audio.play().catch((error: unknown) => {
        if (cancelled || playerStore.getSnapshot().audioActivation !== activationKey) return;
        if (error instanceof DOMException && error.name === "AbortError" && isTransitioning()) return;
        playerStore.updateTransport({ playing: false, loading: false, error: "Playback could not start. Retry playback." });
      });
    }
    if (!transport.playing && !audio.paused) audio.pause();
    return () => { cancelled = true; };
  }, [item, activationKey, transport.playing, media.source, media.error, isTransitioning]);

  useEffect(() => {
    const audio = audioRef.current;
    if (!audio) return;
    // In the TV app the TV's own volume is the level (VolumeControl); the element stays at full.
    audio.volume = tvVolume ? 1 : transport.volume;
    audio.muted = transport.muted;
    audio.playbackRate = transport.rate;
  }, [transport.volume, transport.muted, transport.rate, tvVolume]);

  // Media Session integration for lock screens, headsets and TV remotes.
  useEffect(() => {
    if (!("mediaSession" in navigator) || !item) return;
    navigator.mediaSession.metadata = new MediaMetadata({ title: item.title, artist: item.subtitle ?? "", artwork: item.artwork ? [{ src: item.artwork, sizes: "512x512" }] : [] });
    const handlers: Array<[MediaSessionAction, MediaSessionActionHandler]> = [
      ["play", () => playerStore.updateTransport({ playing: true })],
      ["pause", () => playerStore.updateTransport({ playing: false })],
      ["previoustrack", () => skipAudio(-1)],
      ["nexttrack", () => skipAudio(1)],
      ["seekto", (details) => details.seekTime !== undefined && audioCommands.seek?.(details.seekTime)],
      ["seekbackward", () => audioCommands.seek?.(Math.max(0, (audioRef.current?.currentTime ?? 0) - 15))],
      ["seekforward", () => audioCommands.seek?.((audioRef.current?.currentTime ?? 0) + 30)],
      ["stop", () => playerStore.stopAudio()],
    ];
    for (const [action, handler] of handlers) {
      try {
        navigator.mediaSession.setActionHandler(action, handler);
      } catch {
        // Unsupported action on this platform.
      }
    }
    return () => {
      for (const [action] of handlers) {
        try { navigator.mediaSession.setActionHandler(action, null); } catch { /* Unsupported action. */ }
      }
      navigator.mediaSession.metadata = null;
    };
  }, [item]);

  useEffect(() => {
    if ("mediaSession" in navigator) navigator.mediaSession.playbackState = transport.playing ? "playing" : "paused";
  }, [transport.playing]);

  // The TV app keeps the screen on while something plays.
  const screenToken = useRef(Symbol("audio"));
  useEffect(() => {
    const token = screenToken.current;
    holdScreenOn(token, Boolean(item) && transport.playing);
    return () => holdScreenOn(token, false);
  }, [item, transport.playing]);

  const desktop = useIsDesktop();
  const bodyHeight = useShellBodyHeight(desktop && state.sheetOpen);

  // On a TV, audio a person starts opens now playing in full, with Play/Pause focused; Back
  // collapses it to the bar and focus goes back to what started it (design.md Q125). Add to queue,
  // and Play next while something plays, leave the screen alone: they do not start anything.
  const tv = useIsTvMode();
  const starts = useRef(state.audioStarts);
  useEffect(() => {
    if (state.audioStarts === starts.current) return;
    starts.current = state.audioStarts;
    if (!tv || !currentAudioItem(playerStore.getSnapshot())) return;
    const opener = document.activeElement instanceof HTMLElement && document.activeElement !== document.body ? document.activeElement : null;
    if (opener && !opener.closest("[data-dock]")) nowPlayingReturn.current = opener;
    drawerFocus.current = "transport";
    playerStore.setSheetOpen(true);
  }, [state.audioStarts, tv]);

  // The bar is never a trap: Back collapses an open drawer, and Back or Up from the bar returns
  // focus to the page control it came from.
  const dockKeys = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    if (event.altKey || event.ctrlKey || event.metaKey) return;
    const target = event.target instanceof HTMLElement ? event.target : null;
    if (target?.closest('[role="menu"], [role="listbox"], input, textarea')) return;
    const back = event.key === "Escape" || event.key === "GoBack" || event.keyCode === 461 || event.keyCode === 10009 || (tv && event.key === "Backspace");
    // A focused icon button's tooltip takes Escape first (it prevents the default); on a TV that
    // would make Back do nothing, so Back is still ours. Anything else already handled is not.
    if (event.defaultPrevented && !back) return;
    const open = playerStore.getSnapshot().sheetOpen;
    if (back && open) {
      event.preventDefault();
      event.stopPropagation();
      playerStore.setSheetOpen(false);
      return;
    }
    const inBar = Boolean(target?.closest("[data-dock-bar]")) && target?.getAttribute("role") !== "slider";
    if (!open && (back || (event.key === "ArrowUp" && inBar))) {
      const previous = nowPlayingReturn.current;
      if (previous?.isConnected && !previous.closest("[data-dock]")) {
        event.preventDefault();
        event.stopPropagation();
        nowPlayingReturn.current = null;
        previous.focus({ preventScroll: true });
      }
    }
  };
  const rememberEntry = (event: ReactFocusEvent<HTMLDivElement>) => {
    const from = event.relatedTarget;
    if (from instanceof HTMLElement && !event.currentTarget.contains(from)) nowPlayingReturn.current = from;
  };
  const audioElement = <audio ref={audioRef} preload="metadata" crossOrigin="use-credentials" className="hidden" />;
  if (!item) return audioElement;
  const drawerOpen = desktop && state.sheetOpen;
  const span = seekSpan(item, transport.position, transport.duration);

  return (
    <>
      {audioElement}
      {/* On larger screens the dock floats in the content area with the same gap on every side, and
          keeps its place in the layout so nothing scrolls under it. Opening now playing grows the
          same panel upward, gaps kept, into a drawer above the controls. */}
      <div className="relative shrink-0 max-md:contents md:h-[calc(var(--dock-height)+var(--space-3))]">
      <div
        className={cn(
          "glass-chrome z-30 flex flex-col rounded-card border shadow-dock",
          "max-md:fixed max-md:inset-x-3 max-md:bottom-[calc(var(--tabbar-height)+max(var(--space-3),var(--safe-bottom))+var(--space-2))]",
          "md:absolute md:inset-x-3 md:bottom-3 md:transition-[height] md:duration-base md:ease-fluid",
        )}
        style={desktop ? { height: drawerOpen && bodyHeight > 0 ? `calc(${bodyHeight}px - 2 * var(--space-3))` : "var(--dock-height)" } : undefined}
        role="region"
        aria-label="Now playing"
        data-dock
        onKeyDownCapture={dockKeys}
        onFocusCapture={rememberEntry}
      >
        {desktop ? <NowPlayingDrawer open={drawerOpen} onClose={() => playerStore.setSheetOpen(false)} onOpenEntity={(href) => { playerStore.setSheetOpen(false); void navigate({ href }); }} /> : null}
      <div className={cn("relative flex shrink-0 items-center gap-3 px-3 h-dock md:px-4", "max-md:h-16")} data-dock-bar>
        {transport.error ? (
          <Notice tone="danger" compact role="alert" className="absolute inset-x-0 bottom-full mb-2" action={<Button size="sm" onClick={() => audioCommands.retry?.()}>Retry</Button>}>
            {transport.error}
          </Notice>
        ) : null}
        <ListRow
          onClick={() => playerStore.setSheetOpen(!drawerOpen)}
          aria-label={drawerOpen ? "Close now playing" : "Open now playing"}
          aria-expanded={desktop ? drawerOpen : undefined}
          title={item.title}
          meta={currentChapterTitle(item, transport.position) ?? item.subtitle}
          icon={<Artwork src={item.artwork} alt="" aspect="square" className="size-11 shrink-0 rounded-md md:size-12" />}
          trailing={<IconChevronUp size="sm" className="text-muted md:hidden" />}
          className="min-w-0 flex-1 md:w-48 md:flex-none lg:w-72"
          data-focusable
        />

        <div className="hidden min-w-0 flex-1 flex-col items-center gap-1 md:flex">
          <div className="flex items-center gap-1">
            <SkipButton direction="back" onPress={() => skipAudio(-1)} disabled={!canSkipAudio(state.queue, state.index, transport.position, -1)} />
            <PlayPauseButton playing={transport.playing} onToggle={() => playerStore.updateTransport({ playing: !transport.playing })} size="sm" start={desktop} />
            <SkipButton direction="forward" onPress={() => skipAudio(1)} disabled={!canSkipAudio(state.queue, state.index, transport.position, 1)} />
          </div>
          <div className="flex w-full max-w-2xl items-center gap-3 text-label-sm text-muted">
            <span className="text-numeric w-10 text-right">{formatTime(transport.position - span.start)}</span>
            <SeekBar thin mediaKeys position={transport.position - span.start} duration={span.end - span.start} buffered={Math.max(0, transport.buffered - span.start)} onSeek={(position) => audioCommands.seek?.(span.start + position)} labelAt={(position) => currentChapterTitle(item, span.start + position)} />
            <span className="text-numeric w-10">{formatTime(span.end - span.start)}</span>
          </div>
        </div>

        <div className="flex shrink-0 items-center gap-1 md:w-auto md:justify-end lg:w-72">
          <PlayPauseButton playing={transport.playing} onToggle={() => playerStore.updateTransport({ playing: !transport.playing })} size="sm" className="md:hidden" start={!desktop} />
          <VolumeControl volume={transport.volume} muted={transport.muted} onVolume={(volume) => playerStore.updateTransport({ volume, muted: volume === 0 })} onMuted={(muted) => playerStore.updateTransport({ muted })} className="hidden md:flex" />
          <CastButton item={item} activationKey={activationKey} position={transport.position} onStarted={() => {
            if (playerStore.getSnapshot().audioActivation === activationKey) playerStore.updateTransport({ playing: false });
          }} />
          <Button variant="ghost" shape="circle" aria-label="Queue" aria-expanded={desktop ? drawerOpen : undefined} onClick={() => playerStore.setSheetOpen(!drawerOpen)} icon={<IconList />} className="hidden md:inline-flex" data-focusable />
          <Button variant="ghost" shape="circle" aria-label="Stop" onClick={() => playerStore.stopAudio()} icon={<IconClose />} data-focusable />
        </div>
        <div className="absolute inset-x-3 top-0 h-0.5 overflow-hidden rounded-full bg-white/10 md:hidden">
          <div className="h-full bg-brand" style={{ width: `${transport.duration ? (transport.position / transport.duration) * 100 : 0}%` }} />
        </div>
      </div>
      </div>
      </div>
      {desktop ? null : <NowPlayingSheet open={state.sheetOpen} onClose={() => playerStore.setSheetOpen(false)} onOpenEntity={(href) => void navigate({ href })} />}
    </>
  );
}

function formatTime(seconds: number): string {
  const total = Math.max(0, Math.floor(seconds || 0));
  const m = Math.floor(total / 60);
  const s = total % 60;
  const h = Math.floor(m / 60);
  return h > 0 ? `${h}:${String(m % 60).padStart(2, "0")}:${String(s).padStart(2, "0")}` : `${m}:${String(s).padStart(2, "0")}`;
}

/**
 * The height of the shell body the dock sits in, while the drawer is open: the drawer fills it
 * less the gaps. Measured rather than a percentage so the height can transition.
 */
function useShellBodyHeight(active: boolean): number {
  const [height, setHeight] = useState(0);
  useEffect(() => {
    if (!active) return;
    const body = document.querySelector<HTMLElement>(".app-shell-body");
    if (!body) return;
    const measure = () => setHeight(body.clientHeight);
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(body);
    return () => observer.disconnect();
  }, [active]);
  return height;
}
