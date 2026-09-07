import { Link, useNavigate } from "@tanstack/react-router";
import { Card } from "@heroui/card";
import {
  Dropdown,
  DropdownTrigger,
  DropdownMenu,
  DropdownItem,
} from "@heroui/dropdown";
import { Button } from "@heroui/button";
import { Image } from "@heroui/image";
import type {
  ContentStatus,
  Movie,
} from "../../lib/graphql/generated/graphql";
import {
  IconEye,
  IconTrash,
  IconMovie,
  IconDotsVertical,
  IconPlayerPlay,
  IconPlayerPause,
  IconStar,
} from "@tabler/icons-react";

// ============================================================================
// Types
// ============================================================================

export interface MovieCardProps {
  movie: Movie;
  onDelete: () => void;
  onPlay?: (movie: Movie) => void;
  isCurrentMovie?: boolean;
  isPlaying?: boolean;
  status?: ContentStatus;
}

// ============================================================================
// Component
// ============================================================================

export function MovieCard({
  movie,
  onDelete,
  onPlay,
  isCurrentMovie = false,
  isPlaying = false,
  status,
}: MovieCardProps) {
  const navigate = useNavigate();
  const displayStatus =
    status ??
    (movie.mediaFileId ? "AVAILABLE" : movie.wanted ? "WANTED" : "MISSING");

  return (
    <div className="aspect-[2/3] w-full">
      <Card className="relative isolate overflow-hidden h-full w-full group border-none bg-content2">
        {/* Clickable overlay for navigation - covers the entire card */}
        <Link
          to="/movies/$movieId"
          params={{ movieId: movie.id }}
          className="absolute inset-0 z-20 w-full h-full cursor-pointer bg-transparent border-none outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2"
          aria-label={`View ${movie.title}`}
        />

        {/* Background artwork with gradient overlay */}
        <div className="absolute inset-0 w-full h-full">
          {movie.collectionPosterUrl ? (
            <>
              <Image
                src={movie.collectionPosterUrl}
                alt={movie.title}
                loading="eager"
                className="absolute inset-0 h-full w-full object-cover"
                radius="none"
                removeWrapper
              />
              {/* Dark gradient overlay for text readability */}
              <div className="absolute inset-0 bg-gradient-to-t from-black/90 via-black/20 to-black/40" />
            </>
          ) : (
            // Fallback gradient background with icon
            <div className="absolute inset-0 bg-gradient-to-br from-purple-900 via-indigo-800 to-pink-900">
              <div className="absolute inset-0 flex items-center justify-center opacity-30">
                <IconMovie size={64} className="text-purple-400" />
              </div>
            </div>
          )}
        </div>

        {/* Status badge - top left (only for wanted/missing) */}
        {!["AVAILABLE", "PLAYING"].includes(displayStatus) && (
          <div className="absolute top-2 left-2 z-10 pointer-events-none">
            <div className="px-2 py-1 rounded-md backdrop-blur-sm text-xs font-medium bg-black/45 text-white/90">
              {displayStatus.charAt(0) + displayStatus.slice(1).toLowerCase()}
            </div>
          </div>
        )}

        {/* Rating badge - top right */}
        {movie.tmdbRating && Number(movie.tmdbRating) > 0 && (
          <div className="absolute top-2 right-2 z-10 pointer-events-none">
            <div
              className={`px-2 py-1 rounded-md backdrop-blur-sm text-xs font-semibold ${
                Number(movie.tmdbRating) >= 7
                  ? "bg-success/80 text-success-foreground"
                  : Number(movie.tmdbRating) >= 5
                    ? "bg-warning/80 text-warning-foreground"
                    : "bg-danger/80 text-danger-foreground"
              }`}
            >
              <span className="inline-flex items-center gap-1">
                <IconStar size={10} />
                {Number(movie.tmdbRating).toFixed(1)}
              </span>
            </div>
          </div>
        )}

        {/* Play/Pause overlay button */}
        {movie.mediaFileId && onPlay && (
          <div className="absolute inset-0 z-30 flex items-center justify-center bg-black/40 opacity-0 group-hover:opacity-100 transition-opacity duration-200 rounded-lg pointer-events-none">
            <button
              type="button"
              onClick={(event) => {
                event.preventDefault();
                event.stopPropagation();
                onPlay(movie);
              }}
              className={`pointer-events-auto w-14 h-14 rounded-full ${isCurrentMovie && isPlaying ? "bg-warning" : "bg-primary"} flex items-center justify-center shadow-lg hover:scale-110 transition-transform`}
              aria-label={
                isCurrentMovie && isPlaying ? "Pause Movie" : "Play Movie"
              }
            >
              {isCurrentMovie && isPlaying ? (
                <IconPlayerPause size={28} className="text-white" />
              ) : (
                <IconPlayerPlay size={28} className="text-white ml-1" />
              )}
            </button>
          </div>
        )}

        {/* Bottom content */}
        <div className="absolute bottom-0 left-0 right-0 z-10 h-20 overflow-hidden rounded-b-[inherit] bg-black/50 p-3 pointer-events-none backdrop-blur-sm flex flex-col">
          <h3 className="text-sm font-bold text-white mb-0.5 line-clamp-2 drop-shadow-lg grow">
            {movie.title}
            {movie.year != null && (
              <span className="font-normal opacity-70"> ({movie.year})</span>
            )}
          </h3>
          <div className="flex items-center gap-1.5 text-xs text-white/70">
            {movie.runtime != null && (
              <span className="text-nowrap">
                {Math.floor(movie.runtime / 60)}h {movie.runtime % 60}m
              </span>
            )}
            {movie.genres.length > 0 && (
              <>
                <span>•</span>
                <span className="truncate">{movie.genres[0]}</span>
              </>
            )}
          </div>
        </div>

        {/* Action menu - bottom right, visible on hover, above the clickable overlay */}
        <div className="absolute bottom-2 right-2 z-40 opacity-0 group-hover:opacity-100 transition-opacity duration-200">
          <Dropdown>
            <DropdownTrigger>
              <Button
                isIconOnly
                size="sm"
                variant="flat"
                className="bg-black/50 backdrop-blur-sm text-white hover:bg-black/70 min-w-6 w-6 h-6"
                aria-label="Movie actions"
              >
                <IconDotsVertical size={16} />
              </Button>
            </DropdownTrigger>
            <DropdownMenu
              aria-label="Movie actions"
              onAction={(key) => {
                if (key === "view") {
                  navigate({
                    to: "/movies/$movieId",
                    params: { movieId: movie.id },
                  });
                } else if (key === "play" && movie.mediaFileId && onPlay) {
                  onPlay(movie);
                } else if (key === "delete") {
                  onDelete();
                }
              }}
            >
              {movie.mediaFileId && onPlay ? (
                <DropdownItem
                  key="play"
                  startContent={<IconPlayerPlay size={16} />}
                >
                  Play
                </DropdownItem>
              ) : null}
              <DropdownItem key="view" startContent={<IconEye size={16} />}>
                View Details
              </DropdownItem>
              <DropdownItem
                key="delete"
                startContent={<IconTrash size={16} className="text-red-400" />}
                className="text-danger"
                color="danger"
              >
                Delete
              </DropdownItem>
            </DropdownMenu>
          </Dropdown>
        </div>
      </Card>
    </div>
  );
}
