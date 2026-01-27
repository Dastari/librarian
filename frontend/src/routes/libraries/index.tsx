import { createFileRoute, redirect } from "@tanstack/react-router";
import { useState, useEffect, useCallback, useTransition } from "react";
import { Button } from "@heroui/button";
import { Card, CardBody } from "@heroui/card";
import { useDisclosure } from "@heroui/modal";
import { Skeleton } from "@heroui/skeleton";
import { addToast } from "@heroui/toast";
import { IconPlus } from "@tabler/icons-react";
import { Image } from "@heroui/image";

import { RouteError } from "../../components/RouteError";
import { DataTable } from "../../components/data-table/DataTable";
import {
  AddLibraryModal,
  DeleteLibraryModal,
  ScanLibraryModal,
  LibraryGridCard,
  type CreateLibraryFormInput,
} from "../../components/library";
import { graphqlClient } from "../../lib/graphql";
import {
  LibraryChangedDocument,
  CreateLibraryDocument,
  MeDocument,
  ChangeAction,
  type CreateLibraryInput as GenCreateLibraryInput,
  type Library,
} from "../../lib/graphql/generated/graphql";

// ============================================================================
// Route Config
// ============================================================================

export const Route = createFileRoute("/libraries/")({
  beforeLoad: ({ context, location }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({
        to: "/",
        search: {
          signin: true,
          redirect: location.href,
        },
      });
    }
  },
  component: LibrariesPage,
  errorComponent: RouteError,
});

// ============================================================================
// Types
// ============================================================================

interface LibraryWithCounts extends Library {
  _showCount?: number;
  _movieCount?: number;
  _albumCount?: number;
  _audiobookCount?: number;
}

// ============================================================================
// GraphQL Query with Counts
// ============================================================================

const LIBRARIES_WITH_COUNTS_QUERY = `
  query LibrariesWithCounts {
    Libraries {
      Edges {
        Node {
          Id
          UserId
          Name
          Path
          LibraryType
          Icon
          Color
          AutoScan
          ScanIntervalMinutes
          WatchForChanges
          AutoAddDiscovered
          AutoDownload
          AutoHunt
          Scanning
          LastScannedAt
          CreatedAt
          UpdatedAt
          Shows {
            PageInfo {
              TotalCount
            }
          }
          Movies {
            PageInfo {
              TotalCount
            }
          }
          Albums {
            PageInfo {
              TotalCount
            }
          }
          Audiobooks {
            PageInfo {
              TotalCount
            }
          }
        }
      }
      PageInfo {
        TotalCount
      }
    }
  }
`;

// ============================================================================
// Component
// ============================================================================

function LibrariesPage() {
  // State
  const [isPending, startTransition] = useTransition();
  const [libraries, setLibraries] = useState<LibraryWithCounts[]>([]);
  const [currentUserId, setCurrentUserId] = useState<string | null>(null);
  const [actionLoading, setActionLoading] = useState(false);

  // Modal states
  const {
    isOpen: isAddOpen,
    onOpen: onAddOpen,
    onClose: onAddClose,
  } = useDisclosure();
  const {
    isOpen: isDeleteOpen,
    onOpen: onDeleteOpen,
    onClose: onDeleteClose,
  } = useDisclosure();
  const {
    isOpen: isScanOpen,
    onOpen: onScanOpen,
    onClose: onScanClose,
  } = useDisclosure();

  // Track which library is being acted upon
  const [targetLibrary, setTargetLibrary] = useState<{
    id: string;
    name: string;
  } | null>(null);

  // Fetch current user
  useEffect(() => {
    graphqlClient
      .query(MeDocument, {})
      .toPromise()
      .then((result) => {
        if (result.data?.Me) {
          setCurrentUserId(result.data.Me.Id);
        }
      });
  }, []);

  // Fetch libraries with counts
  const fetchLibraries = useCallback(async () => {
    startTransition(async () => {
      const { data, error } = await graphqlClient
        .query<{
          Libraries: {
            Edges: Array<{
              Node: Library & {
                Shows: { PageInfo: { TotalCount: number } };
                Movies: { PageInfo: { TotalCount: number } };
                Albums: { PageInfo: { TotalCount: number } };
                Audiobooks: { PageInfo: { TotalCount: number } };
              };
            }>;
          };
        }>(LIBRARIES_WITH_COUNTS_QUERY, {})
        .toPromise();

      if (error) {
        console.error("Failed to fetch libraries:", error);
        return;
      }

      const libs =
        data?.Libraries?.Edges.map((edge) => ({
          ...edge.Node,
          _showCount: edge.Node.Shows?.PageInfo?.TotalCount ?? 0,
          _movieCount: edge.Node.Movies?.PageInfo?.TotalCount ?? 0,
          _albumCount: edge.Node.Albums?.PageInfo?.TotalCount ?? 0,
          _audiobookCount: edge.Node.Audiobooks?.PageInfo?.TotalCount ?? 0,
        })) ?? [];

      setLibraries(libs);
    });
  }, []);

  // Initial fetch
  useEffect(() => {
    fetchLibraries();
  }, [fetchLibraries]);

  // Subscribe to library changes for real-time updates
  useEffect(() => {
    const subscription = graphqlClient
      .subscription(LibraryChangedDocument, {})
      .subscribe({
        next: (result) => {
          const event = result.data?.LibraryChanged;
          if (!event) return;

          switch (event.Action) {
            case ChangeAction.Created:
            case ChangeAction.Updated:
              // Refetch to get updated counts
              fetchLibraries();
              break;
            case ChangeAction.Deleted:
              setLibraries((prev) => prev.filter((lib) => lib.Id !== event.Id));
              break;
          }
        },
      });

    return () => subscription.unsubscribe();
  }, [fetchLibraries]);

  // Handlers
  const handleAddLibrary = async (input: CreateLibraryFormInput) => {
    if (!currentUserId) {
      addToast({
        title: "Error",
        description: "User not loaded. Please refresh and try again.",
        color: "danger",
      });
      return;
    }

    const now = new Date().toISOString();
    const genInput: GenCreateLibraryInput = {
      ...input,
      UserId: currentUserId,
      CreatedAt: now,
      UpdatedAt: now,
    };

    try {
      setActionLoading(true);
      const { data, error } = await graphqlClient
        .mutation(CreateLibraryDocument, { Input: genInput })
        .toPromise();

      if (error || !data?.CreateLibrary.Success) {
        addToast({
          title: "Error",
          description:
            data?.CreateLibrary.Error || error?.message || "Unknown error",
          color: "danger",
        });
        return;
      }

      addToast({
        title: "Success",
        description: `Library "${input.Name}" created`,
        color: "success",
      });

      onAddClose();
      await fetchLibraries();
    } catch (err) {
      console.error("Failed to create library:", err);
      addToast({
        title: "Error",
        description: "Failed to create library",
        color: "danger",
      });
    } finally {
      setActionLoading(false);
    }
  };

  const handleScanClick = (id: string, name: string) => {
    setTargetLibrary({ id, name });
    onScanOpen();
  };

  const handleDeleteClick = (id: string, name: string) => {
    setTargetLibrary({ id, name });
    onDeleteOpen();
  };

  // Empty state
  const emptyContent = (
    <Card className="bg-content1/50 border-default-300 border-dashed border-2">
      <CardBody className="py-16 text-center">
        <div className="mx-auto w-20 h-20 flex items-center justify-center mb-6">
          <Image src="/logo.svg" alt="Library" width={80} height={80} />
        </div>
        <h3 className="text-xl font-semibold mb-2">No libraries yet</h3>
        <p className="text-default-500 mb-6 max-w-md mx-auto">
          Libraries help you organize your media. Add a library to start
          managing your movies, TV shows, music, and more.
        </p>
        <Button color="primary" size="lg" onPress={onAddOpen}>
          Add Your First Library
        </Button>
      </CardBody>
    </Card>
  );

  // Card skeleton
  const cardSkeleton = () => (
    <Card className="relative overflow-hidden aspect-[2/3] bg-content2">
      <Skeleton className="absolute inset-0 w-full h-full" />
      <div className="absolute bottom-0 left-0 right-0 p-3 bg-black/50">
        <Skeleton className="h-4 w-3/4 mb-2 rounded" />
        <Skeleton className="h-3 w-1/2 rounded" />
      </div>
    </Card>
  );

  return (
    <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <DataTable
        stateKey="libraries"
        data={libraries}
        columns={[]}
        getRowKey={(lib) => lib.Id}
        isLoading={isPending && libraries.length === 0}
        skeletonDelay={300}
        emptyContent={emptyContent}
        // Card view only
        defaultViewMode="cards"
        cardRenderer={({ item }) => (
          <LibraryGridCard
            library={item}
            showCount={item._showCount}
            movieCount={item._movieCount}
            albumCount={item._albumCount}
            audiobookCount={item._audiobookCount}
            onScan={() => handleScanClick(item.Id, item.Name)}
            onDelete={() => handleDeleteClick(item.Id, item.Name)}
          />
        )}
        cardSkeleton={cardSkeleton}
        skeletonCardCount={6}
        cardGridClassName="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4"
        // Header with title, description, and add button
        headerContent={
          <div className="flex items-start justify-between mb-6">
            <div>
              <h1 className="text-2xl font-bold">Libraries</h1>
              <p className="text-default-500">
                Organize and manage your media collections
              </p>
            </div>
            <Button
              color="primary"
              size="sm"
              startContent={<IconPlus size={16} />}
              onPress={onAddOpen}
            >
              Add Library
            </Button>
          </div>
        }
        // Hide default toolbar (search, etc.) and item count
        hideToolbar
        showItemCount={false}
      />

      {/* Modals */}
      <AddLibraryModal
        isOpen={isAddOpen}
        onClose={onAddClose}
        onAdd={handleAddLibrary}
        isLoading={actionLoading}
      />

      <DeleteLibraryModal
        isOpen={isDeleteOpen}
        onClose={onDeleteClose}
        libraryId={targetLibrary?.id ?? null}
        libraryName={targetLibrary?.name ?? null}
        onDeleted={fetchLibraries}
      />

      <ScanLibraryModal
        isOpen={isScanOpen}
        onClose={onScanClose}
        libraryId={targetLibrary?.id ?? null}
        libraryName={targetLibrary?.name ?? null}
      />
    </div>
  );
}
