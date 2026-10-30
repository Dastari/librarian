import { useNavigate } from "@tanstack/react-router";
import { Button } from "glass-ux/button";
import { ListRow } from "glass-ux/list-row";
import { Notice } from "glass-ux/notice";
import { IconChevronUp, IconList, IconClose } from "glass-ux/icons";
import { useEffect, useRef } from "react";

import { Artwork } from "@/components/ui";
import { cn } from "@/lib/utils";

import { PlayPauseButton, SeekBar, SkipButton, VolumeControl } from "./controls";
import { NowPlayingSheet } from "./NowPlayingSheet";
import { CastButton } from "./CastButton";
import { audioCommands } from "./usePlayer";
import { currentAudioItem, playerStore, usePlayerState } from "./store";
import { useMediaSource } from "./useMediaSource";
import { useProgressSync } from "./useProgressSync";

/**
 * Persistent audio player. Holds the single <audio> element for the whole app, mirrors its
 * state into the player store and renders the dock (desktop) or the mini bar (phones).
 */
export function PlayerDock() {
  const state = usePlayerState();
  const item = currentAudioItem(state);
  const audioRef = useRef<HTMLAudioElement>(null);
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
    const onTime = () => {
      if (!active()) return;
      playerStore.updateTransport({ position: audio.currentTime, buffered: audio.buffered.length ? audio.buffered.end(audio.buffered.length - 1) : 0 });
      checkpoint(!audio.paused);
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
      const snapshot = playerStore.getSnapshot();
      if (snapshot.index < snapshot.queue.length - 1) playerStore.skip(1);
      else playerStore.updateTransport({ playing: false });
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
    audio.volume = transport.volume;
    audio.muted = transport.muted;
    audio.playbackRate = transport.rate;
  }, [transport.volume, transport.muted, transport.rate]);

  // Media Session integration for lock screens, headsets and TV remotes.
  useEffect(() => {
    if (!("mediaSession" in navigator) || !item) return;
    navigator.mediaSession.metadata = new MediaMetadata({ title: item.title, artist: item.subtitle ?? "", artwork: item.artwork ? [{ src: item.artwork, sizes: "512x512" }] : [] });
    const handlers: Array<[MediaSessionAction, MediaSessionActionHandler]> = [
      ["play", () => playerStore.updateTransport({ playing: true })],
      ["pause", () => playerStore.updateTransport({ playing: false })],
      ["previoustrack", () => playerStore.skip(-1)],
      ["nexttrack", () => playerStore.skip(1)],
      ["seekto", (details) => details.seekTime !== undefined && audioCommands.seek?.(details.seekTime)],
      ["seekbackward", () => audioCommands.seek?.(Math.max(0, (audioRef.current?.currentTime ?? 0) - 15))],
      ["seekforward", () => audioCommands.seek?.((audioRef.current?.currentTime ?? 0) + 30)],
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

  const audioElement = <audio ref={audioRef} preload="metadata" crossOrigin="use-credentials" className="hidden" />;
  if (!item) return audioElement;

  return (
    <>
      {audioElement}
      <div
        className={cn(
          "glass-chrome relative z-30 flex shrink-0 items-center gap-3 rounded-none border-0 border-t px-3",
          "h-dock md:px-4",
          "max-md:fixed max-md:inset-x-0 max-md:bottom-[calc(var(--tabbar-height)+var(--safe-bottom))] max-md:h-16",
        )}
        role="region"
        aria-label="Now playing"
      >
        {transport.error ? (
          <Notice tone="danger" compact role="alert" className="absolute inset-x-0 bottom-full" action={<Button size="sm" onClick={() => audioCommands.retry?.()}>Retry</Button>}>
            {transport.error}
          </Notice>
        ) : null}
        <ListRow
          onClick={() => playerStore.setSheetOpen(true)}
          aria-label="Open now playing"
          title={item.title}
          meta={item.subtitle}
          icon={<Artwork src={item.artwork} alt="" aspect="square" className="size-11 shrink-0 rounded-md md:size-12" />}
          trailing={<IconChevronUp size="sm" className="text-muted md:hidden" />}
          className="min-w-0 flex-1 md:w-48 md:flex-none lg:w-72"
          data-focusable
        />

        <div className="hidden min-w-0 flex-1 flex-col items-center gap-1 md:flex">
          <div className="flex items-center gap-1">
            <SkipButton direction="back" onPress={() => playerStore.skip(-1)} disabled={state.index <= 0} />
            <PlayPauseButton playing={transport.playing} onToggle={() => playerStore.updateTransport({ playing: !transport.playing })} size="sm" />
            <SkipButton direction="forward" onPress={() => playerStore.skip(1)} disabled={state.index >= state.queue.length - 1} />
          </div>
          <div className="flex w-full max-w-2xl items-center gap-3 text-label-sm text-muted">
            <span className="text-numeric w-10 text-right">{formatTime(transport.position)}</span>
            <SeekBar thin position={transport.position} duration={transport.duration} buffered={transport.buffered} onSeek={(position) => audioCommands.seek?.(position)} />
            <span className="text-numeric w-10">{formatTime(transport.duration)}</span>
          </div>
        </div>

        <div className="flex shrink-0 items-center gap-1 md:w-auto md:justify-end lg:w-72">
          <PlayPauseButton playing={transport.playing} onToggle={() => playerStore.updateTransport({ playing: !transport.playing })} size="sm" className="md:hidden" />
          <VolumeControl volume={transport.volume} muted={transport.muted} onVolume={(volume) => playerStore.updateTransport({ volume, muted: volume === 0 })} onMuted={(muted) => playerStore.updateTransport({ muted })} className="hidden md:flex" />
          <CastButton item={item} activationKey={activationKey} position={transport.position} onStarted={() => {
            if (playerStore.getSnapshot().audioActivation === activationKey) playerStore.updateTransport({ playing: false });
          }} />
          <Button variant="ghost" shape="circle" aria-label="Queue" onClick={() => playerStore.setSheetOpen(true)} icon={<IconList />} className="hidden md:inline-flex" data-focusable />
          <Button variant="ghost" shape="circle" aria-label="Stop" onClick={() => playerStore.stopAudio()} icon={<IconClose />} data-focusable />
        </div>
        <div className="absolute inset-x-0 top-0 h-0.5 bg-white/10 md:hidden">
          <div className="h-full bg-brand" style={{ width: `${transport.duration ? (transport.position / transport.duration) * 100 : 0}%` }} />
        </div>
      </div>
      <NowPlayingSheet open={state.sheetOpen} onClose={() => playerStore.setSheetOpen(false)} onOpenEntity={(href) => void navigate({ href })} />
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
