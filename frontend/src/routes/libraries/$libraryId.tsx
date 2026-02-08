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
  ScanLibraryModal,
  type LibraryTab,
} from "../../components/library";
import { sanitizeError } from "../../lib/format";
import type {
  DeleteShowRouteMutation,
  DeleteShowRouteMutationVariables,
  LibraryChangedSubscription,
  LibraryChangedSubscriptionVariables,
  LibraryDetailRouteQuery,
  LibraryDetailRouteQueryVariables,
  ShowChangedSubscription,
  ShowChangedSubscriptionVariables,
  UpdateLibraryRouteMutation,
  UpdateLibraryRouteMutationVariables,
} from "../../lib/graphql/generated/graphql";
import { IconRefresh } from "@tabler/icons-react";
import {
  useQuery,
  useMutation,
  useSubscription,
} from "../../lib/graphql/client";
import {
  DeleteShowRouteDocument,
  LibraryChangedDocument,
  LibraryDetailRouteDocument,
  ShowChangedDocument,
  UpdateLibraryRouteDocument,
} from "../../lib/graphql/generated/graphql";
import {
  getLibraryTypeInfo,
  type LibraryType,
  type UpdateLibraryInput,
} from "../../lib/graphql";

type LibraryRouteData = NonNullable<LibraryDetailRouteQuery["Library"]>;

// Context for sharing library data with subroutes
export interface LibraryContextValue {
  library: LibraryRouteData;
  loading: boolean;
  refetch: () => void;
  actionLoading: boolean;
  handleDeleteShowClick: (showId: string, showName: string) => void;
  handleUpdateLibrary: (input: UpdateLibraryInput) => Promise<void>;
  onOpenAddShow: () => void;
  downloadProgress: ContentProgressMap;
  mediaRefreshToken: number;
}

const defaultContextValue: LibraryContextValue = {
  library: {} as LibraryRouteData,
  loading: true,
  refetch: () => {},
  actionLoading: false,
  handleDeleteShowClick: () => {},
  handleUpdateLibrary: async () => {},
  onOpenAddShow: () => {},
  downloadProgress: new Map(),
  mediaRefreshToken: 0,
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
    isOpen: isScanOpen,
    onOpen: onScanOpen,
    onClose: onScanClose,
  } = useDisclosure();
  const {
    isOpen: isConfirmOpen,
    onOpen: onConfirmOpen,
    onClose: onConfirmClose,
  } = useDisclosure();

  const [showToDelete, setShowToDelete] = useState<{
    id: string;
    name: string;
  } | null>(null);
  const [isScanning, setIsScanning] = useState(false);
  const [mediaRefreshToken, setMediaRefreshToken] = useState(0);

  const {
    data: libraryData,
    previousData: previousLibraryData,
    loading: libraryLoading,
    refetch: refetchLibrary,
  } = useQuery<LibraryDetailRouteQuery, LibraryDetailRouteQueryVariables>(
    LibraryDetailRouteDocument,
    {
    variables: { Id: libraryId },
    fetchPolicy: "cache-and-network",
    },
  );

  const library = libraryData?.Library ?? previousLibraryData?.Library ?? null;

  const refetchAll = useCallback(() => {
    void refetchLibrary();
  }, [refetchLibrary]);

  const [deleteShow, { loading: deletingShow }] = useMutation<
    DeleteShowRouteMutation,
    DeleteShowRouteMutationVariables
  >(DeleteShowRouteDocument);

  const [updateLibraryMutation, { loading: updatingLibrary }] = useMutation<
    UpdateLibraryRouteMutation,
    UpdateLibraryRouteMutationVariables
  >(UpdateLibraryRouteDocument);

  const { data: libraryChangedData } = useSubscription<
    LibraryChangedSubscription,
    LibraryChangedSubscriptionVariables
  >(LibraryChangedDocument, {
    skip: !library,
  });
  const { data: showChangedData } = useSubscription<
    ShowChangedSubscription,
    ShowChangedSubscriptionVariables
  >(ShowChangedDocument, {
    skip: !library,
    variables: { Filter: { Actions: ["Created", "Updated", "Deleted"] } },
  });

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
  }, [library?.Scanning, isScanning, library]);

  // Update page title
  useEffect(() => {
    if (library) {
      document.title = `Librarian - ${library.Name}`;
    }
    return () => {
      document.title = "Librarian";
    };
  }, [library]);

  // Subscribe to data changes for live updates
  useDataReactivity(refetchAll, {
    onTorrentComplete: true,
    periodicInterval: false,
    onFocus: false,
  });

  // Subscribe to content download progress
  const initialLoading = libraryLoading && !library;
  const downloadProgress = useContentDownloadProgress({
    libraryId,
    enabled: !initialLoading && !!library,
  });

  // Track previous scanning state for toast notifications
  const prevScanningRef = useRef(library?.Scanning);

  useEffect(() => {
    if (!library) return;

    const event = libraryChangedData?.LibraryChanged;
    if (!event || event.Id !== library.Id || !event.Library) return;

    const wasScanning = prevScanningRef.current;
    const nowScanning = event.Library.Scanning;
    prevScanningRef.current = nowScanning;

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

    refetchAll();
  }, [libraryChangedData, library, refetchAll]);

  useEffect(() => {
    if (!library) return;
    const event = showChangedData?.ShowChanged;
    if (!event) return;
    if (event.Show?.LibraryId && event.Show.LibraryId !== library.Id) return;

    setMediaRefreshToken((v) => v + 1);
  }, [library, showChangedData]);

  const handleDeleteShowClick = useCallback(
    (showId: string, showName: string) => {
      setShowToDelete({ id: showId, name: showName });
      onConfirmOpen();
    },
    [onConfirmOpen],
  );

  const handleDeleteShow = useCallback(async () => {
    if (!showToDelete) return;

    try {
      const { data } = await deleteShow({ variables: { Id: showToDelete.id } });

      if (!data?.DeleteShow.Success) {
        addToast({
          title: "Error",
          description: sanitizeError(
            data?.DeleteShow.Error || "Failed to delete show",
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

      refetchAll();
    } catch (err) {
      console.error("Failed to delete show:", err);
    }
    onConfirmClose();
  }, [deleteShow, onConfirmClose, refetchAll, showToDelete]);

  const handleUpdateLibrary = useCallback(
    async (input: UpdateLibraryInput) => {
      if (!library) return;

      try {
        const { data } = await updateLibraryMutation({
          variables: {
            Id: library.Id,
            Input: input,
          },
        });

        if (!data?.UpdateLibrary.Success) {
          addToast({
            title: "Error",
            description: `Failed to update library: ${data?.UpdateLibrary.Error || "Unknown error"}`,
            color: "danger",
          });
          return;
        }

        addToast({
          title: "Success",
          description: "Library settings saved",
          color: "success",
        });

        refetchAll();
      } catch (err) {
        console.error("Failed to update library:", err);
        addToast({
          title: "Error",
          description: "Failed to update library",
          color: "danger",
        });
      }
    },
    [library, refetchAll, updateLibraryMutation],
  );

  if (!libraryLoading && !library) {
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

  if (initialLoading && !library) {
    return (
      <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-8 flex flex-col grow">
        <div className="flex items-center justify-center py-12">
          <Spinner size="lg" />
        </div>
      </div>
    );
  }

  if (!library) return null;

  const typeInfo = getLibraryTypeInfo(library.LibraryType as LibraryType);
  const scanningActive = isScanning || library.Scanning;

  const contextValue: LibraryContextValue = {
    library,
    loading: libraryLoading,
    refetch: refetchAll,
    actionLoading: updatingLibrary || deletingShow,
    handleDeleteShowClick,
    handleUpdateLibrary,
    onOpenAddShow: onOpen,
    downloadProgress,
    mediaRefreshToken,
  };

  return (
    <LibraryContext.Provider value={contextValue}>
      <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-8 flex flex-col grow">
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
                isLoading={scanningActive}
                startContent={!scanningActive ? <IconRefresh size={16} /> : undefined}
                onPress={onScanOpen}
              >
                {scanningActive ? "Scanning..." : "Scan Library"}
              </Button>
            </div>
          </div>
        </div>

        <LibraryLayout
          activeTab={getActiveTab()}
          libraryId={libraryId}
          libraryType={library.LibraryType as LibraryType}
        >
          <Outlet />
        </LibraryLayout>

        <AddShowModal
          isOpen={isOpen}
          onClose={onClose}
          libraryId={libraryId}
          onAdded={() => {
            refetchAll();
            setMediaRefreshToken((v) => v + 1);
          }}
        />

        <ScanLibraryModal
          isOpen={isScanOpen}
          onClose={onScanClose}
          libraryId={libraryId}
          libraryName={library.Name}
          onScanStarted={() => {
            setIsScanning(true);
            refetchAll();
          }}
        />

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
