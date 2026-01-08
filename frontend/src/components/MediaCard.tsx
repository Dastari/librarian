import { Link } from '@tanstack/react-router'
import { Card } from '@heroui/react'
import type { MediaItem } from '../lib/graphql'

interface MediaCardProps {
  media: MediaItem
}

export function MediaCard({ media }: MediaCardProps) {
  return (
    <Link to="/library/$mediaId" params={{ mediaId: media.id }}>
      <Card
        isPressable
        className="overflow-hidden group hover:ring-2 hover:ring-primary transition-all"
      >
        {/* Poster */}
        <div className="aspect-[2/3] bg-content2 relative">
          {media.posterUrl ? (
            <img
              src={media.posterUrl}
              alt={media.title}
              className="w-full h-full object-cover"
            />
          ) : (
            <div className="w-full h-full flex items-center justify-center">
              <span className="text-default-500 text-4xl">
                {media.mediaType === 'movie' ? '🎬' : '📺'}
              </span>
            </div>
          )}

          {/* Info overlay */}
          <div className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/90 via-black/60 to-transparent p-4">
            <h3 className="text-white font-semibold truncate">{media.title}</h3>
            {media.year && (
              <p className="text-default-400 text-sm">{media.year}</p>
            )}
          </div>

          {/* Play button overlay on hover */}
          <div className="absolute inset-0 bg-black/40 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center">
            <div className="w-16 h-16 bg-white/20 rounded-full flex items-center justify-center backdrop-blur-sm">
              <span className="text-white text-3xl ml-1">▶</span>
            </div>
          </div>
        </div>
      </Card>
    </Link>
  )
}
