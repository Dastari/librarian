import { IconMicrophone2 } from "@tabler/icons-react";
import { useMemo } from "react";

import { EmptyState, ErrorState, PosterCard, PosterGallery } from "@/components/ui";
import { LibraryArtistsDocument, type ArtistOrderByInput, type ArtistWhereInput, type LibraryArtistsQuery } from "@/graphql/generated/graphql";
import { useServerTable } from "@/hooks/useServerTable";
import { artistImage } from "@/lib/artwork";
import { pluralize } from "@/lib/format";
import { LIBRARY_TYPES } from "@/lib/library-types";

import { BrowserLetterRail, BrowserToolbar, type BrowserSortOption } from "./BrowserToolbar";
import { useBrowserFilters, titleFilter } from "./useBrowserFilters";
import { useInfiniteConnection } from "./useInfiniteConnection";
import { useLetterJump } from "./useLetterJump";

type ArtistRow = LibraryArtistsQuery["artists"]["edges"][number]["node"];
const getRowId = (row: ArtistRow) => row.id;
const SORTS: readonly BrowserSortOption[] = [
  { id: "name", desc: false, label: "Name" },
  { id: "albumCount", desc: true, label: "Most albums" },
];

export function ArtistsBrowser({ libraryId }: { libraryId: string }) {
  const filters = useBrowserFilters();
  const table = useServerTable<ArtistOrderByInput>({ persistKey: `artists.${libraryId}`, defaultSorting: [{ id: "name", desc: false }], sortMap: { name: "sortName" } });

  const where = useMemo<ArtistWhereInput>(() => {
    const clause: ArtistWhereInput = { libraryId: { eq: libraryId } };
    const name = titleFilter(filters.debouncedQuery, null);
    if (name) clause.name = name;
    return clause;
  }, [libraryId, filters.debouncedQuery]);

  const list = useInfiniteConnection(LibraryArtistsDocument, { where, orderBy: table.orderBy }, (data) => data.artists);
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
      <BrowserToolbar query={filters.query} onQueryChange={filters.setQuery} letter={jump.active} onLetterChange={jump.jump} sort={{ options: SORTS, value: table.sorting, onChange: table.setSorting }} total={list.totalCount} placeholder="Filter artists" />
      <div className="flex min-h-0 min-w-0 flex-1 gap-3 sm:gap-4">
        <PosterGallery<ArtistRow>
          label="Artists"
          rows={list.rows}
          getRowId={getRowId}
          loading={list.loading}
          refetching={list.fetching}
          queryKey={list.queryKey}
          totalCount={list.totalCount}
          noun="artists"
          infinite={{ hasMore: list.hasMore, loadMore: list.loadMore, loadingMore: list.loadingMore }}
          error={list.error && list.rows.length === 0 ? <ErrorState error={list.error} onRetry={() => void list.refetch()} /> : undefined}
          emptyState={<EmptyState icon={IconMicrophone2} title="No artists yet" description="Artists are created when albums are added or scanned." />}
          renderCard={(row) => (
            <PosterCard aspect="square" title={row.name} meta={pluralize(row.albumCount ?? 0, "album")} image={artistImage(row)} tint={LIBRARY_TYPES.music.tintVar} to="/artists/$artistId" params={{ artistId: row.id }} className="[&_img]:rounded-full [&>div>div]:rounded-full" />
          )}
        />
        <BrowserLetterRail letter={jump.active} onLetterChange={jump.jump} />
      </div>
    </div>
  );
}
