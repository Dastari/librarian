import { Link, useNavigate } from '@tanstack/react-router'
import { Card } from '@heroui/card'
import { Dropdown, DropdownTrigger, DropdownMenu, DropdownItem } from '@heroui/dropdown'
import { Button } from '@heroui/button'
import { Image } from '@heroui/image'
import type { Show } from '../../lib/graphql/generated/graphql'
import {
  IconEye,
  IconTrash,
  IconDeviceTv,
  IconDotsVertical,
  IconPlayerPlay,
} from "@tabler/icons-react";

// ============================================================================
// Types
// ============================================================================

export interface TvShowCardProps {
  show: Show
  onDelete: () => void
}

// ============================================================================
// Component
// ============================================================================

export function TvShowCard({ show, onDelete }: TvShowCardProps) {
  const navigate = useNavigate()

  return (
    <div className="aspect-[2/3]">
      <Card className="relative overflow-hidden h-full w-full group border-none bg-content2">
        {/* Clickable overlay for navigation - covers the entire card */}
        <Link
          to="/shows/$showId"
          params={{ showId: show.Id }}
          className="absolute inset-0 z-20 w-full h-full cursor-pointer bg-transparent border-none outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2"
          aria-label={`View ${show.Name}`}
        />

        {/* Background artwork with gradient overlay */}
        <div className="absolute inset-0 w-full h-full">
          {show.PosterUrl ? (
            <>
              <Image
                src={show.PosterUrl}
                alt={show.Name}
                loading="lazy"
                classNames={{
                  wrapper: "absolute inset-0 w-full h-full !max-w-full",
                  img: "w-full h-full object-cover",
                }}
                radius="none"
                removeWrapper={false}
              />
              {/* Dark gradient overlay for text readability */}
              <div className="absolute inset-0 bg-gradient-to-t from-black/90 via-black/20 to-black/40" />
            </>
          ) : (
            // Fallback gradient background with icon
            <div className="absolute inset-0 bg-gradient-to-br from-blue-900 via-indigo-800 to-cyan-900">
              <div className="absolute inset-0 flex items-center justify-center opacity-30">
                <IconDeviceTv size={64} className="text-blue-400" />
              </div>
            </div>
          )}
        </div>

        {/* Year badge - top left */}
        {show.Year && (
          <div className="absolute top-2 left-2 z-10 pointer-events-none">
            <div className="px-2 py-1 rounded-md backdrop-blur-sm text-xs font-medium bg-default-100/80 text-default-foreground">
              {show.Year}
            </div>
          </div>
        )}

        {/* Bottom content */}
        <div className="absolute bottom-0 left-0 right-0 z-10 p-3 pointer-events-none bg-black/50 backdrop-blur-sm h-20 flex flex-col">
          <h3 className="text-sm font-bold text-white mb-0.5 line-clamp-2 drop-shadow-lg grow">
            {show.Name}
            {show.Year != null && (
              <span className="font-normal opacity-70"> ({show.Year})</span>
            )}
          </h3>
          <div className="flex items-center gap-1.5 text-xs text-white/70">
            {show.Network && <span className="truncate">{show.Network}</span>}
          </div>
        </div>

        {/* Play overlay button (open detail for playback controls) */}
        <div className="absolute inset-0 z-30 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity duration-200 pointer-events-none">
          <Link
            to="/shows/$showId"
            params={{ showId: show.Id }}
            className="pointer-events-auto w-14 h-14 rounded-full bg-primary/90 text-white flex items-center justify-center shadow-lg hover:scale-110 transition-transform"
            aria-label={`Open ${show.Name} to play`}
          >
            <IconPlayerPlay size={28} className="ml-1" />
          </Link>
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
                aria-label="Show actions"
              >
                <IconDotsVertical size={16} />
              </Button>
            </DropdownTrigger>
            <DropdownMenu
              aria-label="Show actions menu"
              onAction={(key) => {
                if (key === "view") {
                  navigate({
                    to: "/shows/$showId",
                    params: { showId: show.Id },
                  });
                } else if (key === "delete") {
                  onDelete();
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
  );
}
