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
import { ShimmerLoader } from "../../components/shared/ShimmerLoader";
import { libraryTemplate } from "../../lib/template-data";
import {
  AutoHuntBadge,
  FileOrganizationBadge,
  QualityFilterBadge,
} from "../../components/shared";
import { addToast } from "@heroui/toast";
import { Breadcrumbs, BreadcrumbItem } from "@heroui/breadcrumbs";
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
import {
  graphqlClient,
  LIBRARY_QUERY,
  TV_SHOWS_QUERY,
  DELETE_TV_SHOW_MUTATION,
  UPDATE_LIBRARY_MUTATION,
  SCAN_LIBRARY_MUTATION,
  LIBRARY_CHANGED_SUBSCRIPTION,
  getLibraryTypeInfo,
  type Library,
  type TvShow,
  type UpdateLibraryInput,
  type LibraryChangedEvent,
} from "../../lib/graphql";
import { formatBytes } from "../../lib/format";

// Context for sharing library data with subroutes
export interface LibraryContextValue {
  library: Library;
  loading: boolean;
  tvShows: TvShow[];
  fetchData: (isBackgroundRefresh?: boolean) => Promise<void>;
  actionLoading: boolean;
  handleDeleteShowClick: (showId: string, showName: string) => void;
  handleUpdateLibrary: (input: UpdateLibraryInput) => Promise<void>;
  onOpenAddShow: () => void;
  /** Map of content IDs to download progress (0-1) for real-time updates */
  downloadProgress: ContentProgressMap;
}

// Default context with loading state - used when context not yet initialized
const defaultContextValue: LibraryContextValue = {
  library: libraryTemplate,
  loading: true,
  tvShows: [],
  fetchData: async () => {},
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
  const [library, setLibrary] = useState<Library | null>(null);
  const [tvShows, setTvShows] = useState<TvShow[]>([]);
  const [loading, setLoading] = useState(true);
  const [actionLoading, setActionLoading] = useState(false);
  const [showToDelete, setShowToDelete] = useState<{
    id: string;
    name: string;
  } | null>(null);
  const [isScanning, setIsScanning] = useState(false);

  // Determine active tab from current URL
  const getActiveTab = (): LibraryTab => {
    const path = location.pathname;
    // Common tabs
    if (path.endsWith("/unmatched")) return "unmatched";
    if (path.endsWith("/browser")) return "browser";
    if (path.endsWith("/settings")) return "settings";
    // TV tabs
    if (path.endsWith("/shows")) return "shows";
    // Movie tabs
    if (path.endsWith("/movies")) return "movies";
    if (path.endsWith("/collections")) return "collections";
    // Music tabs
    if (path.endsWith("/artists")) return "artists";
    if (path.endsWith("/albums")) return "albums";
    if (path.endsWith("/tracks")) return "tracks";
    // Audiobook tabs
    if (path.endsWith("/books")) return "books";
    if (path.endsWith("/authors")) return "authors";

    // Return default based on library type
    if (library) {
      switch (library.libraryType) {
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
    return "shows"; // fallback
  };

  // Track if initial load is done to avoid showing spinner on background refreshes
  const initialLoadDone = useRef(false);

  const fetchData = useCallback(
    async (isBackgroundRefresh = false) => {
      try {
        // Only show loading spinner on initial load
        if (!isBackgroundRefresh) {
          setLoading(true);
        }

        // Fetch library and TV shows in parallel
        const [libraryResult, showsResult] = await Promise.all([
          graphqlClient
            .query<{
              library: Library | null;
            }>(LIBRARY_QUERY, { id: libraryId })
            .toPromise(),
          graphqlClient
            .query<{ tvShows: TvShow[] }>(TV_SHOWS_QUERY, { libraryId })
            .toPromise(),
        ]);

        if (libraryResult.data?.library) {
          setLibrary(libraryResult.data.library);
        }
        if (showsResult.data?.tvShows) {
          setTvShows(showsResult.data.tvShows);
        }
      } catch (err) {
        console.error("Failed to fetch data:", err);
      } finally {
        setLoading(false);
        initialLoadDone.current = true;
      }
    },
    [libraryId],
  );

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  // Sync local scanning state with library scanning state
  useEffect(() => {
    if (library && !library.scanning && isScanning) {
      // Library finished scanning, update local state
      setIsScanning(false);
    } else if (library?.scanning && !isScanning) {
      // Library is scanning (e.g., started from elsewhere), sync local state
      setIsScanning(true);
    }
  }, [library?.scanning, isScanning]);

  // Update page title when library data is loaded
  useEffect(() => {
    if (library) {
      document.title = `Librarian - ${library.name}`;
    }
    return () => {
      document.title = "Librarian";
    };
  }, [library]);

  // Subscribe to data changes for live updates
  useDataReactivity(
    () => {
      if (initialLoadDone.current) {
        fetchData(true);
      }
    },
    { onTorrentComplete: true, periodicInterval: 30000, onFocus: true },
  );

  // Subscribe to content download progress for real-time updates on this library
  const downloadProgress = useContentDownloadProgress({
    libraryId: libraryId,
    enabled: !loading && !!library,
  });

  const handleDeleteShowClick = (showId: string, showName: string) => {
    setShowToDelete({ id: showId, name: showName });
    onConfirmOpen();
  };

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

      await fetchData();
    } catch (err) {
      console.error("Failed to delete show:", err);
    }
    onConfirmClose();
  };

  const handleUpdateLibrary = async (input: UpdateLibraryInput) => {
    if (!library) return;

    try {
      setActionLoading(true);
      const { data, error } = await graphqlClient
        .mutation<{
          updateLibrary: {
            success: boolean;
            library: Library | null;
            error: string | null;
          };
        }>(UPDATE_LIBRARY_MUTATION, { id: library.id, input })
        .toPromise();

      if (error || !data?.updateLibrary.success) {
        const errorMsg =
          data?.updateLibrary.error || error?.message || "Unknown error";
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

      // Refresh library data
      await fetchData();
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
  };

  const handleScanLibrary = async () => {
    if (!library) return;

    setIsScanning(true);
    try {
      const { data, error } = await graphqlClient
        .mutation<{
          scanLibrary: { status: string; message: string | null };
        }>(SCAN_LIBRARY_MUTATION, { id: library.id })
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
        description: data?.scanLibrary.message || `Scanning ${library.name}...`,
        color: "primary",
      });
      // Scan completion will be detected via subscription
    } catch (err) {
      console.error("Failed to scan library:", err);
      setIsScanning(false);
    }
  };

  // Track previous scanning state to detect transitions
  const prevScanningRef = useRef(library?.scanning);

  // Subscribe to library changes to refresh data on any change
  useEffect(() => {
    if (!library) return;

    const sub = graphqlClient
      .subscription<{
        libraryChanged: LibraryChangedEvent;
      }>(LIBRARY_CHANGED_SUBSCRIPTION, {})
      .subscribe({
        next: (result) => {
          if (result.data?.libraryChanged) {
            const event = result.data.libraryChanged;
            // Only handle events for this library
            if (event.libraryId === library.id && event.library) {
              const wasScanning = prevScanningRef.current;
              const nowScanning = event.library.scanning;

              // Update local library state
              setLibrary(event.library);
              prevScanningRef.current = nowScanning;

              // Handle scan state transitions for UI feedback
              if (wasScanning && !nowScanning) {
                setIsScanning(false);
                addToast({
                  title: "Scan Complete",
                  description: `Finished scanning ${library.name}`,
                  color: "success",
                });
              } else if (!wasScanning && nowScanning) {
                // Scan started (e.g., from another client)
                setIsScanning(true);
              }

              // Always refresh data on any library change
              // (new content, scan completion, metadata updates, etc.)
              fetchData(true);
            }
          }
        },
      });

    return () => sub.unsubscribe();
  }, [library?.id, library?.name, fetchData]);

  // Show not found state only after loading is complete
  if (!loading && !library) {
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

  // Use template data during loading, real data when available
  const displayLibrary = library ?? libraryTemplate;
  const typeInfo = getLibraryTypeInfo(displayLibrary.libraryType);

  // Always provide context with loading state so subroutes can show shimmer
  const contextValue: LibraryContextValue = {
    library: displayLibrary,
    loading,
    tvShows,
    fetchData,
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
        <ShimmerLoader
          loading={loading}
          templateProps={{ library: libraryTemplate }}
        >
          <div className="mb-6">
            {/* Breadcrumb */}
            <Breadcrumbs className="mb-2">
              <BreadcrumbItem href="/libraries">Libraries</BreadcrumbItem>
              <BreadcrumbItem isCurrent>{displayLibrary.name}</BreadcrumbItem>
            </Breadcrumbs>

            {/* Title and Stats */}
            <div className="flex items-start justify-between">
              <div className="flex items-center gap-4">
                <typeInfo.Icon className="w-10 h-10" />
                <div>
                  <h1 className="text-2xl font-bold">{displayLibrary.name}</h1>
                  <div className="flex items-center gap-3 text-sm text-default-500 mt-1">
                    <span>
                      {displayLibrary.libraryType === "TV"
                        ? `${displayLibrary.showCount} shows`
                        : displayLibrary.libraryType === "MOVIES"
                          ? `${displayLibrary.movieCount} movies`
                          : `${displayLibrary.itemCount} items`}
                    </span>
                    <span>•</span>
                    <span>{formatBytes(displayLibrary.totalSizeBytes)}</span>
                    <span>•</span>
                    <span className="font-mono text-xs">
                      {displayLibrary.path}
                    </span>
                  </div>
                </div>
              </div>

              <div className="flex items-center gap-2">
                {/* Setting Badges - Auto Hunt covers both search and RSS download */}
                <AutoHuntBadge isEnabled={displayLibrary.autoHunt || displayLibrary.autoDownload} />
                <FileOrganizationBadge
                  isEnabled={displayLibrary.organizeFiles}
                />
                <QualityFilterBadge
                  resolutions={displayLibrary.allowedResolutions || []}
                  codecs={displayLibrary.allowedVideoCodecs || []}
                  requireHdr={displayLibrary.requireHdr || false}
                />
                <Button
                  color="primary"
                  variant="flat"
                  size="sm"
                  onPress={handleScanLibrary}
                  isLoading={isScanning || displayLibrary.scanning}
                  isDisabled={loading || isScanning || displayLibrary.scanning}
                >
                  {isScanning || displayLibrary.scanning
                    ? "Scanning..."
                    : "Scan Now"}
                </Button>
              </div>
            </div>
          </div>
        </ShimmerLoader>

        {/* Tabbed Content with Outlet for subroutes */}
        <LibraryLayout
          activeTab={getActiveTab()}
          libraryId={libraryId}
          libraryType={displayLibrary.libraryType}
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
