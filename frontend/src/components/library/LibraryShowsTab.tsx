import { useMemo, useCallback, useEffect } from "react";
import { useQueryState, parseAsString, parseAsStringLiteral } from "nuqs";
import { Button } from "@heroui/button";
import { Image } from "@heroui/image";
import { Card, CardBody } from "@heroui/card";
import { Link } from "@tanstack/react-router";
import {
  DataTable,
  AlphabetFilter,
  getFirstLetter,
  type DataTableColumn,
  type RowAction,
  type CardRendererProps,
} from "../data-table";
import {
  LibraryShowsTabDocument,
  type OrderDirection,
  type Show,
  type ShowOrderByInput,
} from "../../lib/graphql/generated/graphql";
import { useQuery } from "../../lib/graphql/client";
import {
  IconPlus,
  IconTrash,
  IconEye,
  IconDeviceTv,
} from "@tabler/icons-react";
import { TvShowCard } from "./TvShowCard";
import { MediaCardSkeleton } from "./MediaCardSkeleton";

// ============================================================================
// Component Props
// ============================================================================

interface LibraryShowsTabProps {
  libraryId: string;
  /** Parent loading state (e.g., library context still loading) */
  loading?: boolean;
  onDeleteShow: (showId: string, showName: string) => void;
  onAddShow: () => void;
  /** Callback to provide the refresh function to the parent */
  onRefreshReady?: (refreshFn: () => void) => void;
}

// ============================================================================
// Types for GraphQL response
// ============================================================================

interface TvShowsConnectionResponse {
  shows: {
    edges: Array<{ node: Show; cursor: string }>;
    pageInfo: {
      hasNextPage: boolean;
      hasPreviousPage: boolean;
      startCursor: string | null;
      endCursor: string | null;
      totalCount: number | null;
    };
  };
}

// ============================================================================
// Main Component
// ============================================================================

// Map column keys to GraphQL ShowOrderByInput field names
const SORT_FIELD_MAP: Record<string, keyof ShowOrderByInput> = {
  name: "sortName",
  year: "year",
  createdAt: "createdAt",
};

export function LibraryShowsTab({
  libraryId,
  loading: _parentLoading,
  onDeleteShow,
  onAddShow,
  onRefreshReady,
}: LibraryShowsTabProps) {
  // URL-persisted state via nuqs (clean URLs when using defaults)
  const [selectedLetter, setSelectedLetter] = useQueryState(
    "letter",
    parseAsString.withDefault(""),
  );
  const [searchTerm, setSearchTerm] = useQueryState(
    "q",
    parseAsString.withDefault(""),
  );
  const [sortColumn, setSortColumn] = useQueryState(
    "sort",
    parseAsString.withDefault("name"),
  );
  const [sortDirection, setSortDirection] = useQueryState(
    "order",
    parseAsStringLiteral(["asc", "desc"] as const).withDefault("asc"),
  );

  // Normalize selectedLetter: empty string becomes null for the filter logic
  const normalizedLetter = selectedLetter === "" ? null : selectedLetter;

  // Check if we should skip queries (loading or template ID)
  const shouldSkipQueries = !libraryId || libraryId.startsWith("template");

  // Handle sort change from DataTable
  const handleSortChange = useCallback(
    (column: string, direction: "asc" | "desc") => {
      setSortColumn(column);
      setSortDirection(direction);
    },
    [setSortColumn, setSortDirection],
  );

  // Build filter variables for GraphQL query.
  const queryVariables = useMemo(() => {
    const where: Record<string, unknown> = { libraryId: { eq: libraryId } };
    if (searchTerm) where.name = { contains: searchTerm };
    const graphqlField = SORT_FIELD_MAP[sortColumn || "name"] || "sortName";
    const direction: OrderDirection = sortDirection === "asc" ? "ASC" : "DESC";
    const orderBy: ShowOrderByInput[] = [{ [graphqlField]: direction }];
    return { where: where, page: { limit: 5000 }, orderBy: orderBy };
  }, [libraryId, searchTerm, sortColumn, sortDirection]);

  const {
    data,
    previousData,
    loading: queryLoading,
    refetch,
  } = useQuery<TvShowsConnectionResponse>(LibraryShowsTabDocument, {
    variables: queryVariables,
    skip: shouldSkipQueries,
    fetchPolicy: "cache-and-network",
    notifyOnNetworkStatusChange: false,
  });

  const shows = useMemo(
    () =>
      (data?.shows?.edges ?? previousData?.shows?.edges ?? []).map(
        (edge) => edge.node,
      ),
    [data?.shows?.edges, previousData?.shows?.edges],
  );

  const totalCount =
    data?.shows?.pageInfo?.totalCount ??
    previousData?.shows?.pageInfo?.totalCount ??
    null;

  // Provide refresh function to parent for subscription updates
  useEffect(() => {
    if (onRefreshReady) {
      onRefreshReady(() => {
        void refetch();
      });
    }
  }, [refetch, onRefreshReady]);

  // Get letters that have shows (from loaded data)
  const availableLetters = useMemo(() => {
    const letters = new Set<string>();
    shows.forEach((show) => {
      letters.add(getFirstLetter(show.name));
    });
    return letters;
  }, [shows]);

  // Filter shows by selected letter (client-side for alphabet filter)
  const filteredShows = useMemo(() => {
    if (!normalizedLetter) return shows;
    return shows.filter(
      (show) => getFirstLetter(show.name) === normalizedLetter,
    );
  }, [shows, normalizedLetter]);

  // Handle letter change - toggle filter
  const handleLetterChange = useCallback(
    (letter: string | null) => {
      setSelectedLetter(normalizedLetter === letter ? "" : (letter ?? ""));
    },
    [normalizedLetter, setSelectedLetter],
  );

  // Handle search change for server-side filtering
  const handleSearchChange = useCallback(
    (term: string) => {
      setSearchTerm(term || "");
      setSelectedLetter(""); // Reset letter filter when searching
    },
    [setSearchTerm, setSelectedLetter],
  );

  // Column definitions
  const columns: DataTableColumn<Show>[] = useMemo(
    () => [
      {
        key: "name",
        label: "SHOW",
        render: (show) => (
          <Link
            to="/shows/$showId"
            params={{ showId: show.id }}
            className="flex items-center gap-3 hover:opacity-80"
          >
            {show.posterUrl ? (
              <Image
                src={show.posterUrl}
                alt={show.name}
                className="w-10 h-14 object-cover rounded"
                loading="lazy"
              />
            ) : (
              <div className="w-10 h-14 bg-default-200 rounded flex items-center justify-center">
                <IconDeviceTv size={20} className="text-blue-400" />
              </div>
            )}
            <div>
              <p className="font-medium">{show.name}</p>
            </div>
          </Link>
        ),
      },
      {
        key: "year",
        label: "YEAR",
        width: 80,
        render: (show) => <span>{show.year ?? "—"}</span>,
      },
      {
        key: "network",
        label: "NETWORK",
        width: 150,
        sortable: false,
        render: (show) => <span>{show.network ?? "—"}</span>,
      },
    ],
    [],
  );

  // Row actions
  const rowActions: RowAction<Show>[] = useMemo(
    () => [
      {
        key: "view",
        label: "View",
        icon: <IconEye size={16} />,
        inDropdown: true,
        onAction: () => {},
      },
      {
        key: "delete",
        label: "Delete",
        icon: <IconTrash size={16} className="text-red-400" />,
        isDestructive: true,
        inDropdown: true,
        onAction: (show) => onDeleteShow(show.id, show.name),
      },
    ],
    [onDeleteShow],
  );

  // Card renderer
  const cardRenderer = useCallback(
    ({ item }: CardRendererProps<Show>) => (
      <TvShowCard
        show={item}
        onDelete={() => onDeleteShow(item.id, item.name)}
      />
    ),
    [onDeleteShow],
  );

  return (
    <div className="flex h-full min-h-0 flex-1 flex-col w-full">
      <div className="flex min-h-0 flex-1 flex-col">
        <DataTable
          stateKey="library-shows"
          skeletonDelay={500}
          data={filteredShows}
          columns={columns}
          getRowKey={(show) => show.id}
          toolbarQueryPlaceholder="Search shows..."
          sortColumn={sortColumn || "name"}
          sortDirection={sortDirection}
          onSortChange={handleSortChange}
          showViewModeToggle
          defaultViewMode="cards"
          cardRenderer={cardRenderer}
          cardSkeleton={() => <MediaCardSkeleton />}
          skeletonCardCount={12}
          cardGridClassName="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4"
          rowActions={rowActions}
          showItemCount
          ariaLabel="TV Shows table"
          fillHeight
          serverSide
          serverTotalCount={totalCount ?? undefined}
          onSearchChange={handleSearchChange}
          isLoading={queryLoading && shows.length === 0}
          headerContent={
            <AlphabetFilter
              selectedLetter={normalizedLetter}
              availableLetters={availableLetters}
              onLetterChange={handleLetterChange}
            />
          }
          emptyContent={
            <Card className="bg-content1/50 border-default-300 border-dashed border-2">
              <CardBody className="py-12 text-center">
                <IconDeviceTv
                  size={48}
                  className="mx-auto mb-4 text-blue-400"
                />
                <h3 className="text-lg font-semibold mb-2">No shows yet</h3>
                <p className="text-default-500 mb-4">
                  Add TV shows to start tracking episodes.
                </p>
                <Button color="primary" onPress={onAddShow}>
                  Add Show
                </Button>
              </CardBody>
            </Card>
          }
          toolbarContent={
            <Button color="primary" size="sm" onPress={onAddShow} isIconOnly>
              <IconPlus size={16} />
            </Button>
          }
          toolbarContentPosition="end"
        />
      </div>
    </div>
  );
}
