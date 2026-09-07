import { Drawer } from "@heroui/react";
import { IconExternalLink, IconX } from "@tabler/icons-react";

import { Artwork } from "@/components/ui";
import { formatClock } from "@/lib/format";
import { cn } from "@/lib/utils";

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
  const state = usePlayerState();
  const item = currentAudioItem(state);
  const { transport } = state;
  if (!item) return null;

  return (
    <Drawer isOpen={open} onOpenChange={(next) => !next && onClose()}>
      <Drawer.Backdrop variant="blur">
        <Drawer.Content placement="bottom" className="glass-surface glass-highlight max-h-[92svh] rounded-t-[1.75rem] border-b-0">
          <Drawer.Dialog className="flex h-full flex-col">
            <Drawer.Handle />
            <div className="flex items-center justify-between px-5 pt-2">
              <Drawer.Heading className="text-overline text-muted">Now playing</Drawer.Heading>
              <button type="button" data-focusable aria-label="Close" onClick={onClose} className="nav-focus grid size-9 place-items-center rounded-full hover:bg-white/10">
                <IconX size={18} />
              </button>
            </div>
            <Drawer.Body className="grid gap-6 px-5 pb-[calc(var(--safe-bottom)+1.5rem)] md:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
              <div className="flex flex-col items-center gap-5">
                <Artwork src={item.artwork} alt="" aspect="square" className="w-full max-w-72 rounded-card shadow-poster-hover" />
                <div className="w-full text-center">
                  <p className="truncate text-title-lg text-foreground">{item.title}</p>
                  {item.subtitle ? <p className="truncate text-body-sm text-muted">{item.subtitle}</p> : null}
                  {item.href ? (
                    <button type="button" data-focusable className="nav-focus mt-1 inline-flex items-center gap-1 rounded text-label text-brand" onClick={() => onOpenEntity(item.href!)}>
                      Open <IconExternalLink size={14} />
                    </button>
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
                <VolumeControl volume={transport.volume} muted={transport.muted} onVolume={(volume) => playerStore.updateTransport({ volume, muted: volume === 0 })} onMuted={(muted) => playerStore.updateTransport({ muted })} className="[&_input]:w-32" />
              </div>
              <div className="flex min-h-0 flex-col">
                <p className="text-title-md mb-2 text-foreground">Up next</p>
                <ol className="scrollbar-thin flex min-h-0 flex-1 flex-col gap-0.5 overflow-y-auto">
                  {state.queue.map((entry, index) => {
                    const active = index === state.index;
                    return (
                      <li key={`${entry.mediaFileId}-${index}`}>
                        <button
                          type="button"
                          data-focusable
                          onClick={() => playerStore.jumpTo(index)}
                          className={cn("nav-focus flex w-full items-center gap-3 rounded-lg px-2 py-1.5 text-left", active ? "bg-brand-soft" : "hover:bg-surface-hover")}
                        >
                          <span className="text-numeric w-6 text-label-sm text-muted">{index + 1}</span>
                          <Artwork src={entry.artwork} alt="" aspect="square" className="size-9 rounded-md" />
                          <span className="min-w-0 flex-1">
                            <span className={cn("block truncate text-body-sm", active ? "text-brand" : "text-foreground")}>{entry.title}</span>
                            {entry.subtitle ? <span className="block truncate text-label-sm text-muted">{entry.subtitle}</span> : null}
                          </span>
                          {entry.duration ? <span className="text-numeric text-label-sm text-muted">{formatClock(entry.duration)}</span> : null}
                        </button>
                      </li>
                    );
                  })}
                </ol>
              </div>
            </Drawer.Body>
          </Drawer.Dialog>
        </Drawer.Content>
      </Drawer.Backdrop>
    </Drawer>
  );
}
