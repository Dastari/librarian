import { Link, useNavigate } from '@tanstack/react-router'
import { Card } from '@heroui/card'
import { Dropdown, DropdownTrigger, DropdownMenu, DropdownItem } from '@heroui/dropdown'
import { Button } from '@heroui/button'
import { Image } from '@heroui/image'
import type { Album } from '../../lib/graphql'
import { IconEye, IconTrash, IconDisc, IconCheck, IconDotsVertical } from '@tabler/icons-react'

// ============================================================================
// Types
// ============================================================================

export interface AlbumCardProps {
  album: Album
  artistName?: string
  onDelete?: () => void
}

// ============================================================================
// Component
// ============================================================================

export function AlbumCard({ album, artistName, onDelete }: AlbumCardProps) {
  const navigate = useNavigate()

  return (
    <div className="aspect-square">
      <Card
        className="relative overflow-hidden h-full w-full group border-none bg-content2"
      >
        {/* Clickable overlay for navigation - covers the entire card */}
        <Link
          to="/albums/$albumId"
          params={{ albumId: album.id }}
          className="absolute inset-0 z-20 w-full h-full cursor-pointer bg-transparent border-none outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2"
          aria-label={`View ${album.name}`}
        />

        {/* Background artwork with gradient overlay */}
        <div className="absolute inset-0 w-full h-full">
          {album.coverUrl ? (
            <>
              <Image
                src={album.coverUrl}
                alt={album.name}
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
            <div className="absolute inset-0 bg-gradient-to-br from-green-900 via-emerald-800 to-teal-900">
              <div className="absolute inset-0 flex items-center justify-center opacity-30">
                <IconDisc size={64} className="text-green-400" />
              </div>
            </div>
          )}
        </div>

        {/* Progress badge - top left */}
        <div className="absolute top-2 left-2 z-10 pointer-events-none">
          {(() => {
            const downloaded = album.downloadedTrackCount ?? 0
            const total = album.trackCount ?? 0
            const isComplete = total > 0 && downloaded >= total
            return (
              <div
                className={`px-2 py-1 rounded-md backdrop-blur-sm text-xs font-medium ${
                  isComplete
                    ? 'bg-success/80 text-success-foreground'
                    : 'bg-warning/80 text-warning-foreground'
                }`}
              >
                {isComplete && <IconCheck size={12} className="inline mr-1" />}
                {downloaded}/{total}
              </div>
            )
          })()}
        </div>

        {/* Bottom content */}
        <div className="absolute bottom-0 left-0 right-0 z-10 p-3 pointer-events-none bg-black/50 backdrop-blur-sm h-20 flex flex-col">
          <h3 className="text-sm font-bold text-white mb-0.5 line-clamp-2 drop-shadow-lg grow">
            {album.name}
            {album.year && <span className="font-normal opacity-70"> ({album.year})</span>}
          </h3>
          <div className="flex items-center gap-1.5 text-xs text-white/70">
            {artistName && (
              <span className="truncate">{artistName}</span>
            )}
            {album.genres && album.genres.length > 0 && (
              <>
                <span>•</span>
                <span className="truncate">{album.genres[0]}</span>
              </>
            )}
          </div>
        </div>

        {/* Action menu - bottom right, visible on hover, above the clickable overlay */}
        {onDelete && (
          <div className="absolute bottom-2 right-2 z-30 opacity-0 group-hover:opacity-100 transition-opacity duration-200">
            <Dropdown>
              <DropdownTrigger>
                <Button
                  isIconOnly
                  size="sm"
                  variant="flat"
                  className="bg-black/50 backdrop-blur-sm text-white hover:bg-black/70 min-w-6 w-6 h-6"
                  aria-label="Album actions"
                >
                  <IconDotsVertical size={16} />
                </Button>
              </DropdownTrigger>
              <DropdownMenu
                aria-label="Album actions menu"
                onAction={(key) => {
                  if (key === 'view') {
                    navigate({ to: '/albums/$albumId', params: { albumId: album.id } })
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
        )}
      </Card>
    </div>
  )
}
