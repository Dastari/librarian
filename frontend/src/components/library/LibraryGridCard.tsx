import { useCallback } from "react";
import { useNavigate } from "@tanstack/react-router";
import { Card } from "@heroui/card";
import {
  Dropdown,
  DropdownTrigger,
  DropdownMenu,
  DropdownItem,
} from "@heroui/dropdown";
import { Button } from "@heroui/button";
import {
  IconDotsVertical,
  IconRefresh,
  IconSettings,
  IconTrash,
  IconEye,
} from "@tabler/icons-react";
import type { LibraryNode, LibraryType } from "../../lib/graphql";
import { getLibraryTypeInfo } from "../../lib/graphql";

// ============================================================================
// Types
// ============================================================================

export interface LibraryGridCardProps {
  library: LibraryNode;
  showCount?: number;
  movieCount?: number;
  albumCount?: number;
  audiobookCount?: number;
  onScan: () => void;
  onDelete: () => void;
}

// ============================================================================
// Gradient backgrounds based on library type
// ============================================================================

const LIBRARY_GRADIENTS: Record<string, string> = {
  MOVIES: "from-violet-900 via-purple-800 to-fuchsia-900",
  TV: "from-blue-900 via-indigo-800 to-cyan-900",
  MUSIC: "from-emerald-900 via-green-800 to-teal-900",
  AUDIOBOOKS: "from-amber-900 via-orange-800 to-yellow-900",
  OTHER: "from-slate-800 via-gray-700 to-zinc-800",
};

// ============================================================================
// Component
// ============================================================================

export function LibraryGridCard({
  library,
  showCount,
  movieCount,
  albumCount,
  audiobookCount,
  onScan,
  onDelete,
}: LibraryGridCardProps) {
  const navigate = useNavigate();
  const typeInfo = getLibraryTypeInfo(library.LibraryType as LibraryType);
  const gradient =
    LIBRARY_GRADIENTS[library.LibraryType] || LIBRARY_GRADIENTS.OTHER;

  const handleCardClick = useCallback(() => {
    navigate({
      to: "/libraries/$libraryId",
      params: { libraryId: library.Id },
    });
  }, [navigate, library.Id]);

  // Get count based on library type
  const itemCount = (() => {
    if (library.LibraryType === "TV") return showCount ?? 0;
    if (library.LibraryType === "MOVIES") return movieCount ?? 0;
    if (library.LibraryType === "MUSIC") return albumCount ?? 0;
    if (library.LibraryType === "AUDIOBOOKS") return audiobookCount ?? 0;
    return 0;
  })();

  return (
    <Card className="relative overflow-hidden aspect-[2/3] group border-none bg-content2">
      {/* Clickable overlay for navigation - covers the entire card */}
      <button
        type="button"
        className="absolute inset-0 z-20 w-full h-full cursor-pointer bg-transparent border-none outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2"
        onClick={handleCardClick}
        aria-label={`Open ${library.Name} library`}
      />

      {/* Background gradient with icon */}
      <div className="absolute inset-0 w-full h-full">
        <div className={`absolute inset-0 bg-gradient-to-br ${gradient}`}>
          <div className="absolute inset-0 flex items-center justify-center opacity-30">
            <typeInfo.Icon size={80} />
          </div>
        </div>
      </div>

      {/* Type badge - top left */}
      <div className="absolute top-2 left-2 z-10 pointer-events-none">
        <div className="px-2 py-1 rounded-md bg-black/50 backdrop-blur-sm text-xs font-medium text-white/90">
          <typeInfo.Icon size={16} className="inline mr-1" />
          {typeInfo.label}
        </div>
      </div>

      {/* Bottom content */}
      <div className="absolute bottom-0 left-0 right-0 z-10 p-3 pointer-events-none bg-black/50 backdrop-blur-sm">
        <h3 className="text-sm font-bold text-white mb-0.5 line-clamp-2 drop-shadow-lg">
          {library.Name}
        </h3>
        <div className="flex items-center gap-1.5 text-xs text-white/70">
          <span>
            {itemCount}{" "}
            {library.LibraryType === "TV"
              ? itemCount === 1
                ? "Show"
                : "Shows"
              : library.LibraryType === "MOVIES"
                ? itemCount === 1
                  ? "Movie"
                  : "Movies"
                : library.LibraryType === "MUSIC"
                  ? itemCount === 1
                    ? "Album"
                    : "Albums"
                  : library.LibraryType === "AUDIOBOOKS"
                    ? itemCount === 1
                      ? "Audiobook"
                      : "Audiobooks"
                    : "Items"}
          </span>
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
              if (key === "view") {
                navigate({
                  to: "/libraries/$libraryId",
                  params: { libraryId: library.Id },
                });
              } else if (key === "scan") {
                onScan();
              } else if (key === "settings") {
                navigate({
                  to: "/libraries/$libraryId/settings",
                  params: { libraryId: library.Id },
                });
              } else if (key === "delete") {
                onDelete();
              }
            }}
          >
            <DropdownItem key="view" startContent={<IconEye size={16} />}>
              Open
            </DropdownItem>
            <DropdownItem key="scan" startContent={<IconRefresh size={16} />}>
              Scan
            </DropdownItem>
            <DropdownItem
              key="settings"
              startContent={<IconSettings size={16} />}
            >
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
  );
}
