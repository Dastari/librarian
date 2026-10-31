
import { useNavigate } from "@tanstack/react-router";
import { IconMovie, IconPlus } from "@tabler/icons-react";
import { useMemo, useState } from "react";

import { Button, EmptyState, ErrorState, PosterCard, PosterGallery } from "@/components/ui";
import { ProviderSearchDialog } from "@/features/library-add/ProviderSearchDialog";
import { usePlayer } from "@/features/player/usePlayer";
import { LibraryMoviesDocument, type LibraryMoviesQuery, type MovieOrderByInput, type MovieWhereInput } from "@/graphql/generated/graphql";
import { useContentStatuses } from "@/hooks/useContentStatuses";
import { useServerTable } from "@/hooks/useServerTable";
import { moviePoster } from "@/lib/artwork";
import { useIsAdmin } from "@/lib/auth/useSession";
import { formatRuntime } from "@/lib/format";
import { LIBRARY_TYPES } from "@/lib/library-types";
import { statusMeta } from "@/lib/status";

import { BrowserLetterRail, BrowserToolbar, type BrowserSortOption } from "./BrowserToolbar";
import { useBrowserFilters, titleFilter } from "./useBrowserFilters";
import { useInfiniteConnection } from "./useInfiniteConnection";
import { useLetterJump } from "./useLetterJump";

type MovieRow = LibraryMoviesQuery["movies"]["edges"][number]["node"];

const getRowId = (row: MovieRow) => row.id;
const SORTS: readonly BrowserSortOption[] = [
  { id: "title", desc: false, label: "Title" },
  { id: "createdAt", desc: true, label: "Recently added" },
  { id: "year", desc: true, label: "Newest releases" },
  { id: "year", desc: false, label: "Oldest releases" },
  { id: "runtime", desc: false, label: "Shortest" },
  { id: "runtime", desc: true, label: "Longest" },
];

export function MoviesBrowser({ libraryId }: { libraryId: string }) {
  const navigate = useNavigate();
  const player = usePlayer();
  const isAdmin = useIsAdmin();
  const filters = useBrowserFilters();
  const table = useServerTable<MovieOrderByInput>({ persistKey: `movies.${libraryId}`, defaultSorting: [{ id: "title", desc: false }], sortMap: { title: "sortTitle" } });
  const [adding, setAdding] = useState(false);

  const where = useMemo<MovieWhereInput>(() => {
    const clause: MovieWhereInput = { libraryId: { eq: libraryId } };
    const title = titleFilter(filters.debouncedQuery, null);
    if (title) clause.title = title;
    if (filters.availability === "available") clause.hasFile = { eq: true };
    if (filters.availability === "wanted") {
      clause.hasFile = { eq: false };
      clause.wanted = { eq: true };
    }
    if (filters.availability === "missing") {
      clause.hasFile = { eq: false };
      clause.wanted = { eq: false };
    }
    return clause;
  }, [libraryId, filters.debouncedQuery, filters.availability]);

  const list = useInfiniteConnection(LibraryMoviesDocument, { where, orderBy: table.orderBy }, (data) => data.movies);
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
  const ids = useMemo(() => list.rows.map((row) => row.id), [list.rows]);
  const statuses = useContentStatuses("MOVIE", ids);

  const play = (row: MovieRow) => {
    if (!row.mediaFileId) return;
    player.playVideo({ mediaFileId: row.mediaFileId, title: row.title, subtitle: row.year ? String(row.year) : undefined, artwork: moviePoster(row.id), entity: { kind: "movie", id: row.id }, href: `/movies/${row.id}` });
    void navigate({ to: "/watch/$mediaFileId", params: { mediaFileId: row.mediaFileId } });
  };

  const filtered = Boolean(filters.debouncedQuery) || filters.availability !== "all";
  const emptyState = (
    <EmptyState
      icon={IconMovie}
      title={filtered ? "No movies match" : "No movies yet"}
      description={filtered ? "Try clearing the filters." : "Scan the library folder or add a movie from TMDB."}
      action={isAdmin && !filtered ? <Button variant="primary" onPress={() => setAdding(true)}><IconPlus size={16} /> Add movie</Button> : undefined}
    />
  );

  return (
    <div ref={jump.containerRef} className="flex min-h-0 min-w-0 flex-1 flex-col gap-3 sm:gap-4">
      <BrowserToolbar
        query={filters.query}
        onQueryChange={filters.setQuery}
        letter={jump.active}
        onLetterChange={jump.jump}
        availability={filters.availability}
        onAvailabilityChange={filters.setAvailability}
        sort={{ options: SORTS, value: table.sorting, onChange: table.setSorting }}
        total={list.totalCount}
        placeholder="Filter movies"
        trailing={isAdmin ? <Button variant="primary" size="sm" onPress={() => setAdding(true)}><IconPlus size={16} /> Add</Button> : null}
      />
      <div className="flex min-h-0 min-w-0 flex-1 gap-3 sm:gap-4">
        <PosterGallery<MovieRow>
          label="Movies"
          noun="movies"
          rows={list.rows}
          getRowId={getRowId}
          loading={list.loading}
          refetching={list.fetching}
          queryKey={list.queryKey}
          totalCount={list.totalCount}
          infinite={{ hasMore: list.hasMore, loadMore: list.loadMore, loadingMore: list.loadingMore }}
          error={list.error && list.rows.length === 0 ? <ErrorState error={list.error} onRetry={() => void list.refetch()} /> : undefined}
          emptyState={emptyState}
          renderCard={(row) => (
            <PosterCard
              title={row.title}
              meta={[row.year, row.runtime ? formatRuntime(row.runtime, "minutes") : null].filter(Boolean).join(" · ")}
              image={moviePoster(row.id)}
              tint={LIBRARY_TYPES.movies.tintVar}
              to="/movies/$movieId"
              params={{ movieId: row.id }}
              status={statusMeta(statuses.get(row.id))}
              onPlay={row.mediaFileId ? () => play(row) : undefined}
            />
          )}
        />
        <BrowserLetterRail letter={jump.active} onLetterChange={jump.jump} />
      </div>
      <ProviderSearchDialog kind="movie" libraryId={libraryId} isOpen={adding} onOpenChange={setAdding} onAdded={() => void list.refetch()} />
    </div>
  );
}
