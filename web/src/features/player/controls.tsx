import { IconPlayerPauseFilled, IconPlayerPlayFilled, IconPlayerSkipBackFilled, IconPlayerSkipForwardFilled, IconVolume, IconVolume2, IconVolumeOff } from "@tabler/icons-react";
import { useCallback, useRef, useState, type KeyboardEvent, type PointerEvent } from "react";

import { formatClock } from "@/lib/format";
import { clamp, cn } from "@/lib/utils";

interface SeekBarProps {
  position: number;
  duration: number;
  buffered?: number;
  onSeek: (position: number) => void;
  chapters?: Array<{ start: number; title?: string | null }>;
  className?: string;
  /** Compact variant for the dock. */
  thin?: boolean;
}

/**
 * Scrubber with buffered range, hover/drag preview and chapter ticks. Works with pointer,
 * keyboard (arrows step 5s, Shift+arrows 30s) and remote controls.
 */
export function SeekBar({ position, duration, buffered = 0, onSeek, chapters, className, thin }: SeekBarProps) {
  const track = useRef<HTMLDivElement>(null);
  const [preview, setPreview] = useState<number | null>(null);
  const [dragging, setDragging] = useState(false);
  const safeDuration = duration > 0 ? duration : 0;
  const fraction = safeDuration ? clamp(position / safeDuration, 0, 1) : 0;
  const bufferedFraction = safeDuration ? clamp(buffered / safeDuration, 0, 1) : 0;

  const positionFromEvent = useCallback(
    (event: PointerEvent<HTMLDivElement>) => {
      const rect = track.current?.getBoundingClientRect();
      if (!rect || !safeDuration) return 0;
      return clamp((event.clientX - rect.left) / rect.width, 0, 1) * safeDuration;
    },
    [safeDuration],
  );

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const step = event.shiftKey ? 30 : 5;
    if (event.key === "ArrowLeft") onSeek(clamp(position - step, 0, safeDuration));
    else if (event.key === "ArrowRight") onSeek(clamp(position + step, 0, safeDuration));
    else if (event.key === "Home") onSeek(0);
    else if (event.key === "End") onSeek(safeDuration);
    else return;
    event.preventDefault();
    event.stopPropagation();
  };

  const shown = preview ?? position;

  return (
    <div
      ref={track}
      role="slider"
      tabIndex={0}
      data-focusable
      aria-label="Seek"
      aria-valuemin={0}
      aria-valuemax={Math.round(safeDuration)}
      aria-valuenow={Math.round(position)}
      aria-valuetext={`${formatClock(position)} of ${formatClock(safeDuration)}`}
      onKeyDown={onKeyDown}
      onPointerDown={(event) => {
        event.currentTarget.setPointerCapture(event.pointerId);
        setDragging(true);
        setPreview(positionFromEvent(event));
      }}
      onPointerMove={(event) => {
        if (dragging || event.pointerType === "mouse") setPreview(positionFromEvent(event));
      }}
      onPointerUp={(event) => {
        const next = positionFromEvent(event);
        setDragging(false);
        setPreview(null);
        onSeek(next);
      }}
      onPointerLeave={() => {
        if (!dragging) setPreview(null);
      }}
      className={cn("nav-focus group/seek relative flex w-full touch-none items-center rounded-full", thin ? "h-3" : "h-6", className)}
    >
      <div className={cn("glass-track relative w-full overflow-hidden rounded-full transition-[height] duration-fast", thin ? "h-1.5 group-hover/seek:h-2" : "h-2 group-hover/seek:h-2.5")}>
        <div className="absolute inset-y-0 left-0 bg-white/30" style={{ width: `${bufferedFraction * 100}%` }} />
        <div className="absolute inset-y-0 left-0 bg-brand" style={{ width: `${(safeDuration ? (preview ?? position) / safeDuration : 0) * 100}%` }} />
        {chapters?.map((chapter) =>
          chapter.start > 0 && safeDuration ? <span key={chapter.start} className="absolute inset-y-0 w-0.5 bg-black/50" style={{ left: `${(chapter.start / safeDuration) * 100}%` }} /> : null,
        )}
      </div>
      <div
        className={cn("glass-thumb absolute size-4 -translate-x-1/2 rounded-full transition-transform duration-fast", thin ? "scale-0 group-hover/seek:scale-100" : "scale-75 group-hover/seek:scale-100 group-focus-visible/seek:scale-100", dragging && "!scale-125")}
        style={{ left: `${(safeDuration ? (preview ?? position) / safeDuration : fraction) * 100}%` }}
      />
      {preview !== null ? (
        <span className="pointer-events-none absolute -top-8 -translate-x-1/2 rounded-md bg-scrim px-2 py-1 text-label-sm text-foreground" style={{ left: `${(safeDuration ? shown / safeDuration : 0) * 100}%` }}>
          {formatClock(shown)}
        </span>
      ) : null}
    </div>
  );
}

interface TransportButtonProps {
  playing: boolean;
  onToggle: () => void;
  size?: "sm" | "md" | "lg";
  className?: string;
}

const TRANSPORT_SIZE = { sm: "size-9 [&_svg]:size-4", md: "size-12 [&_svg]:size-6", lg: "size-16 [&_svg]:size-8" };

export function PlayPauseButton({ playing, onToggle, size = "md", className }: TransportButtonProps) {
  return (
    <button
      type="button"
      data-focusable
      aria-label={playing ? "Pause" : "Play"}
      onClick={onToggle}
      className={cn("nav-focus grid place-items-center rounded-full bg-foreground text-background transition-transform duration-fast hover:scale-105 active:scale-95", TRANSPORT_SIZE[size], className)}
    >
      {playing ? <IconPlayerPauseFilled /> : <IconPlayerPlayFilled className="translate-x-px" />}
    </button>
  );
}

export function SkipButton({ direction, onPress, disabled, className }: { direction: "back" | "forward"; onPress: () => void; disabled?: boolean; className?: string }) {
  const Icon = direction === "back" ? IconPlayerSkipBackFilled : IconPlayerSkipForwardFilled;
  return (
    <button
      type="button"
      data-focusable
      disabled={disabled}
      aria-label={direction === "back" ? "Previous" : "Next"}
      onClick={onPress}
      className={cn("nav-focus grid size-10 place-items-center rounded-full text-foreground/90 transition-colors hover:bg-white/10 disabled:opacity-30", className)}
    >
      <Icon size={20} />
    </button>
  );
}

interface VolumeControlProps {
  volume: number;
  muted: boolean;
  onVolume: (volume: number) => void;
  onMuted: (muted: boolean) => void;
  className?: string;
}

export function VolumeControl({ volume, muted, onVolume, onMuted, className }: VolumeControlProps) {
  const effective = muted ? 0 : volume;
  const Icon = effective === 0 ? IconVolumeOff : effective < 0.5 ? IconVolume2 : IconVolume;
  return (
    <div className={cn("group/volume flex items-center gap-1", className)}>
      <button type="button" data-focusable aria-label={muted ? "Unmute" : "Mute"} onClick={() => onMuted(!muted)} className="nav-focus grid size-10 place-items-center rounded-full text-foreground/90 hover:bg-white/10">
        <Icon size={20} />
      </button>
      <input
        type="range"
        min={0}
        max={1}
        step={0.02}
        value={effective}
        aria-label="Volume"
        data-spatial-ignore
        onChange={(event) => onVolume(Number(event.target.value))}
        className="glass-range h-1.5 w-0 cursor-pointer appearance-none rounded-full transition-[width] duration-base group-hover/volume:w-24 focus-visible:w-24 tv:w-24"
      />
    </div>
  );
}
