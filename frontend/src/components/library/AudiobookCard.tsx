import { Card } from '@heroui/card'
import { Dropdown, DropdownTrigger, DropdownMenu, DropdownItem } from '@heroui/dropdown'
import { Button } from '@heroui/button'
import { Image } from '@heroui/image'
import { Link } from '@tanstack/react-router'
import type { Audiobook } from '../../lib/graphql'
import { IconEye, IconTrash, IconHeadphones, IconCheck, IconDotsVertical, IconClock } from '@tabler/icons-react'

// ============================================================================
// Types
// ============================================================================

export interface AudiobookCardProps {
  audiobook: Audiobook
  authorName?: string
  onDelete?: () => void
}

// ============================================================================
// Utility Functions
// ============================================================================

function formatDuration(seconds: number | null): string {
  if (!seconds) return ''
  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  if (hours > 0) {
    return `${hours}h ${minutes}m`
  }
  return `${minutes}m`
}

// ============================================================================
// Component
// ============================================================================

export function AudiobookCard({ audiobook, authorName, onDelete }: AudiobookCardProps) {

  return (
    <div className="aspect-[2/3]">
      <Card
        className="relative overflow-hidden h-full w-full group border-none bg-content2"
      >
        {/* Clickable overlay for navigation - covers the entire card */}
        <Link
          to="/audiobooks/$audiobookId"
          params={{ audiobookId: audiobook.id }}
          className="absolute inset-0 z-20 w-full h-full cursor-pointer bg-transparent border-none outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2"
          aria-label={`View ${audiobook.title}`}
        />

        {/* Background artwork with gradient overlay */}
        <div className="absolute inset-0 w-full h-full">
          {audiobook.coverUrl ? (
            <>
              <Image
                src={audiobook.coverUrl}
                alt={audiobook.title}
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
            <div className="absolute inset-0 bg-gradient-to-br from-orange-900 via-amber-800 to-yellow-900">
              <div className="absolute inset-0 flex items-center justify-center opacity-30">
                <IconHeadphones size={64} className="text-orange-400" />
              </div>
            </div>
          )}
        </div>

        {/* Progress badge - top left */}
        <div className="absolute top-2 left-2 z-10 pointer-events-none">
          {(() => {
            const downloaded = audiobook.downloadedChapterCount ?? 0
            const total = audiobook.chapterCount ?? 0
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

        {/* Duration badge - top right */}
        {audiobook.durationSecs && audiobook.durationSecs > 0 && (
          <div className="absolute top-2 right-2 z-10 pointer-events-none">
            <div className="px-2 py-1 rounded-md bg-black/50 backdrop-blur-sm text-xs font-medium text-white/90">
              <IconClock size={12} className="inline mr-1" />
              {formatDuration(audiobook.durationSecs)}
            </div>
          </div>
        )}

        {/* Bottom content */}
        <div className="absolute bottom-0 left-0 right-0 z-10 p-3 pointer-events-none bg-black/50 backdrop-blur-sm h-20 flex flex-col">
          <h3 className="text-sm font-bold text-white mb-0.5 line-clamp-2 drop-shadow-lg grow">
            {audiobook.title}
          </h3>
          <div className="flex items-center gap-1.5 text-xs text-white/70">
            {authorName && (
              <span className="truncate">{authorName}</span>
            )}
            {audiobook.seriesName && (
              <>
                <span>•</span>
                <span className="truncate">{audiobook.seriesName}</span>
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
                >
                  <IconDotsVertical size={16} />
                </Button>
              </DropdownTrigger>
              <DropdownMenu
                aria-label="Audiobook actions"
                onAction={(key) => {
                  if (key === 'view') {
                    // View details when route is available
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
