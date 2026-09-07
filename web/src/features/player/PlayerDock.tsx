import { useNavigate } from "@tanstack/react-router";
import { IconChevronUp, IconListDetails, IconX } from "@tabler/icons-react";
import { useEffect, useRef } from "react";

import { Artwork } from "@/components/ui";
import { cn } from "@/lib/utils";

import { PlayPauseButton, SeekBar, SkipButton, VolumeControl } from "./controls";
import { NowPlayingSheet } from "./NowPlayingSheet";
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
  const progress = useProgressSync(item);
  const media = useMediaSource(item && !progress.loading ? item.mediaFileId : null, audioRef, progress.resumePosition ?? item?.startPosition);
  const { transport } = state;

  useEffect(() => {
    const audio = audioRef.current;
    if (!audio) return;
    audioCommands.seek = (position) => {
      audio.currentTime = position;
      playerStore.updateTransport({ position });
    };
    const onTime = () => {
      playerStore.updateTransport({ position: audio.currentTime, buffered: audio.buffered.length ? audio.buffered.end(audio.buffered.length - 1) : 0 });
      progress.report(audio.currentTime, audio.duration || 0, !audio.paused);
    };
    const onDuration = () => playerStore.updateTransport({ duration: Number.isFinite(audio.duration) ? audio.duration : 0 });
    const onPlay = () => playerStore.updateTransport({ playing: true, loading: false });
    const onPause = () => {
      playerStore.updateTransport({ playing: false });
      void progress.flush(true);
    };
    const onWaiting = () => playerStore.updateTransport({ loading: true });
    const onReady = () => playerStore.updateTransport({ loading: false });
    const onEnded = () => {
      void progress.flush(true);
      const snapshot = playerStore.getSnapshot();
      if (snapshot.index < snapshot.queue.length - 1) playerStore.skip(1);
      else playerStore.updateTransport({ playing: false });
    };
    const onError = () => playerStore.updateTransport({ error: "This track could not be played.", loading: false, playing: false });
    audio.addEventListener("timeupdate", onTime);
    audio.addEventListener("durationchange", onDuration);
    audio.addEventListener("play", onPlay);
    audio.addEventListener("pause", onPause);
    audio.addEventListener("waiting", onWaiting);
    audio.addEventListener("canplay", onReady);
    audio.addEventListener("playing", onReady);
    audio.addEventListener("ended", onEnded);
    audio.addEventListener("error", onError);
    return () => {
      audio.removeEventListener("timeupdate", onTime);
      audio.removeEventListener("durationchange", onDuration);
      audio.removeEventListener("play", onPlay);
      audio.removeEventListener("pause", onPause);
      audio.removeEventListener("waiting", onWaiting);
      audio.removeEventListener("canplay", onReady);
      audio.removeEventListener("playing", onReady);
      audio.removeEventListener("ended", onEnded);
      audio.removeEventListener("error", onError);
    };
  }, [progress]);

  // Store → element: play/pause, volume, rate.
  useEffect(() => {
    const audio = audioRef.current;
    if (!audio || !media.source) return;
    if (transport.playing && audio.paused) void audio.play().catch(() => playerStore.updateTransport({ playing: false }));
    if (!transport.playing && !audio.paused) audio.pause();
  }, [transport.playing, media.source]);

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
        <button type="button" data-focusable onClick={() => playerStore.setSheetOpen(true)} className="nav-focus flex min-w-0 flex-1 items-center gap-3 rounded-lg text-left md:flex-none md:w-72" aria-label="Open now playing">
          <Artwork src={item.artwork} alt="" aspect="square" className="size-11 shrink-0 rounded-md md:size-12" />
          <span className="min-w-0">
            <span className="block truncate text-title-sm text-foreground">{item.title}</span>
            {item.subtitle ? <span className="block truncate text-label-sm text-muted">{item.subtitle}</span> : null}
          </span>
          <IconChevronUp size={16} className="ml-auto text-muted md:hidden" />
        </button>

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

        <div className="flex items-center gap-1 md:w-72 md:justify-end">
          <PlayPauseButton playing={transport.playing} onToggle={() => playerStore.updateTransport({ playing: !transport.playing })} size="sm" className="md:hidden" />
          <VolumeControl volume={transport.volume} muted={transport.muted} onVolume={(volume) => playerStore.updateTransport({ volume, muted: volume === 0 })} onMuted={(muted) => playerStore.updateTransport({ muted })} className="hidden md:flex" />
          <button type="button" data-focusable aria-label="Queue" onClick={() => playerStore.setSheetOpen(true)} className="nav-focus hidden size-10 place-items-center rounded-full text-foreground/90 hover:bg-white/10 md:grid">
            <IconListDetails size={20} />
          </button>
          <button type="button" data-focusable aria-label="Stop" onClick={() => playerStore.stopAudio()} className="nav-focus grid size-10 place-items-center rounded-full text-muted hover:bg-white/10 hover:text-foreground">
            <IconX size={18} />
          </button>
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
