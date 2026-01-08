import { Link } from '@tanstack/react-router'
import { Button, Card, Chip, Image } from '@heroui/react'
import type { TvShow } from '../../lib/graphql'

function formatBytes(bytes: number | null): string {
  if (!bytes) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let unitIndex = 0
  let size = bytes
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024
    unitIndex++
  }
  return `${size.toFixed(1)} ${units[unitIndex]}`
}

export interface TvShowCardProps {
  show: TvShow
  onDelete: () => void
}

export function TvShowCard({ show, onDelete }: TvShowCardProps) {
  const missingEpisodes = show.episodeCount - show.episodeFileCount

  return (
    <Card className="bg-content1 overflow-hidden">
      <div className="flex">
        {show.posterUrl ? (
          <Image
            src={show.posterUrl}
            alt={show.name}
            className="w-24 h-36 object-cover flex-shrink-0"
            radius="none"
          />
        ) : (
          <div className="w-24 h-36 bg-default-200 flex items-center justify-center flex-shrink-0">
            <span className="text-4xl">📺</span>
          </div>
        )}
        <div className="flex-1 p-3">
          <div className="flex justify-between items-start mb-1">
            <h3 className="font-semibold line-clamp-1">
              {show.name}
              {show.year && (
                <span className="text-default-500 ml-1">({show.year})</span>
              )}
            </h3>
            <Chip
              size="sm"
              color={show.monitored ? 'success' : 'default'}
              variant="flat"
            >
              {show.monitored ? 'Monitored' : 'Unmonitored'}
            </Chip>
          </div>

          {show.network && (
            <p className="text-xs text-default-500 mb-2">{show.network}</p>
          )}

          <div className="flex gap-4 text-xs text-default-500 mb-2">
            <span>
              {show.episodeFileCount}/{show.episodeCount} episodes
            </span>
            <span>{formatBytes(show.sizeBytes)}</span>
            {missingEpisodes > 0 && (
              <Chip size="sm" color="warning" variant="flat">
                {missingEpisodes} missing
              </Chip>
            )}
          </div>

          <div className="flex gap-2">
            <Button size="sm" variant="flat" as={Link} to={`/shows/${show.id}`}>
              View
            </Button>
            <Button
              size="sm"
              color="danger"
              variant="flat"
              onPress={onDelete}
            >
              Delete
            </Button>
          </div>
        </div>
      </div>
    </Card>
  )
}
