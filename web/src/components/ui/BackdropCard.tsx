import { Link, type LinkProps } from "@tanstack/react-router";
import { IconPlay } from "glass-ux/icons";
import { Progress } from "glass-ux/progress";
import { Surface } from "glass-ux/surface";
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
  corner?: ReactNode;
  className?: string;
  fallbackIcon?: ReactNode;
  width?: "row" | "fill";
}

/** Readable media tile with a permanent playback affordance and one activation target. */
export function BackdropCard({ title, subtitle, meta, image, tint, to, params, onPress, progress, corner, className, fallbackIcon, width = "fill" }: BackdropCardProps) {
  const body = <>
    <div className="relative">
      <Artwork src={image} alt="" aspect="backdrop" tint={tint} fallback={fallbackIcon} displayWidth={420} />
      {corner ? <Surface material="control" radius="control" padding="none" className="over-imagery absolute right-2 top-2 px-2 py-1 text-caption">{corner}</Surface> : null}
      <div className="over-imagery pointer-events-none absolute bottom-2 right-2">
        <Surface material="control" radius="control" padding="none" className="grid size-10 place-items-center"><IconPlay /></Surface>
      </div>
    </div>
    <div className="min-w-0 p-tile text-left">
      {subtitle ? <p className="truncate text-label-sm text-muted">{subtitle}</p> : null}
      <p className="truncate text-title-sm text-foreground">{title}</p>
      {meta ? <p className="truncate text-label-sm text-muted">{meta}</p> : null}
      {typeof progress === "number" && progress > 0 ? <Progress value={Math.max(0, Math.min(1, progress))} max={1} showValue={false} size="sm" aria-label={`Watched progress for ${title}`} className="mt-2" /> : null}
    </div>
  </>;
  const classes = cn("nav-focus block min-w-0 rounded-[inherit]", className);
  return <Surface material="card" radius="tile" padding="none" className={cn("relative min-w-0", width === "row" && "w-[calc(var(--poster-min)*1.9)] xl:w-[calc(var(--poster-max)*1.85)] shrink-0")}>
    {to ? <Link to={to} params={params} className={classes} data-focusable draggable={false}>{body}</Link> : <button type="button" onClick={onPress} className={cn(classes, "w-full")} data-focusable>{body}</button>}
  </Surface>;
}
