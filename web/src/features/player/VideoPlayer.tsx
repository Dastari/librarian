import { useQuery } from "@apollo/client/react";
import { Spinner } from "@/components/ui";
import { Dropdown } from "@heroui/react";
import { useNavigate } from "@tanstack/react-router";
import {
  IconArrowLeft,
  IconBadgeCc,
  IconCast,
  IconLanguage,
  IconMaximize,
  IconMinimize,
  IconPictureInPicture,
  IconPlayerTrackNext,
  IconRewindBackward10,
  IconRewindForward10,
  IconSettings,
} from "@tabler/icons-react";
import { useCallback, useEffect, useMemo, useRef, useState, type Key } from "react";

import { MediaFileStreamsDocument } from "@/graphql/generated/graphql";
import { formatClock } from "@/lib/format";
import { clamp, cn } from "@/lib/utils";

import { PlayPauseButton, SeekBar, VolumeControl } from "./controls";
import type { PlayItem } from "./store";
import { useMediaSource } from "./useMediaSource";
import { useProgressSync } from "./useProgressSync";

const RATES = [0.5, 0.75, 1, 1.25, 1.5, 2];
const HIDE_DELAY_MS = 3200;

interface VideoPlayerProps {
  item: PlayItem;
  onBack: () => void;
  /** Called when the file finishes; the caller decides whether to autoplay the next episode. */
  onEnded?: () => void;
  nextLabel?: string;
  onNext?: () => void;
}

/**
 * Immersive video player. Controls fade after a few seconds of inactivity and come back on
 * pointer movement, touch or any key. Keyboard: space/k play, j/l or arrows seek 10s, f fullscreen,
 * m mute, c captions, esc back.
 */
export function VideoPlayer({ item, onBack, onEnded, nextLabel, onNext }: VideoPlayerProps) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const shellRef = useRef<HTMLDivElement>(null);
  const hideTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const navigate = useNavigate();

  const progress = useProgressSync(item);
  const media = useMediaSource(progress.loading ? null : item.mediaFileId, videoRef, progress.resumePosition);
  const streams = useQuery(MediaFileStreamsDocument, { variables: { mediaFileId: item.mediaFileId } });

  const [playing, setPlaying] = useState(false);
  const [position, setPosition] = useState(0);
  const [duration, setDuration] = useState(0);
  const [buffered, setBuffered] = useState(0);
  const [volume, setVolume] = useState(1);
  const [muted, setMuted] = useState(false);
  const [rate, setRate] = useState(1);
  const [waiting, setWaiting] = useState(true);
  const [controlsVisible, setControlsVisible] = useState(true);
  const [fullscreen, setFullscreen] = useState(false);
  const [textTrack, setTextTrack] = useState<string>("off");
  const [audioTrack, setAudioTrack] = useState<number>(-1);

  const fullDuration = media.duration && media.duration > duration ? media.duration : duration;
  const chapters = useMemo(() => streams.data?.mediaChapters.edges.map((edge) => ({ start: edge.node.startSecs, title: edge.node.title })) ?? [], [streams.data]);
  const currentChapter = useMemo(() => [...chapters].reverse().find((chapter) => chapter.start <= position), [chapters, position]);

  const wake = useCallback(() => {
    setControlsVisible(true);
    if (hideTimer.current) clearTimeout(hideTimer.current);
    hideTimer.current = setTimeout(() => {
      if (videoRef.current && !videoRef.current.paused) setControlsVisible(false);
    }, HIDE_DELAY_MS);
  }, []);

  useEffect(() => {
    wake();
    return () => {
      if (hideTimer.current) clearTimeout(hideTimer.current);
    };
  }, [wake]);

  // Wire media element events.
  useEffect(() => {
    const video = videoRef.current;
    if (!video) return;
    const onTime = () => {
      setPosition(video.currentTime);
      if (video.buffered.length) setBuffered(video.buffered.end(video.buffered.length - 1));
      progress.report(video.currentTime, fullDuration || video.duration, !video.paused);
    };
    const onDuration = () => setDuration(Number.isFinite(video.duration) ? video.duration : 0);
    const onPlay = () => {
      setPlaying(true);
      wake();
    };
    const onPause = () => {
      setPlaying(false);
      setControlsVisible(true);
      void progress.flush(true);
    };
    const onWaiting = () => setWaiting(true);
    const onReady = () => setWaiting(false);
    const onVolume = () => {
      setVolume(video.volume);
      setMuted(video.muted);
    };
    const onRate = () => setRate(video.playbackRate);
    const onEnd = () => {
      void progress.flush(true);
      onEnded?.();
    };
    video.addEventListener("timeupdate", onTime);
    video.addEventListener("durationchange", onDuration);
    video.addEventListener("play", onPlay);
    video.addEventListener("pause", onPause);
    video.addEventListener("waiting", onWaiting);
    video.addEventListener("playing", onReady);
    video.addEventListener("canplay", onReady);
    video.addEventListener("volumechange", onVolume);
    video.addEventListener("ratechange", onRate);
    video.addEventListener("ended", onEnd);
    return () => {
      video.removeEventListener("timeupdate", onTime);
      video.removeEventListener("durationchange", onDuration);
      video.removeEventListener("play", onPlay);
      video.removeEventListener("pause", onPause);
      video.removeEventListener("waiting", onWaiting);
      video.removeEventListener("playing", onReady);
      video.removeEventListener("canplay", onReady);
      video.removeEventListener("volumechange", onVolume);
      video.removeEventListener("ratechange", onRate);
      video.removeEventListener("ended", onEnd);
    };
  }, [fullDuration, onEnded, progress, wake]);

  // Autoplay once the source is attached.
  useEffect(() => {
    const video = videoRef.current;
    if (!video || !media.source) return;
    void video.play().catch(() => setControlsVisible(true));
  }, [media.source]);

  useEffect(() => {
    const onChange = () => setFullscreen(Boolean(document.fullscreenElement));
    document.addEventListener("fullscreenchange", onChange);
    return () => document.removeEventListener("fullscreenchange", onChange);
  }, []);

  const togglePlay = useCallback(() => {
    const video = videoRef.current;
    if (!video) return;
    if (video.paused) void video.play();
    else video.pause();
  }, []);

  const seekTo = useCallback((next: number) => {
    const video = videoRef.current;
    if (!video) return;
    video.currentTime = clamp(next, 0, fullDuration || video.duration || next);
    setPosition(video.currentTime);
  }, [fullDuration]);

  const seekBy = useCallback((delta: number) => seekTo((videoRef.current?.currentTime ?? 0) + delta), [seekTo]);

  const toggleFullscreen = useCallback(() => {
    const shell = shellRef.current;
    if (!shell) return;
    if (document.fullscreenElement) void document.exitFullscreen();
    else if (shell.requestFullscreen) void shell.requestFullscreen();
    else {
      // iOS Safari only supports fullscreen on the video element itself.
      const video = videoRef.current as (HTMLVideoElement & { webkitEnterFullscreen?: () => void }) | null;
      video?.webkitEnterFullscreen?.();
    }
  }, []);

  const togglePip = useCallback(() => {
    const video = videoRef.current;
    if (!video || !document.pictureInPictureEnabled) return;
    if (document.pictureInPictureElement) void document.exitPictureInPicture();
    else void video.requestPictureInPicture();
  }, []);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (target && (target.tagName === "INPUT" || target.getAttribute("role") === "slider")) return;
      wake();
      switch (event.key) {
        case " ":
        case "k":
        case "MediaPlayPause":
          event.preventDefault();
          togglePlay();
          break;
        case "ArrowLeft":
        case "j":
        case "MediaRewind":
          if (!event.altKey) {
            event.preventDefault();
            seekBy(-10);
          }
          break;
        case "ArrowRight":
        case "l":
        case "MediaFastForward":
          event.preventDefault();
          seekBy(10);
          break;
        case "f":
          toggleFullscreen();
          break;
        case "m":
          if (videoRef.current) videoRef.current.muted = !videoRef.current.muted;
          break;
        case "Escape":
        case "Backspace":
        case "GoBack":
          if (!document.fullscreenElement) onBack();
          break;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onBack, seekBy, toggleFullscreen, togglePlay, wake]);

  // Subtitle and audio track lists come from hls.js or the native element.
  const textTracks = useMemo(() => {
    const video = videoRef.current;
    const fromHls = media.hls?.subtitleTracks.map((track, index) => ({ id: `hls-${index}`, label: track.name || track.lang || `Track ${index + 1}` })) ?? [];
    const fromNative = video ? Array.from(video.textTracks).map((track, index) => ({ id: `native-${index}`, label: track.label || track.language || `Track ${index + 1}` })) : [];
    return fromHls.length ? fromHls : fromNative;
  }, [media.hls, media.source]);

  const audioTracks = useMemo(() => media.hls?.audioTracks.map((track, index) => ({ id: index, label: track.name || track.lang || `Audio ${index + 1}` })) ?? [], [media.hls, media.source]);

  const applyTextTrack = (id: string) => {
    setTextTrack(id);
    const video = videoRef.current;
    if (media.hls) {
      media.hls.subtitleTrack = id === "off" ? -1 : Number(id.replace("hls-", ""));
      media.hls.subtitleDisplay = id !== "off";
      return;
    }
    if (!video) return;
    Array.from(video.textTracks).forEach((track, index) => {
      track.mode = id === `native-${index}` ? "showing" : "disabled";
    });
  };

  const applyAudioTrack = (id: number) => {
    setAudioTrack(id);
    if (media.hls) media.hls.audioTrack = id;
  };

  const onSettingsAction = (key: Key) => {
    const value = String(key);
    if (value.startsWith("rate-")) {
      const next = Number(value.slice(5));
      if (videoRef.current) videoRef.current.playbackRate = next;
    } else if (value.startsWith("audio-")) applyAudioTrack(Number(value.slice(6)));
  };

  return (
    <div
      ref={shellRef}
      className={cn("relative h-svh w-full select-none overflow-hidden bg-black text-white", !controlsVisible && playing && "cursor-none")}
      onPointerMove={wake}
      onPointerDown={wake}
      onTouchStart={wake}
    >
      <video ref={videoRef} className="h-full w-full object-contain" playsInline preload="metadata" onClick={togglePlay} onDoubleClick={toggleFullscreen} crossOrigin="use-credentials" />

      {(waiting || progress.loading) && !media.error ? (
        <div className="pointer-events-none absolute inset-0 grid place-items-center">
          <Spinner size={28} className="text-white" />
        </div>
      ) : null}

      {media.error ? (
        <div role="alert" className="absolute inset-0 grid place-items-center bg-black/70 p-6 text-center">
          <div className="glass-strong max-w-md rounded-card p-6">
            <p className="text-title-lg">Playback stopped</p>
            <p className="mt-2 text-body-sm text-white/75">{media.error}</p>
            <button type="button" className="nav-focus mt-4 rounded-pill bg-brand px-5 py-2 text-brand-foreground" onClick={onBack} data-focusable>
              Go back
            </button>
          </div>
        </div>
      ) : null}

      {/* Top bar */}
      <div className={cn("absolute inset-x-0 top-0 flex items-center gap-3 bg-gradient-to-b from-black/80 to-transparent px-4 pb-10 pt-[calc(var(--safe-top)+0.75rem)] transition-opacity duration-base", controlsVisible ? "opacity-100" : "pointer-events-none opacity-0")}>
        <button type="button" data-focusable data-spatial-start aria-label="Back" onClick={onBack} className="nav-focus grid size-11 place-items-center rounded-full text-white hover:bg-white/10">
          <IconArrowLeft size={22} />
        </button>
        <div className="min-w-0 flex-1">
          <p className="truncate text-title-md text-white text-shadow-hero">{item.title}</p>
          {item.subtitle || currentChapter?.title ? <p className="truncate text-label text-white/70">{[item.subtitle, currentChapter?.title].filter(Boolean).join(" · ")}</p> : null}
        </div>
        <button type="button" data-focusable aria-label="Cast" className="nav-focus grid size-11 place-items-center rounded-full text-white hover:bg-white/10" onClick={() => void navigate({ to: "/settings/casting" })}>
          <IconCast size={20} />
        </button>
      </div>

      {/* Bottom controls */}
      <div className={cn("absolute inset-x-0 bottom-0 flex flex-col gap-2 bg-gradient-to-t from-black/85 via-black/40 to-transparent px-4 pb-[calc(var(--safe-bottom)+0.75rem)] pt-16 transition-opacity duration-base sm:px-6", controlsVisible ? "opacity-100" : "pointer-events-none opacity-0")}>
        <SeekBar position={position} duration={fullDuration} buffered={buffered} onSeek={seekTo} chapters={chapters} />
        <div className="flex items-center gap-1 sm:gap-2">
          <PlayPauseButton playing={playing} onToggle={togglePlay} size="md" className="bg-white text-black" />
          <button type="button" data-focusable aria-label="Back 10 seconds" onClick={() => seekBy(-10)} className="nav-focus grid size-10 place-items-center rounded-full hover:bg-white/10">
            <IconRewindBackward10 size={22} />
          </button>
          <button type="button" data-focusable aria-label="Forward 10 seconds" onClick={() => seekBy(10)} className="nav-focus grid size-10 place-items-center rounded-full hover:bg-white/10">
            <IconRewindForward10 size={22} />
          </button>
          <VolumeControl
            volume={volume}
            muted={muted}
            onVolume={(next) => {
              if (videoRef.current) {
                videoRef.current.volume = next;
                videoRef.current.muted = next === 0;
              }
            }}
            onMuted={(next) => {
              if (videoRef.current) videoRef.current.muted = next;
            }}
            className="hidden sm:flex"
          />
          <span className="text-numeric ml-1 text-label text-white/80">
            {formatClock(position)} <span className="text-white/40">/</span> {formatClock(fullDuration)}
          </span>
          <div className="flex-1" />
          {onNext ? (
            <button type="button" data-focusable onClick={onNext} className="nav-focus hidden items-center gap-2 rounded-pill px-3 py-2 text-label hover:bg-white/10 sm:inline-flex">
              <IconPlayerTrackNext size={18} /> {nextLabel ?? "Next"}
            </button>
          ) : null}
          {textTracks.length > 0 ? (
            <Dropdown>
              <Dropdown.Trigger aria-label="Subtitles" className="nav-focus grid size-10 place-items-center rounded-full hover:bg-white/10" data-focusable>
                <IconBadgeCc size={22} className={textTrack !== "off" ? "text-brand" : undefined} />
              </Dropdown.Trigger>
              <Dropdown.Popover placement="top end" className="glass-surface">
                <Dropdown.Menu aria-label="Subtitles" selectionMode="single" selectedKeys={[textTrack]} onSelectionChange={(keys) => applyTextTrack(String([...keys][0] ?? "off"))}>
                  <Dropdown.Item id="off" textValue="Off">
                    Off
                  </Dropdown.Item>
                  {textTracks.map((track) => (
                    <Dropdown.Item key={track.id} id={track.id} textValue={track.label}>
                      {track.label}
                    </Dropdown.Item>
                  ))}
                </Dropdown.Menu>
              </Dropdown.Popover>
            </Dropdown>
          ) : null}
          {audioTracks.length > 1 ? (
            <Dropdown>
              <Dropdown.Trigger aria-label="Audio track" className="nav-focus grid size-10 place-items-center rounded-full hover:bg-white/10" data-focusable>
                <IconLanguage size={22} />
              </Dropdown.Trigger>
              <Dropdown.Popover placement="top end" className="glass-surface">
                <Dropdown.Menu aria-label="Audio track" selectionMode="single" selectedKeys={[String(audioTrack)]} onAction={onSettingsAction}>
                  {audioTracks.map((track) => (
                    <Dropdown.Item key={track.id} id={`audio-${track.id}`} textValue={track.label}>
                      {track.label}
                    </Dropdown.Item>
                  ))}
                </Dropdown.Menu>
              </Dropdown.Popover>
            </Dropdown>
          ) : null}
          <Dropdown>
            <Dropdown.Trigger aria-label="Playback settings" className="nav-focus grid size-10 place-items-center rounded-full hover:bg-white/10" data-focusable>
              <IconSettings size={22} />
            </Dropdown.Trigger>
            <Dropdown.Popover placement="top end" className="glass-surface">
              <Dropdown.Menu aria-label="Playback speed" selectionMode="single" selectedKeys={[`rate-${rate}`]} onAction={onSettingsAction}>
                <Dropdown.Section>
                  {RATES.map((value) => (
                    <Dropdown.Item key={value} id={`rate-${value}`} textValue={`${value}x`}>
                      {value === 1 ? "Normal speed" : `${value}×`}
                    </Dropdown.Item>
                  ))}
                </Dropdown.Section>
              </Dropdown.Menu>
            </Dropdown.Popover>
          </Dropdown>
          {typeof document !== "undefined" && document.pictureInPictureEnabled ? (
            <button type="button" data-focusable aria-label="Picture in picture" onClick={togglePip} className="nav-focus hidden size-10 place-items-center rounded-full hover:bg-white/10 sm:grid">
              <IconPictureInPicture size={22} />
            </button>
          ) : null}
          <button type="button" data-focusable aria-label={fullscreen ? "Exit fullscreen" : "Fullscreen"} onClick={toggleFullscreen} className="nav-focus grid size-10 place-items-center rounded-full hover:bg-white/10">
            {fullscreen ? <IconMinimize size={22} /> : <IconMaximize size={22} />}
          </button>
        </div>
      </div>
    </div>
  );
}
