import { createFileRoute, redirect } from "@tanstack/react-router";
import { useState, useEffect, useCallback, useRef } from "react";
import { Button } from "@heroui/button";
import { Card, CardBody } from "@heroui/card";
import { useDisclosure } from "@heroui/modal";
import { ShimmerLoader } from "../../components/shared/ShimmerLoader";
import { librariesTemplateNodes } from "../../lib/template-data";
import { Tooltip } from "@heroui/tooltip";
import { addToast } from "@heroui/toast";
import { ConfirmModal } from "../../components/ConfirmModal";
import { AddLibraryModal, LibraryGridCard } from "../../components/library";
import { IconPlus } from "@tabler/icons-react";
import { RouteError } from "../../components/RouteError";
import { sanitizeError } from "../../lib/format";
import { graphqlClient } from "../../lib/graphql";
import {
  LibrariesDocument,
  LibraryChangedDocument,
  CreateLibraryDocument,
  DeleteLibraryDocument,
  MeDocument,
  ChangeAction,
  type CreateLibraryInput as GenCreateLibraryInput,
} from "../../lib/graphql/generated/graphql";
import {
  TV_SHOWS_QUERY,
  MOVIES_QUERY,
  ALBUMS_QUERY,
  AUDIOBOOKS_QUERY,
  SCAN_LIBRARY_MUTATION,
  type LibraryNode,
  type TvShow,
  type Movie,
  type Album,
  type Audiobook,
  type CreateLibraryInput,
} from "../../lib/graphql";
import { Image } from "@heroui/image";

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
  const {
    isOpen: isAddOpen,
    onOpen: onAddOpen,
    onClose: onAddClose,
  } = useDisclosure();
  const {
    isOpen: isConfirmOpen,
    onOpen: onConfirmOpen,
    onClose: onConfirmClose,
  } = useDisclosure();
  const [libraries, setLibraries] = useState<LibraryNode[]>([]);
  const [showsByLibrary, setShowsByLibrary] = useState<
    Record<string, TvShow[]>
  >({});
  const [moviesByLibrary, setMoviesByLibrary] = useState<
    Record<string, Movie[]>
  >({});
  const [albumsByLibrary, setAlbumsByLibrary] = useState<
    Record<string, Album[]>
  >({});
  const [audiobooksByLibrary, setAudiobooksByLibrary] = useState<
    Record<string, Audiobook[]>
  >({});
  const [libraryToDelete, setLibraryToDelete] = useState<{
    id: string;
    name: string;
  } | null>(null);
  const [loading, setLoading] = useState(true);
  const [actionLoading, setActionLoading] = useState(false);
  const [currentUserId, setCurrentUserId] = useState<string | null>(null);

  // Track if initial load is done to avoid showing spinner on background refreshes
  const initialLoadDone = useRef(false);

  const fetchLibraries = useCallback(async (isBackgroundRefresh = false) => {
    try {
      // Only show loading spinner on initial load
      if (!isBackgroundRefresh) {
        setLoading(true);
      }
      const { data, error } = await graphqlClient
        .query(LibrariesDocument, {})
        .toPromise();

      if (error) {
        // Silently ignore auth errors - they can happen during login race conditions
        const isAuthError = error.message
          ?.toLowerCase()
          .includes("authentication");
        if (!isAuthError) {
          console.error("Failed to fetch libraries:", error);
          if (!isBackgroundRefresh) {
            addToast({
              title: "Error",
              description: "Failed to load libraries",
              color: "danger",
            });
          }
        }
        return;
      }

      const connection = data?.Libraries;
      if (connection) {
        const list = connection.Edges.map((e) => e.Node);
        setLibraries(list);

        const tvLibraries = list.filter((l) => l.LibraryType === "TV");
        const showsPromises = tvLibraries.map(async (lib) => {
          try {
            const result = await graphqlClient
              .query<{
                tvShows: TvShow[];
              }>(TV_SHOWS_QUERY, { libraryId: lib.Id })
              .toPromise();
            return { libraryId: lib.Id, shows: result.data?.tvShows || [] };
          } catch {
            return { libraryId: lib.Id, shows: [] };
          }
        });

        const showsResults = await Promise.all(showsPromises);
        const showsMap: Record<string, TvShow[]> = {};
        for (const result of showsResults) {
          showsMap[result.libraryId] = result.shows;
        }
        setShowsByLibrary(showsMap);

        const movieLibraries = list.filter((l) => l.LibraryType === "MOVIES");
        const moviesPromises = movieLibraries.map(async (lib) => {
          try {
            const result = await graphqlClient
              .query<{ movies: Movie[] }>(MOVIES_QUERY, { libraryId: lib.Id })
              .toPromise();
            return { libraryId: lib.Id, movies: result.data?.movies || [] };
          } catch {
            return { libraryId: lib.Id, movies: [] };
          }
        });

        const moviesResults = await Promise.all(moviesPromises);
        const moviesMap: Record<string, Movie[]> = {};
        for (const result of moviesResults) {
          moviesMap[result.libraryId] = result.movies;
        }
        setMoviesByLibrary(moviesMap);

        const musicLibraries = list.filter((l) => l.LibraryType === "MUSIC");
        const albumsPromises = musicLibraries.map(async (lib) => {
          try {
            const result = await graphqlClient
              .query<{ albums: Album[] }>(ALBUMS_QUERY, { libraryId: lib.Id })
              .toPromise();
            return { libraryId: lib.Id, albums: result.data?.albums || [] };
          } catch {
            return { libraryId: lib.Id, albums: [] };
          }
        });

        const albumsResults = await Promise.all(albumsPromises);
        const albumsMap: Record<string, Album[]> = {};
        for (const result of albumsResults) {
          albumsMap[result.libraryId] = result.albums;
        }
        setAlbumsByLibrary(albumsMap);

        const audiobookLibraries = list.filter(
          (l) => l.LibraryType === "AUDIOBOOKS",
        );
        const audiobooksPromises = audiobookLibraries.map(async (lib) => {
          try {
            const result = await graphqlClient
              .query<{
                audiobooks: Audiobook[];
              }>(AUDIOBOOKS_QUERY, { libraryId: lib.Id })
              .toPromise();
            return {
              libraryId: lib.Id,
              audiobooks: result.data?.audiobooks || [],
            };
          } catch {
            return { libraryId: lib.Id, audiobooks: [] };
          }
        });

        const audiobooksResults = await Promise.all(audiobooksPromises);
        const audiobooksMap: Record<string, Audiobook[]> = {};
        for (const result of audiobooksResults) {
          audiobooksMap[result.libraryId] = result.audiobooks;
        }
        setAudiobooksByLibrary(audiobooksMap);
      }
    } catch (err) {
      console.error("Failed to fetch libraries:", err);
    } finally {
      setLoading(false);
      initialLoadDone.current = true;
    }
  }, []);

  useEffect(() => {
    fetchLibraries();
  }, [fetchLibraries]);

  // Fetch current user Id for CreateLibrary (required by schema)
  useEffect(() => {
    graphqlClient
      .query(MeDocument, {})
      .toPromise()
      .then((res) => {
        if (res.data?.Me?.Id) setCurrentUserId(res.data.Me.Id);
      })
      .catch(() => {});
  }, []);

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
              if (event.Library) {
                setLibraries((prev) => [...prev, event.Library!]);
              }
              break;
            case ChangeAction.Updated:
              if (event.Library) {
                setLibraries((prev) =>
                  prev.map((lib) =>
                    lib.Id === event.Id ? event.Library! : lib,
                  ),
                );
              }
              break;
            case ChangeAction.Deleted:
              setLibraries((prev) => prev.filter((lib) => lib.Id !== event.Id));
              break;
          }
        },
      });

    return () => subscription.unsubscribe();
  }, []);

  const handleAddLibrary = async (input: CreateLibraryInput) => {
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
      UserId: currentUserId,
      Name: input.name,
      Path: input.path,
      LibraryType: input.libraryType,
      Icon: input.icon ?? null,
      Color: input.color ?? null,
      AutoScan: input.autoScan ?? true,
      ScanIntervalMinutes: input.scanIntervalMinutes ?? 60,
      WatchForChanges: input.watchForChanges ?? false,
      AutoAddDiscovered: input.autoAddDiscovered ?? false,
      AutoDownload: input.autoDownload ?? false,
      AutoHunt: input.autoHunt ?? false,
      Scanning: false,
      CreatedAt: now,
      UpdatedAt: now,
      LastScannedAt: null,
    };
    try {
      setActionLoading(true);
      const { data, error } = await graphqlClient
        .mutation(CreateLibraryDocument, { Input: genInput })
        .toPromise();

      if (error || !data?.CreateLibrary.Success) {
        const errorMsg =
          data?.CreateLibrary.Error || error?.message || "Unknown error";
        addToast({
          title: "Error",
          description: `Failed to create library: ${errorMsg}`,
          color: "danger",
        });
        return;
      }

      addToast({
        title: "Success",
        description: `Library "${input.name}" created`,
        color: "success",
      });

      // Refresh libraries
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

  const handleScan = async (libraryId: string, libraryName: string) => {
    try {
      const { data, error } = await graphqlClient
        .mutation<{
          scanLibrary: { status: string; message: string | null };
        }>(SCAN_LIBRARY_MUTATION, { id: libraryId })
        .toPromise();

      if (error) {
        addToast({
          title: "Error",
          description: sanitizeError(error),
          color: "danger",
        });
        return;
      }

      addToast({
        title: "Scan Started",
        description: data?.scanLibrary.message || `Scanning ${libraryName}...`,
        color: "primary",
      });
    } catch (err) {
      console.error("Failed to scan library:", err);
    }
  };

  const handleDeleteClick = (libraryId: string, libraryName: string) => {
    setLibraryToDelete({ id: libraryId, name: libraryName });
    onConfirmOpen();
  };

  const handleDelete = async () => {
    if (!libraryToDelete) return;

    try {
      const { data, error } = await graphqlClient
        .mutation(DeleteLibraryDocument, { Id: libraryToDelete.id })
        .toPromise();

      if (error || !data?.DeleteLibrary.Success) {
        addToast({
          title: "Error",
          description: sanitizeError(
            data?.DeleteLibrary.Error || "Failed to delete library",
          ),
          color: "danger",
        });
        onConfirmClose();
        return;
      }

      addToast({
        title: "Deleted",
        description: `Library "${libraryToDelete.name}" deleted`,
        color: "success",
      });

      // Refresh libraries
      await fetchLibraries();
    } catch (err) {
      console.error("Failed to delete library:", err);
    }
    onConfirmClose();
  };

  return (
    <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-8">
      {/* Header with title and add button */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold">Libraries</h1>
          <p className="text-default-500">
            Organize and manage your media collections
          </p>
        </div>
        <Tooltip content="Add Library">
          <Button isIconOnly color="primary" size="sm" onPress={onAddOpen}>
            <IconPlus size={16} />
          </Button>
        </Tooltip>
      </div>

      {/* Content */}
      {!loading && libraries.length === 0 ? (
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
      ) : (
        <ShimmerLoader
          loading={loading}
          delay={500}
          templateProps={{ libraries: librariesTemplateNodes }}
        >
          <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
            {(loading ? librariesTemplateNodes : libraries).map((library) => (
              <LibraryGridCard
                key={library.Id}
                library={library}
                shows={showsByLibrary[library.Id] || []}
                movies={moviesByLibrary[library.Id] || []}
                albums={albumsByLibrary[library.Id] || []}
                audiobooks={audiobooksByLibrary[library.Id] || []}
                onScan={() => handleScan(library.Id, library.Name)}
                onDelete={() => handleDeleteClick(library.Id, library.Name)}
              />
            ))}
          </div>
        </ShimmerLoader>
      )}

      {/* Confirm Delete Modal */}
      <ConfirmModal
        isOpen={isConfirmOpen}
        onClose={onConfirmClose}
        onConfirm={handleDelete}
        title="Delete Library"
        message={`Are you sure you want to delete "${libraryToDelete?.name}"?`}
        description="This will remove the library and all associated shows from your collection. Downloaded files will not be deleted."
        confirmLabel="Delete"
        confirmColor="danger"
      />

      <AddLibraryModal
        isOpen={isAddOpen}
        onClose={onAddClose}
        onAdd={handleAddLibrary}
        isLoading={actionLoading}
      />
    </div>
  );
}
