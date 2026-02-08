import { useMemo, useState, useCallback, useEffect } from "react";
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
import { type Audiobook, type AudiobookAuthor } from "../../lib/graphql";
import { apolloClient } from "../../lib/graphql/client";
import { LibraryAudiobooksTabDocument } from "../../lib/graphql/generated/graphql";
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
}

// ============================================================================
// Main Component
// ============================================================================

export function LibraryAudiobooksTab({
  libraryId,
  loading: parentLoading,
  onDeleteAudiobook,
  onAddAudiobook,
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

  const [audiobooks, setAudiobooks] = useState<Audiobook[]>([]);
  const [audiobooksLoading, setAudiobooksLoading] = useState(true);
  const [authors, setAuthors] = useState<AudiobookAuthor[]>([]);
  const [authorsLoading, setAuthorsLoading] = useState(true);

  // Check if we should skip queries (loading or template ID)
  const shouldSkipQueries = parentLoading || libraryId.startsWith("template");

  // Fetch authors separately (still needed for name lookup)
  useEffect(() => {
    if (shouldSkipQueries) {
      return;
    }
    const fetchAudiobooks = async () => {
      try {
        const result = await apolloClient.query({
          query: LibraryAudiobooksTabDocument,
          variables: { LibraryId: libraryId },
          fetchPolicy: "network-only",
        });

        const edges = result.data?.Audiobooks?.Edges ?? [];
        const mappedAudiobooks = edges.map((e) => ({
          id: e.Node.Id,
          authorId: e.Node.AuthorName ?? null,
          libraryId: e.Node.LibraryId,
          title: e.Node.Title,
          sortTitle: e.Node.SortTitle ?? null,
          subtitle: null,
          openlibraryId: null,
          isbn: e.Node.Isbn ?? null,
          description: e.Node.Description ?? null,
          publisher: e.Node.Publisher ?? null,
          language: e.Node.Language ?? null,
          narrators: e.Node.Narrators,
          seriesName: null,
          durationSecs: e.Node.TotalDurationSecs ?? null,
          coverUrl: e.Node.CoverUrl ?? null,
          hasFiles: e.Node.HasFiles,
          sizeBytes: e.Node.SizeBytes ?? null,
          path: e.Node.Path ?? null,
          chapterCount: e.Node.ChapterCount ?? null,
          downloadedChapterCount: null,
        }));
        setAudiobooks(mappedAudiobooks);
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
          id: authorName,
          libraryId,
          name: authorName,
          sortName: authorName,
          openlibraryId: null,
        }));
        setAuthors(derivedAuthors);
      } catch (err) {
        console.error("Failed to fetch audiobooks:", err);
      } finally {
        setAudiobooksLoading(false);
        setAuthorsLoading(false);
      }
    };
    fetchAudiobooks();
  }, [libraryId, shouldSkipQueries]);

  const isLoading = audiobooksLoading || authorsLoading;

  // Create author lookup map
  const authorMap = useMemo(() => {
    const map = new Map<string, string>();
    authors.forEach((author) => {
      map.set(author.id, author.name);
    });
    return map;
  }, [authors]);

  // Get letters that have audiobooks
  const availableLetters = useMemo(() => {
    const letters = new Set<string>();
    audiobooks.forEach((audiobook) => {
      letters.add(getFirstLetter(audiobook.title));
    });
    return letters;
  }, [audiobooks]);

  const visibleAudiobooks = useMemo(() => {
    let list = audiobooks;

    if (searchTerm) {
      const q = searchTerm.toLowerCase();
      list = list.filter((item) => item.title.toLowerCase().includes(q));
    }

    if (normalizedLetter) {
      list = list.filter(
        (item) => getFirstLetter(item.title) === normalizedLetter,
      );
    }

    const sorted = [...list];
    sorted.sort((a, b) => {
      let av: string | number = a.title;
      let bv: string | number = b.title;
      if (sortColumn === "author") {
        av = (a.authorId && authorMap.get(a.authorId)) ?? "";
        bv = (b.authorId && authorMap.get(b.authorId)) ?? "";
      } else if (sortColumn === "duration") {
        av = a.durationSecs ?? 0;
        bv = b.durationSecs ?? 0;
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
  const columns: DataTableColumn<Audiobook>[] = useMemo(
    () => [
      {
        key: "title",
        label: "AUDIOBOOK",
        // sortable: true (default) - server handles actual sorting
        render: (audiobook) => (
          <Link
            to="/audiobooks/$audiobookId"
            params={{ audiobookId: audiobook.id }}
            className="flex items-center gap-3 hover:opacity-80"
          >
            {audiobook.coverUrl ? (
              <Image
                src={audiobook.coverUrl}
                alt={audiobook.title}
                className="w-10 h-14 object-cover rounded"
              />
            ) : (
              <div className="w-10 h-14 bg-default-200 rounded flex items-center justify-center">
                <IconHeadphones size={20} className="text-orange-400" />
              </div>
            )}
            <div>
              <p className="font-medium">{audiobook.title}</p>
              {audiobook.authorId && authorMap.get(audiobook.authorId) && (
                <p className="text-xs text-default-400">
                  {authorMap.get(audiobook.authorId)}
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
            {(audiobook.authorId && authorMap.get(audiobook.authorId)) || "—"}
          </span>
        ),
      },
      {
        key: "series",
        label: "SERIES",
        width: 150,
        sortable: false,
        render: (audiobook) => <span>{audiobook.seriesName || "—"}</span>,
      },
      {
        key: "progress",
        label: "PROGRESS",
        width: 80,
        sortable: false,
        render: (audiobook) => {
          const downloaded = audiobook.downloadedChapterCount ?? 0;
          const total = audiobook.chapterCount ?? 0;
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
  const rowActions: RowAction<Audiobook>[] = useMemo(
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
              onAction: (audiobook: Audiobook) =>
                onDeleteAudiobook(audiobook.id, audiobook.title),
            },
          ]
        : []),
    ],
    [onDeleteAudiobook],
  );

  // Card renderer
  const cardRenderer = useCallback(
    ({ item }: CardRendererProps<Audiobook>) => (
      <AudiobookCard
        audiobook={item}
        authorName={item.authorId ? authorMap.get(item.authorId) : undefined}
        onDelete={
          onDeleteAudiobook
            ? () => onDeleteAudiobook(item.id, item.title)
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
          getRowKey={(audiobook) => audiobook.id}
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
          isLoading={parentLoading || isLoading}
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
