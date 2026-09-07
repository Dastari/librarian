import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

import { Artwork } from "./Artwork";

interface HeroBannerProps {
  backdrop: string | null | undefined;
  /** Optional poster shown beside the text on wide screens. */
  poster?: string | null;
  posterAspect?: "poster" | "square";
  eyebrow?: ReactNode;
  title: ReactNode;
  /** Meta line: year, runtime, rating. */
  meta?: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
  /** Extra content under the actions (season picker, progress). */
  children?: ReactNode;
  tint?: string;
  className?: string;
  height?: "home" | "detail";
}

/**
 * Full-bleed backdrop hero with a left-anchored scrim. Used on Home and every detail page.
 * The backdrop is fixed-height so the layout is stable before images load.
 */
export function HeroBanner({ backdrop, poster, posterAspect = "poster", eyebrow, title, meta, description, actions, children, tint, className, height = "detail" }: HeroBannerProps) {
  return (
    <section
      className={cn(
        "relative isolate flex flex-col justify-end overflow-hidden",
        height === "home" ? "min-h-[62svh] lg:min-h-[70svh]" : "min-h-[46svh] lg:min-h-[56svh]",
        className,
      )}
    >
      <Artwork
        src={backdrop}
        alt=""
        aspect="backdrop"
        tint={tint}
        priority
        className="absolute inset-0 -z-10 !aspect-auto h-full w-full"
        imageClassName="object-cover object-[center_20%] scale-[1.02] motion-safe:animate-[fade-in_600ms_var(--ease-fluid)_both]"
      />
      <div className="pointer-events-none absolute inset-0 -z-10 bg-gradient-to-t from-background via-background/60 to-background/10" />
      <div className="pointer-events-none absolute inset-0 -z-10 bg-gradient-to-r from-background/85 via-background/30 to-transparent" />

      <div className="page-gutter relative flex w-full items-end gap-6 pb-8 pt-28 lg:gap-10 lg:pb-12">
        {poster ? (
          <Artwork
            src={poster}
            alt=""
            aspect={posterAspect}
            tint={tint}
            priority
            className="hidden w-40 shrink-0 rounded-poster shadow-poster-hover lg:block xl:w-48"
          />
        ) : null}
        <div className="min-w-0 max-w-3xl">
          {eyebrow ? <div className="text-overline mb-3 text-brand">{eyebrow}</div> : null}
          <h1 className="text-display-lg text-shadow-hero text-foreground">{title}</h1>
          {meta ? <div className="mt-3 flex flex-wrap items-center gap-x-3 gap-y-1 text-body-sm text-foreground/80">{meta}</div> : null}
          {description ? <p className="mt-4 line-clamp-3 max-w-2xl text-body text-foreground/85 lg:text-body-lg">{description}</p> : null}
          {actions ? <div className="mt-6 flex flex-wrap items-center gap-3">{actions}</div> : null}
          {children}
        </div>
      </div>
    </section>
  );
}
