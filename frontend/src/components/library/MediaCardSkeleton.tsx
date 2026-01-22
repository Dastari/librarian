import { Card } from '@heroui/card'
import { Skeleton } from '@heroui/skeleton'

/**
 * Skeleton card for media grid loading states.
 * Matches the aspect ratio and structure of TvShowCard, MovieCard, AlbumCard, etc.
 */
export function MediaCardSkeleton() {
  return (
    <div className="aspect-[2/3]">
      <Card className="relative overflow-hidden h-full w-full border-none bg-content2">
        {/* Background skeleton */}
        <Skeleton className="absolute inset-0 w-full h-full rounded-none" />

        {/* Status badge skeleton - top left */}
        <div className="absolute top-2 left-2 z-10">
          <Skeleton className="w-20 h-5 rounded-md" />
        </div>

        {/* Bottom content skeleton */}
        <div className="absolute bottom-0 left-0 right-0 z-10 p-3 bg-black/50 h-20 flex flex-col gap-2">
          {/* Title skeleton */}
          <Skeleton className="w-3/4 h-4 rounded" />
          {/* Subtitle skeleton */}
          <Skeleton className="w-1/2 h-3 rounded" />
        </div>
      </Card>
    </div>
  )
}

/**
 * Square skeleton card for album/artist grid loading states.
 */
export function SquareCardSkeleton() {
  return (
    <div className="aspect-square">
      <Card className="relative overflow-hidden h-full w-full border-none bg-content2">
        {/* Background skeleton */}
        <Skeleton className="absolute inset-0 w-full h-full rounded-none" />

        {/* Bottom content skeleton */}
        <div className="absolute bottom-0 left-0 right-0 z-10 p-3 bg-black/50 h-16 flex flex-col gap-2">
          {/* Title skeleton */}
          <Skeleton className="w-3/4 h-4 rounded" />
          {/* Subtitle skeleton */}
          <Skeleton className="w-1/2 h-3 rounded" />
        </div>
      </Card>
    </div>
  )
}
