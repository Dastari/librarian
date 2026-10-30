import { Spinner } from "@/components/ui";
import { Menu } from "glass-ux/menu";
import { Button } from "glass-ux/button";
import { Surface } from "glass-ux/surface";
import { IconArrowLeft, IconFullscreen, IconFullscreenExit, IconSettings } from "glass-ux/icons";
import {
  IconBadgeCc,
  IconLanguage,
  IconPictureInPicture,
  IconPlayerTrackNext,
  IconRewindBackward10,
  IconRewindForward10,
} from "@tabler/icons-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { EntityAudioStreamListDocument, EntitySubtitleListDocument, EntityMediaChapterListDocument } from "@/graphql/generated/graphql";
import { useSettingsConnection } from "@/features/settings/useSettingsConnection";
import { mediaSubtitleUrl } from "@/lib/api/urls";
import { formatClock } from "@/lib/format";
import { clamp, cn } from "@/lib/utils";

import { PlayPauseButton, SeekBar, VolumeControl } from "./controls";
import type { PlayItem } from "./store";
import { useMediaSource } from "./useMediaSource";
import { useProgressSync } from "./useProgressSync";
import { CastButton } from "./CastButton";
import type { Events } from "hls.js";

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
  const playIntent = useRef(true);
  const [selectedAudio, setSelectedAudio] = useState<number>();

  const progress = useProgressSync(item);
  const media = useMediaSource(progress.loading ? null : item.mediaFileId, videoRef, progress.resumePosition, { audioStreamIndex: selectedAudio });
  const audioRows = useSettingsConnection(EntityAudioStreamListDocument, { where: { mediaFileId: { eq: item.mediaFileId } }, orderBy: [{ streamIndex: "ASC" }] }, (data) => data.audioStreams);
  const subtitleRows = useSettingsConnection(EntitySubtitleListDocument, { where: { mediaFileId: { eq: item.mediaFileId } }, orderBy: [{ createdAt: "ASC" }] }, (data) => data.subtitles);
  const chapterRows = useSettingsConnection(EntityMediaChapterListDocument, { where: { mediaFileId: { eq: item.mediaFileId } }, orderBy: [{ chapterIndex: "ASC" }] }, (data) => data.mediaChapters);

  const [playing, setPlaying] = useState(false);
  const [position, setPosition] = useState(0);
  const [duration, setDuration] = useState(0);
  const [buffered, setBuffered] = useState(0);
  const [volume, setVolume] = useState(1);
  const [muted, setMuted] = useState(false);
  const [rate, setRate] = useState(1);
  const [waiting, setWaiting] = useState(true);
  const [controlsVisible, setControlsVisible] = useState(true);
  const [menuOpen, setMenuOpen] = useState(false);
  const showControls = controlsVisible || menuOpen;
  const [fullscreen, setFullscreen] = useState(false);
  const [textTrack, setTextTrack] = useState<string>("off");
  const [audioTrack, setAudioTrack] = useState<number>(-1);
  const [trackRevision, setTrackRevision] = useState(0);
  const [controlError, setControlError] = useState<string | null>(null);

  const fullDuration = media.duration && media.duration > duration ? media.duration : duration;
  const chapters = useMemo(() => chapterRows.rows.map((row) => ({ start: row.startSecs, title: row.title })), [chapterRows.rows]);
  const deliveredSubtitles = useMemo(() => subtitleRows.rows.filter((row) => ["subrip", "srt", "webvtt", "vtt", "ass", "ssa", "mov_text", "text"].includes(row.codec?.toLowerCase() ?? "")), [subtitleRows.rows]);
  const currentChapter = useMemo(() => [...chapters].reverse().find((chapter) => chapter.start <= position), [chapters, position]);

  const wake = useCallback(() => {
    setControlsVisible(true);
    if (hideTimer.current) clearTimeout(hideTimer.current);
    hideTimer.current = setTimeout(() => {
      const focused = document.activeElement;
      const controlFocused = focused !== videoRef.current && focused !== shellRef.current && shellRef.current?.contains(focused);
      if (videoRef.current && !videoRef.current.paused && !controlFocused) setControlsVisible(false);
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
      if (media.isTransitioning()) return;
      setPosition(video.currentTime);
      if (video.buffered.length) setBuffered(video.buffered.end(video.buffered.length - 1));
      progress.report(video.currentTime, fullDuration || video.duration, !video.paused);
    };
    const onDuration = () => setDuration(Number.isFinite(video.duration) ? video.duration : 0);
    const onPlay = () => {
      playIntent.current = true;
      setPlaying(true);
      progress.report(video.currentTime, fullDuration || video.duration, true);
      wake();
    };
    const onPause = () => {
      if (media.isTransitioning()) return;
      playIntent.current = false;
      setPlaying(false);
      setControlsVisible(true);
      progress.report(video.currentTime, fullDuration || video.duration, false);
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
      if (media.isTransitioning()) return;
      progress.report(video.currentTime, fullDuration || video.duration, false);
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
  }, [fullDuration, onEnded, progress, media.isTransitioning, wake]);

  // Autoplay once the source is attached.
  useEffect(() => {
    const video = videoRef.current;
    if (!video || !media.source || !playIntent.current) return;
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
    if (video.paused) {
      playIntent.current = true;
      void video.play().catch(() => setControlError("Playback was blocked. Press Play to try again."));
    }
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
    const video = videoRef.current as (HTMLVideoElement & { webkitEnterFullscreen?: () => void; webkitExitFullscreen?: () => void; webkitDisplayingFullscreen?: boolean }) | null;
    if (video?.webkitDisplayingFullscreen) video.webkitExitFullscreen?.();
    else if (document.fullscreenElement) void document.exitFullscreen().catch(() => setControlError("Could not exit fullscreen."));
    else if (shell.requestFullscreen) void shell.requestFullscreen().catch(() => setControlError("Fullscreen is unavailable. Try the video’s native controls."));
    else {
      // iOS Safari only supports fullscreen on the video element itself.
      video?.webkitEnterFullscreen?.();
    }
  }, []);

  const togglePip = useCallback(() => {
    const video = videoRef.current;
    if (!video || !document.pictureInPictureEnabled) return;
    if (document.pictureInPictureElement) void document.exitPictureInPicture().catch(() => setControlError("Could not close picture in picture."));
    else void video.requestPictureInPicture().catch(() => setControlError("Picture in picture is unavailable for this video."));
  }, []);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const target = event.target instanceof Element ? event.target : null;
      if (event.defaultPrevented || target?.closest('[role="menu"], [role="dialog"], [role="listbox"], [aria-haspopup="menu"], input, textarea, [role="slider"]')) return;
      // Buttons own activation keys; shortcut Space must not also toggle playback.
      if ((event.key === " " || event.key === "Enter") && target?.closest('button, a, [role="button"]')) return;
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
          if (!fullscreen && !document.fullscreenElement) onBack();
          break;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onBack, seekBy, toggleFullscreen, togglePlay, wake, fullscreen]);

  useEffect(() => {
    const video = videoRef.current;
    const hls = media.hls;
    const refresh = () => setTrackRevision((value) => value + 1);
    const begin = () => setFullscreen(true);
    const end = () => setFullscreen(false);
    const events = ["hlsManifestParsed", "hlsAudioTracksUpdated", "hlsSubtitleTracksUpdated", "hlsAudioTrackSwitched"] as Events[];
    events.forEach((event) => hls?.on?.(event, refresh));
    video?.textTracks.addEventListener?.("addtrack", refresh);
    video?.textTracks.addEventListener?.("removetrack", refresh);
    video?.addEventListener("webkitbeginfullscreen", begin);
    video?.addEventListener("webkitendfullscreen", end);
    return () => {
      events.forEach((event) => hls?.off?.(event, refresh));
      video?.textTracks.removeEventListener?.("addtrack", refresh);
      video?.textTracks.removeEventListener?.("removetrack", refresh);
      video?.removeEventListener("webkitbeginfullscreen", begin);
      video?.removeEventListener("webkitendfullscreen", end);
    };
  }, [media.hls]);

  // Subtitle and audio track lists come from hls.js or the native element.
  const textTracks = useMemo(() => {
    const video = videoRef.current;
    if (deliveredSubtitles.length) return deliveredSubtitles.map((row) => ({ id: `file-${row.id}`, label: row.title || row.language || "Subtitles" }));
    const fromHls = media.hls?.subtitleTracks.map((track, index) => ({ id: `hls-${index}`, label: track.name || track.lang || `Track ${index + 1}` })) ?? [];
    const fromNative = video ? Array.from(video.textTracks).map((track, index) => ({ id: `native-${index}`, label: track.label || track.language || `Track ${index + 1}` })) : [];
    return fromHls.length ? fromHls : fromNative;
  }, [media.hls, media.source, trackRevision, deliveredSubtitles]);

  const audioTracks = useMemo(() => audioRows.rows.length
    ? audioRows.rows.map((row) => ({ id: row.streamIndex, label: row.title || row.language || `Audio ${row.streamIndex}` }))
    : media.hls?.audioTracks.map((track, index) => ({ id: index, label: track.name || track.lang || `Audio ${index + 1}` })) ?? [], [audioRows.rows, media.hls, media.source, trackRevision]);

  const syncNativeSubtitle = useCallback(() => {
    const video = videoRef.current;
    if (!video) return;
    Array.from(video.querySelectorAll("track")).forEach((track) => {
      if (track.track) track.track.mode = textTrack === `file-${track.dataset.subtitleId}` ? "showing" : "disabled";
    });
  }, [textTrack]);
  useEffect(syncNativeSubtitle, [syncNativeSubtitle, deliveredSubtitles, media.source]);

  const applyTextTrack = (id: string) => {
    setTextTrack(id);
    const video = videoRef.current;
    if (id.startsWith("file-") || deliveredSubtitles.length) {
      if (media.hls) { media.hls.subtitleTrack = -1; media.hls.subtitleDisplay = false; }
      return;
    }
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
    if (audioRows.rows.length) {
      playIntent.current = videoRef.current ? !videoRef.current.paused : playing;
      setSelectedAudio(id);
    } else if (media.hls) media.hls.audioTrack = id;
  };


  return (
    <div
      ref={shellRef}
      className={cn("over-imagery relative h-svh w-full select-none overflow-hidden bg-black text-white", !showControls && playing && "cursor-none")}
      onPointerMove={wake}
      onPointerDown={wake}
      onTouchStart={wake}
      onFocusCapture={wake}
    >
      <video ref={videoRef} className="h-full w-full object-contain" playsInline preload="metadata" onClick={togglePlay} onDoubleClick={toggleFullscreen} crossOrigin="use-credentials">
        {deliveredSubtitles.map((row) => <track key={row.id} data-subtitle-id={row.id} kind="subtitles" label={row.title || row.language || "Subtitles"} srcLang={row.language || "en"} src={mediaSubtitleUrl(item.mediaFileId, row.id)} onLoad={syncNativeSubtitle} onError={() => setControlError("These subtitles could not be loaded. Try another subtitle track.")} />)}
      </video>

      {(waiting || progress.loading || media.loading) && !media.error ? (
        <div className="pointer-events-none absolute inset-0 grid place-items-center">
          <Spinner size={28} className="text-white" />
        </div>
      ) : null}

      {media.error ? (
        <div role="alert" className="absolute inset-0 grid place-items-center bg-black/70 p-6 text-center">
          <Surface material="panel" radius="dialog" padding="dialog" className="max-w-md">
            <p className="text-title-lg text-foreground">Playback stopped</p>
            <p className="mt-2 text-body-sm text-muted">{media.error}</p>
            <div className="mt-4 flex justify-center gap-2"><Button variant="secondary" onClick={onBack}>Go back</Button><Button variant="primary" onClick={() => { playIntent.current = true; media.retry(); }}>Retry playback</Button></div>
          </Surface>
        </div>
      ) : null}

      {/* Top bar */}
      <div className={cn("absolute inset-x-0 top-0 flex items-center gap-3 bg-gradient-to-b from-black/80 to-transparent px-4 pb-10 pt-[calc(var(--safe-top)+0.75rem)] transition-opacity duration-base", showControls ? "opacity-100" : "pointer-events-none opacity-0")}>
        <Button variant="secondary" data-spatial-start aria-label="Back" onClick={onBack} icon={<IconArrowLeft />} />
        <div className="min-w-0 flex-1">
          <p className="truncate text-title-md text-white text-shadow-hero">{item.title}</p>
          {item.subtitle || currentChapter?.title ? <p className="truncate text-label text-white/70">{[item.subtitle, currentChapter?.title].filter(Boolean).join(" · ")}</p> : null}
        </div>
        <CastButton item={item} position={position} onStarted={() => videoRef.current?.pause()} />
      </div>

      {/* Bottom controls */}
      <div className={cn("absolute inset-x-0 bottom-0 flex flex-col gap-2 bg-gradient-to-t from-black/85 via-black/40 to-transparent px-4 pb-[calc(var(--safe-bottom)+0.75rem)] pt-16 transition-opacity duration-base sm:px-6", showControls ? "opacity-100" : "pointer-events-none opacity-0")}>
        <SeekBar position={position} duration={fullDuration} buffered={buffered} onSeek={seekTo} chapters={chapters} />
        <p role="status" aria-live="polite" className="min-h-5 text-label-sm text-white/80">{controlError ?? (audioRows.error || subtitleRows.error || chapterRows.error ? "Some playback options could not be loaded." : "")}</p>
        <div className="flex flex-wrap items-center gap-1 sm:gap-2">
          <PlayPauseButton playing={playing} onToggle={togglePlay} size="md" className="bg-white text-black" />
          <Button variant="secondary" aria-label="Back 10 seconds" onClick={() => seekBy(-10)} icon={<IconRewindBackward10 />} />
          <Button variant="secondary" aria-label="Forward 10 seconds" onClick={() => seekBy(10)} icon={<IconRewindForward10 />} />
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
            <Button variant="secondary" aria-label={nextLabel ?? "Next episode"} onClick={onNext} icon={<IconPlayerTrackNext />}><span className="hidden sm:inline">{nextLabel ?? "Next"}</span></Button>
          ) : null}
          {textTracks.length > 0 ? (
            <Menu.Root onOpenChange={(open) => { setMenuOpen(open); wake(); }}>
              <Menu.Trigger render={<Button variant={textTrack !== "off" ? "soft" : "secondary"} aria-label="Subtitles" icon={<IconBadgeCc />} />} />
              <Menu.Popup side="top" align="end">
                <Menu.RadioGroup aria-label="Subtitles" value={textTrack} onValueChange={(value) => applyTextTrack(String(value))}>
                  <Menu.RadioItem closeOnClick value="off">
                    Off
                  </Menu.RadioItem>
                  {textTracks.map((track) => (
                    <Menu.RadioItem closeOnClick key={track.id} value={track.id}>
                      {track.label}
                    </Menu.RadioItem>
                  ))}
                </Menu.RadioGroup>
              </Menu.Popup>
            </Menu.Root>
          ) : null}
          {audioTracks.length > 1 ? (
            <Menu.Root onOpenChange={(open) => { setMenuOpen(open); wake(); }}>
              <Menu.Trigger render={<Button variant="secondary" aria-label="Audio track" icon={<IconLanguage />} />} />
              <Menu.Popup side="top" align="end">
                <Menu.RadioGroup aria-label="Audio track" value={audioRows.rows.length ? selectedAudio ?? audioRows.rows.find((row) => row.isDefault)?.streamIndex ?? audioRows.rows[0]?.streamIndex : audioTrack >= 0 ? audioTrack : media.hls?.audioTrack ?? -1} onValueChange={(value) => applyAudioTrack(Number(value))}>
                  {audioTracks.map((track) => (
                    <Menu.RadioItem closeOnClick key={track.id} value={track.id}>
                      {track.label}
                    </Menu.RadioItem>
                  ))}
                </Menu.RadioGroup>
              </Menu.Popup>
            </Menu.Root>
          ) : null}
          <Menu.Root onOpenChange={(open) => { setMenuOpen(open); wake(); }}>
            <Menu.Trigger render={<Button variant="secondary" aria-label="Playback settings" icon={<IconSettings />} />} />
            <Menu.Popup side="top" align="end">
              <Menu.RadioGroup aria-label="Playback speed" value={rate} onValueChange={(value) => { if (videoRef.current) videoRef.current.playbackRate = Number(value); }}>
                <Menu.Group>
                  {RATES.map((value) => (
                    <Menu.RadioItem closeOnClick key={value} value={value}>
                      {value === 1 ? "Normal speed" : `${value}×`}
                    </Menu.RadioItem>
                  ))}
                </Menu.Group>
              </Menu.RadioGroup>
            </Menu.Popup>
          </Menu.Root>
          {typeof document !== "undefined" && document.pictureInPictureEnabled ? (
            <Button variant="secondary" aria-label="Picture in picture" onClick={togglePip} className="hidden sm:inline-flex" icon={<IconPictureInPicture />} />
          ) : null}
          <Button variant="secondary" aria-label={fullscreen ? "Exit fullscreen" : "Fullscreen"} onClick={toggleFullscreen} icon={fullscreen ? <IconFullscreenExit /> : <IconFullscreen />} />
        </div>
      </div>
    </div>
  );
}
