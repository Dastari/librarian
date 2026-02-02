import {
  createFileRoute,
  Link,
  redirect,
  Outlet,
  useLocation,
} from "@tanstack/react-router";
import {
  useState,
  useEffect,
  useCallback,
  useRef,
  createContext,
  useContext,
} from "react";
import { Button } from "@heroui/button";
import { Card, CardBody } from "@heroui/card";
import { useDisclosure } from "@heroui/modal";
import { addToast } from "@heroui/toast";
import { Breadcrumbs, BreadcrumbItem } from "@heroui/breadcrumbs";
import { Spinner } from "@heroui/spinner";
import { ConfirmModal } from "../../components/ConfirmModal";
import { useDataReactivity } from "../../hooks/useSubscription";
import {
  useContentDownloadProgress,
  type ContentProgressMap,
} from "../../hooks/useContentDownloadProgress";
import { RouteError } from "../../components/RouteError";
import {
  AddShowModal,
  LibraryLayout,
  type LibraryTab,
} from "../../components/library";
import { sanitizeError } from "../../lib/format";
import type {
  Library,
  Show,
  LibraryChangedEvent,
  LibraryResult,
} from "../../lib/graphql/generated/graphql";
import {
  graphqlClient,
  LIBRARY_QUERY,
  TV_SHOWS_QUERY,
  DELETE_TV_SHOW_MUTATION,
  UPDATE_LIBRARY_MUTATION,
  SCAN_LIBRARY_MUTATION,
  LIBRARY_CHANGED_SUBSCRIPTION,
  getLibraryTypeInfo,
  type LibraryType,
  type UpdateLibraryInput,
} from "../../lib/graphql";

// Context for sharing library data with subroutes
export interface LibraryContextValue {
  library: Library;
  loading: boolean;
  tvShows: Show[];
  refetch: () => void;
  actionLoading: boolean;
  handleDeleteShowClick: (showId: string, showName: string) => void;
  handleUpdateLibrary: (input: UpdateLibraryInput) => Promise<void>;
  onOpenAddShow: () => void;
  downloadProgress: ContentProgressMap;
}

const defaultContextValue: LibraryContextValue = {
  library: {} as Library,
  loading: true,
  tvShows: [],
  refetch: () => {},
  actionLoading: false,
  handleDeleteShowClick: () => {},
  handleUpdateLibrary: async () => {},
  onOpenAddShow: () => {},
  downloadProgress: new Map(),
};

export const LibraryContext =
  createContext<LibraryContextValue>(defaultContextValue);

export function useLibraryContext() {
  return useContext(LibraryContext);
}

export const Route = createFileRoute("/libraries/$libraryId")({
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
  component: LibraryDetailLayout,
  errorComponent: RouteError,
});

function LibraryDetailLayout() {
  const { libraryId } = Route.useParams();
  const location = useLocation();
  const { isOpen, onOpen, onClose } = useDisclosure();
  const {
    isOpen: isConfirmOpen,
    onOpen: onConfirmOpen,
    onClose: onConfirmClose,
  } = useDisclosure();

  // State - keep previous data to prevent flashes during refetch
  const [library, setLibrary] = useState<Library | null>(null);
  const [tvShows, setTvShows] = useState<Show[]>([]);
  const [initialLoading, setInitialLoading] = useState(true);
  const [actionLoading, setActionLoading] = useState(false);
  const [showToDelete, setShowToDelete] = useState<{
    id: string;
    name: string;
  } | null>(null);
  const [isScanning, setIsScanning] = useState(false);

  const fetchData = useCallback(async () => {
    try {
      const [libraryResult, showsResult] = await Promise.all([
        graphqlClient
          .query<{ Library: Library | null }>(LIBRARY_QUERY, { Id: libraryId })
          .toPromise(),
        graphqlClient
          .query<{
            Shows: { Edges: Array<{ Node: Show }> };
          }>(TV_SHOWS_QUERY, { libraryId })
          .toPromise(),
      ]);

      if (libraryResult.data?.Library) {
        setLibrary(libraryResult.data.Library);
      }
      if (showsResult.data?.Shows?.Edges) {
        setTvShows(showsResult.data.Shows.Edges.map((edge) => edge.Node));
      }
    } catch (err) {
      console.error("Failed to fetch data:", err);
    } finally {
      setInitialLoading(false);
    }
  }, [libraryId]);

  useEffect(() => {
    setInitialLoading(true);
    fetchData();
  }, [fetchData]);

  // Determine active tab from URL
  const getActiveTab = useCallback((): LibraryTab => {
    const path = location.pathname;
    if (path.endsWith("/unmatched")) return "unmatched";
    if (path.endsWith("/browser")) return "browser";
    if (path.endsWith("/settings")) return "settings";
    if (path.endsWith("/shows")) return "shows";
    if (path.endsWith("/movies")) return "movies";
    if (path.endsWith("/collections")) return "collections";
    if (path.endsWith("/artists")) return "artists";
    if (path.endsWith("/albums")) return "albums";
    if (path.endsWith("/tracks")) return "tracks";
    if (path.endsWith("/books")) return "books";
    if (path.endsWith("/authors")) return "authors";

    if (library) {
      switch (library.LibraryType) {
        case "MOVIES":
          return "movies";
        case "TV":
          return "shows";
        case "MUSIC":
          return "albums";
        case "AUDIOBOOKS":
          return "books";
        default:
          return "browser";
      }
    }
    return "shows";
  }, [location.pathname, library]);

  // Sync scanning state
  useEffect(() => {
    if (library && !library.Scanning && isScanning) {
      setIsScanning(false);
    } else if (library?.Scanning && !isScanning) {
      setIsScanning(true);
    }
  }, [library?.Scanning, isScanning]);

  // Update page title
  useEffect(() => {
    if (library) {
      document.title = `Librarian - ${library.Name}`;
    }
    return () => {
      document.title = "Librarian";
    };
  }, [library?.Name]);

  // Subscribe to data changes for live updates
  useDataReactivity(fetchData, {
    onTorrentComplete: true,
    periodicInterval: 30000,
    onFocus: true,
  });

  // Subscribe to content download progress
  const downloadProgress = useContentDownloadProgress({
    libraryId,
    enabled: !initialLoading && !!library,
  });

  // Track previous scanning state for toast notifications
  const prevScanningRef = useRef(library?.Scanning);

  // Subscribe to library changes
  useEffect(() => {
    if (!library) return;

    const sub = graphqlClient
      .subscription<{
        LibraryChanged: LibraryChangedEvent;
      }>(LIBRARY_CHANGED_SUBSCRIPTION, {})
      .subscribe({
        next: (result) => {
          const event = result.data?.LibraryChanged;
          if (event?.Id === library.Id && event.Library) {
            const wasScanning = prevScanningRef.current;
            const nowScanning = event.Library.Scanning;
            prevScanningRef.current = nowScanning;

            // Update library state directly from subscription
            setLibrary(event.Library);

            if (wasScanning && !nowScanning) {
              setIsScanning(false);
              addToast({
                title: "Scan Complete",
                description: `Finished scanning ${library.Name}`,
                color: "success",
              });
            } else if (!wasScanning && nowScanning) {
              setIsScanning(true);
            }

            // Refresh shows data on library changes
            fetchData();
          }
        },
      });

    return () => sub.unsubscribe();
  }, [library?.Id, library?.Name, fetchData]);

  const handleDeleteShowClick = useCallback(
    (showId: string, showName: string) => {
      setShowToDelete({ id: showId, name: showName });
      onConfirmOpen();
    },
    [onConfirmOpen],
  );

  const handleDeleteShow = async () => {
    if (!showToDelete) return;

    try {
      const { data, error } = await graphqlClient
        .mutation<{
          deleteTvShow: { success: boolean; error: string | null };
        }>(DELETE_TV_SHOW_MUTATION, { id: showToDelete.id })
        .toPromise();

      if (error || !data?.deleteTvShow.success) {
        addToast({
          title: "Error",
          description: sanitizeError(
            data?.deleteTvShow.error || "Failed to delete show",
          ),
          color: "danger",
        });
        onConfirmClose();
        return;
      }

      addToast({
        title: "Deleted",
        description: `"${showToDelete.name}" removed from library`,
        color: "success",
      });

      fetchData();
    } catch (err) {
      console.error("Failed to delete show:", err);
    }
    onConfirmClose();
  };

  const handleUpdateLibrary = useCallback(
    async (input: UpdateLibraryInput) => {
      if (!library) return;

      try {
        setActionLoading(true);
        const { data, error } = await graphqlClient
          .mutation<{ UpdateLibrary: LibraryResult }>(UPDATE_LIBRARY_MUTATION, {
            Id: library.Id,
            Input: input,
          })
          .toPromise();

        if (error || !data?.UpdateLibrary.Success) {
          const errorMsg =
            data?.UpdateLibrary.Error || error?.message || "Unknown error";
          addToast({
            title: "Error",
            description: `Failed to update library: ${errorMsg}`,
            color: "danger",
          });
          return;
        }

        addToast({
          title: "Success",
          description: "Library settings saved",
          color: "success",
        });

        fetchData();
      } catch (err) {
        console.error("Failed to update library:", err);
        addToast({
          title: "Error",
          description: "Failed to update library",
          color: "danger",
        });
      } finally {
        setActionLoading(false);
      }
    },
    [library, fetchData],
  );

  const handleScanLibrary = async () => {
    if (!library) return;

    setIsScanning(true);
    try {
      const { data, error } = await graphqlClient
        .mutation<{
          ScanLibrary: { Status: string; Message: string | null };
        }>(SCAN_LIBRARY_MUTATION, { Id: library.Id })
        .toPromise();

      if (error) {
        addToast({
          title: "Error",
          description: sanitizeError(error),
          color: "danger",
        });
        setIsScanning(false);
        return;
      }

      addToast({
        title: "Scan Started",
        description: data?.ScanLibrary.Message || `Scanning ${library.Name}...`,
        color: "primary",
      });
    } catch (err) {
      console.error("Failed to scan library:", err);
      setIsScanning(false);
    }
  };

  // Not found state - only show after loading is complete
  if (!initialLoading && !library) {
    return (
      <div className="max-w-7xl mx-auto px-4 py-8">
        <Card className="bg-content1">
          <CardBody className="text-center py-12">
            <h2 className="text-xl font-semibold mb-4">Library not found</h2>
            <Link to="/libraries">
              <Button color="primary">Back to Libraries</Button>
            </Link>
          </CardBody>
        </Card>
      </div>
    );
  }

  // Initial loading state
  if (initialLoading && !library) {
    return (
      <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-8 flex flex-col grow">
        <div className="flex items-center justify-center py-12">
          <Spinner size="lg" />
        </div>
      </div>
    );
  }

  // Safety check - should not reach here but TypeScript needs it
  if (!library) return null;

  const typeInfo = getLibraryTypeInfo(library.LibraryType as LibraryType);
  const scanningActive = isScanning || library.Scanning;

  const contextValue: LibraryContextValue = {
    library,
    loading: initialLoading,
    tvShows,
    refetch: fetchData,
    actionLoading,
    handleDeleteShowClick,
    handleUpdateLibrary,
    onOpenAddShow: onOpen,
    downloadProgress,
  };

  return (
    <LibraryContext.Provider value={contextValue}>
      <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-8 flex flex-col grow">
        {/* Header */}
        <div className="mb-6">
          <Breadcrumbs className="mb-2">
            <BreadcrumbItem href="/libraries">Libraries</BreadcrumbItem>
            <BreadcrumbItem isCurrent>{library.Name}</BreadcrumbItem>
          </Breadcrumbs>

          <div className="flex items-start justify-between">
            <div className="flex items-center gap-4">
              <typeInfo.Icon className="w-10 h-10" />
              <div>
                <h1 className="text-2xl font-bold">{library.Name}</h1>
                <div className="flex items-center gap-3 text-sm text-default-500 mt-1">
                  <span className="font-mono text-xs">{library.Path}</span>
                </div>
              </div>
            </div>

            <div className="flex items-center gap-2">
              <Button
                color="primary"
                variant="flat"
                size="sm"
                onPress={handleScanLibrary}
                isLoading={scanningActive}
                isDisabled={scanningActive}
              >
                {scanningActive ? "Scanning..." : "Scan Now"}
              </Button>
            </div>
          </div>
        </div>

        {/* Tabbed Content */}
        <LibraryLayout
          activeTab={getActiveTab()}
          libraryId={libraryId}
          libraryType={library.LibraryType as LibraryType}
        >
          <Outlet />
        </LibraryLayout>

        {/* Add Show Modal */}
        <AddShowModal
          isOpen={isOpen}
          onClose={onClose}
          libraryId={libraryId}
          onAdded={fetchData}
        />

        {/* Confirm Delete Modal */}
        <ConfirmModal
          isOpen={isConfirmOpen}
          onClose={onConfirmClose}
          onConfirm={handleDeleteShow}
          title="Delete Show"
          message={`Are you sure you want to delete "${showToDelete?.name}"?`}
          description="This will remove the show from your library. Downloaded files will not be deleted."
          confirmLabel="Delete"
          confirmColor="danger"
        />
      </div>
    </LibraryContext.Provider>
  );
}
