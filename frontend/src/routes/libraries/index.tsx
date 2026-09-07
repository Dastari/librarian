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
import { Image } from "@heroui/image";
import { IconPlus } from "@tabler/icons-react";

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

type LibraryListNode = LibrariesQuery["libraries"]["edges"][number]["node"];

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
    libraryType: string;
  } | null>(null);
  const [pathAvailability, setPathAvailability] = useState<
    Record<string, LibraryPathAvailabilityStatus>
  >({});

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
      (librariesData?.libraries ?? previousLibrariesData?.libraries)?.edges.map(
        (edge) => edge.node
      ) ?? [],
    [librariesData?.libraries, previousLibrariesData?.libraries]
  );

  const uniqueLibraryPaths = useMemo(
    () => Array.from(new Set(libraries.map((lib) => lib.path).filter(Boolean))).sort(),
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
          map[s.path] = s;
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
        const event = data.data?.libraryChanged;
        if (!event) return;

        switch (event.action) {
          case ChangeAction.CREATED:
          case ChangeAction.UPDATED:
            // Refetch to get updated counts
            refetch();
            break;
          case ChangeAction.DELETED:
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
        filter: {
          actions: [ChangeAction.CREATED, ChangeAction.UPDATED, ChangeAction.DELETED],
        },
      },
      onData: ({ data }) => {
        const event = data.data?.movieChanged;
        if (!event?.movie?.libraryId) return;
        scheduleLibrariesRefetch();
      },
    }
  );

  useSubscription<ShowChangedSubscription, ShowChangedSubscriptionVariables>(
    ShowChangedDocument,
    {
      variables: {
        filter: {
          actions: [ChangeAction.CREATED, ChangeAction.UPDATED, ChangeAction.DELETED],
        },
      },
      onData: ({ data }) => {
        const event = data.data?.showChanged;
        if (!event?.show?.libraryId) return;
        scheduleLibrariesRefetch();
      },
    }
  );

  // Handlers
  const handleAddLibrary = async (input: CreateLibraryFormInput) => {
    const createInput: CreateLibraryInput = {
      ...input,
      scanning: false,
      userId: user?.id ?? "",
    };

    try {
      const { data } = await createLibrary({ variables: { input: createInput } });

      if (!data?.createLibrary.success) {
        addToast({
          title: "Error",
          description: data?.createLibrary.error || "Unknown error",
          color: "danger",
        });
        return;
      }

      addToast({
        title: "Success",
        description: `Library "${input.name}" created`,
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

  const handleScanClick = (id: string, name: string, libraryType: string) => {
    setScanTargetLibrary({ id, name, libraryType });
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
    <Card className="relative aspect-2/3 w-full overflow-hidden bg-content2">
      <Skeleton className="absolute inset-0 w-full h-full" />
      <div className="absolute bottom-0 left-0 right-0 p-3 bg-black/50">
        <Skeleton className="h-4 w-3/4 mb-2 rounded" />
        <Skeleton className="h-3 w-1/2 rounded" />
      </div>
    </Card>
  );

  return (
    <div className="container mx-auto flex h-full min-h-0 flex-1 flex-col overflow-hidden px-4 py-8 sm:px-6 lg:px-8">
      <div className="mb-6 shrink-0">
        <h1 className="text-2xl font-bold">Libraries</h1>
        <p className="text-default-500">
          Organize and manage your media collections
        </p>
      </div>

      <section className="flex h-0 min-h-0 flex-1 flex-col">
        <DataTable
          stateKey="libraries"
          data={libraries}
          columns={[]}
          getRowKey={(lib: LibraryListNode) => lib.id}
          isLoading={librariesLoading && libraries.length === 0}
          skeletonDelay={300}
          fillHeight={true}
          emptyContent={emptyContent}
          // Card view only
          defaultViewMode="cards"
          cardRenderer={({ item }) => (
            <LibraryGridCard
              library={item}
              onScan={() =>
                handleScanClick(item.id, item.name, item.libraryType)
              }
              onDelete={() => handleDeleteClick(item.id, item.name)}
              pathStatus={pathAvailability[item.path]}
              onReconnect={() => handleReconnect(item.path)}
            />
          )}
          cardSkeleton={cardSkeleton}
          skeletonCardCount={6}
          cardGridClassName="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4"
          toolbarActions={[
            {
              key: "add-library",
              label: "Add Library",
              icon: IconPlus,
              onAction: () => onAddOpen(),
              variant: "default",
            },
          ]}
          toolbarVisibility={{
            title: false,
            search: false,
            actions: true,
            trailingActions: false,
            options: false,
            viewToggle: false,
            customToolbar: false,
          }}
          classNames={{
            wrapper: "flex h-full min-h-0 flex-1 flex-col",
          }}
          showItemCount={false}
        />
      </section>

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
        libraryType={scanTargetLibrary?.libraryType ?? null}
        onScanStarted={refetch}
      />

    </div>
  );
}
