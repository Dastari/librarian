import { Dialog } from "glass-ux/dialog";
import { Button as GlassButton } from "glass-ux/button";
import { Notice } from "glass-ux/notice";
import { IconExternalLink } from "glass-ux/icons";
import { ListRow } from "glass-ux/list-row";

import { Artwork, Button } from "@/components/ui";
import { useDialogReturnFocus } from "@/components/ui/Dialog";
import { formatClock } from "@/lib/format";

import { PlayPauseButton, SeekBar, SkipButton, VolumeControl } from "./controls";
import { audioCommands } from "./usePlayer";
import { currentAudioItem, playerStore, usePlayerState } from "./store";

interface NowPlayingSheetProps {
  open: boolean;
  onClose: () => void;
  onOpenEntity: (href: string) => void;
}

/** Full "now playing" view with large artwork, transport and the queue. */
export function NowPlayingSheet({ open, onClose, onOpenEntity }: NowPlayingSheetProps) {
  const finalFocus = useDialogReturnFocus(open);
  const state = usePlayerState();
  const item = currentAudioItem(state);
  const { transport } = state;
  if (!item) return null;

  return (
    <Dialog.Root open={open} onOpenChange={(next) => !next && onClose()} sheet>
      <Dialog.Popup size="lg" finalFocus={finalFocus}>
        <Dialog.Header title="Now playing" />
        <Dialog.Body className="grid gap-6 md:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
          <div className="flex flex-col items-center gap-5">
            <Artwork src={item.artwork} alt="" aspect="square" className="w-full max-w-72 rounded-card shadow-poster-hover" />
            <div className="w-full text-center">
              <p className="truncate text-title-lg text-foreground">{item.title}</p>
              {item.subtitle ? <p className="truncate text-body-sm text-muted">{item.subtitle}</p> : null}
              {item.href ? (
                <Button variant="ghost" size="sm" className="mt-1" onPress={() => onOpenEntity(item.href!)}>
                  Open <IconExternalLink />
                </Button>
              ) : null}
            </div>
            <div className="w-full">
              <SeekBar position={transport.position} duration={transport.duration} buffered={transport.buffered} onSeek={(position) => audioCommands.seek?.(position)} />
              <div className="text-numeric flex justify-between text-label-sm text-muted">
                <span>{formatClock(transport.position)}</span>
                <span>{formatClock(transport.duration)}</span>
              </div>
            </div>
            <div className="flex items-center gap-4">
              <SkipButton direction="back" onPress={() => playerStore.skip(-1)} disabled={state.index <= 0} />
              <PlayPauseButton playing={transport.playing} onToggle={() => playerStore.updateTransport({ playing: !transport.playing })} size="lg" />
              <SkipButton direction="forward" onPress={() => playerStore.skip(1)} disabled={state.index >= state.queue.length - 1} />
            </div>
            <div className="min-h-12 w-full">
              {transport.error ? <Notice tone="danger" role="alert" action={<GlassButton size="sm" onClick={() => audioCommands.retry?.()}>Retry</GlassButton>}>{transport.error}</Notice> : null}
            </div>
            <VolumeControl volume={transport.volume} muted={transport.muted} onVolume={(volume) => playerStore.updateTransport({ volume, muted: volume === 0 })} onMuted={(muted) => playerStore.updateTransport({ muted })} />
          </div>
          <div className="flex min-h-0 flex-col">
            <p className="text-title-md mb-2 text-foreground">Up next</p>
            <ol className="flex flex-col gap-0.5">
              {state.queue.map((entry, index) => {
                const active = index === state.index;
                return (
                  <li key={`${entry.mediaFileId}-${index}`}>
                    <ListRow
                      onClick={() => playerStore.jumpTo(index)}
                      selected={active}
                      aria-current={active ? "true" : undefined}
                      icon={<Artwork src={entry.artwork} alt="" aspect="square" className="size-9 rounded-md" />}
                      title={entry.title}
                      meta={entry.subtitle}
                      trailing={<span className="text-numeric text-label-sm text-muted">{entry.duration ? formatClock(entry.duration) : index + 1}</span>}
                      className="w-full"
                    />
                  </li>
                );
              })}
            </ol>
          </div>
        </Dialog.Body>
      </Dialog.Popup>
    </Dialog.Root>
  );
}
