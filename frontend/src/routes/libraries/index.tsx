import { createFileRoute, redirect } from "@tanstack/react-router";
import { useEffect, useMemo, useRef, useState } from "react";
import {
  useQuery,
  useMutation,
  useSubscription,
} from "../../lib/graphql/client";
import { Button } from "@heroui/button";
import { Card, CardBody } from "@heroui/card";
import { useDisclosure } from "@heroui/modal";
import { Skeleton } from "@heroui/skeleton";
import { addToast } from "@heroui/toast";
import { IconPlus } from "@tabler/icons-react";
import { IconRefresh } from "@tabler/icons-react";
import { Image } from "@heroui/image";

import { RouteError } from "../../components/RouteError";
import { DataTable } from "../../components/data-table/DataTable";
import {
  AddLibraryModal,
  DeleteLibraryModal,
  LibraryGridCard,
  ScanLibraryModal,
  type CreateLibraryFormInput,
} from "../../components/library";
import {
  ChangeAction,
  type CreateLibraryInput,
  type CreateLibraryMutation,
  type CreateLibraryMutationVariables,
  type LibrariesQuery,
  type LibrariesQueryVariables,
  type LibraryChangedSubscription,
  type LibraryChangedSubscriptionVariables,
  type MovieChangedSubscription,
  type MovieChangedSubscriptionVariables,
  type ShowChangedSubscription,
  type ShowChangedSubscriptionVariables,
  LibrariesDocument,
  LibraryChangedDocument,
  MovieChangedDocument,
  ShowChangedDocument,
  CreateLibraryDocument,
} from "../../lib/graphql/generated/graphql";
import {
  getLibraryPathAvailability,
  reconnectLibraryPath,
  type LibraryPathAvailabilityStatus,
} from "../../lib/graphql";
import { useAuth } from "@/hooks/useAuth";

type LibraryListNode = LibrariesQuery["Libraries"]["Edges"][number]["Node"];

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

function LibrariesPage() {
  const { user } = useAuth();
  const refetchDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(
    null,
  );

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
  const [scanTargetLibrary, setScanTargetLibrary] = useState<{
    id: string;
    name: string;
  } | null>(null);
  const [pathAvailability, setPathAvailability] = useState<
    Record<string, LibraryPathAvailabilityStatus>
  >({});
  const [isRecheckingOffline, setIsRecheckingOffline] = useState(false);

  // Query libraries
  const {
    data: librariesData,
    previousData: previousLibrariesData,
    loading: librariesLoading,
    refetch,
  } = useQuery<LibrariesQuery, LibrariesQueryVariables>(LibrariesDocument, {
    fetchPolicy: "no-cache",
    notifyOnNetworkStatusChange: true,
  });
  const libraries = useMemo(
    () =>
      (librariesData?.Libraries ?? previousLibrariesData?.Libraries)?.Edges.map(
        (edge) => edge.Node
      ) ?? [],
    [librariesData?.Libraries, previousLibrariesData?.Libraries]
  );

  const uniqueLibraryPaths = useMemo(
    () => Array.from(new Set(libraries.map((lib) => lib.Path).filter(Boolean))).sort(),
    [libraries]
  );
  const libraryPathsKey = useMemo(() => uniqueLibraryPaths.join("|"), [uniqueLibraryPaths]);

  useEffect(() => {
    return () => {
      if (refetchDebounceRef.current) {
        clearTimeout(refetchDebounceRef.current);
        refetchDebounceRef.current = null;
      }
    };
  }, []);

  useEffect(() => {
    const handleWindowFocus = () => {
      void refetch();
    };
    window.addEventListener("focus", handleWindowFocus);
    return () => window.removeEventListener("focus", handleWindowFocus);
  }, [refetch]);

  useEffect(() => {
    if (uniqueLibraryPaths.length === 0) {
      setPathAvailability({});
      return;
    }

    let active = true;
    getLibraryPathAvailability(uniqueLibraryPaths, false)
      .then((statuses) => {
        if (!active) return;
        const map: Record<string, LibraryPathAvailabilityStatus> = {};
        statuses.forEach((s) => {
          map[s.Path] = s;
        });
        setPathAvailability(map);
      })
      .catch(() => {
        if (active) setPathAvailability({});
      });

    return () => {
      active = false;
    };
  }, [libraryPathsKey, uniqueLibraryPaths]);

  // Create library mutation
  const [createLibrary, { loading: createLoading }] = useMutation<
    CreateLibraryMutation,
    CreateLibraryMutationVariables
  >(CreateLibraryDocument);

  // Subscribe to library changes for real-time updates
  useSubscription<LibraryChangedSubscription, LibraryChangedSubscriptionVariables>(
    LibraryChangedDocument,
    {
      onData: ({ data }) => {
        const event = data.data?.LibraryChanged;
        if (!event) return;

        switch (event.Action) {
          case ChangeAction.Created:
          case ChangeAction.Updated:
            // Refetch to get updated counts
            refetch();
            break;
          case ChangeAction.Deleted:
            // Apollo will automatically update the cache
            refetch();
            break;
        }
      },
    }
  );

  const scheduleLibrariesRefetch = () => {
    if (refetchDebounceRef.current) {
      clearTimeout(refetchDebounceRef.current);
    }
    const timeout = setTimeout(() => {
      void refetch();
    }, 250);
    refetchDebounceRef.current = timeout;
  };

  useSubscription<MovieChangedSubscription, MovieChangedSubscriptionVariables>(
    MovieChangedDocument,
    {
      variables: {
        Filter: {
          Actions: [ChangeAction.Created, ChangeAction.Updated, ChangeAction.Deleted],
        },
      },
      onData: ({ data }) => {
        const event = data.data?.MovieChanged;
        if (!event?.Movie?.LibraryId) return;
        scheduleLibrariesRefetch();
      },
    }
  );

  useSubscription<ShowChangedSubscription, ShowChangedSubscriptionVariables>(
    ShowChangedDocument,
    {
      variables: {
        Filter: {
          Actions: [ChangeAction.Created, ChangeAction.Updated, ChangeAction.Deleted],
        },
      },
      onData: ({ data }) => {
        const event = data.data?.ShowChanged;
        if (!event?.Show?.LibraryId) return;
        scheduleLibrariesRefetch();
      },
    }
  );

  // Handlers
  const handleAddLibrary = async (input: CreateLibraryFormInput) => {
    const Input: CreateLibraryInput = {
      ...input,
      Scanning: false,
      UserId: user?.id ?? "",
    };

    try {
      const { data } = await createLibrary({ variables: { Input } });

      if (!data?.CreateLibrary.Success) {
        addToast({
          title: "Error",
          description: data?.CreateLibrary.Error || "Unknown error",
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
      await refetch();
    } catch (err) {
      console.error("Failed to create library:", err);
      addToast({
        title: "Error",
        description: "Failed to create library",
        color: "danger",
      });
    }
  };

  const handleDeleteClick = (id: string, name: string) => {
    setTargetLibrary({ id, name });
    onDeleteOpen();
  };

  const handleScanClick = (id: string, name: string) => {
    setScanTargetLibrary({ id, name });
    onScanOpen();
  };

  const handleReconnect = async (path: string) => {
    const result = await reconnectLibraryPath(path);
    if (!result.success) {
      addToast({
        title: "Reconnect Failed",
        description: result.error ?? "Unable to reconnect path",
        color: "danger",
      });
      return;
    }

    addToast({
      title: "Reconnect Requested",
      description: "Retrying access for library path",
      color: "success",
    });
    const statuses = await getLibraryPathAvailability([path], true).catch(
      () => [],
    );
    if (statuses[0]) {
      setPathAvailability((prev) => ({ ...prev, [path]: statuses[0] }));
    }
  };

  const recheckOfflineLibraries = async () => {
    const offlinePaths = Object.values(pathAvailability)
      .filter((s) => !s.Reachable)
      .map((s) => s.Path);
    if (offlinePaths.length === 0) return;

    setIsRecheckingOffline(true);
    try {
      const statuses = await getLibraryPathAvailability(offlinePaths, true);
      const next = { ...pathAvailability };
      statuses.forEach((s) => {
        next[s.Path] = s;
      });
      setPathAvailability(next);

      const stillOffline = statuses.filter((s) => !s.Reachable).length;
      if (stillOffline === 0) {
        addToast({
          title: "Libraries Reconnected",
          description: "All previously offline libraries are now reachable",
          color: "success",
        });
      } else {
        addToast({
          title: "Recheck Complete",
          description: `${stillOffline} library path(s) remain offline`,
          color: "warning",
        });
      }
    } catch (err) {
      addToast({
        title: "Recheck Failed",
        description: err instanceof Error ? err.message : "Failed to recheck offline libraries",
        color: "danger",
      });
    } finally {
      setIsRecheckingOffline(false);
    }
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
    <Card className="relative overflow-hidden aspect-2/3 bg-content2">
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
        getRowKey={(lib: LibraryListNode) => lib.Id}
        isLoading={librariesLoading && libraries.length === 0}
        skeletonDelay={300}
        emptyContent={emptyContent}
        // Card view only
        defaultViewMode="cards"
        cardRenderer={({ item }) => (
          <LibraryGridCard
            library={item}
            onScan={() => handleScanClick(item.Id, item.Name)}
            onDelete={() => handleDeleteClick(item.Id, item.Name)}
            pathStatus={pathAvailability[item.Path]}
            onReconnect={() => handleReconnect(item.Path)}
          />
        )}
        cardSkeleton={cardSkeleton}
        skeletonCardCount={6}
        cardGridClassName="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4"
        headerContent={
          <div className="flex items-start justify-between mb-6">
            <div>
              <h1 className="text-2xl font-bold">Libraries</h1>
              <p className="text-default-500">
                Organize and manage your media collections
              </p>
            </div>
            <div className="flex items-center gap-2">
              <Button
                size="sm"
                variant="flat"
                startContent={<IconRefresh size={16} />}
                isLoading={isRecheckingOffline}
                isDisabled={
                  isRecheckingOffline ||
                  !Object.values(pathAvailability).some((s) => !s.Reachable)
                }
                onPress={() => void recheckOfflineLibraries()}
              >
                Recheck Offline
              </Button>
              <Button
                color="primary"
                size="sm"
                startContent={<IconPlus size={16} />}
                onPress={onAddOpen}
              >
                Add Library
              </Button>
            </div>
          </div>
        }
        hideToolbar
        showItemCount={false}
      />

      <AddLibraryModal
        isOpen={isAddOpen}
        onClose={onAddClose}
        onAdd={handleAddLibrary}
        isLoading={createLoading}
      />

      <DeleteLibraryModal
        isOpen={isDeleteOpen}
        onClose={onDeleteClose}
        libraryId={targetLibrary?.id ?? null}
        libraryName={targetLibrary?.name ?? null}
        onDeleted={refetch}
      />

      <ScanLibraryModal
        isOpen={isScanOpen}
        onClose={onScanClose}
        libraryId={scanTargetLibrary?.id ?? null}
        libraryName={scanTargetLibrary?.name ?? null}
        onScanStarted={refetch}
      />

    </div>
  );
}
