import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { useMemo } from "react";
import { Button } from "@heroui/button";
import { Card, CardBody } from "@heroui/card";
import { useDisclosure } from "@heroui/modal";
import { addToast } from "@heroui/toast";
import { useQuery, gql } from "../../../lib/graphql/client";
import {
  DataTable,
  type DataTableColumn,
  type RowAction,
} from "../../../components/data-table";
import { useLibraryContext } from "../$libraryId";
import { IconEye, IconPlus, IconStack } from "@tabler/icons-react";
import { AddCollectionModal } from "../../../components/library/AddCollectionModal";
import { CollectionSummaryCard } from "../../../components/library/CollectionSummaryCard";
import { CollectionPoster } from "../../../components/library/CollectionCardParts";

export const Route = createFileRoute("/libraries/$libraryId/collections")({
  component: CollectionsPage,
});

interface CollectionNode {
  Id: string;
  TmdbCollectionId: number;
  Name: string;
  PosterUrl: string | null;
  BackdropUrl: string | null;
  MovieCount: number;
  DownloadedMovies: {
    PageInfo: {
      TotalCount: number | null;
    };
  };
}

interface CollectionSummary {
  rowId: string;
  dbId: string | null;
  tmdbId: number | null;
  name: string;
  posterUrl: string | null;
  backdropUrl: string | null;
  totalMovieCount: number;
  hasFileCount: number;
}

interface LibraryCollectionsQueryData {
  Collections: {
    Edges: Array<{ Node: CollectionNode }>;
  };
}

const LIBRARY_COLLECTIONS_QUERY = gql`
  query LibraryCollectionsRoute(
    $Where: CollectionWhereInput
    $Page: PageInput
    $LibraryId: String!
  ) {
    Collections(Where: $Where, Page: $Page) {
      Edges {
        Node {
          Id
          TmdbCollectionId
          Name
          PosterUrl
          BackdropUrl
          MovieCount
          DownloadedMovies: Movies(
            Where: {
              LibraryId: { Eq: $LibraryId }
              HasFile: { Eq: true }
            }
            Page: { Limit: 1, Offset: 0 }
          ) {
            PageInfo {
              TotalCount
            }
          }
        }
      }
    }
  }
`;

function CollectionsPage() {
  const { library } = useLibraryContext();
  const navigate = useNavigate();
  const {
    isOpen: isAddOpen,
    onOpen: onAddOpen,
    onClose: onAddClose,
  } = useDisclosure();

  const { data, previousData, loading, refetch } =
    useQuery<LibraryCollectionsQueryData>(LIBRARY_COLLECTIONS_QUERY, {
      variables: {
        LibraryId: library.Id,
        Where: {
          LibraryId: { Eq: library.Id },
        },
        Page: { Limit: 5000, Offset: 0 },
      },
      fetchPolicy: "cache-and-network",
    });
  const collectionNodes = useMemo(
    () =>
      (data?.Collections?.Edges ?? previousData?.Collections?.Edges ?? []).map(
        (edge) => edge.Node,
      ),
    [data?.Collections?.Edges, previousData?.Collections?.Edges],
  );

  const collections = useMemo<CollectionSummary[]>(() => {
    return collectionNodes
      .map((collection) => {
        const hasFileCount = collection.DownloadedMovies?.PageInfo?.TotalCount ?? 0;
        return {
          rowId: collection.Id,
          dbId: collection.Id,
          tmdbId: collection.TmdbCollectionId,
          name: collection.Name,
          posterUrl: collection.PosterUrl ?? null,
          backdropUrl: collection.BackdropUrl ?? null,
          totalMovieCount: collection.MovieCount ?? 0,
          hasFileCount,
        };
      })
      .sort((a, b) => a.name.localeCompare(b.name));
  }, [collectionNodes]);

  const collectionColumns: DataTableColumn<CollectionSummary>[] = [
    {
      key: "name",
      label: "Collection",
      sortable: true,
      render: (collection) => (
        <div className="flex items-center gap-3">
          <CollectionPoster
            posterUrl={collection.posterUrl}
            name={collection.name}
            imageClassName="w-10 h-14 object-cover rounded"
            fallbackClassName="w-10 h-14 bg-default-200 rounded flex items-center justify-center"
          />
          <div>
            <p className="font-medium">{collection.name}</p>
            {collection.tmdbId ? (
              <p className="text-xs text-default-500">
                TMDB #{collection.tmdbId}
              </p>
            ) : null}
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
          {collection.hasFileCount}/{collection.totalMovieCount}
        </span>
      ),
    },
  ];

  const collectionActions: RowAction<CollectionSummary>[] = [
    {
      key: "open",
      label: "Open Collection",
      icon: <IconEye size={16} />,
      onAction: (collection) => {
        if (!collection.dbId) {
          addToast({
            title: "Collection Not Synced",
            description: "This collection does not have an internal ID yet.",
            color: "warning",
          });
          return;
        }
        void navigate({
          to: "/collections/$collectionId",
          params: { collectionId: collection.dbId },
        });
      },
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
          <Button
            size="sm"
            color="primary"
            onPress={onAddOpen}
            startContent={<IconPlus size={14} />}
          >
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
          isLoading={loading && collections.length === 0}
          showViewModeToggle
          defaultViewMode="cards"
          cardGridClassName="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-4"
          cardRenderer={({ item }) => (
            <CollectionSummaryCard
              name={item.name}
              posterUrl={item.posterUrl}
              backdropUrl={item.backdropUrl}
              hasFileCount={item.hasFileCount}
              totalMovieCount={item.totalMovieCount}
              onPress={() => {
                if (!item.dbId) {
                  addToast({
                    title: "Collection Not Synced",
                    description:
                      "This collection does not have an internal ID yet.",
                    color: "warning",
                  });
                  return;
                }
                void navigate({
                  to: "/collections/$collectionId",
                  params: { collectionId: item.dbId },
                });
              }}
            />
          )}
          emptyContent={
            <Card className="bg-content1/50 border-default-300 border-dashed border-2">
              <CardBody className="py-12 text-center">
                <IconStack size={48} className="mx-auto mb-4 text-purple-400" />
                <h3 className="text-lg font-semibold mb-2">
                  No collections yet
                </h3>
                <p className="text-default-500 mb-4">
                  Collections appear automatically when added movies include
                  TMDB collection data.
                </p>
                <p className="text-xs text-default-400">
                  Library: {library.Name}
                </p>
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
