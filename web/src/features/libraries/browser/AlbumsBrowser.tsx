
import { IconMusic, IconPlus } from "@tabler/icons-react";
import { useMemo, useState } from "react";

import { Button, EmptyState, ErrorState, PosterCard, PosterGallery } from "@/components/ui";
import { ProviderSearchDialog } from "@/features/library-add/ProviderSearchDialog";
import { LibraryAlbumsDocument, type AlbumOrderByInput, type AlbumWhereInput, type LibraryAlbumsQuery } from "@/graphql/generated/graphql";
import { useServerTable } from "@/hooks/useServerTable";
import { albumCover } from "@/lib/artwork";
import { useIsAdmin } from "@/lib/auth/useSession";
import { formatYear } from "@/lib/format";
import { LIBRARY_TYPES } from "@/lib/library-types";

import { BrowserLetterRail, BrowserToolbar, type BrowserSortOption } from "./BrowserToolbar";
import { ProgressBadge } from "./ShowsBrowser";
import { useBrowserFilters, titleFilter } from "./useBrowserFilters";
import { useInfiniteConnection } from "./useInfiniteConnection";
import { useLetterJump } from "./useLetterJump";

type AlbumRow = LibraryAlbumsQuery["albums"]["edges"][number]["node"];
const getRowId = (row: AlbumRow) => row.id;
const SORTS: readonly BrowserSortOption[] = [
  { id: "name", desc: false, label: "Name" },
  { id: "year", desc: true, label: "Newest releases" },
  { id: "year", desc: false, label: "Oldest releases" },
  { id: "sizeBytes", desc: true, label: "Largest" },
];

export function AlbumsBrowser({ libraryId }: { libraryId: string }) {
  const isAdmin = useIsAdmin();
  const filters = useBrowserFilters();
  const table = useServerTable<AlbumOrderByInput>({ persistKey: `albums.${libraryId}`, defaultSorting: [{ id: "name", desc: false }], sortMap: { name: "sortName" } });
  const [adding, setAdding] = useState(false);

  const where = useMemo<AlbumWhereInput>(() => {
    const clause: AlbumWhereInput = { libraryId: { eq: libraryId } };
    const name = titleFilter(filters.debouncedQuery, null);
    if (name) clause.name = name;
    if (filters.availability === "available") clause.hasFiles = { eq: true };
    if (filters.availability === "wanted" || filters.availability === "missing") clause.hasFiles = { eq: false };
    return clause;
  }, [libraryId, filters.debouncedQuery, filters.availability]);

  const list = useInfiniteConnection(LibraryAlbumsDocument, { where, orderBy: table.orderBy }, (data) => data.albums);
  const jump = useLetterJump({
    rows: list.rows,
    getRowId,
    getTitle: (row) => row.name,
    hasMore: list.hasMore,
    loading: list.fetching || list.loadingMore,
    queryKey: JSON.stringify(where),
    isTitleSorted: table.sorting.length === 1 && table.sorting[0]?.id === "name" && !table.sorting[0].desc,
    loadMore: list.loadMore,
    ensureTitleSort: () => table.setSorting([{ id: "name", desc: false }]),
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
        placeholder="Filter albums"
        trailing={isAdmin ? <Button variant="primary" size="sm" onPress={() => setAdding(true)}><IconPlus size={16} /> Add</Button> : null}
      />
      <div className="flex min-h-0 min-w-0 flex-1 gap-3 sm:gap-4">
        <PosterGallery<AlbumRow>
          label="Albums"
          rows={list.rows}
          getRowId={getRowId}
          loading={list.loading}
          refetching={list.fetching}
          queryKey={list.queryKey}
          totalCount={list.totalCount}
          noun="albums"
          infinite={{ hasMore: list.hasMore, loadMore: list.loadMore, loadingMore: list.loadingMore }}
          error={list.error && list.rows.length === 0 ? <ErrorState error={list.error} onRetry={() => void list.refetch()} /> : undefined}
          emptyState={<EmptyState icon={IconMusic} title="No albums yet" description="Scan the library folder or add an album from MusicBrainz." action={isAdmin ? <Button variant="primary" onPress={() => setAdding(true)}><IconPlus size={16} /> Add album</Button> : undefined} />}
          renderCard={(row) => (
            <PosterCard
              aspect="square"
              title={row.name}
              meta={[formatYear(row.releaseDate) || row.year, row.albumType].filter(Boolean).join(" · ")}
              image={albumCover(row)}
              tint={LIBRARY_TYPES.music.tintVar}
              to="/albums/$albumId"
              params={{ albumId: row.id }}
              badge={row.trackCount ? <ProgressBadge have={row.downloaded.pageInfo.totalCount ?? 0} total={row.trackCount} className="text-foreground" /> : undefined}
            />
          )}
        />
        <BrowserLetterRail letter={jump.active} onLetterChange={jump.jump} />
      </div>
      <ProviderSearchDialog kind="album" libraryId={libraryId} isOpen={adding} onOpenChange={setAdding} onAdded={() => void list.refetch()} />
    </div>
  );
}
