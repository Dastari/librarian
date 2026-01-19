import { useState, useEffect, useCallback } from 'react'
import { useNavigate } from '@tanstack/react-router'
import { Card } from '@heroui/card'
import { Dropdown, DropdownTrigger, DropdownMenu, DropdownItem } from '@heroui/dropdown'
import { Button } from '@heroui/button'
import { Image } from '@heroui/image'
import { IconDotsVertical, IconRefresh, IconSettings, IconTrash, IconEye } from '@tabler/icons-react'
import type { Library, TvShow } from '../../lib/graphql'
import { getLibraryTypeInfo } from '../../lib/graphql'
import { formatBytes } from '../../lib/format'

// ============================================================================
// Types
// ============================================================================

export interface LibraryGridCardProps {
  library: Library
  shows?: TvShow[]
  onScan: () => void
  onDelete: () => void
}

// ============================================================================
// Gradient backgrounds based on library type
// ============================================================================

const LIBRARY_GRADIENTS: Record<string, string> = {
  MOVIES: 'from-violet-900 via-purple-800 to-fuchsia-900',
  TV: 'from-blue-900 via-indigo-800 to-cyan-900',
  MUSIC: 'from-emerald-900 via-green-800 to-teal-900',
  AUDIOBOOKS: 'from-amber-900 via-orange-800 to-yellow-900',
  OTHER: 'from-slate-800 via-gray-700 to-zinc-800',
}

// ============================================================================
// Component
// ============================================================================

export function LibraryGridCard({
  library,
  shows = [],
  onScan,
  onDelete,
}: LibraryGridCardProps) {
  const navigate = useNavigate()
  const typeInfo = getLibraryTypeInfo(library.libraryType)
  const gradient = LIBRARY_GRADIENTS[library.libraryType] || LIBRARY_GRADIENTS.OTHER

  // Get artwork from shows - prefer poster URLs (portrait) over backdrop URLs (landscape)
  const artworks = shows
    .filter((show) => show.posterUrl || show.backdropUrl)
    .map((show) => show.posterUrl || show.backdropUrl)
    .filter((url): url is string => !!url)
    .slice(0, 6) // Max 6 for cycling

  const [currentArtIndex, setCurrentArtIndex] = useState(0)
  const [isTransitioning, setIsTransitioning] = useState(false)

  // Cycle through artwork
  useEffect(() => {
    if (artworks.length <= 1) return

    const interval = setInterval(() => {
      setIsTransitioning(true)
      setTimeout(() => {
        setCurrentArtIndex((prev) => (prev + 1) % artworks.length)
        setIsTransitioning(false)
      }, 400)
    }, 4000) // Change every 4 seconds

    return () => clearInterval(interval)
  }, [artworks.length])

  const handleCardClick = useCallback(() => {
    navigate({ to: '/libraries/$libraryId', params: { libraryId: library.id } })
  }, [navigate, library.id])

  const currentArtwork = artworks[currentArtIndex]

  return (
    <Card
      className="relative overflow-hidden aspect-[2/3] group border-none bg-content2"
    >
      {/* Clickable overlay for navigation - covers the entire card */}
      <button
        type="button"
        className="absolute inset-0 z-20 w-full h-full cursor-pointer bg-transparent border-none outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2"
        onClick={handleCardClick}
        aria-label={`Open ${library.name} library`}
      />

      {/* Background artwork with gradient overlay */}
      <div className="absolute inset-0 w-full h-full">
        {currentArtwork ? (
          <>
            <Image
              src={currentArtwork}
              alt={library.name}
              classNames={{
                wrapper: `absolute inset-0 w-full h-full !max-w-full transition-opacity duration-800 ${
                  isTransitioning ? 'opacity-0' : 'opacity-100'
                }`,
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
          <div className={`absolute inset-0 bg-gradient-to-br ${gradient}`}>
            <div className="absolute inset-0 flex items-center justify-center opacity-30">
              <typeInfo.Icon size={80} />
            </div>
          </div>
        )}
      </div>

      {/* Type badge - top left */}
      <div className="absolute top-2 left-2 z-10 pointer-events-none">
        <div className="px-2 py-1 rounded-md bg-black/50 backdrop-blur-sm text-xs font-medium text-white/90">
          <typeInfo.Icon size={16} className="inline mr-1" />{typeInfo.label}
        </div>
      </div>

      {/* Artwork indicator dots - only if multiple artworks */}
      {artworks.length > 1 && (
        <div className="absolute top-2 right-2 z-10 flex gap-1 pointer-events-none">
          {artworks.map((_, idx) => (
            <div
              key={idx}
              className={`w-1.5 h-1.5 rounded-full transition-all duration-300 ${
                idx === currentArtIndex ? 'bg-white' : 'bg-white/40'
              }`}
            />
          ))}
        </div>
      )}

      {/* Bottom content */}
      <div className="absolute bottom-0 left-0 right-0 z-10 p-3 pointer-events-none bg-black/50 backdrop-blur-sm">
        <h3 className="text-sm font-bold text-white mb-0.5 line-clamp-2 drop-shadow-lg">
          {library.name}
        </h3>
        <div className="flex items-center gap-1.5 text-xs text-white/70">
          <span>
            {library.libraryType === 'TV'
              ? `${library.showCount} Shows`
              : library.libraryType === 'MOVIES'
              ? `${library.movieCount} Movies`
              : `${library.itemCount} Items`}
          </span>
          <span>•</span>
          <span>{formatBytes(library.totalSizeBytes)}</span>
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
            >
              <IconDotsVertical size={16} />
            </Button>
          </DropdownTrigger>
          <DropdownMenu
            aria-label="Library actions"
            onAction={(key) => {
              if (key === 'view') {
                navigate({ to: '/libraries/$libraryId', params: { libraryId: library.id } })
              } else if (key === 'scan') {
                onScan()
              } else if (key === 'settings') {
                navigate({ to: '/libraries/$libraryId/settings', params: { libraryId: library.id } })
              } else if (key === 'delete') {
                onDelete()
              }
            }}
          >
            <DropdownItem key="view" startContent={<IconEye size={16} />}>
              Open
            </DropdownItem>
            <DropdownItem key="scan" startContent={<IconRefresh size={16} />}>
              Scan
            </DropdownItem>
            <DropdownItem key="settings" startContent={<IconSettings size={16} />}>
              Settings
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
  )
}
