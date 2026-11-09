import { Button } from "glass-ux/button";
import { IconArrowLeft, IconCaptions, IconSkipForward, IconVolume } from "glass-ux/icons";
import { VideoPlayer as GlassVideoPlayer, type MediaAttach, type MediaPlayerController, type MediaStreamContext, type MediaViewMode, type MediaViewModeChangeDetails } from "glass-ux/media-player";
import { Menu } from "glass-ux/menu";
import { IconAspectRatio } from "@tabler/icons-react";
import { toast } from "glass-ux/toast";
import { useCallback, useEffect, useMemo, useRef, useState, type KeyboardEvent as ReactKeyboardEvent } from "react";

import { useSettingsConnection } from "@/features/settings/useSettingsConnection";
import { EntityAudioStreamListDocument, EntityMediaChapterListDocument, EntitySubtitleListDocument, MediaFilePreviewsDocument } from "@/graphql/generated/graphql";
import { formatClock } from "@/lib/format";
import { useLiveQuery } from "@/lib/graphql/live/hooks";
import { bindTvVolume, holdScreenOn } from "@/lib/tv-app";
import { useInputMode } from "@/lib/input-mode";
import { backHandled, isBackKey } from "@/lib/remote-back";
import { usePref } from "@/lib/prefs";
import { cn } from "@/lib/utils";

import { CastButton } from "./CastButton";
import { FindSubtitlesDialog } from "./FindSubtitlesDialog";
import { OFFSET_STEP_MS, choiceOf, clampOffset, formatOffset, isBitmapSubtitle, pickSubtitle, readShowChoice, subtitleLabel, subtitleTracks, writeShowChoice } from "./subtitles";
import { usePlayerState, type PlayItem } from "./store";
import { useCastAdapter } from "./useCastAdapter";
import { useMediaSource } from "./useMediaSource";
import { playbackDetails, useProgressSync } from "./useProgressSync";
import { videoKeyTarget, type PlayerKeyTarget } from "./mediaKeys";
import { formatScrubDelta } from "./scrub";
import { usePictureFill } from "./usePictureFill";
import { useScrubber } from "./useScrubber";
import { PICTURE_MODE_PREF, storyboardFrame, storyboardProps, type PictureMode } from "./videoPreviews";
import { useTvVolume } from "./useTvVolume";
import { readMiniRect, saveMiniRect, saveVideoSession } from "./videoSession";

const VIEW_MODES: readonly MediaViewMode[] = ["inline", "windowed", "fullscreen", "pip"];
/** A remote has nothing to drag a window with. */
const TV_VIEW_MODES: readonly MediaViewMode[] = ["inline", "fullscreen"];
const SESSION_SAVE_MS = 5000;
const CHROME_DELAY_MS = 3000;
/** webOS (461) and Tizen (10009) report their remote's Back key only as a key code. */
export { subtitleTracks } from "./subtitles";

interface VideoPlayerProps {
  item: PlayItem;
  onBack: () => void;
  /** Called when the file finishes; the caller decides whether to autoplay the next episode. */
  onEnded?: () => void;
  nextLabel?: string;
  onNext?: () => void;
  /** In the floating window, over the app, rather than filling the screen on /watch. */
  mini?: boolean;
  /** Whose session to keep for the mini player to resume after a refresh. */
  userId?: string;
  onMinimise?: () => void;
  onExpand?: () => void;
  onClose?: () => void;
}

/**
 * The immersive video route: Glass UX's player owns the controls, captions, view modes and
 * shortcuts. Librarian supplies what it alone knows: the stream (`useMediaSource`, through
 * `attach`), watch progress, the file's tracks and chapters, and the server's cast sessions.
 */
export function VideoPlayer({ item, onBack, onEnded, nextLabel, onNext, mini = false, userId, onMinimise, onExpand, onClose }: VideoPlayerProps) {
  const controller = useRef<MediaPlayerController>(null);
  const element = useRef<HTMLMediaElement | null>(null);
  const stream = useRef<MediaStreamContext | null>(null);
  const attached = useRef(false);
  const playIntent = useRef(!item.startPaused);
  const [viewMode, setViewMode] = useState<MediaViewMode>(mini ? "windowed" : "inline");
  const miniRef = useRef(mini);
  const tvRef = useRef(false);
  miniRef.current = mini;
  const audioPlaying = usePlayerState().transport.playing;
  const [video, setVideo] = useState<HTMLMediaElement | null>(null);
  const [selectedAudio, setSelectedAudio] = useState<number>();
  const [paused, setPaused] = useState(false);
  const [chromeAwake, setChromeAwake] = useState(true);
  const [castPosition, setCastPosition] = useState<number>();
  // Subtitles: a picture one burned in (a new stream), the timing offset, and the search dialog.
  const [burned, setBurned] = useState<string>();
  const [offsetMs, setOffsetMs] = useState(0);
  const [findOpen, setFindOpen] = useState(false);
  useEffect(() => { setBurned(undefined); setOffsetMs(0); }, [item.mediaFileId]);

  const progress = useProgressSync(item, undefined, {
    onStopRequested: () => {
      element.current?.pause();
      toast.info("Playback was stopped from another device");
    },
  });
  const media = useMediaSource(progress.loading ? null : item.mediaFileId, element, progress.resumePosition, { audioStreamIndex: selectedAudio, burnSubtitleId: burned });
  const { describe } = progress;
  useEffect(() => { describe(playbackDetails(media.source)); }, [describe, media.source]);
  const cast = useCastAdapter(item.mediaFileId);
  const audioRows = useSettingsConnection(EntityAudioStreamListDocument, { where: { mediaFileId: { eq: item.mediaFileId } }, orderBy: [{ streamIndex: "ASC" }] }, (data) => data.audioStreams);
  const subtitleRows = useSettingsConnection(EntitySubtitleListDocument, { where: { mediaFileId: { eq: item.mediaFileId } }, orderBy: [{ createdAt: "ASC" }] }, (data) => data.subtitles);
  const chapterRows = useSettingsConnection(EntityMediaChapterListDocument, { where: { mediaFileId: { eq: item.mediaFileId } }, orderBy: [{ chapterIndex: "ASC" }] }, (data) => data.mediaChapters);

  // Seek-bar previews and the picture inside any black bars (design.md Q122/Q123).
  const previews = useLiveQuery(MediaFilePreviewsDocument, { variables: { id: item.mediaFileId } });
  const previewFile = (previews.data ?? previews.previousData)?.mediaFile;
  const previewsForFile = previewFile?.id === item.mediaFileId ? previewFile : null;
  const seekPreview = useMemo(() => storyboardProps(item.mediaFileId, previewsForFile?.storyboard), [item.mediaFileId, previewsForFile?.storyboard]);
  const pictureCrop = previewsForFile?.pictureCrop ?? null;
  // Fit or fill is remembered on this device, for every file.
  const [pictureMode, setPictureMode] = usePref<PictureMode>(PICTURE_MODE_PREF, "fit");
  const fillZoom = usePictureFill(video, pictureMode === "fill" ? pictureCrop : null);

  const chapters = useMemo(() => chapterRows.rows.map((row) => ({ start: row.startSecs, title: row.title ?? `Chapter ${row.chapterIndex + 1}` })), [chapterRows.rows]);
  const tracks = useMemo(() => subtitleTracks(item.mediaFileId, subtitleRows.rows, offsetMs), [item.mediaFileId, subtitleRows.rows, offsetMs]);
  const bitmapRows = useMemo(() => subtitleRows.rows.filter(isBitmapSubtitle), [subtitleRows.rows]);
  const currentAudio = selectedAudio ?? audioRows.rows.find((row) => row.isDefault)?.streamIndex ?? audioRows.rows[0]?.streamIndex;

  // The caption showing, as Glass's own captions menu sets it.
  const [caption, setCaption] = useState<string | null>(null);
  useEffect(() => {
    const player = controller.current;
    if (!player) return;
    const read = () => setCaption(player.getState().caption);
    read();
    return player.subscribe(read);
  }, [video]);
  // A show remembers its subtitles (Q120): applied once a file's tracks are known, saved when the
  // person changes them. Per person, on this device.
  const showId = item.showId;
  const applied = useRef<string | null>(null);
  const pendingCaption = useRef<string | null>(null);
  // What this player itself last selected, so only a person's change is remembered.
  const selectedByPlayer = useRef<string | null>(null);
  useEffect(() => {
    const player = controller.current;
    if (!player || !video || applied.current === item.mediaFileId || subtitleRows.loading) return;
    applied.current = item.mediaFileId;
    const row = pickSubtitle(subtitleRows.rows, readShowChoice(userId, showId));
    selectedByPlayer.current = row?.id ?? null;
    if (!row) return;
    if (isBitmapSubtitle(row)) setBurned(row.id);
    else player.setCaption(row.id);
  }, [item.mediaFileId, showId, subtitleRows.loading, subtitleRows.rows, userId, video]);
  useEffect(() => {
    if (applied.current !== item.mediaFileId) return;
    const chosen = burned ?? caption;
    // Settling on what the player chose, or a track id left from the previous file, says nothing.
    if (chosen === selectedByPlayer.current) return;
    const row = subtitleRows.rows.find((candidate) => candidate.id === chosen);
    if (chosen && !row) return;
    selectedByPlayer.current = chosen;
    writeShowChoice(userId, showId, choiceOf(row, Boolean(burned)));
  }, [burned, caption, item.mediaFileId, showId, subtitleRows.rows, userId]);
  // A new track (just downloaded) or a moved offset reloads the tracks; keep showing the one chosen.
  useEffect(() => {
    const player = controller.current;
    const wanted = pendingCaption.current ?? caption;
    if (!player || !wanted || !tracks.some((track) => track.id === wanted)) return;
    pendingCaption.current = null;
    if (player.getState().caption !== wanted) player.setCaption(wanted);
  }, [tracks, caption]);
  const changeBurn = (id: string | undefined) => {
    playIntent.current = element.current ? !element.current.paused : true;
    if (id) controller.current?.setCaption(null);
    setBurned(id);
  };
  const openFind = () => {
    // A dialog cannot show over a full-screen element.
    if (document.fullscreenElement) void document.exitFullscreen().catch(() => undefined);
    setFindOpen(true);
  };

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
    if (!miniRef.current) node.focus({ preventScroll: true });
    // `media.retry` is stable; the source hook owns everything else.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => { if (media.error) stream.current?.fail(); }, [media.error]);

  // The route decides the size: leaving /watch shrinks it, coming back grows it.
  useEffect(() => {
    setViewMode((current) => mini ? "windowed" : current === "windowed" ? "inline" : current);
  }, [mini]);
  const changeViewMode = useCallback((mode: MediaViewMode, details: MediaViewModeChangeDetails) => {
    if (mode !== "windowed" && details.previous === "windowed" && miniRef.current) {
      // Glass's close pauses and leaves; its other button brings the player back.
      if (details.reason === "close") onClose?.();
      else onExpand?.();
      return;
    }
    setViewMode(mode);
    if (mode === "windowed" && !miniRef.current) onMinimise?.();
  }, [onClose, onExpand, onMinimise]);
  // Two things shouldn't talk at once: starting an audiobook pauses the small video.
  useEffect(() => { if (mini && audioPlaying) element.current?.pause(); }, [audioPlaying, mini]);

  // What a refresh needs to pick the mini player back up.
  const savedAt = useRef(0);
  const saveSession = useCallback(() => {
    const node = element.current;
    if (!miniRef.current || !userId || !node || media.isTransitioning()) return;
    savedAt.current = Date.now();
    const { startPosition: _position, startPaused: _paused, ...rest } = item;
    saveVideoSession(userId, { item: rest, position: node.currentTime, playing: !node.paused });
    // `media.isTransitioning` is stable.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [item, userId]);
  useEffect(() => {
    if (!mini) return;
    saveSession();
    window.addEventListener("pagehide", saveSession);
    return () => window.removeEventListener("pagehide", saveSession);
  }, [mini, saveSession]);
  // A growing playlist under-reports its length; the server knows the file's real one.
  useEffect(() => { if (media.duration) stream.current?.setDuration(media.duration); }, [media.duration, media.source]);

  // Progress follows the element, and ignores the events a source change makes.
  const fullDuration = useCallback(() => media.duration && video && media.duration > video.duration ? media.duration : video?.duration ?? 0, [media.duration, video]);

  // Remote scrubbing (design.md Q125): Left/Right on the picture or the seek bar, and the media
  // keys' Rewind/Fast-forward, move a target with a preview; the player seeks once, on OK or a
  // short pause, and Back leaves playback where it was.
  const { scrub, scrubber } = useScrubber({
    position: () => controller.current?.getTime().currentTime ?? 0,
    duration: () => fullDuration() || controller.current?.getTime().duration || 0,
    commit: (target) => controller.current?.seek(target),
  });
  const scrubFrame = scrub && previewsForFile?.storyboard ? storyboardFrame(item.mediaFileId, previewsForFile.storyboard, scrub.target) : null;
  const scrubChapter = scrub ? chapters.filter((chapter) => chapter.start <= scrub.target).at(-1)?.title : undefined;
  // The media keys reach this player wherever focus is (`mediaKeys.ts`).
  const nextRef = useRef(onNext);
  nextRef.current = onNext;
  useEffect(() => {
    const target: PlayerKeyTarget = {
      toggle: () => controller.current?.toggle(),
      play: () => controller.current?.play(),
      pause: () => controller.current?.pause(),
      stop: () => controller.current?.pause(),
      next: () => nextRef.current?.(),
      previous: () => controller.current?.seek(0),
      scrub: (direction, repeat) => { scrubber.step(direction, repeat); },
    };
    videoKeyTarget.current = target;
    return () => { if (videoKeyTarget.current === target) videoKeyTarget.current = null; };
  }, [scrubber]);
  useEffect(() => {
    if (!video) return;
    const onTime = () => {
      if (media.isTransitioning()) return;
      progress.report(video.currentTime, fullDuration(), !video.paused);
      if (Date.now() - savedAt.current > SESSION_SAVE_MS) saveSession();
    };
    const onPlay = () => {
      playIntent.current = true;
      setPaused(false);
      progress.report(video.currentTime, fullDuration(), true);
      saveSession();
    };
    const onPause = () => {
      if (media.isTransitioning()) return;
      playIntent.current = false;
      setPaused(true);
      progress.report(video.currentTime, fullDuration(), false);
      void progress.flush(true);
      saveSession();
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
  }, [fullDuration, media.isTransitioning, onEnded, progress, saveSession, video]);

  // Start once the source is attached; a blocked autoplay leaves the player's Play button.
  useEffect(() => {
    if (!video || !media.source || !playIntent.current) return;
    void video.play().catch((error: unknown) => {
      // After a refresh a browser may refuse sound before you've touched the page. A mini player
      // that was playing carries on silently; its volume control brings the sound back.
      if (miniRef.current && error instanceof DOMException && error.name === "NotAllowedError") {
        video.muted = true;
        void video.play().catch(() => undefined);
      }
    });
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
      // In the floating window the keyboard belongs to the page around it.
      if (miniRef.current) return;
      wake();
      const target = event.target instanceof Element ? event.target : null;
      // Backspace is Back only on a TV. A tooltip closing on Escape doesn't count as taking it.
      const back = isBackKey(event, { tv: tvRef.current });
      if ((back ? backHandled(event) : event.defaultPrevented) || target?.closest('[role="menu"], [role="dialog"], [role="listbox"], input, textarea')) return;
      const player = controller.current;
      // Media keys are handled once, for the whole app (`mediaKeys.ts`).
      if (back && !document.fullscreenElement) {
        // Taken here, so the TV app does not also step back through the WebView's history.
        event.preventDefault();
        onBack();
      } else if (document.activeElement === document.body && player) {
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

  // In the TV app the player's volume and mute are the TV's own (Q137); the element stays at full.
  const tvVolume = useTvVolume();
  const tvVolumeBound = tvVolume !== null;
  useEffect(() => (video && tvVolumeBound ? bindTvVolume(video) : undefined), [video, tvVolumeBound]);

  // The TV app keeps the screen on while the picture plays.
  const screenToken = useRef(Symbol("video"));
  useEffect(() => {
    const token = screenToken.current;
    holdScreenOn(token, Boolean(video) && !paused);
    return () => holdScreenOn(token, false);
  }, [video, paused]);

  // The receiver starts from where this player is when the picker opens.
  useEffect(() => { if (cast.pickerOpen) setCastPosition(controller.current?.getTime().currentTime); }, [cast.pickerOpen]);

  // A remote has no Tab: Down reaches the controls, Left and Right walk them, Up returns to the
  // picture, where Left and Right seek. Menus and the seek bar's own keys stay theirs.
  const tv = useInputMode().mode === "tv";
  tvRef.current = tv;
  const showTitle = !mini;
  const remoteKeys = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    if (!tv || (isBackKey(event, { tv }) ? backHandled(event) : event.defaultPrevented)) return;
    const target = event.target instanceof HTMLElement ? event.target : null;
    if (!target || target.closest('[role="menu"], [role="dialog"], [role="listbox"]')) return;
    const controls = Array.from(event.currentTarget.querySelectorAll<HTMLButtonElement>('[data-kind="video"] button')).filter((button) => button.offsetParent !== null && button.tabIndex >= 0 && !button.disabled);
    const consume = () => {
      event.preventDefault();
      event.stopPropagation();
    };
    const move = (next: HTMLElement | undefined) => {
      if (!next) return;
      if (scrubber.active) scrubber.commit();
      consume();
      next.focus();
      wake();
    };
    // While scrubbing, OK seeks and Back puts it back; neither reaches Glass or the page.
    if (scrubber.active && event.key === "Enter") { consume(); scrubber.commit(); return; }
    if (scrubber.active && isBackKey(event, { tv })) { consume(); scrubber.cancel(); return; }
    const onPicture = target === element.current;
    const onSeekBar = target.classList.contains("fa-media-seek-input");
    const controlsHidden = event.currentTarget.querySelector(".fa-media-stage")?.getAttribute("data-controls") === "hidden";
    if ((event.key === "ArrowLeft" || event.key === "ArrowRight") && (onPicture || onSeekBar || controlsHidden) && !event.altKey && !event.ctrlKey && !event.metaKey) {
      // Glass's own arrows would seek on every press; scrubbing seeks once.
      if (scrubber.step(event.key === "ArrowRight" ? 1 : -1, event.repeat)) consume();
      return;
    }
    if (onSeekBar) {
      // Up and Down leave the seek bar instead of nudging it.
      if (event.key === "ArrowUp") move(element.current ?? undefined);
      else if (event.key === "ArrowDown") move(controls[0]);
      return;
    }
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
    // Full screen, the picture and its controls keep clear of a landscape iPhone's notch.
    <div className={cn("relative h-full w-full overflow-clip bg-black text-white", !mini && "ps-(--safe-inline-start) pe-(--safe-inline-end)")} data-tv-volume={tvVolume ? (tvVolume.fixed ? "fixed" : "tv") : undefined} onPointerMove={wake} onPointerDown={wake} onKeyDownCapture={remoteKeys}>
      <GlassVideoPlayer
        fill
        className="h-full"
        playerRef={controller}
        attach={attach}
        title={item.title}
        subtitle={item.subtitle}
        poster={item.backdrop ?? item.artwork}
        tracks={tracks}
        chapters={chapters}
        crossOrigin="use-credentials"
        viewModes={tv ? TV_VIEW_MODES : VIEW_MODES}
        viewMode={viewMode}
        onViewModeChange={changeViewMode}
        defaultWindowRect={readMiniRect()}
        onWindowRectChange={saveMiniRect}
        castAdapter={cast.adapter}
        loading={progress.loading}
        storyboard={seekPreview.storyboard}
        thumbnail={seekPreview.thumbnail}
        controlsEnd={
          <>
            {pictureCrop ? (
              <Menu.Root>
                <Menu.Trigger render={<Button variant={pictureMode === "fill" ? "secondary" : "ghost"} size="sm" shape="circle" icon={<IconAspectRatio />} aria-label="Picture size" tooltip={fillZoom > 1 ? "Picture size: filling the screen" : "Picture size"} />} />
                <Menu.Popup side="top" align="end">
                  <Menu.RadioGroup aria-label="Picture size" value={pictureMode} onValueChange={(value) => setPictureMode(value === "fill" ? "fill" : "fit")}>
                    <Menu.RadioItem closeOnClick value="fit">Fit the whole frame</Menu.RadioItem>
                    <Menu.RadioItem closeOnClick value="fill">Fill the screen (zoom past black bars)</Menu.RadioItem>
                  </Menu.RadioGroup>
                </Menu.Popup>
              </Menu.Root>
            ) : null}
            {tvVolume?.fixed ? <span className="tv-volume-note text-label-sm text-white/70">Volume is controlled by your TV or sound system</span> : null}
            {onNext ? <Button variant="ghost" size="sm" shape="circle" icon={<IconSkipForward />} aria-label={nextLabel ?? "Next episode"} tooltip={nextLabel ?? "Next episode"} onClick={onNext} /> : null}
            <Menu.Root>
              <Menu.Trigger render={<Button variant="ghost" size="sm" shape="circle" icon={<IconCaptions />} aria-label="Subtitle options" tooltip="Subtitle options" />} />
              <Menu.Popup side="top" align="end">
                {bitmapRows.length ? (
                  <Menu.Group>
                    <Menu.GroupLabel>Burned into the picture</Menu.GroupLabel>
                    <Menu.RadioGroup aria-label="Burned-in subtitles" value={burned ?? "none"} onValueChange={(value) => changeBurn(value === "none" ? undefined : String(value))}>
                      <Menu.RadioItem closeOnClick value="none">None</Menu.RadioItem>
                      {bitmapRows.map((row) => <Menu.RadioItem closeOnClick key={row.id} value={row.id}>{subtitleLabel(row)}</Menu.RadioItem>)}
                    </Menu.RadioGroup>
                  </Menu.Group>
                ) : null}
                {bitmapRows.length ? <Menu.Separator /> : null}
                <Menu.Group>
                  <Menu.GroupLabel>Timing</Menu.GroupLabel>
                  <Menu.Item closeOnClick={false} disabledReason={tracks.length ? null : "No text subtitles"} onClick={() => setOffsetMs((value) => clampOffset(value - OFFSET_STEP_MS))}>Earlier by ¼ s</Menu.Item>
                  <Menu.Item closeOnClick={false} disabledReason={tracks.length ? null : "No text subtitles"} onClick={() => setOffsetMs((value) => clampOffset(value + OFFSET_STEP_MS))}>Later by ¼ s</Menu.Item>
                  <Menu.Item closeOnClick={false} disabledReason={offsetMs ? null : "Already in sync"} trail={formatOffset(offsetMs)} onClick={() => setOffsetMs(0)}>Reset timing</Menu.Item>
                </Menu.Group>
                <Menu.Separator />
                <Menu.Item onClick={openFind}>Find subtitles…</Menu.Item>
              </Menu.Popup>
            </Menu.Root>
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
      {showTitle ? (
      <div className={cn("over-imagery pointer-events-none absolute inset-x-0 top-0 z-10 flex items-center gap-3 bg-linear-to-b from-black/70 to-transparent ps-[calc(var(--safe-inline-start)+1rem)] pe-[calc(var(--safe-inline-end)+1rem)] pb-12 pt-[calc(var(--safe-top)+0.75rem)] transition-opacity duration-base", !showChrome && "opacity-0")}>
        <Button variant="secondary" className={showChrome ? "pointer-events-auto" : undefined} tabIndex={showChrome ? undefined : -1} data-nav-skip={showChrome ? undefined : ""} data-spatial-start aria-label="Back" onClick={onBack} icon={<IconArrowLeft className="rtl:rotate-180" />} />
        <div className="min-w-0 flex-1">
          <p className="truncate text-title-md text-white text-shadow-hero">{item.title}</p>
          {item.subtitle ? <p className="truncate text-label text-white/70">{item.subtitle}</p> : null}
          {media.error || failed ? <p role="status" className="truncate text-label-sm text-white/80">{media.error ?? "Some playback options could not be loaded."}</p> : null}
        </div>
      </div>
      ) : null}
      {scrub ? (
        <div className="pointer-events-none absolute inset-x-0 bottom-[calc(var(--safe-bottom)+6rem)] z-20 flex justify-center" role="status" aria-live="polite" data-scrub-preview>
          <div className="over-imagery flex flex-col items-center gap-2 rounded-card bg-black/70 p-2 text-white shadow-dock">
            {scrubFrame ? <span aria-hidden className="block rounded-md bg-no-repeat" style={{ width: scrubFrame.width, height: scrubFrame.height, backgroundImage: `url("${scrubFrame.src}")`, backgroundPosition: `${-(scrubFrame.x ?? 0)}px ${-(scrubFrame.y ?? 0)}px` }} /> : null}
            <p className="text-numeric text-title-md">
              {formatClock(scrub.target)} <span className="text-white/70">{formatScrubDelta(scrub.target - scrub.origin)}</span>
            </p>
            {scrubChapter ? <p className="max-w-64 truncate text-label-sm text-white/70">{scrubChapter}</p> : null}
          </div>
        </div>
      ) : null}
      <CastButton item={item} position={castPosition} open={cast.pickerOpen} onOpenChange={cast.setPickerOpen} hideTrigger onStarted={cast.refresh} />
      <FindSubtitlesDialog
        open={findOpen}
        onOpenChange={setFindOpen}
        mediaFileId={item.mediaFileId}
        title={item.subtitle ? `${item.title} · ${item.subtitle}` : item.title}
        onDownloaded={(id) => {
          // Shown as soon as its row arrives.
          pendingCaption.current = id;
          if (burned) changeBurn(undefined);
        }}
      />
    </div>
  );
}
