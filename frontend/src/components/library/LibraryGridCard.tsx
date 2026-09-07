import { useCallback, useEffect, useMemo, useRef, useState } from "react";
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
  IconSettings,
  IconTrash,
  IconEye,
  IconRefresh,
  IconPlugConnected,
} from "@tabler/icons-react";
import { getLibraryTypeInfo } from "../../lib/graphql";
import type { LibrariesQuery } from "@/lib/graphql/generated/graphql";
import type { LibraryPathAvailabilityStatus } from "../../lib/graphql";
type LibraryType = "MOVIES" | "TV" | "MUSIC" | "AUDIOBOOKS" | "OTHER";

type LibraryGridNode = LibrariesQuery["libraries"]["edges"][number]["node"];

// ============================================================================
// Types
// ============================================================================

export interface LibraryGridCardProps {
  library: LibraryGridNode;
  onScan: () => void;
  onDelete: () => void;
  pathStatus?: LibraryPathAvailabilityStatus;
  onReconnect: () => void;
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
  onScan,
  onDelete,
  pathStatus,
  onReconnect,
}: LibraryGridCardProps) {
  const navigate = useNavigate();
  const typeInfo = getLibraryTypeInfo(library.libraryType as LibraryType);
  const gradient =
    LIBRARY_GRADIENTS[library.libraryType] || LIBRARY_GRADIENTS.OTHER;
  const recentArtworkUrls = useMemo(() => {
    if (library.libraryType === "MOVIES") {
      return (library.movieArtwork?.edges ?? [])
        .map((edge) => edge.node.collectionPosterUrl)
        .filter((url): url is string => Boolean(url));
    }

    if (library.libraryType === "TV") {
      return (library.showArtwork?.edges ?? [])
        .map((edge) => edge.node.posterUrl)
        .filter((url): url is string => Boolean(url));
    }

    if (library.libraryType === "MUSIC") {
      return (library.albumArtwork?.edges ?? [])
        .map((edge) => edge.node.coverUrl)
        .filter((url): url is string => Boolean(url));
    }

    if (library.libraryType === "AUDIOBOOKS") {
      return (library.audiobookArtwork?.edges ?? [])
        .map((edge) => edge.node.coverUrl)
        .filter((url): url is string => Boolean(url));
    }

    return [];
  }, [
    library.libraryType,
    library.movieArtwork?.edges,
    library.showArtwork?.edges,
    library.albumArtwork?.edges,
    library.audiobookArtwork?.edges,
  ]);
  const coverSignature = useMemo(
    () => recentArtworkUrls.join("|"),
    [recentArtworkUrls],
  );
  const [frontCoverIndex, setFrontCoverIndex] = useState(0);
  const [backCoverIndex, setBackCoverIndex] = useState<number | null>(null);
  const [showFrontLayer, setShowFrontLayer] = useState(true);
  const intervalRef = useRef<number | null>(null);
  const frameRef = useRef<number | null>(null);
  const frontCoverIndexRef = useRef(0);
  const backCoverIndexRef = useRef<number | null>(null);
  const showFrontLayerRef = useRef(true);

  useEffect(() => {
    setFrontCoverIndex(0);
    setBackCoverIndex(null);
    setShowFrontLayer(true);
    frontCoverIndexRef.current = 0;
    backCoverIndexRef.current = null;
    showFrontLayerRef.current = true;
  }, [library.id, coverSignature]);

  useEffect(() => {
    if (intervalRef.current !== null) {
      window.clearInterval(intervalRef.current);
      intervalRef.current = null;
    }
    if (frameRef.current !== null) {
      window.cancelAnimationFrame(frameRef.current);
      frameRef.current = null;
    }
    if (recentArtworkUrls.length < 2) return;

    intervalRef.current = window.setInterval(() => {
      if (showFrontLayerRef.current) {
        const nextBackIndex =
          (frontCoverIndexRef.current + 1) % recentArtworkUrls.length;
        setBackCoverIndex(nextBackIndex);
        backCoverIndexRef.current = nextBackIndex;
        frameRef.current = window.requestAnimationFrame(() => {
          setShowFrontLayer(false);
          showFrontLayerRef.current = false;
        });
      } else {
        const currentBackIndex =
          backCoverIndexRef.current ?? frontCoverIndexRef.current;
        const nextFrontIndex =
          (currentBackIndex + 1) % recentArtworkUrls.length;
        setFrontCoverIndex(nextFrontIndex);
        frontCoverIndexRef.current = nextFrontIndex;
        frameRef.current = window.requestAnimationFrame(() => {
          setShowFrontLayer(true);
          showFrontLayerRef.current = true;
        });
      }
    }, 5000);

    return () => {
      if (intervalRef.current !== null) {
        window.clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
      if (frameRef.current !== null) {
        window.cancelAnimationFrame(frameRef.current);
        frameRef.current = null;
      }
    };
  }, [recentArtworkUrls.length]);

  const frontCoverUrl = recentArtworkUrls[frontCoverIndex];
  const backCoverUrl =
    backCoverIndex !== null ? recentArtworkUrls[backCoverIndex] : null;

  const handleCardClick = useCallback(() => {
    navigate({
      to: "/libraries/$libraryId",
      params: { libraryId: library.id },
    });
  }, [navigate, library.id]);

  // Get count based on library type
  const itemCount = (() => {
    if (library.libraryType === "TV")
      return library.shows?.pageInfo?.totalCount ?? 0;
    if (library.libraryType === "MOVIES")
      return library.movies?.pageInfo?.totalCount ?? 0;
    if (library.libraryType === "MUSIC")
      return library.albums?.pageInfo?.totalCount ?? 0;
    if (library.libraryType === "AUDIOBOOKS")
      return library.audiobooks?.pageInfo?.totalCount ?? 0;
    return 0;
  })();

  return (
    <Card className="group relative isolate aspect-2/3 w-full overflow-hidden border-none bg-content2">
      {/* Clickable overlay for navigation - covers the entire card */}
      <button
        type="button"
        className="absolute inset-0 z-20 w-full h-full cursor-pointer bg-transparent border-none outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2"
        onClick={handleCardClick}
        aria-label={`Open ${library.name} library`}
      />

      {/* Background gradient with icon */}
      <div className="absolute inset-0 w-full h-full">
        {frontCoverUrl ? (
          <>
            {backCoverUrl && (
              <img
                src={backCoverUrl}
                alt={`${library.name} artwork`}
                className={`absolute inset-0 h-full w-full object-cover transition-opacity duration-700 ${showFrontLayer ? "opacity-0" : "opacity-100"
                  }`}
              />
            )}
            <img
              src={frontCoverUrl}
              alt={`${library.name} artwork`}
              className={`absolute inset-0 h-full w-full object-cover transition-opacity duration-700 ${showFrontLayer ? "opacity-100" : "opacity-0"
                }`}
            />
            <div className="absolute inset-0 bg-gradient-to-t from-black/85 via-black/30 to-black/45" />
            <div className="absolute inset-0 flex items-center justify-center opacity-20">
              <typeInfo.icon size={80} />
            </div>
          </>
        ) : (
          <div className={`absolute inset-0 bg-linear-to-br ${gradient}`}>
            <div className="absolute inset-0 flex items-center justify-center opacity-30">
              <typeInfo.icon size={80} />
            </div>
          </div>
        )}
      </div>

      {/* Type badge - top left */}
      <div className="absolute top-2 left-2 z-10 pointer-events-none">
        <div className="px-2 py-1 rounded-md bg-black/50 backdrop-blur-sm text-xs font-medium text-white/90">
          <typeInfo.icon size={16} className="inline mr-1" />
          {typeInfo.label}
        </div>
        {pathStatus && !pathStatus.reachable && (
          <div className="mt-1 px-2 py-1 rounded-md bg-danger/85 text-xs font-semibold text-white">
            Offline
          </div>
        )}
      </div>

      {/* Bottom content */}
      <div className="absolute bottom-0 left-0 right-0 z-10 overflow-hidden rounded-b-[inherit] bg-black/50 p-3 pointer-events-none backdrop-blur-sm">
        <h3 className="text-sm font-bold text-white mb-0.5 line-clamp-2 drop-shadow-lg">
          {library.name}
        </h3>
        <div className="flex items-center gap-1.5 text-xs text-white/70">
          <span>
            {itemCount}{" "}
            {library.libraryType === "TV"
              ? itemCount === 1
                ? "Show"
                : "Shows"
              : library.libraryType === "MOVIES"
                ? itemCount === 1
                  ? "Movie"
                  : "Movies"
                : library.libraryType === "MUSIC"
                  ? itemCount === 1
                    ? "Album"
                    : "Albums"
                  : library.libraryType === "AUDIOBOOKS"
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
                  params: { libraryId: library.id },
                });
              } else if (key === "settings") {
                navigate({
                  to: "/libraries/$libraryId/settings",
                  params: { libraryId: library.id },
                });
              } else if (key === "scan") {
                onScan();
              } else if (key === "reconnect") {
                onReconnect();
              } else if (key === "delete") {
                onDelete();
              }
            }}
          >
            <DropdownItem key="view" startContent={<IconEye size={16} />}>
              Open
            </DropdownItem>
            <DropdownItem
              key="settings"
              startContent={<IconSettings size={16} />}
            >
              Settings
            </DropdownItem>
            <DropdownItem key="scan" startContent={<IconRefresh size={16} />}>
              Scan Library
            </DropdownItem>
            <DropdownItem
              key="reconnect"
              startContent={<IconPlugConnected size={16} />}
              isDisabled={!pathStatus || pathStatus.reachable}
            >
              Reconnect Path
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
