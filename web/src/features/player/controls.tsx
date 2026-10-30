import { IconPlayerSkipBackFilled, IconPlayerSkipForwardFilled, IconVolume, IconVolume2, IconVolumeOff } from "@tabler/icons-react";
import { Button } from "glass-ux/button";
import { IconPause, IconPlay } from "glass-ux/icons";
import { Slider } from "glass-ux/slider";
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
      <div className={cn("glass-inset relative w-full overflow-hidden rounded-full transition-[height] duration-fast", thin ? "h-1.5 group-hover/seek:h-2" : "h-2 group-hover/seek:h-2.5")}>
        <div className="absolute inset-y-0 left-0 bg-white/30" style={{ width: `${bufferedFraction * 100}%` }} />
        <div className="absolute inset-y-0 left-0 bg-brand" style={{ width: `${(safeDuration ? (preview ?? position) / safeDuration : 0) * 100}%` }} />
        {chapters?.map((chapter) =>
          chapter.start > 0 && safeDuration ? <span key={chapter.start} className="absolute inset-y-0 w-0.5 bg-black/50" style={{ left: `${(chapter.start / safeDuration) * 100}%` }} /> : null,
        )}
      </div>
      <div
        className={cn("bg-fg absolute size-4 -translate-x-1/2 rounded-full transition-transform duration-fast", thin ? "scale-0 group-hover/seek:scale-100" : "scale-75 group-hover/seek:scale-100 group-focus-visible/seek:scale-100", dragging && "!scale-125")}
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

export function PlayPauseButton({ playing, onToggle, size = "md", className }: TransportButtonProps) {
  return <Button type="button" variant="primary" shape="circle" size={size} aria-label={playing ? "Pause" : "Play"} onClick={onToggle} icon={playing ? <IconPause /> : <IconPlay />} className={className} data-focusable />;
}

export function SkipButton({ direction, onPress, disabled, className }: { direction: "back" | "forward"; onPress: () => void; disabled?: boolean; className?: string }) {
  const Icon = direction === "back" ? IconPlayerSkipBackFilled : IconPlayerSkipForwardFilled;
  return <Button type="button" variant="ghost" shape="circle" aria-label={direction === "back" ? "Previous" : "Next"} onClick={onPress} disabled={disabled} disabledReason={disabled ? direction === "back" ? "No previous track" : "No next track" : undefined} icon={<Icon />} className={className} data-focusable />;
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
    <div className={cn("flex min-w-0 items-center gap-1", className)}>
      <Button type="button" variant="ghost" shape="circle" aria-label={muted ? "Unmute" : "Mute"} onClick={() => onMuted(!muted)} icon={<Icon />} data-focusable />
      <Slider
        min={0}
        max={1}
        step={0.02}
        largeStep={0.2}
        value={effective}
        aria-label="Volume"
        size="sm"
        format={(value) => `${Math.round(value * 100)}%`}
        onValueChange={(value) => { if (typeof value === "number") onVolume(value); }}
        className="w-24 shrink-0"
      />
    </div>
  );
}
