import { Link, type LinkProps } from "@tanstack/react-router";
import { Button } from "glass-ux/button";
import { IconPlay } from "glass-ux/icons";
import { Progress } from "glass-ux/progress";
import { Surface } from "glass-ux/surface";
import { ToneDot } from "glass-ux/tone-dot";
import { AspectRatio } from "glass-ux/aspect-ratio";
import { createContext, useContext, useId, useState, type CSSProperties, type FocusEvent, type KeyboardEvent, type MouseEvent, type ReactNode } from "react";

import { sizedArtwork } from "@/lib/api/urls";
import type { StatusMeta } from "@/lib/status";
import { cn } from "@/lib/utils";
import { claimMorph } from "@/lib/view-transition";

import { Artwork, type ArtworkAspect } from "./Artwork";

export interface PosterCardProps {
  title: string;
  meta?: ReactNode;
  image: string | null | undefined;
  aspect?: ArtworkAspect;
  /** Fit the picture inside the tile instead of filling it (logos, which have transparency). */
  contain?: boolean;
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
  /**
   * A tile made of several pictures, for a list or playlist: up to four covers, two to a row
   * (the first three or four of its entries). Fewer than two covers fall back to `image`.
   */
  mosaic?: readonly string[];
  selected?: boolean;
  /** `row` sizes a shelf tile; `compact` a smaller one (seasons); `fill` takes the grid cell. */
  width?: "row" | "compact" | "fill";
  /** Two lines for the title, where titles differ only near the end (search results). */
  wrapTitle?: boolean;
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
/**
 * Set by a table whose cards are posters. The table's card takes focus and owns the arrow keys, so
 * the poster's own link and play button stop being stops of their own.
 */
export const PosterInTable = createContext(false);

/** Up to four covers in one square. A cover that fails to load drops out, and the rest fill the tile. */
function CoverMosaic({ covers, tint, fallback }: { covers: readonly string[]; tint?: string; fallback?: ReactNode }) {
  const [failed, setFailed] = useState<readonly string[]>([]);
  const shown = covers.filter((cover) => !failed.includes(cover)).slice(0, 4);
  if (shown.length < 2) return <>{fallback}</>;
  return (
    <AspectRatio ratio={1} className="bg-surface-secondary placeholder-gradient" style={tint ? ({ "--placeholder-tint": tint } as CSSProperties) : undefined}>
      <span className="entity-art" data-shape="square" data-mosaic={shown.length} style={{ borderRadius: 0, boxShadow: "none" }} aria-hidden>
        {shown.map((cover) => <img key={cover} src={sizedArtwork(cover, 160)} alt="" loading="lazy" decoding="async" draggable={false} onError={() => setFailed((previous) => [...previous, cover])} />)}
      </span>
    </AspectRatio>
  );
}

export function PosterCard({ title, meta, image, aspect = "poster", contain, tint, to, params, onPress, progress, status, badge, actions, onPlay, className, style, fallbackIcon, mosaic, selected, width = "fill", priority, onFocus, wrapTitle }: PosterCardProps) {
  const labelId = useId();
  const navigationClass = "poster-navigation absolute inset-0 z-10 rounded-tile outline-none";
  const inTable = useContext(PosterInTable);
  // A click still focuses the link, so hand that on to the card for the arrow keys to carry on from.
  const stops = inTable
    ? { tabIndex: -1, onFocus: (event: FocusEvent<HTMLElement>) => { event.currentTarget.closest<HTMLElement>(".fa-dt-card")?.focus({ preventScroll: true }); onFocus?.(event); } }
    : { onFocus, "data-focusable": true };
  const morph = (event: MouseEvent<HTMLElement>) => claimMorph(event.currentTarget.nextElementSibling);
  // A card is one stop: its on-screen play button is for pointers (the card's link opens the page
  // with Play on it, the Play key plays the focused tile, and the menu has the rest).
  const playKey = onPlay ? (event: KeyboardEvent<HTMLElement>) => {
    if (event.key !== "MediaPlayPause" && event.key !== "MediaPlay") return;
    event.preventDefault();
    onPlay();
  } : undefined;
  // The selected tile is where a remote enters its row (the current season).
  const entry = selected ? "" : undefined;
  const navigation = to ? <Link to={to} params={params} aria-labelledby={labelId} className={navigationClass} data-nav-default={entry} draggable={false} onClick={morph} onKeyDown={playKey} {...stops} />
    : onPress ? <button type="button" aria-labelledby={labelId} aria-pressed={selected === undefined ? undefined : selected} onClick={onPress} onKeyDown={playKey} className={navigationClass} data-nav-default={entry} {...stops} />
    : null;
  const watched = typeof progress === "number" && progress > 0 ? Math.max(0, Math.min(1, progress)) : null;

  return (
    <article style={style} data-selected={selected ? "true" : undefined}
      className={cn("poster-card group relative flex min-w-0 flex-col", width === "row" && "w-(--poster-min) shrink-0 sm:w-[calc(var(--poster-min)*1.15)] xl:w-(--poster-max)", width === "compact" && "w-[calc(var(--poster-min)*0.72)] shrink-0 xl:w-[calc(var(--poster-max)*0.62)]", className)}>
      {navigation}
      <Surface material="card" radius="tile" padding="none"
        className="poster-art pointer-events-none relative overflow-clip">
        {(() => {
          const picture = <Artwork src={image} alt="" aspect={aspect} tint={tint} priority={priority} fallback={fallbackIcon} displayWidth={width === "compact" ? 180 : 300} imageClassName={contain ? "object-contain p-3" : undefined} />;
          return mosaic && mosaic.length > 1 ? <CoverMosaic covers={mosaic} tint={tint} fallback={picture} /> : picture;
        })()}
        {badge ? <Surface as="span" material="control" radius="control" padding="none" className="over-imagery absolute start-2 top-2 px-2 py-1 text-caption">{badge}</Surface> : null}
        {status ? <ToneDot overlay label={status.label} tone={status.tone === "default" ? "neutral" : status.tone} className="absolute end-2 top-2" /> : null}
        {onPlay || actions ? (
          <div data-print="hide" className="over-imagery absolute inset-0 z-20 flex items-end justify-end gap-1 p-[max(var(--space-2),var(--focus-reach))]">
            {/* Pointer shortcuts on a card that opens: a remote reaches them from the page it opens. */}
            {actions ? <div className="pointer-events-auto flex gap-1" data-nav-skip={navigation ? "" : undefined}>{actions}</div> : null}
            {onPlay ? <Button variant="primary" shape="circle" icon={<IconPlay />} aria-label={`Play ${title}`} onClick={onPlay} className="pointer-events-auto" tabIndex={-1} data-nav-skip data-tv-skip /> : null}
          </div>
        ) : null}
        {watched !== null ? (
          <div className="over-imagery absolute inset-x-0 bottom-0">
            <Progress value={watched} max={1} showValue={false} size="sm" aria-label={`Watched progress for ${title}`} />
          </div>
        ) : null}
      </Surface>
      <div className="pointer-events-none min-w-0 px-0.5 pt-2">
        <p id={labelId} className={cn("text-title-sm text-foreground", wrapTitle ? "line-clamp-2" : "truncate")}>{title}</p>
        {meta ? <p className="truncate text-label-sm text-muted">{meta}</p> : null}
      </div>
    </article>
  );
}
