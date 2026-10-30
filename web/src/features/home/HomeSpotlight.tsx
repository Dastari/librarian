import { Link } from "@tanstack/react-router";
import { Button } from "glass-ux/button";
import { IconChevronLeft, IconChevronRight, IconInfo, IconPlay } from "glass-ux/icons";
import { useEffect, useState, type ReactNode } from "react";

import { HeroBanner, MediaBackdrop, MetaLine } from "@/components/ui";
import type { RecentMoviesQuery, RecentShowsQuery } from "@/graphql/generated/graphql";
import { movieBackdrop, moviePoster, showBackdrop, showPoster } from "@/lib/artwork";
import { formatRuntime } from "@/lib/format";
import { LIBRARY_TYPES } from "@/lib/library-types";
import { cn } from "@/lib/utils";

type Movie = RecentMoviesQuery["movies"]["edges"][number]["node"];
type Show = RecentShowsQuery["shows"]["edges"][number]["node"];

export interface SpotlightItem {
  key: string;
  kind: "movie" | "show";
  id: string;
  title: string;
  meta: Array<string | number | null | undefined>;
  overview?: string | null;
  backdrop: string | null | undefined;
  poster: string | null | undefined;
  tint: string;
  mediaFileId?: string | null;
}

/** Recently added titles with something to show, alternating films and series. */
export function spotlightItems(movies: Movie[], shows: Show[], limit = 6): SpotlightItem[] {
  const films: SpotlightItem[] = movies.filter((movie) => movie.overview).map((movie) => ({
    key: `movie:${movie.id}`, kind: "movie", id: movie.id, title: movie.title,
    meta: [movie.year, movie.runtime ? formatRuntime(movie.runtime, "minutes") : null, movie.certification, movie.genres.slice(0, 2).join(", ") || null],
    overview: movie.overview, backdrop: movieBackdrop(movie.id), poster: moviePoster(movie.id), tint: LIBRARY_TYPES.movies.tintVar, mediaFileId: movie.mediaFileId,
  }));
  const series: SpotlightItem[] = shows.filter((show) => show.overview).map((show) => ({
    key: `show:${show.id}`, kind: "show", id: show.id, title: show.name,
    meta: [show.year, show.network, show.genres.slice(0, 2).join(", ") || null],
    overview: show.overview?.replace(/<[^>]+>/g, ""), backdrop: show.backdropUrl ? showBackdrop(show) : null, poster: showPoster(show), tint: LIBRARY_TYPES.tv.tintVar,
  }));
  const mixed: SpotlightItem[] = [];
  for (let index = 0; mixed.length < limit && (index < films.length || index < series.length); index += 1) {
    if (films[index]) mixed.push(films[index]!);
    if (series[index] && mixed.length < limit) mixed.push(series[index]!);
  }
  return mixed;
}

export interface SpotlightArt {
  src: string | null | undefined;
  fallback: string | null | undefined;
  tint: string;
}

interface HomeSpotlightProps {
  items: SpotlightItem[];
  onPlay: (item: SpotlightItem) => void;
  /** `library` leaves room for the first shelf on the first screen. */
  fill?: "home" | "library";
  /** Show the poster beside the text (Home); a library spotlight is text over art. */
  poster?: boolean;
  eyebrow?: (item: SpotlightItem) => ReactNode;
  /** When the page draws the artwork itself (behind its own header), report it instead. */
  onArtChange?: (art: SpotlightArt) => void;
  className?: string;
}

/**
 * Home's opening screen: one recently added title over its own artwork, with Play and
 * details. People page through the titles themselves; nothing advances on a timer (R18.2).
 */
export function HomeSpotlight({ items, onPlay, fill = "home", poster = true, eyebrow, onArtChange, className }: HomeSpotlightProps) {
  const [index, setIndex] = useState(0);
  const item = items[Math.min(index, items.length - 1)];
  useEffect(() => {
    if (item && onArtChange) onArtChange({ src: item.backdrop, fallback: item.poster, tint: item.tint });
  }, [item, onArtChange]);
  if (!item) return null;
  const step = (delta: number) => setIndex((current) => (current + delta + items.length) % items.length);
  const detail = item.kind === "movie"
    ? <Link to="/movies/$movieId" params={{ movieId: item.id }} />
    : <Link to="/shows/$showId" params={{ showId: item.id }} />;

  return (
    <div className={cn("relative isolate flex flex-col", className)} aria-roledescription="carousel" aria-label="Recently added">
      {onArtChange ? null : <MediaBackdrop key={`art:${item.key}`} src={item.backdrop} fallback={item.poster} tint={item.tint} />}
      <HeroBanner
        key={`hero:${item.key}`}
        fill={fill}
        className="flex-1"
        poster={poster ? item.poster : undefined}
        tint={item.tint}
        eyebrow={eyebrow ? eyebrow(item) : item.kind === "movie" ? "New film" : "New series"}
        title={item.title}
        meta={<MetaLine items={item.meta} />}
        description={item.overview}
        actions={
          <div className="flex flex-wrap items-center gap-3">
            {item.mediaFileId ? <Button variant="primary" size="lg" icon={<IconPlay />} onClick={() => onPlay(item)} data-tv-autofocus data-focusable>Play</Button> : null}
            <Button variant={item.mediaFileId ? "secondary" : "primary"} size="lg" icon={<IconInfo />} nativeButton={false} render={detail} data-tv-autofocus={item.mediaFileId ? undefined : true} data-focusable>More info</Button>
            {items.length > 1 ? (
              <span className="ms-auto flex items-center gap-2" role="group" aria-label="Choose a title">
                <Button variant="ghost" shape="circle" icon={<IconChevronLeft className="rtl:rotate-180" />} aria-label="Previous title" onClick={() => step(-1)} data-focusable />
                <span className="text-numeric text-label-sm text-muted" aria-live="polite">{index + 1} / {items.length}</span>
                <Button variant="ghost" shape="circle" icon={<IconChevronRight className="rtl:rotate-180" />} aria-label="Next title" onClick={() => step(1)} data-focusable />
              </span>
            ) : null}
          </div>
        }
      />
    </div>
  );
}
