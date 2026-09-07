import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { useMemo } from "react";
import { Button } from "@heroui/button";
import { Card, CardBody } from "@heroui/card";
import { useDisclosure } from "@heroui/modal";
import { addToast } from "@heroui/toast";
import { useQuery } from "../../../lib/graphql/client";
import {
  DataTable,
  type DataTableColumn,
  type RowAction,
} from "../../../components/data-table";
import {
  LibraryCollectionsRouteDocument,
  type LibraryCollectionsRouteQuery,
} from "../../../lib/graphql/generated/graphql";
import { useLibraryContext } from "../$libraryId";
import { IconEye, IconPlus, IconStack } from "@tabler/icons-react";
import { AddCollectionModal } from "../../../components/library/AddCollectionModal";
import { CollectionSummaryCard } from "../../../components/library/CollectionSummaryCard";
import { CollectionPoster } from "../../../components/library/CollectionCardParts";

export const Route = createFileRoute("/libraries/$libraryId/collections")({
  component: CollectionsPage,
});

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

function CollectionsPage() {
  const { library } = useLibraryContext();
  const navigate = useNavigate();
  const {
    isOpen: isAddOpen,
    onOpen: onAddOpen,
    onClose: onAddClose,
  } = useDisclosure();

  const { data, previousData, loading, refetch } =
    useQuery<LibraryCollectionsRouteQuery>(LibraryCollectionsRouteDocument, {
      variables: {
        libraryId: library.id,
        where: {
          libraryId: { eq: library.id },
        },
        page: { limit: 5000, offset: 0 },
      },
      fetchPolicy: "cache-and-network",
    });
  const collectionNodes = useMemo(
    () =>
      (data?.collections?.edges ?? previousData?.collections?.edges ?? []).map(
        (edge) => edge.node,
      ),
    [data?.collections?.edges, previousData?.collections?.edges],
  );

  const collections = useMemo<CollectionSummary[]>(() => {
    return collectionNodes
      .map((collection) => {
        const hasFileCount =
          collection.downloadedMovies?.pageInfo?.totalCount ?? 0;
        return {
          rowId: collection.id,
          dbId: collection.id,
          tmdbId: collection.tmdbCollectionId,
          name: collection.name,
          posterUrl: collection.posterUrl ?? null,
          backdropUrl: collection.backdropUrl ?? null,
          totalMovieCount: collection.movieCount ?? 0,
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
      key: "totalMovieCount",
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
    <div className="flex h-full min-h-0 flex-1 flex-col gap-4 overflow-hidden w-full">
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

      <div className="flex min-h-0 flex-1 flex-col">
        <DataTable
          stateKey="library-collections"
          data={collections}
          columns={collectionColumns}
          rowActions={collectionActions}
          getRowKey={(collection) => collection.rowId}
          ariaLabel="Movie collections table"
          toolbarQueryPlaceholder="Search collections..."
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
                  library: {library.name}
                </p>
              </CardBody>
            </Card>
          }
        />
      </div>

      <AddCollectionModal
        isOpen={isAddOpen}
        onClose={onAddClose}
        libraryId={library.id}
        onAdded={() => void refetch()}
      />
    </div>
  );
}
