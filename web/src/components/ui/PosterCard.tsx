import { Link, type LinkProps } from "@tanstack/react-router";
import { Button } from "glass-ux/button";
import { IconPlay } from "glass-ux/icons";
import { Progress } from "glass-ux/progress";
import { Surface } from "glass-ux/surface";
import { ToneDot } from "glass-ux/tone-dot";
import { useId, type CSSProperties, type FocusEvent, type KeyboardEvent, type MouseEvent, type ReactNode } from "react";

import type { StatusMeta } from "@/lib/status";
import { cn } from "@/lib/utils";
import { claimMorph } from "@/lib/view-transition";

import { Artwork, type ArtworkAspect } from "./Artwork";

export interface PosterCardProps {
  title: string;
  meta?: ReactNode;
  image: string | null | undefined;
  aspect?: ArtworkAspect;
  tint?: string;
  to?: LinkProps["to"];
  params?: LinkProps["params"];
  onPress?: () => void;
  progress?: number;
  status?: StatusMeta;
  badge?: ReactNode;
  actions?: ReactNode;
  onPlay?: () => void;
  className?: string;
  style?: CSSProperties;
  fallbackIcon?: ReactNode;
  selected?: boolean;
  /** `row` sizes a shelf tile; `compact` a smaller one (seasons); `fill` takes the grid cell. */
  width?: "row" | "compact" | "fill";
  priority?: boolean;
  /** Focus on the tile's link or button, so a page can preview the title (see `useFocusPreview`). */
  onFocus?: (event: FocusEvent<HTMLElement>) => void;
}

/**
 * A media tile the way players show them: the whole poster is the card, uncropped, with the
 * title and one meta line underneath on the canvas. One link or button covers the tile;
 * playback and extra actions are separate targets over the artwork and never hide on hover.
 * Watched progress runs along the poster's bottom edge.
 */
export function PosterCard({ title, meta, image, aspect = "poster", tint, to, params, onPress, progress, status, badge, actions, onPlay, className, style, fallbackIcon, selected, width = "fill", priority, onFocus }: PosterCardProps) {
  const labelId = useId();
  const navigationClass = "poster-navigation absolute inset-0 z-10 rounded-tile outline-none";
  const morph = (event: MouseEvent<HTMLElement>) => claimMorph(event.currentTarget.nextElementSibling);
  // A remote's Play key plays the focused tile; its on-screen play button is for pointers.
  const playKey = onPlay ? (event: KeyboardEvent<HTMLElement>) => {
    if (event.key !== "MediaPlayPause" && event.key !== "MediaPlay") return;
    event.preventDefault();
    onPlay();
  } : undefined;
  // The selected tile is where a remote enters its row (the current season).
  const entry = selected ? "" : undefined;
  const navigation = to ? <Link to={to} params={params} aria-labelledby={labelId} className={navigationClass} data-focusable data-nav-default={entry} draggable={false} onClick={morph} onKeyDown={playKey} onFocus={onFocus} />
    : onPress ? <button type="button" aria-labelledby={labelId} aria-pressed={selected === undefined ? undefined : selected} onClick={onPress} onKeyDown={playKey} onFocus={onFocus} className={navigationClass} data-focusable data-nav-default={entry} />
    : null;
  const watched = typeof progress === "number" && progress > 0 ? Math.max(0, Math.min(1, progress)) : null;

  return (
    <article style={style} data-selected={selected ? "true" : undefined}
      className={cn("poster-card group relative flex min-w-0 flex-col", width === "row" && "w-(--poster-min) shrink-0 sm:w-[calc(var(--poster-min)*1.15)] xl:w-(--poster-max)", width === "compact" && "w-[calc(var(--poster-min)*0.72)] shrink-0 xl:w-[calc(var(--poster-max)*0.62)]", className)}>
      {navigation}
      <Surface material="card" radius="tile" padding="none"
        className={cn("poster-art pointer-events-none relative overflow-clip", selected && "ring-2 ring-brand ring-offset-2 ring-offset-background")}>
        <Artwork src={image} alt="" aspect={aspect} tint={tint} priority={priority} fallback={fallbackIcon} displayWidth={width === "compact" ? 180 : 300} />
        {badge ? <Surface as="span" material="control" radius="control" padding="none" className="over-imagery absolute start-2 top-2 px-2 py-1 text-caption">{badge}</Surface> : null}
        {status ? <ToneDot overlay label={status.label} tone={status.tone === "default" ? "neutral" : status.tone} className="absolute end-2 top-2" /> : null}
        {onPlay || actions ? (
          <div data-print="hide" className="over-imagery absolute inset-0 z-20 flex items-end justify-end gap-1 p-[max(var(--space-2),var(--focus-reach))]">
            {actions ? <div className="pointer-events-auto flex gap-1">{actions}</div> : null}
            {onPlay ? <Button variant="primary" shape="circle" icon={<IconPlay />} aria-label={`Play ${title}`} onClick={onPlay} className="pointer-events-auto" data-focusable data-tv-skip /> : null}
          </div>
        ) : null}
        {watched !== null ? (
          <div className="over-imagery absolute inset-x-0 bottom-0">
            <Progress value={watched} max={1} showValue={false} size="sm" aria-label={`Watched progress for ${title}`} />
          </div>
        ) : null}
      </Surface>
      <div className="pointer-events-none min-w-0 px-0.5 pt-2">
        <p id={labelId} className="truncate text-title-sm text-foreground">{title}</p>
        {meta ? <p className="truncate text-label-sm text-muted">{meta}</p> : null}
      </div>
    </article>
  );
}
