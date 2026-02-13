import { useMemo, useState, useCallback, useEffect, useRef } from "react";
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
import { apolloClient } from "../../lib/graphql/client";
import {
  LibraryAudiobooksTabDocument,
  type LibraryAudiobooksTabQuery,
} from "../../lib/graphql/generated/graphql";
import {
  IconPlus,
  IconTrash,
  IconEye,
  IconHeadphones,
  IconUser,
} from "@tabler/icons-react";
import { AudiobookCard } from "./AudiobookCard";
import { MediaCardSkeleton } from "./MediaCardSkeleton";

// ============================================================================
// Component Props
// ============================================================================

interface LibraryAudiobooksTabProps {
  libraryId: string;
  /** Parent loading state (e.g., library context still loading) */
  loading?: boolean;
  onDeleteAudiobook?: (audiobookId: string, audiobookTitle: string) => void;
  onAddAudiobook?: () => void;
  /** Callback to provide refresh function to parent */
  onRefreshReady?: (refreshFn: () => void) => void;
}

type AudiobookNode =
  LibraryAudiobooksTabQuery["Audiobooks"]["Edges"][number]["Node"];
type AuthorRow = {
  Id: string;
  Name: string;
};

// ============================================================================
// Main Component
// ============================================================================

export function LibraryAudiobooksTab({
  libraryId,
  loading: _parentLoading,
  onDeleteAudiobook,
  onAddAudiobook,
  onRefreshReady,
}: LibraryAudiobooksTabProps) {
  // URL-persisted state via nuqs
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
    parseAsString.withDefault("title"),
  );
  const [sortDirection, setSortDirection] = useQueryState(
    "order",
    parseAsStringLiteral(["asc", "desc"] as const).withDefault("asc"),
  );
  const normalizedLetter = selectedLetter === "" ? null : selectedLetter;

  // Handle sort change from DataTable
  const handleSortChange = useCallback(
    (column: string, direction: "asc" | "desc") => {
      setSortColumn(column);
      setSortDirection(direction);
    },
    [setSortColumn, setSortDirection],
  );

  const [audiobooks, setAudiobooks] = useState<AudiobookNode[]>([]);
  const [authors, setAuthors] = useState<AuthorRow[]>([]);
  const [queryLoading, setQueryLoading] = useState(true);
  const hasLoadedRef = useRef(false);
  const inFlightRef = useRef<Promise<void> | null>(null);

  // Keep behavior consistent with movies/shows/albums tabs.
  const shouldSkipQueries = !libraryId || libraryId.startsWith("template");

  const fetchAudiobooks = useCallback(
    async (showLoading: boolean) => {
      if (shouldSkipQueries) {
        setQueryLoading(false);
        return;
      }

      if (inFlightRef.current) {
        return inFlightRef.current;
      }

      if (showLoading || !hasLoadedRef.current) {
        setQueryLoading(true);
      }

      const request = (async () => {
        try {
          const result = await apolloClient.query({
            query: LibraryAudiobooksTabDocument,
            variables: { LibraryId: libraryId },
            fetchPolicy: "network-only",
          });

          const edges = result.data?.Audiobooks?.Edges ?? [];
          setAudiobooks(edges.map((e) => e.Node));
          const derivedAuthors = Array.from(
            new Map(
              edges
                .filter((e) => Boolean(e.Node.AuthorName))
                .map((e) => [
                  e.Node.AuthorName as string,
                  e.Node.AuthorName as string,
                ]),
            ).values(),
          ).map((authorName) => ({
            Id: authorName,
            Name: authorName,
          }));
          setAuthors(derivedAuthors);
        } catch (err) {
          console.error("Failed to fetch audiobooks:", err);
        } finally {
          hasLoadedRef.current = true;
          setQueryLoading(false);
        }
      })();

      inFlightRef.current = request;
      request.finally(() => {
        inFlightRef.current = null;
      });
      return request;
    },
    [libraryId, shouldSkipQueries],
  );

  useEffect(() => {
    void fetchAudiobooks(true);
  }, [fetchAudiobooks]);

  useEffect(() => {
    if (onRefreshReady) {
      onRefreshReady(() => {
        void fetchAudiobooks(false);
      });
    }
  }, [fetchAudiobooks, onRefreshReady]);

  // Create author lookup map
  const authorMap = useMemo(() => {
    const map = new Map<string, string>();
    authors.forEach((author) => {
      map.set(author.Id, author.Name);
    });
    return map;
  }, [authors]);

  // Get letters that have audiobooks
  const availableLetters = useMemo(() => {
    const letters = new Set<string>();
    audiobooks.forEach((audiobook) => {
      letters.add(getFirstLetter(audiobook.Title));
    });
    return letters;
  }, [audiobooks]);

  const visibleAudiobooks = useMemo(() => {
    let list = audiobooks;

    if (searchTerm) {
      const q = searchTerm.toLowerCase();
      list = list.filter((item) => item.Title.toLowerCase().includes(q));
    }

    if (normalizedLetter) {
      list = list.filter(
        (item) => getFirstLetter(item.Title) === normalizedLetter,
      );
    }

    const sorted = [...list];
    sorted.sort((a, b) => {
      let av: string | number = a.Title;
      let bv: string | number = b.Title;
      if (sortColumn === "author") {
        av = (a.AuthorName && authorMap.get(a.AuthorName)) ?? "";
        bv = (b.AuthorName && authorMap.get(b.AuthorName)) ?? "";
      } else if (sortColumn === "duration") {
        av = a.TotalDurationSecs ?? 0;
        bv = b.TotalDurationSecs ?? 0;
      }

      if (typeof av === "number" && typeof bv === "number") {
        return sortDirection === "asc" ? av - bv : bv - av;
      }
      const cmp = String(av).localeCompare(String(bv));
      return sortDirection === "asc" ? cmp : -cmp;
    });

    return sorted;
  }, [
    audiobooks,
    searchTerm,
    normalizedLetter,
    sortColumn,
    sortDirection,
    authorMap,
  ]);

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
  const columns: DataTableColumn<AudiobookNode>[] = useMemo(
    () => [
      {
        key: "title",
        label: "AUDIOBOOK",
        // sortable: true (default) - server handles actual sorting
        render: (audiobook) => (
            <Link
              to="/audiobooks/$audiobookId"
              params={{ audiobookId: audiobook.Id }}
              className="flex items-center gap-3 hover:opacity-80"
            >
            {audiobook.CoverUrl ? (
              <Image
                src={audiobook.CoverUrl}
                alt={audiobook.Title}
                className="w-10 h-14 object-cover rounded"
                loading="lazy"
              />
            ) : (
              <div className="w-10 h-14 bg-default-200 rounded flex items-center justify-center">
                <IconHeadphones size={20} className="text-orange-400" />
              </div>
            )}
            <div>
              <p className="font-medium">{audiobook.Title}</p>
              {audiobook.AuthorName && authorMap.get(audiobook.AuthorName) && (
                <p className="text-xs text-default-400">
                  {authorMap.get(audiobook.AuthorName)}
                </p>
              )}
            </div>
          </Link>
        ),
      },
      {
        key: "author",
        label: "AUTHOR",
        width: 150,
        sortable: false,
        render: (audiobook) => (
          <span className="flex items-center gap-1">
            <IconUser size={14} className="text-default-400" />
            {(audiobook.AuthorName && authorMap.get(audiobook.AuthorName)) ||
              "—"}
          </span>
        ),
      },
      {
        key: "series",
        label: "SERIES",
        width: 150,
        sortable: false,
        render: () => <span>—</span>,
      },
      {
        key: "progress",
        label: "PROGRESS",
        width: 80,
        sortable: false,
        render: (audiobook) => {
          const downloaded = 0;
          const total = audiobook.ChapterCount ?? 0;
          const isComplete = total > 0 && downloaded >= total;
          return (
            <span
              className={
                isComplete
                  ? "text-success font-medium"
                  : "text-warning font-medium"
              }
            >
              {downloaded}/{total}
            </span>
          );
        },
      },
    ],
    [authorMap],
  );

  // Row actions
  const rowActions: RowAction<AudiobookNode>[] = useMemo(
    () => [
      {
        key: "view",
        label: "View",
        icon: <IconEye size={16} />,
        inDropdown: true,
        onAction: () => {
          // View details when route is available
        },
      },
      ...(onDeleteAudiobook
        ? [
            {
              key: "delete",
              label: "Delete",
              icon: <IconTrash size={16} className="text-red-400" />,
              isDestructive: true,
              inDropdown: true,
              onAction: (audiobook: AudiobookNode) =>
                onDeleteAudiobook(audiobook.Id, audiobook.Title),
            },
          ]
        : []),
    ],
    [onDeleteAudiobook],
  );

  // Card renderer
  const cardRenderer = useCallback(
    ({ item }: CardRendererProps<AudiobookNode>) => (
      <AudiobookCard
        audiobook={item}
        authorName={item.AuthorName ? authorMap.get(item.AuthorName) : undefined}
        onDelete={
          onDeleteAudiobook
            ? () => onDeleteAudiobook(item.Id, item.Title)
            : undefined
        }
      />
    ),
    [authorMap, onDeleteAudiobook],
  );

  return (
    <div className="flex flex-col grow w-full">
      <div className="flex-1 min-h-0">
        <DataTable
          stateKey="library-audiobooks"
          skeletonDelay={500}
          data={visibleAudiobooks}
          columns={columns}
          getRowKey={(audiobook) => audiobook.Id}
          searchPlaceholder="Search audiobooks..."
          sortColumn={sortColumn || "title"}
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
          ariaLabel="Audiobooks table"
          fillHeight
          serverTotalCount={visibleAudiobooks.length}
          onSearchChange={handleSearchChange}
          isLoading={queryLoading && visibleAudiobooks.length === 0}
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
                <IconHeadphones
                  size={48}
                  className="mx-auto mb-4 text-orange-400"
                />
                <h3 className="text-lg font-semibold mb-2">
                  No audiobooks yet
                </h3>
                <p className="text-default-500 mb-4">
                  Add audiobooks to this library to start listening.
                </p>
                {onAddAudiobook && (
                  <Button color="primary" onPress={onAddAudiobook}>
                    Add Audiobook
                  </Button>
                )}
              </CardBody>
            </Card>
          }
          toolbarContent={
            onAddAudiobook ? (
              <Button
                color="primary"
                size="sm"
                onPress={onAddAudiobook}
                isIconOnly
              >
                <IconPlus size={16} />
              </Button>
            ) : undefined
          }
          toolbarContentPosition="end"
        />
      </div>
    </div>
  );
}
