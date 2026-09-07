import { Link } from "@tanstack/react-router";
import type { LinkProps } from "@tanstack/react-router";
import { IconPlayerPlayFilled } from "@tabler/icons-react";
import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

import { Artwork } from "./Artwork";

export interface BackdropCardProps {
  title: string;
  subtitle?: ReactNode;
  meta?: ReactNode;
  image: string | null | undefined;
  tint?: string;
  to?: LinkProps["to"];
  params?: LinkProps["params"];
  onPress?: () => void;
  progress?: number;
  /** Bottom-right corner label such as remaining time. */
  corner?: ReactNode;
  className?: string;
  fallbackIcon?: ReactNode;
  width?: "row" | "fill";
}

/** 16:9 card for episodes and "continue watching"; text sits over a bottom scrim. */
export function BackdropCard({ title, subtitle, meta, image, tint, to, params, onPress, progress, corner, className, fallbackIcon, width = "fill" }: BackdropCardProps) {
  const body = (
    <>
      <Artwork
        src={image}
        alt=""
        aspect="backdrop"
        tint={tint}
        fallback={fallbackIcon}
        className="rounded-poster shadow-poster transition-[transform,box-shadow] duration-base ease-fluid group-hover:shadow-poster-hover group-hover:[transform:translateY(-4px)_scale(1.02)] group-focus-within:[transform:translateY(-4px)_scale(1.02)]"
      />
      <div className="pointer-events-none absolute inset-x-0 bottom-0 rounded-b-poster bg-gradient-to-t from-black/85 via-black/40 to-transparent p-3 pt-10 text-left">
        {subtitle ? <p className="truncate text-label-sm text-white/70">{subtitle}</p> : null}
        <p className="truncate text-title-sm text-white">{title}</p>
        {meta ? <p className="truncate text-label-sm text-white/70">{meta}</p> : null}
      </div>
      {corner ? <span className="absolute right-2 top-2 rounded-md bg-scrim px-1.5 py-0.5 text-label-sm text-foreground">{corner}</span> : null}
      {typeof progress === "number" && progress > 0 ? (
        <span className="absolute inset-x-0 bottom-0 h-1 overflow-hidden rounded-b-poster bg-white/20">
          <span className="block h-full bg-brand" style={{ width: `${Math.min(100, Math.round(progress * 100))}%` }} />
        </span>
      ) : null}
      <span className="absolute inset-0 m-auto grid size-12 place-items-center rounded-full bg-brand/95 text-brand-foreground opacity-0 shadow-poster transition-opacity duration-fast group-hover:opacity-100 group-focus-visible:opacity-100">
        <IconPlayerPlayFilled size={20} />
      </span>
    </>
  );
  const classes = cn(
    "group nav-focus relative block rounded-poster outline-none",
    width === "row" && "w-[calc(var(--poster-min)*1.9)] xl:w-[calc(var(--poster-max)*1.85)]",
    className,
  );
  if (to) {
    return (
      <Link to={to} params={params} className={classes} data-focusable draggable={false}>
        {body}
      </Link>
    );
  }
  return (
    <button type="button" onClick={onPress} className={classes} data-focusable>
      {body}
    </button>
  );
}
