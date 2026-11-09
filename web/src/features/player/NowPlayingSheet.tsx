import { useEffect, useRef, type RefObject } from "react";
import { Dialog } from "glass-ux/dialog";
import { Button as GlassButton } from "glass-ux/button";
import { Notice } from "glass-ux/notice";
import { IconChevronDown, IconExternalLink } from "glass-ux/icons";
import { Segmented } from "glass-ux/segmented";

import { Artwork, Button } from "@/components/ui";
import { useDialogReturnFocus } from "@/components/ui/Dialog";
import { formatClock } from "@/lib/format";
import { usePref } from "@/lib/prefs";

import { PlayPauseButton, SeekBar, SkipButton, VolumeControl } from "./controls";
import { LyricsPanel } from "./LyricsPanel";
import { drawerFocus, focusDock } from "./nowPlaying";
import { QueuePanel } from "./QueuePanel";
import { audioCommands, canSkipAudio, skipAudio } from "./usePlayer";
import { currentChapterTitle, seekSpan } from "./chapterNav";
import { currentAudioItem, playerStore, usePlayerState } from "./store";

interface NowPlayingSheetProps {
  open: boolean;
  onClose: () => void;
  onOpenEntity: (href: string) => void;
}

type PlayerState = ReturnType<typeof usePlayerState>;
type SidePane = "queue" | "lyrics";

const BODY_LAYOUT = "now-playing grid gap-6 md:grid-cols-[minmax(0,1fr)_minmax(0,1fr)] md:grid-rows-[minmax(0,1fr)] md:overflow-hidden";

/**
 * What now playing shows: the artwork, the title and chapter, and the queue. With `withTransport`
 * the controls too (the phone sheet); the desktop drawer keeps them in the dock beneath it.
 */
function NowPlayingBody({ state, withTransport, onOpenEntity, queueRef, fillQueue }: { state: PlayerState; withTransport: boolean; onOpenEntity: (href: string) => void; queueRef: RefObject<HTMLDivElement | null>; fillQueue: boolean }) {
  const item = currentAudioItem(state);
  const { transport } = state;
  const [pane, setPane] = usePref<SidePane>("player.nowPlaying.pane", "queue");
  if (!item) return null;
  const span = seekSpan(item, transport.position, transport.duration);
  const chapterLine = span.chapter !== null ? `Chapter ${span.chapter + 1} of ${span.chapters} · ${formatClock(Math.max(0, transport.duration - transport.position))} left in all` : null;
  const subtitle = currentChapterTitle(item, transport.position) ?? item.subtitle;
  return (
    <>
      <div className="flex flex-col items-center gap-5">
        <Artwork src={item.artwork} alt="" aspect="square" className="w-full max-w-72 rounded-card shadow-poster-hover" />
        <div className="w-full text-center">
          <p className="truncate text-title-lg text-foreground">{item.title}</p>
          {subtitle ? <p className="truncate text-body-sm text-muted">{subtitle}</p> : null}
          {!withTransport && chapterLine ? <p className="text-numeric mt-1 text-label-sm text-muted">{chapterLine}</p> : null}
          {item.href ? (
            <Button variant="ghost" size="sm" className="mt-1" onPress={() => onOpenEntity(item.href!)}>
              Open <IconExternalLink />
            </Button>
          ) : null}
        </div>
        {withTransport ? (
          <>
            <div className="w-full">
              <SeekBar position={transport.position - span.start} duration={span.end - span.start} buffered={Math.max(0, transport.buffered - span.start)} onSeek={(position) => audioCommands.seek?.(span.start + position)} />
              <div className="text-numeric flex justify-between text-label-sm text-muted">
                <span>{formatClock(transport.position - span.start)}</span>
                {chapterLine ? <span>{chapterLine}</span> : null}
                <span>{formatClock(span.end - span.start)}</span>
              </div>
            </div>
            <div className="flex items-center gap-4">
              <SkipButton direction="back" onPress={() => skipAudio(-1)} disabled={!canSkipAudio(state.queue, state.index, transport.position, -1)} />
              <PlayPauseButton playing={transport.playing} onToggle={() => playerStore.updateTransport({ playing: !transport.playing })} size="lg" />
              <SkipButton direction="forward" onPress={() => skipAudio(1)} disabled={!canSkipAudio(state.queue, state.index, transport.position, 1)} />
            </div>
            <div className="min-h-12 w-full">
              {transport.error ? <Notice tone="danger" role="alert" action={<GlassButton size="sm" onClick={() => audioCommands.retry?.()}>Retry</GlassButton>}>{transport.error}</Notice> : null}
            </div>
            <VolumeControl volume={transport.volume} muted={transport.muted} onVolume={(volume) => playerStore.updateTransport({ volume, muted: volume === 0 })} onMuted={(muted) => playerStore.updateTransport({ muted })} />
          </>
        ) : null}
      </div>
      {/* Beside the player, only the queue (or the lyrics) scrolls: the controls stay put. */}
      <div className="flex min-h-0 flex-col gap-3">
        {item.entity.kind === "track" ? (
          <Segmented<SidePane>
            size="sm"
            aria-label="Beside the artwork"
            className="self-start"
            options={[{ value: "queue", label: "Up next" }, { value: "lyrics", label: "Lyrics" }]}
            value={pane}
            onValueChange={setPane}
          />
        ) : null}
        {item.entity.kind === "track" && pane === "lyrics" ? (
          <LyricsPanel mediaFileId={item.mediaFileId} position={transport.position} onSeek={(seconds) => audioCommands.seek?.(seconds)} fill={fillQueue} />
        ) : (
          <QueuePanel state={state} listRef={queueRef} fill={fillQueue} />
        )}
      </div>
    </>
  );
}

/** Opening on what's playing, which may be far down a long queue. */
function useCurrentInView(open: boolean, queueId: string | undefined) {
  const queueRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open || !queueId) return;
    // The table draws rows a moment after it opens.
    let tries = 0;
    const look = setInterval(() => {
      const row = queueRef.current?.querySelector(`[data-row-id="${CSS.escape(queueId)}"]`);
      if (!row && ++tries < 20) return;
      clearInterval(look);
      row?.scrollIntoView({ block: "nearest" });
    }, 100);
    return () => clearInterval(look);
  }, [open, queueId]);
  return queueRef;
}

/** Now playing on a phone: a bottom sheet with everything in it, the controls included. */
export function NowPlayingSheet({ open, onClose, onOpenEntity }: NowPlayingSheetProps) {
  const finalFocus = useDialogReturnFocus(open);
  const state = usePlayerState();
  const queueRef = useCurrentInView(open, state.queue[state.index]?.queueId);
  if (!currentAudioItem(state)) return null;
  return (
    <Dialog.Root open={open} onOpenChange={(next) => !next && onClose()} sheet>
      <Dialog.Popup size="lg" finalFocus={finalFocus}>
        <Dialog.Header title="Now playing" />
        <Dialog.Body className={BODY_LAYOUT}>
          <NowPlayingBody state={state} withTransport onOpenEntity={onOpenEntity} queueRef={queueRef} fillQueue={false} />
        </Dialog.Body>
      </Dialog.Popup>
    </Dialog.Root>
  );
}

/**
 * Now playing on a larger screen: the dock's panel grows up into a drawer with the same gaps
 * around it, the artwork and queue above and the dock's controls still at the bottom. Escape or
 * the collapse button closes it, and focus goes back to whatever opened it.
 */
export function NowPlayingDrawer({ open, onClose, onOpenEntity }: NowPlayingSheetProps) {
  const state = usePlayerState();
  const queueRef = useCurrentInView(open, state.queue[state.index]?.queueId);
  const panel = useRef<HTMLElement>(null);
  const hasItem = Boolean(currentAudioItem(state));
  useEffect(() => {
    if (!open || !hasItem) return;
    const opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    // Opened for a remote (playback just started on a TV, or "Now playing"): Play/Pause first.
    if (drawerFocus.current === "transport") {
      drawerFocus.current = null;
      if (!focusDock()) panel.current?.querySelector<HTMLElement>("[data-drawer-close]")?.focus();
    } else panel.current?.querySelector<HTMLElement>("[data-drawer-close]")?.focus();
    return () => {
      if (opener?.isConnected) opener.focus();
    };
  }, [open, hasItem]);
  if (!open || !hasItem) return null;
  return (
    <section
      ref={panel}
      role="dialog"
      aria-modal="false"
      aria-label="Now playing"
      className="flex min-h-0 flex-1 flex-col"
      onKeyDown={(event) => {
        if (event.key !== "Escape" || event.defaultPrevented) return;
        event.stopPropagation();
        onClose();
      }}
    >
      <header className="flex shrink-0 items-center justify-between px-5 pt-4">
        <h2 className="text-title-md text-foreground">Now playing</h2>
        <GlassButton variant="ghost" shape="circle" aria-label="Close now playing" icon={<IconChevronDown />} onClick={onClose} data-drawer-close data-focusable />
      </header>
      <div className={`${BODY_LAYOUT} min-h-0 flex-1 overflow-y-auto px-5 pb-4 pt-2`}>
        <NowPlayingBody state={state} withTransport={false} onOpenEntity={onOpenEntity} queueRef={queueRef} fillQueue />
      </div>
    </section>
  );
}
