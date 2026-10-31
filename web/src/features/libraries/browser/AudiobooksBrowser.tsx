
import { IconBooks, IconPlus } from "@tabler/icons-react";
import { useMemo, useState } from "react";

import { Button, EmptyState, ErrorState, PosterCard, PosterGallery } from "@/components/ui";
import { ProviderSearchDialog } from "@/features/library-add/ProviderSearchDialog";
import { LibraryAudiobooksDocument, type AudiobookOrderByInput, type AudiobookWhereInput, type LibraryAudiobooksQuery } from "@/graphql/generated/graphql";
import { useServerTable } from "@/hooks/useServerTable";
import { audiobookCover } from "@/lib/artwork";
import { useIsAdmin } from "@/lib/auth/useSession";
import { LIBRARY_TYPES } from "@/lib/library-types";

import { BrowserLetterRail, BrowserToolbar, type BrowserSortOption } from "./BrowserToolbar";
import { ProgressBadge } from "./ShowsBrowser";
import { useBrowserFilters, titleFilter } from "./useBrowserFilters";
import { useInfiniteConnection } from "./useInfiniteConnection";
import { useLetterJump } from "./useLetterJump";

type BookRow = LibraryAudiobooksQuery["audiobooks"]["edges"][number]["node"];
const getRowId = (row: BookRow) => row.id;
const SORTS: readonly BrowserSortOption[] = [
  { id: "title", desc: false, label: "Title" },
  { id: "totalDurationSecs", desc: false, label: "Shortest" },
  { id: "totalDurationSecs", desc: true, label: "Longest" },
  { id: "sizeBytes", desc: true, label: "Largest" },
];

export function AudiobooksBrowser({ libraryId }: { libraryId: string }) {
  const isAdmin = useIsAdmin();
  const filters = useBrowserFilters();
  const table = useServerTable<AudiobookOrderByInput>({ persistKey: `audiobooks.${libraryId}`, defaultSorting: [{ id: "title", desc: false }], sortMap: { title: "sortTitle" } });
  const [adding, setAdding] = useState(false);

  const where = useMemo<AudiobookWhereInput>(() => {
    const clause: AudiobookWhereInput = { libraryId: { eq: libraryId } };
    const title = titleFilter(filters.debouncedQuery, null);
    if (title) clause.title = title;
    if (filters.availability === "available") clause.hasFiles = { eq: true };
    if (filters.availability === "wanted" || filters.availability === "missing") clause.hasFiles = { eq: false };
    return clause;
  }, [libraryId, filters.debouncedQuery, filters.availability]);

  const list = useInfiniteConnection(LibraryAudiobooksDocument, { where, orderBy: table.orderBy }, (data) => data.audiobooks);
  const jump = useLetterJump({
    rows: list.rows,
    getRowId,
    getTitle: (row) => row.title,
    hasMore: list.hasMore,
    loading: list.fetching || list.loadingMore,
    queryKey: JSON.stringify(where),
    isTitleSorted: table.sorting.length === 1 && table.sorting[0]?.id === "title" && !table.sorting[0].desc,
    loadMore: list.loadMore,
    ensureTitleSort: () => table.setSorting([{ id: "title", desc: false }]),
  });

  return (
    <div ref={jump.containerRef} className="flex min-h-0 min-w-0 flex-1 flex-col gap-3 sm:gap-4">
      <BrowserToolbar
        query={filters.query}
        onQueryChange={filters.setQuery}
        letter={jump.active}
        onLetterChange={jump.jump}
        availability={filters.availability}
        onAvailabilityChange={filters.setAvailability} sort={{ options: SORTS, value: table.sorting, onChange: table.setSorting }}
        total={list.totalCount}
        placeholder="Filter audiobooks"
        trailing={isAdmin ? <Button variant="primary" size="sm" onPress={() => setAdding(true)}><IconPlus size={16} /> Add</Button> : null}
      />
      <div className="flex min-h-0 min-w-0 flex-1 gap-3 sm:gap-4">
        <PosterGallery<BookRow>
          label="Audiobooks"
          rows={list.rows}
          getRowId={getRowId}
          loading={list.loading}
          refetching={list.fetching}
          queryKey={list.queryKey}
          totalCount={list.totalCount}
          noun="audiobooks"
          infinite={{ hasMore: list.hasMore, loadMore: list.loadMore, loadingMore: list.loadingMore }}
          error={list.error && list.rows.length === 0 ? <ErrorState error={list.error} onRetry={() => void list.refetch()} /> : undefined}
          emptyState={<EmptyState icon={IconBooks} title="No audiobooks yet" description="Scan the library folder or add a book from Open Library." action={isAdmin ? <Button variant="primary" onPress={() => setAdding(true)}><IconPlus size={16} /> Add audiobook</Button> : undefined} />}
          renderCard={(row) => (
            <PosterCard
              title={row.title}
              meta={row.authorName ?? undefined}
              image={audiobookCover(row)}
              tint={LIBRARY_TYPES.audiobooks.tintVar}
              to="/audiobooks/$audiobookId"
              params={{ audiobookId: row.id }}
              badge={row.chapterCount ? <ProgressBadge have={row.downloaded.pageInfo.totalCount ?? 0} total={row.chapterCount} className="text-foreground" /> : undefined}
            />
          )}
        />
        <BrowserLetterRail letter={jump.active} onLetterChange={jump.jump} />
      </div>
      <ProviderSearchDialog kind="audiobook" libraryId={libraryId} isOpen={adding} onOpenChange={setAdding} onAdded={() => void list.refetch()} />
    </div>
  );
}
