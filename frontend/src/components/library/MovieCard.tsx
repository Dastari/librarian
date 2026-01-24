import { Link, useNavigate } from '@tanstack/react-router'
import { Card } from '@heroui/card'
import { Dropdown, DropdownTrigger, DropdownMenu, DropdownItem } from '@heroui/dropdown'
import { Button } from '@heroui/button'
import { Image } from '@heroui/image'
import type { Movie } from '../../lib/graphql'
import { IconEye, IconTrash, IconMovie, IconCheck, IconDotsVertical, IconClock } from '@tabler/icons-react'

// ============================================================================
// Types
// ============================================================================

export interface MovieCardProps {
  movie: Movie
  onDelete: () => void
}

// ============================================================================
// Component
// ============================================================================

export function MovieCard({ movie, onDelete }: MovieCardProps) {
  const navigate = useNavigate()

  return (
    <div className="aspect-[2/3]">
      <Card
        className="relative overflow-hidden h-full w-full group border-none bg-content2"
      >
      {/* Clickable overlay for navigation - covers the entire card */}
      <Link
        to="/movies/$movieId"
        params={{ movieId: movie.id }}
        className="absolute inset-0 z-20 w-full h-full cursor-pointer bg-transparent border-none outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2"
        aria-label={`View ${movie.title}`}
      />

      {/* Background artwork with gradient overlay */}
      <div className="absolute inset-0 w-full h-full">
        {movie.posterUrl ? (
          <>
            <Image
              src={movie.posterUrl}
              alt={movie.title}
              classNames={{
                wrapper: "absolute inset-0 w-full h-full !max-w-full",
                img: "w-full h-full object-cover"
              }}
              radius="none"
              removeWrapper={false}
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

      {/* Status badge - top left */}
      <div className="absolute top-2 left-2 z-10 pointer-events-none">
        <div
          className={`px-2 py-1 rounded-md backdrop-blur-sm text-xs font-medium ${
            movie.mediaFileId
              ? 'bg-success/80 text-success-foreground'
              : 'bg-warning/80 text-warning-foreground'
          }`}
        >
          {movie.mediaFileId ? <><IconCheck size={12} className="inline mr-1 text-green-400" />Downloaded</> : 'Missing'}
        </div>
      </div>

      {/* Rating badge - top right */}
      {movie.tmdbRating && movie.tmdbRating > 0 && (
        <div className="absolute top-2 right-2 z-10 pointer-events-none">
          <div
            className={`px-2 py-1 rounded-md backdrop-blur-sm text-xs font-semibold ${
              movie.tmdbRating >= 7
                ? 'bg-success/80 text-success-foreground'
                : movie.tmdbRating >= 5
                ? 'bg-warning/80 text-warning-foreground'
                : 'bg-danger/80 text-danger-foreground'
            }`}
          >
            {movie.tmdbRating.toFixed(1)}
          </div>
        </div>
      )}

      {/* Bottom content */}
      <div className="absolute bottom-0 left-0 right-0 z-10 p-3 pointer-events-none bg-black/50 backdrop-blur-sm h-20 flex flex-col">
        <h3 className="text-sm font-bold text-white mb-0.5 line-clamp-2 drop-shadow-lg grow">
          {movie.title}
          {movie.year && <span className="font-normal opacity-70"> ({movie.year})</span>}
        </h3>
        <div className="flex items-center gap-1.5 text-xs text-white/70">
          {movie.runtime && (
            <>
              <IconClock size={12} />
              <span>{Math.floor(movie.runtime / 60)}h {movie.runtime % 60}m</span>
            </>
          )}
          {movie.mediaFileId && (
            <>
              <span>•</span>
              <IconCheck size={12} className="text-success" />
            </>
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
      <div className="absolute bottom-2 right-2 z-20 opacity-0 group-hover:opacity-100 transition-opacity duration-200">
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
              if (key === 'view') {
                navigate({ to: '/movies/$movieId', params: { movieId: movie.id } })
              } else if (key === 'delete') {
                onDelete()
              }
            }}
          >
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
  )
}
