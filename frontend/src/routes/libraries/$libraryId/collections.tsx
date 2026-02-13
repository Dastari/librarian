import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { useMemo } from "react";
import { Button } from "@heroui/button";
import { Card, CardBody } from "@heroui/card";
import { Chip } from "@heroui/chip";
import { Image } from "@heroui/image";
import { useDisclosure } from "@heroui/modal";
import { useQuery, gql } from "../../../lib/graphql/client";
import { DataTable, type DataTableColumn, type RowAction } from "../../../components/data-table";
import { useLibraryContext } from "../$libraryId";
import { IconEye, IconPlus, IconStack } from "@tabler/icons-react";
import { AddCollectionModal } from "../../../components/library/AddCollectionModal";

export const Route = createFileRoute("/libraries/$libraryId/collections")({
  component: CollectionsPage,
});

interface CollectionNode {
  Id: string;
  TmdbCollectionId: number;
  Name: string;
  PosterUrl: string | null;
  MovieCount: number;
  Movies: {
    Edges: Array<{
      Node: {
        Id: string;
        MediaFileId: string | null;
        Wanted: boolean;
      };
    }>;
  };
}

interface FallbackMovieNode {
  CollectionId: number | null;
  CollectionName: string | null;
  CollectionPosterUrl: string | null;
  MediaFileId: string | null;
  Wanted: boolean;
}

interface CollectionSummary {
  rowId: string;
  id: number;
  name: string;
  posterUrl: string | null;
  totalMovieCount: number;
  inLibraryCount: number;
  downloadedCount: number;
  wantedCount: number;
  missingCount: number;
}

interface LibraryCollectionsQueryData {
  Collections: {
    Edges: Array<{ Node: CollectionNode }>;
  };
}

interface LibraryCollectionsFallbackQueryData {
  Movies: {
    Edges: Array<{ Node: FallbackMovieNode }>;
  };
}

const LIBRARY_COLLECTIONS_QUERY = gql`
  query LibraryCollectionsRoute($Where: CollectionWhereInput, $Page: PageInput) {
    Collections(Where: $Where, Page: $Page) {
      Edges {
        Node {
          Id
          TmdbCollectionId
          Name
          PosterUrl
          MovieCount
          Movies(Page: { Limit: 500, Offset: 0 }) {
            Edges {
              Node {
                Id
                MediaFileId
                Wanted
              }
            }
          }
        }
      }
    }
  }
`;

const LIBRARY_COLLECTIONS_FALLBACK_QUERY = gql`
  query LibraryCollectionsFallbackRoute($Where: MovieWhereInput, $Page: PageInput) {
    Movies(Where: $Where, Page: $Page) {
      Edges {
        Node {
          CollectionId
          CollectionName
          CollectionPosterUrl
          MediaFileId
          Wanted
        }
      }
    }
  }
`;

function CollectionsPage() {
  const { library } = useLibraryContext();
  const navigate = useNavigate();
  const { isOpen: isAddOpen, onOpen: onAddOpen, onClose: onAddClose } = useDisclosure();

  const { data, previousData, loading, refetch } = useQuery<LibraryCollectionsQueryData>(
    LIBRARY_COLLECTIONS_QUERY,
    {
      variables: {
        Where: {
          LibraryId: { Eq: library.Id },
        },
        Page: { Limit: 5000, Offset: 0 },
      },
      fetchPolicy: "cache-and-network",
    },
  );
  const {
    data: fallbackData,
    previousData: previousFallbackData,
    loading: fallbackLoading,
  } = useQuery<LibraryCollectionsFallbackQueryData>(LIBRARY_COLLECTIONS_FALLBACK_QUERY, {
    variables: {
      Where: {
        LibraryId: { Eq: library.Id },
      },
      Page: { Limit: 5000, Offset: 0 },
    },
    fetchPolicy: "cache-and-network",
  });

  const collectionNodes = useMemo(
    () => (data?.Collections?.Edges ?? previousData?.Collections?.Edges ?? []).map((edge) => edge.Node),
    [data?.Collections?.Edges, previousData?.Collections?.Edges],
  );

  const collections = useMemo<CollectionSummary[]>(() => {
    const collectionRows = collectionNodes
      .map((collection) => {
        const libraryMovies = collection.Movies?.Edges?.map((edge) => edge.Node) ?? [];
        const inLibraryCount = libraryMovies.length;
        const downloadedCount = libraryMovies.filter((movie) => movie.MediaFileId != null).length;
        const wantedCount = libraryMovies.filter(
          (movie) => movie.MediaFileId == null && movie.Wanted,
        ).length;
        const missingCount = Math.max(0, (collection.MovieCount ?? 0) - downloadedCount - wantedCount);

        return {
          rowId: collection.Id,
          id: collection.TmdbCollectionId,
          name: collection.Name,
          posterUrl: collection.PosterUrl ?? null,
          totalMovieCount: collection.MovieCount ?? 0,
          inLibraryCount,
          downloadedCount,
          wantedCount,
          missingCount,
        };
      })
      .sort((a, b) => a.name.localeCompare(b.name));

    if (collectionRows.length > 0) {
      return collectionRows;
    }

    const fallbackMovies =
      (fallbackData?.Movies?.Edges ?? previousFallbackData?.Movies?.Edges ?? []).map(
        (edge) => edge.Node,
      );
    const grouped = new Map<number, CollectionSummary>();

    for (const movie of fallbackMovies) {
      if (movie.CollectionId == null) continue;
      const existing = grouped.get(movie.CollectionId);
      if (!existing) {
        grouped.set(movie.CollectionId, {
          rowId: String(movie.CollectionId),
          id: movie.CollectionId,
          name: movie.CollectionName ?? `Collection ${movie.CollectionId}`,
          posterUrl: movie.CollectionPosterUrl ?? null,
          totalMovieCount: 0,
          inLibraryCount: 1,
          downloadedCount: movie.MediaFileId ? 1 : 0,
          wantedCount: !movie.MediaFileId && movie.Wanted ? 1 : 0,
          missingCount: !movie.MediaFileId && !movie.Wanted ? 1 : 0,
        });
        continue;
      }
      existing.inLibraryCount += 1;
      if (movie.MediaFileId) existing.downloadedCount += 1;
      else if (movie.Wanted) existing.wantedCount += 1;
      else existing.missingCount += 1;
    }

    return [...grouped.values()]
      .map((row) => ({ ...row, totalMovieCount: row.inLibraryCount }))
      .sort((a, b) => a.name.localeCompare(b.name));
  }, [
    collectionNodes,
    fallbackData?.Movies?.Edges,
    previousFallbackData?.Movies?.Edges,
  ]);

  const collectionColumns: DataTableColumn<CollectionSummary>[] = [
    {
      key: "name",
      label: "Collection",
      sortable: true,
      render: (collection) => (
        <div className="flex items-center gap-3">
          {collection.posterUrl ? (
            <Image
              src={collection.posterUrl}
              alt={collection.name}
              className="w-10 h-14 object-cover rounded"
              loading="lazy"
            />
          ) : (
            <div className="w-10 h-14 bg-default-200 rounded flex items-center justify-center">
              <IconStack size={18} className="text-purple-400" />
            </div>
          )}
          <div>
            <p className="font-medium">{collection.name}</p>
            <p className="text-xs text-default-500">TMDB #{collection.id}</p>
          </div>
        </div>
      ),
    },
    {
      key: "movieCount",
      label: "Movies",
      width: 180,
      sortable: true,
      render: (collection) => (
        <span>
          {collection.inLibraryCount}/{collection.totalMovieCount}
        </span>
      ),
    },
    {
      key: "status",
      label: "Status",
      width: 260,
      render: (collection) => (
        <div className="flex items-center gap-2 flex-wrap">
          <Chip size="sm" color="success" variant="flat">
            {collection.downloadedCount} Downloaded
          </Chip>
          <Chip size="sm" color="warning" variant="flat">
            {collection.wantedCount} Wanted
          </Chip>
          <Chip size="sm" color="danger" variant="flat">
            {collection.missingCount} Missing
          </Chip>
        </div>
      ),
    },
  ];

  const collectionActions: RowAction<CollectionSummary>[] = [
    {
      key: "open",
      label: "Open Collection",
      icon: <IconEye size={16} />,
      onAction: (collection) =>
        void navigate({
          to: "/collections/$collectionId",
          params: { collectionId: collection.rowId },
        }),
    },
  ];

  return (
    <div className="flex flex-col w-full h-full overflow-hidden gap-4">
      <div className="flex items-center justify-between gap-4 shrink-0">
        <h2 className="text-xl font-semibold">Collections</h2>
        <div className="flex items-center gap-2">
          <Button size="sm" variant="flat" onPress={() => void refetch()}>
            Refresh
          </Button>
          <Button size="sm" color="primary" onPress={onAddOpen} startContent={<IconPlus size={14} />}>
            Add Collection
          </Button>
        </div>
      </div>

      <div className="flex-1 min-h-0">
        <DataTable
          stateKey="library-collections"
          data={collections}
          columns={collectionColumns}
          rowActions={collectionActions}
          getRowKey={(collection) => collection.rowId}
          ariaLabel="Movie collections table"
          searchPlaceholder="Search collections..."
          showItemCount
          fillHeight
          isLoading={loading && fallbackLoading && collections.length === 0}
          showViewModeToggle
          defaultViewMode="cards"
          cardGridClassName="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-4"
          cardRenderer={({ item }) => (
            <Card
              isPressable
              className="bg-content1 border border-default-200 hover:border-primary/40 transition-colors"
              onPress={() =>
                void navigate({
                  to: "/collections/$collectionId",
                  params: { collectionId: item.rowId },
                })
              }
            >
              <CardBody className="p-3">
                <div className="flex gap-3">
                  {item.posterUrl ? (
                    <Image
                      src={item.posterUrl}
                      alt={item.name}
                      className="w-14 h-20 object-cover rounded-md shrink-0"
                      loading="lazy"
                    />
                  ) : (
                    <div className="w-14 h-20 bg-default-200 rounded-md flex items-center justify-center shrink-0">
                      <IconStack size={18} className="text-purple-400" />
                    </div>
                  )}
                  <div className="min-w-0 flex-1 space-y-2">
                    <p className="font-medium truncate">{item.name}</p>
                    <p className="text-xs text-default-500">
                      {item.inLibraryCount}/{item.totalMovieCount} in library
                    </p>
                    <div className="flex flex-wrap gap-1">
                      <Chip size="sm" color="success" variant="flat">
                        {item.downloadedCount}
                      </Chip>
                      <Chip size="sm" color="warning" variant="flat">
                        {item.wantedCount}
                      </Chip>
                      <Chip size="sm" color="danger" variant="flat">
                        {item.missingCount}
                      </Chip>
                    </div>
                  </div>
                </div>
              </CardBody>
            </Card>
          )}
          emptyContent={
            <Card className="bg-content1/50 border-default-300 border-dashed border-2">
              <CardBody className="py-12 text-center">
                <IconStack size={48} className="mx-auto mb-4 text-purple-400" />
                <h3 className="text-lg font-semibold mb-2">No collections yet</h3>
                <p className="text-default-500 mb-4">
                  Collections appear automatically when added movies include TMDB collection data.
                </p>
                <p className="text-xs text-default-400">Library: {library.Name}</p>
              </CardBody>
            </Card>
          }
        />
      </div>

      <AddCollectionModal
        isOpen={isAddOpen}
        onClose={onAddClose}
        libraryId={library.Id}
        onAdded={() => void refetch()}
      />
    </div>
  );
}
