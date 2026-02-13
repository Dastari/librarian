import { useMemo, useState, useCallback, useEffect } from "react";
import { useQueryState, parseAsString, parseAsStringLiteral } from "nuqs";
import { Card, CardBody } from "@heroui/card";
import {
  DataTable,
  AlphabetFilter,
  getFirstLetter,
  type DataTableColumn,
  type CardRendererProps,
} from "../data-table";
import { apolloClient } from "../../lib/graphql/client";
import {
  LibraryAudiobooksTabDocument,
  type LibraryAudiobooksTabQuery,
} from "../../lib/graphql/generated/graphql";
import { IconUser, IconBook, IconHeadphones } from "@tabler/icons-react";
import { SquareCardSkeleton } from "./MediaCardSkeleton";

// ============================================================================
// Component Props
// ============================================================================

interface LibraryAuthorsTabProps {
  libraryId: string;
  /** Parent loading state (e.g., library context still loading) */
  loading?: boolean;
  onSelectAuthor?: (authorId: string) => void;
}

// ============================================================================
// Author Card Component
// ============================================================================

interface AuthorCardProps {
  author: { Id: string; Name: string };
  bookCount: number;
  onSelect?: () => void;
}

function AuthorCard({ author, bookCount, onSelect }: AuthorCardProps) {
  return (
    <div className="aspect-square">
      <Card
        isPressable={!!onSelect}
        onPress={onSelect}
        className="relative overflow-hidden h-full w-full group border-none bg-content2"
      >
        {/* Background with gradient */}
        <div className="absolute inset-0 bg-gradient-to-br from-orange-900 via-amber-800 to-yellow-900">
          <div className="absolute inset-0 flex items-center justify-center opacity-30">
            <IconUser size={64} className="text-orange-400" />
          </div>
        </div>

        {/* Book count badge - top right */}
        {bookCount > 0 && (
          <div className="absolute top-2 right-2 z-10 pointer-events-none">
            <div className="px-2 py-1 rounded-md bg-black/50 backdrop-blur-sm text-xs font-medium text-white/90">
              <IconBook size={12} className="inline mr-1" />
              {bookCount}
            </div>
          </div>
        )}

        {/* Bottom content */}
        <div className="absolute bottom-0 left-0 right-0 z-10 p-3 pointer-events-none bg-black/50 backdrop-blur-sm h-16 flex flex-col justify-center">
          <h3 className="text-sm font-bold text-white mb-0.5 line-clamp-2 drop-shadow-lg">
            {author.Name}
          </h3>
          <div className="flex items-center gap-1.5 text-xs text-white/70">
            <span>
              {bookCount} {bookCount === 1 ? "audiobook" : "audiobooks"}
            </span>
          </div>
        </div>
      </Card>
    </div>
  );
}

// ============================================================================
// Main Component
// ============================================================================

export function LibraryAuthorsTab({
  libraryId,
  loading: parentLoading,
  onSelectAuthor,
}: LibraryAuthorsTabProps) {
  type AudiobookNode =
    LibraryAudiobooksTabQuery["Audiobooks"]["Edges"][number]["Node"];
  type AuthorRow = { Id: string; Name: string };
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
    parseAsString.withDefault("name"),
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
  const [audiobooksLoading, setAudiobooksLoading] = useState(true);
  const [authors, setAuthors] = useState<AuthorRow[]>([]);
  const [authorsLoading, setAuthorsLoading] = useState(true);

  // Check if we should skip queries (loading or template ID)
  const shouldSkipQueries =
    parentLoading || !libraryId || libraryId.startsWith("template");

  useEffect(() => {
    if (shouldSkipQueries) return;

    const fetchAudiobooks = async () => {
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
        ).map((authorName) => ({ Id: authorName, Name: authorName }));
        setAuthors(derivedAuthors);
      } catch (err) {
        console.error("Failed to fetch audiobooks:", err);
      } finally {
        setAudiobooksLoading(false);
        setAuthorsLoading(false);
      }
    };
    void fetchAudiobooks();
  }, [libraryId, shouldSkipQueries]);

  const isLoading = authorsLoading || audiobooksLoading;

  // Count audiobooks per author
  const bookCountByAuthor = useMemo(() => {
    const counts = new Map<string, number>();
    audiobooks.forEach((audiobook) => {
      if (audiobook.AuthorName) {
        const current = counts.get(audiobook.AuthorName) || 0;
        counts.set(audiobook.AuthorName, current + 1);
      }
    });
    return counts;
  }, [audiobooks]);

  // Get letters that have authors
  const availableLetters = useMemo(() => {
    const letters = new Set<string>();
    authors.forEach((author) => {
      letters.add(getFirstLetter(author.Name));
    });
    return letters;
  }, [authors]);

  const sortedAuthors = useMemo(() => {
    const sorted = [...authors];
    sorted.sort((a, b) => {
      if (sortColumn === "audiobooks") {
        const av = bookCountByAuthor.get(a.Id) || 0;
        const bv = bookCountByAuthor.get(b.Id) || 0;
        return sortDirection === "asc" ? av - bv : bv - av;
      }
      const cmp = a.Name.localeCompare(b.Name);
      return sortDirection === "asc" ? cmp : -cmp;
    });
    return sorted;
  }, [authors, sortColumn, sortDirection, bookCountByAuthor]);

  const filteredAuthors = useMemo(() => {
    let list = sortedAuthors;
    if (searchTerm) {
      const q = searchTerm.toLowerCase();
      list = list.filter((author) => author.Name.toLowerCase().includes(q));
    }
    if (normalizedLetter) {
      list = list.filter(
        (author) => getFirstLetter(author.Name) === normalizedLetter,
      );
    }
    return list;
  }, [sortedAuthors, searchTerm, normalizedLetter]);

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
  const columns: DataTableColumn<AuthorRow>[] = useMemo(
    () => [
      {
        key: "name",
        label: "AUTHOR",
        // sortable: true (default) - server handles actual sorting
        render: (author) => (
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-full bg-default-200 flex items-center justify-center">
              <IconUser size={20} className="text-orange-400" />
            </div>
            <div>
              <p className="font-medium">{author.Name}</p>
            </div>
          </div>
        ),
      },
      {
        key: "audiobooks",
        label: "AUDIOBOOKS",
        width: 120,
        sortable: false,
        render: (author) => (
          <span className="flex items-center gap-1">
            <IconHeadphones size={14} className="text-default-400" />
            {bookCountByAuthor.get(author.Id) || 0}
          </span>
        ),
      },
    ],
    [bookCountByAuthor],
  );

  // Card renderer
  const cardRenderer = useCallback(
    ({ item }: CardRendererProps<AuthorRow>) => (
      <AuthorCard
        author={item}
        bookCount={bookCountByAuthor.get(item.Id) || 0}
        onSelect={onSelectAuthor ? () => onSelectAuthor(item.Id) : undefined}
      />
    ),
    [bookCountByAuthor, onSelectAuthor],
  );

  return (
    <div className="flex flex-col grow w-full">
      <div className="flex-1 min-h-0">
        <DataTable
          stateKey="library-authors"
          skeletonDelay={500}
          data={filteredAuthors}
          columns={columns}
          getRowKey={(author) => author.Id}
          searchPlaceholder="Search authors..."
          sortColumn={sortColumn || "name"}
          sortDirection={sortDirection}
          onSortChange={handleSortChange}
          showViewModeToggle
          defaultViewMode="cards"
          cardRenderer={cardRenderer}
          cardSkeleton={() => <SquareCardSkeleton />}
          skeletonCardCount={12}
          cardGridClassName="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4"
          showItemCount
          ariaLabel="Authors table"
          fillHeight
          serverTotalCount={filteredAuthors.length}
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
                <IconUser size={48} className="mx-auto mb-4 text-orange-400" />
                <h3 className="text-lg font-semibold mb-2">No authors yet</h3>
                <p className="text-default-500 mb-4">
                  Authors will appear here as you add audiobooks to your
                  library.
                </p>
              </CardBody>
            </Card>
          }
        />
      </div>
    </div>
  );
}
