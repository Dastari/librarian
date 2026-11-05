import { Button } from "glass-ux/button";
import { IconArrowLeft, IconSkipForward, IconVolume } from "glass-ux/icons";
import { VideoPlayer as GlassVideoPlayer, type MediaAttach, type MediaPlayerController, type MediaStreamContext, type MediaTextTrack } from "glass-ux/media-player";
import { Menu } from "glass-ux/menu";
import { useCallback, useEffect, useMemo, useRef, useState, type KeyboardEvent as ReactKeyboardEvent } from "react";

import { useSettingsConnection } from "@/features/settings/useSettingsConnection";
import { EntityAudioStreamListDocument, EntityMediaChapterListDocument, EntitySubtitleListDocument } from "@/graphql/generated/graphql";
import { mediaSubtitleUrl } from "@/lib/api/urls";
import { useInputMode } from "@/lib/input-mode";
import { cn } from "@/lib/utils";

import { CastButton } from "./CastButton";
import type { PlayItem } from "./store";
import { useCastAdapter } from "./useCastAdapter";
import { useMediaSource } from "./useMediaSource";
import { useProgressSync } from "./useProgressSync";

const VIEW_MODES = ["fullscreen", "pip"] as const;
const CHROME_DELAY_MS = 3000;
/** webOS (461) and Tizen (10009) report their remote's Back key only as a key code. */
const BACK_KEY_CODES = new Set([461, 10009]);
const SUBTITLE_CODECS = new Set(["subrip", "srt", "webvtt", "vtt", "ass", "ssa", "mov_text", "text"]);

/** The subtitles the server can deliver as WebVTT; image formats (PGS, VobSub) it cannot. */
export function subtitleTracks(mediaFileId: string, rows: ReadonlyArray<{ id: string; codec?: string | null; title?: string | null; language?: string | null }>): MediaTextTrack[] {
  return rows
    .filter((row) => SUBTITLE_CODECS.has(row.codec?.toLowerCase() ?? ""))
    .map((row) => ({ id: row.id, src: mediaSubtitleUrl(mediaFileId, row.id), label: row.title || row.language || "Subtitles", language: row.language || "en" }));
}

interface VideoPlayerProps {
  item: PlayItem;
  onBack: () => void;
  /** Called when the file finishes; the caller decides whether to autoplay the next episode. */
  onEnded?: () => void;
  nextLabel?: string;
  onNext?: () => void;
}

/**
 * The immersive video route: Glass UX's player owns the controls, captions, view modes and
 * shortcuts. Librarian supplies what it alone knows: the stream (`useMediaSource`, through
 * `attach`), watch progress, the file's tracks and chapters, and the server's cast sessions.
 */
export function VideoPlayer({ item, onBack, onEnded, nextLabel, onNext }: VideoPlayerProps) {
  const controller = useRef<MediaPlayerController>(null);
  const element = useRef<HTMLMediaElement | null>(null);
  const stream = useRef<MediaStreamContext | null>(null);
  const attached = useRef(false);
  const playIntent = useRef(true);
  const [video, setVideo] = useState<HTMLMediaElement | null>(null);
  const [selectedAudio, setSelectedAudio] = useState<number>();
  const [paused, setPaused] = useState(false);
  const [chromeAwake, setChromeAwake] = useState(true);
  const [castPosition, setCastPosition] = useState<number>();

  const progress = useProgressSync(item);
  const media = useMediaSource(progress.loading ? null : item.mediaFileId, element, progress.resumePosition, { audioStreamIndex: selectedAudio });
  const cast = useCastAdapter(item.mediaFileId);
  const audioRows = useSettingsConnection(EntityAudioStreamListDocument, { where: { mediaFileId: { eq: item.mediaFileId } }, orderBy: [{ streamIndex: "ASC" }] }, (data) => data.audioStreams);
  const subtitleRows = useSettingsConnection(EntitySubtitleListDocument, { where: { mediaFileId: { eq: item.mediaFileId } }, orderBy: [{ createdAt: "ASC" }] }, (data) => data.subtitles);
  const chapterRows = useSettingsConnection(EntityMediaChapterListDocument, { where: { mediaFileId: { eq: item.mediaFileId } }, orderBy: [{ chapterIndex: "ASC" }] }, (data) => data.mediaChapters);

  const chapters = useMemo(() => chapterRows.rows.map((row) => ({ start: row.startSecs, title: row.title ?? `Chapter ${row.chapterIndex + 1}` })), [chapterRows.rows]);
  const tracks = useMemo(() => subtitleTracks(item.mediaFileId, subtitleRows.rows), [item.mediaFileId, subtitleRows.rows]);
  const currentAudio = selectedAudio ?? audioRows.rows.find((row) => row.isDefault)?.streamIndex ?? audioRows.rows[0]?.streamIndex;

  // The player hands over its element here; a second call is its Retry button.
  const attach = useCallback<MediaAttach>((node, context) => {
    element.current = node;
    stream.current = context;
    setVideo(node);
    if (attached.current) {
      playIntent.current = true;
      media.retry();
    }
    attached.current = true;
    // Shortcuts work while focus is inside the player, without holding the controls open.
    node.tabIndex = -1;
    node.focus({ preventScroll: true });
    // `media.retry` is stable; the source hook owns everything else.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => { if (media.error) stream.current?.fail(); }, [media.error]);
  // A growing playlist under-reports its length; the server knows the file's real one.
  useEffect(() => { if (media.duration) stream.current?.setDuration(media.duration); }, [media.duration, media.source]);

  // Progress follows the element, and ignores the events a source change makes.
  const fullDuration = useCallback(() => media.duration && video && media.duration > video.duration ? media.duration : video?.duration ?? 0, [media.duration, video]);
  useEffect(() => {
    if (!video) return;
    const onTime = () => { if (!media.isTransitioning()) progress.report(video.currentTime, fullDuration(), !video.paused); };
    const onPlay = () => {
      playIntent.current = true;
      setPaused(false);
      progress.report(video.currentTime, fullDuration(), true);
    };
    const onPause = () => {
      if (media.isTransitioning()) return;
      playIntent.current = false;
      setPaused(true);
      progress.report(video.currentTime, fullDuration(), false);
      void progress.flush(true);
    };
    const onEnd = () => {
      if (media.isTransitioning()) return;
      progress.report(video.currentTime, fullDuration(), false);
      void progress.flush(true);
      onEnded?.();
    };
    video.addEventListener("timeupdate", onTime);
    video.addEventListener("play", onPlay);
    video.addEventListener("pause", onPause);
    video.addEventListener("ended", onEnd);
    return () => {
      video.removeEventListener("timeupdate", onTime);
      video.removeEventListener("play", onPlay);
      video.removeEventListener("pause", onPause);
      video.removeEventListener("ended", onEnd);
    };
  }, [fullDuration, media.isTransitioning, onEnded, progress, video]);

  // Start once the source is attached; a blocked autoplay leaves the player's Play button.
  useEffect(() => {
    if (video && media.source && playIntent.current) void video.play().catch(() => undefined);
  }, [media.source, video]);

  // The title and Back fade with the player's controls.
  const hideTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const wake = useCallback(() => {
    setChromeAwake(true);
    clearTimeout(hideTimer.current);
    hideTimer.current = setTimeout(() => setChromeAwake(false), CHROME_DELAY_MS);
  }, []);
  useEffect(() => {
    wake();
    return () => clearTimeout(hideTimer.current);
  }, [wake]);

  // What the player does not cover: leaving, media keys, and keys when focus has fallen out of it.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      wake();
      const target = event.target instanceof Element ? event.target : null;
      if (event.defaultPrevented || target?.closest('[role="menu"], [role="dialog"], [role="listbox"], input, textarea')) return;
      const player = controller.current;
      if (event.key === "MediaPlayPause") player?.toggle();
      else if (event.key === "MediaRewind") player?.seekBy(-10);
      else if (event.key === "MediaFastForward") player?.seekBy(10);
      else if ((event.key === "Escape" || event.key === "Backspace" || event.key === "GoBack" || BACK_KEY_CODES.has(event.keyCode)) && !document.fullscreenElement) onBack();
      else if (document.activeElement === document.body && player) {
        if (event.key === " " || event.key === "k") { event.preventDefault(); player.toggle(); }
        else if (event.key === "ArrowLeft") { event.preventDefault(); player.seekBy(-5); }
        else if (event.key === "ArrowRight") { event.preventDefault(); player.seekBy(5); }
        else return;
        element.current?.focus({ preventScroll: true });
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onBack, wake]);

  // The receiver starts from where this player is when the picker opens.
  useEffect(() => { if (cast.pickerOpen) setCastPosition(controller.current?.getTime().currentTime); }, [cast.pickerOpen]);

  // A remote has no Tab: Down reaches the controls, Left and Right walk them, Up returns to the
  // picture, where Left and Right seek. Menus and the seek bar's own keys stay theirs.
  const tv = useInputMode().mode === "tv";
  const remoteKeys = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    if (!tv || event.defaultPrevented) return;
    const target = event.target instanceof HTMLElement ? event.target : null;
    if (!target || target.closest('[role="menu"], [role="dialog"], [role="listbox"]')) return;
    const controls = Array.from(event.currentTarget.querySelectorAll<HTMLButtonElement>('[data-kind="video"] button')).filter((button) => button.offsetParent !== null && button.tabIndex >= 0 && !button.disabled);
    const move = (next: HTMLElement | undefined) => {
      if (!next) return;
      event.preventDefault();
      event.stopPropagation();
      next.focus();
      wake();
    };
    if (target === element.current) {
      if (event.key === "ArrowDown") move(controls[0]);
      return;
    }
    const index = controls.indexOf(target as HTMLButtonElement);
    if (index < 0) return;
    if (event.key === "ArrowUp") move(element.current ?? undefined);
    else if (event.key === "ArrowLeft") move(controls[index - 1]);
    else if (event.key === "ArrowRight") move(controls[index + 1]);
  };

  const showChrome = chromeAwake || paused || Boolean(media.error);
  const failed = audioRows.error || subtitleRows.error || chapterRows.error;

  return (
    <div className="relative h-svh w-full overflow-clip bg-black text-white" onPointerMove={wake} onPointerDown={wake} onKeyDownCapture={remoteKeys}>
      <GlassVideoPlayer
        fill
        className="h-full"
        playerRef={controller}
        attach={attach}
        title={item.title}
        subtitle={item.subtitle}
        poster={item.artwork}
        tracks={tracks}
        chapters={chapters}
        crossOrigin="use-credentials"
        viewModes={VIEW_MODES}
        castAdapter={cast.adapter}
        loading={progress.loading}
        controlsEnd={
          <>
            {onNext ? <Button variant="ghost" size="sm" shape="circle" icon={<IconSkipForward />} aria-label={nextLabel ?? "Next episode"} tooltip={nextLabel ?? "Next episode"} onClick={onNext} /> : null}
            {audioRows.rows.length > 1 ? (
              <Menu.Root>
                <Menu.Trigger render={<Button variant="ghost" size="sm" shape="circle" icon={<IconVolume />} aria-label="Audio track" tooltip="Audio track" />} />
                <Menu.Popup side="top" align="end">
                  <Menu.RadioGroup aria-label="Audio track" value={currentAudio} onValueChange={(value) => {
                    playIntent.current = element.current ? !element.current.paused : true;
                    setSelectedAudio(Number(value));
                  }}>
                    {audioRows.rows.map((row) => <Menu.RadioItem closeOnClick key={row.streamIndex} value={row.streamIndex}>{row.title || row.language || `Audio ${row.streamIndex}`}</Menu.RadioItem>)}
                  </Menu.RadioGroup>
                </Menu.Popup>
              </Menu.Root>
            ) : null}
          </>
        }
      />
      <div className={cn("over-imagery pointer-events-none absolute inset-x-0 top-0 z-10 flex items-center gap-3 bg-linear-to-b from-black/70 to-transparent px-4 pb-12 pt-[calc(var(--safe-top)+0.75rem)] transition-opacity duration-base", !showChrome && "opacity-0")}>
        <Button variant="secondary" className={showChrome ? "pointer-events-auto" : undefined} tabIndex={showChrome ? undefined : -1} data-spatial-start aria-label="Back" onClick={onBack} icon={<IconArrowLeft className="rtl:rotate-180" />} />
        <div className="min-w-0 flex-1">
          <p className="truncate text-title-md text-white text-shadow-hero">{item.title}</p>
          {item.subtitle ? <p className="truncate text-label text-white/70">{item.subtitle}</p> : null}
          {media.error || failed ? <p role="status" className="truncate text-label-sm text-white/80">{media.error ?? "Some playback options could not be loaded."}</p> : null}
        </div>
      </div>
      <CastButton item={item} position={castPosition} open={cast.pickerOpen} onOpenChange={cast.setPickerOpen} hideTrigger onStarted={cast.refresh} />
    </div>
  );
}
