import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

import { Artwork } from "./Artwork";

interface HeroBannerProps {
  /** Optional poster shown beside the text on tablets and up. */
  poster?: string | null;
  posterAspect?: "poster" | "square";
  eyebrow?: ReactNode;
  title: ReactNode;
  /** Meta line: year, runtime, rating. */
  meta?: ReactNode;
  description?: ReactNode;
  /** One short italic line, such as a film's tagline. */
  tagline?: ReactNode;
  actions?: ReactNode;
  /** Compact facts under the description (director, genres, network). */
  facts?: ReactNode;
  /** Extra content under the actions (progress, season summary). */
  children?: ReactNode;
  tint?: string;
  className?: string;
  /** `screen` fills the first screen so the next section starts at the fold. */
  fill?: "screen" | "home" | "library" | "content";
}

/**
 * Title block over the page artwork (see `MediaBackdrop`). Text stays left-anchored over the
 * scrim, actions lead with playback, and on wide screens the hero fills the first screen so
 * the following section is never cut in half by the fold.
 */
export function HeroBanner({ poster, posterAspect = "poster", eyebrow, title, meta, description, tagline, actions, facts, children, tint, className, fill = "screen" }: HeroBannerProps) {
  return (
    <section data-fill={fill} data-nav-section className={cn("media-hero relative flex flex-col justify-end", className)}>
      <div className="media-hero-body over-imagery page-gutter relative flex w-full items-end gap-6 pb-8 pt-24 md:pb-12 lg:gap-10 lg:pb-16">
        {poster ? (
          <Artwork
            src={poster}
            alt=""
            aspect={posterAspect}
            tint={tint}
            priority
            displayWidth={260}
            className="media-hero-poster hidden w-40 shrink-0 self-end overflow-clip rounded-poster shadow-poster-hover md:block lg:w-52 xl:w-60"
          />
        ) : null}
        <div className="media-rise flex min-w-0 max-w-3xl flex-1 flex-col">
          {eyebrow ? <div className="text-overline mb-2 text-brand">{eyebrow}</div> : null}
          <h1 className="break-words text-display-lg text-foreground text-balance">{title}</h1>
          {meta ? <div className="mt-3 flex flex-wrap items-center gap-x-3 gap-y-1.5 text-body-sm text-foreground/85">{meta}</div> : null}
          {actions ? <div className="mt-6 min-w-0 w-full">{actions}</div> : null}
          {tagline ? <p className="mt-6 text-body italic text-foreground/70">{tagline}</p> : null}
          {description ? <p className={cn("max-w-2xl text-body text-foreground/85 lg:text-body-lg", fill === "library" ? "line-clamp-2" : "line-clamp-4", tagline ? "mt-2" : "mt-6")}>{description}</p> : null}
          {facts ? <div className="mt-4 max-w-2xl">{facts}</div> : null}
          {children}
        </div>
      </div>
    </section>
  );
}

/** Compact "Label value" facts for a hero: one line each, values may wrap. */
export function HeroFacts({ items }: { items: Array<{ label: string; value: ReactNode }> }) {
  const visible = items.filter((item) => item.value !== null && item.value !== undefined && item.value !== "");
  if (visible.length === 0) return null;
  return (
    <dl className="grid grid-cols-[auto_minmax(0,1fr)] gap-x-4 gap-y-1 text-body-sm">
      {visible.map((item) => (
        <div key={item.label} className="contents">
          <dt className="text-foreground/60">{item.label}</dt>
          <dd className="min-w-0 text-foreground/90">{item.value}</dd>
        </div>
      ))}
    </dl>
  );
}
