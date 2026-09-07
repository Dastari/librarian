import { Link } from "@tanstack/react-router";
import { IconLoader2 } from "@tabler/icons-react";
import { useMemo } from "react";

import { PosterMarquee } from "@/components/ui/PosterMarquee";
import type { LibrariesOverviewQuery } from "@/graphql/generated/graphql";
import { albumCover, audiobookCover, moviePoster, showPoster } from "@/lib/artwork";
import { formatRelative, pluralize } from "@/lib/format";
import { libraryType } from "@/lib/library-types";
import { cn } from "@/lib/utils";

export type LibraryOverview = LibrariesOverviewQuery["libraries"]["edges"][number]["node"];

export function libraryItemCount(library: LibraryOverview): { count: number; noun: string } {
  const meta = libraryType(library.libraryType);
  switch (meta.type) {
    case "movies":
      return { count: library.movies.pageInfo.totalCount ?? 0, noun: "movie" };
    case "tv":
      return { count: library.shows.pageInfo.totalCount ?? 0, noun: "show" };
    case "music":
      return { count: library.albums.pageInfo.totalCount ?? 0, noun: "album" };
    case "audiobooks":
      return { count: library.audiobooks.pageInfo.totalCount ?? 0, noun: "book" };
    default:
      return { count: library.mediaFiles.pageInfo.totalCount ?? 0, noun: "file" };
  }
}

/** Recent artwork for the library's primary item type, used for the rolling preview. */
export function libraryPreviewImages(library: LibraryOverview): string[] {
  const meta = libraryType(library.libraryType);
  switch (meta.type) {
    case "movies":
      return library.movies.edges.map((edge) => moviePoster(edge.node.id));
    case "tv":
      return library.shows.edges.map((edge) => showPoster(edge.node));
    case "music":
      return library.albums.edges.map((edge) => albumCover(edge.node));
    case "audiobooks":
      return library.audiobooks.edges.map((edge) => audiobookCover(edge.node));
    default:
      return [];
  }
}

interface LibraryCardProps {
  library: LibraryOverview;
  className?: string;
  size?: "md" | "lg";
}

/**
 * Library tile: a slow marquee of the library's own artwork behind a tinted glass surface,
 * with the type icon, name and counts on top.
 */
export function LibraryCard({ library, className, size = "md" }: LibraryCardProps) {
  const meta = libraryType(library.libraryType);
  const { count, noun } = libraryItemCount(library);
  const images = useMemo(() => libraryPreviewImages(library), [library]);
  return (
    <Link
      to="/libraries/$libraryId"
      params={{ libraryId: library.id }}
      data-focusable
      className={cn(
        "group nav-focus relative isolate flex flex-col justify-between overflow-hidden rounded-card border border-glass-rim transition-[transform,box-shadow] duration-base ease-fluid hover:-translate-y-0.5 hover:shadow-poster-hover",
        size === "lg" ? "min-h-48" : "min-h-40",
        className,
      )}
      style={{ ["--placeholder-tint" as string]: meta.tintVar }}
    >
      <div className="placeholder-gradient absolute inset-0 -z-20" />
      <PosterMarquee images={images} className="-z-10 opacity-80 saturate-[0.9] transition-opacity duration-slow group-hover:opacity-100" duration={images.length * 9} height={size === "lg" ? 210 : 170} square={meta.aspect === "square"} />
      {/* Glass band: the artwork stays crisp above, the text sits on frosted glass below. */}
      <div className="glass-surface glass-highlight absolute inset-x-0 bottom-0 -z-10 h-[46%] rounded-b-card border-x-0 border-b-0 shadow-none" />
      <div className="absolute inset-0 -z-10 bg-gradient-to-t from-background/70 via-transparent to-background/20" />
      <div className="relative flex items-start justify-between p-4">
        <span className={cn("glass-control grid size-11 place-items-center rounded-xl", meta.tint)}>
          <meta.icon size={24} stroke={1.75} />
        </span>
        {library.scanning ? (
          <span className="glass-control inline-flex items-center gap-1.5 rounded-pill px-2 py-1 text-label-sm text-foreground">
            <IconLoader2 size={12} className="animate-spin" /> Scanning
          </span>
        ) : null}
      </div>
      <div className="relative p-4 pt-0">
        <p className="truncate text-title-lg text-foreground text-shadow-hero">{library.name}</p>
        <p className="text-label-sm text-foreground/75">
          {pluralize(count, noun)}
          {library.lastScannedAt ? ` · scanned ${formatRelative(library.lastScannedAt)}` : ""}
        </p>
      </div>
    </Link>
  );
}
