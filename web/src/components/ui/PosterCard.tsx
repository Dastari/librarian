import { Link } from "@tanstack/react-router";
import type { LinkProps } from "@tanstack/react-router";
import { IconPlayerPlayFilled } from "@tabler/icons-react";
import type { CSSProperties, ReactNode } from "react";

import type { StatusMeta } from "@/lib/status";
import { cn } from "@/lib/utils";

import { Artwork, type ArtworkAspect } from "./Artwork";

export interface PosterCardProps {
  title: string;
  /** One meta line: year, artist, episode count. */
  meta?: ReactNode;
  image: string | null | undefined;
  aspect?: ArtworkAspect;
  tint?: string;
  to?: LinkProps["to"];
  params?: LinkProps["params"];
  onPress?: () => void;
  /** 0..1 watched fraction; renders the bottom progress bar. */
  progress?: number;
  status?: StatusMeta;
  /** Small text badge at the top-left, e.g. "12/24" episodes. */
  badge?: ReactNode;
  /** Optional overlay actions (shown on hover/focus). */
  actions?: ReactNode;
  onPlay?: () => void;
  className?: string;
  style?: CSSProperties;
  fallbackIcon?: ReactNode;
  selected?: boolean;
  /** Fixed width for use inside horizontal rows. */
  width?: "row" | "fill";
  priority?: boolean;
}

/**
 * The card used for movies, shows, albums, artists and books in rows, grids and table card
 * view. The whole card is one focus target; the play affordance is an extra target only
 * when a playable file exists.
 */
export function PosterCard({
  title,
  meta,
  image,
  aspect = "poster",
  tint,
  to,
  params,
  onPress,
  progress,
  status,
  badge,
  actions,
  onPlay,
  className,
  style,
  fallbackIcon,
  selected,
  width = "fill",
  priority,
}: PosterCardProps) {
  const body = (
    <>
      <div className="relative">
        <Artwork
          src={image}
          alt=""
          aspect={aspect}
          tint={tint}
          priority={priority}
          fallback={fallbackIcon}
          className="rounded-poster shadow-poster transition-[transform,box-shadow] duration-base ease-fluid group-hover:shadow-poster-hover group-focus-visible:shadow-poster-hover group-data-[focus-visible=true]:shadow-poster-hover group-hover:[transform:translateY(-4px)_scale(1.02)] group-focus-within:[transform:translateY(-4px)_scale(1.02)]"
        />
        {badge ? (
          <span className="absolute left-2 top-2 rounded-md bg-scrim px-1.5 py-0.5 text-label-sm text-foreground backdrop-blur-sm">{badge}</span>
        ) : null}
        {status ? <span className={cn("absolute right-2 top-2 size-2.5 rounded-full ring-2 ring-scrim", status.dot)} title={status.label} /> : null}
        {typeof progress === "number" && progress > 0 ? (
          <span className="absolute inset-x-2 bottom-2 h-1 overflow-hidden rounded-full bg-white/25">
            <span className="block h-full rounded-full bg-brand" style={{ width: `${Math.min(100, Math.round(progress * 100))}%` }} />
          </span>
        ) : null}
        {onPlay ? (
          <button
            type="button"
            aria-label={`Play ${title}`}
            data-focusable
            onClick={(event) => {
              event.preventDefault();
              event.stopPropagation();
              onPlay();
            }}
            className="nav-focus absolute inset-0 m-auto grid size-12 place-items-center rounded-full bg-brand/95 text-brand-foreground opacity-0 shadow-poster transition-opacity duration-fast group-hover:opacity-100 group-focus-within:opacity-100 tv:opacity-0"
          >
            <IconPlayerPlayFilled size={20} />
          </button>
        ) : null}
        {actions ? (
          <div className="absolute right-1.5 bottom-1.5 flex gap-1 opacity-0 transition-opacity duration-fast group-hover:opacity-100 group-focus-within:opacity-100">{actions}</div>
        ) : null}
      </div>
      <div className="min-w-0 px-0.5">
        <p className="truncate text-title-sm text-foreground">{title}</p>
        {meta ? <p className="truncate text-label-sm text-muted">{meta}</p> : null}
      </div>
    </>
  );

  const classes = cn(
    "group nav-focus relative flex flex-col gap-2 rounded-poster text-left outline-none transition-transform duration-fast",
    width === "row" && "w-(--poster-min) sm:w-[calc(var(--poster-min)*1.15)] xl:w-(--poster-max)",
    selected && "ring-2 ring-brand ring-offset-2 ring-offset-background",
    className,
  );

  if (to) {
    return (
      <Link to={to} params={params} className={classes} style={style} data-focusable draggable={false}>
        {body}
      </Link>
    );
  }
  // Not a <button>: the play and action overlays are buttons of their own, and buttons cannot nest.
  return (
    <div
      role="button"
      tabIndex={0}
      onClick={onPress}
      onKeyDown={(event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          onPress?.();
        }
      }}
      className={cn(classes, "cursor-pointer")}
      style={style}
      data-focusable
    >
      {body}
    </div>
  );
}
