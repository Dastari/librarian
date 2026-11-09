import { createDataTableColumns, RelativeTime, TextCell } from "glass-ux/data-table";

import { useNavigate } from "@tanstack/react-router";
import { IconPlus } from "glass-ux/icons";

import { useCallback, useMemo, useState } from "react";

import { Button, PosterCard } from "@/components/ui";
import { MovieIdentity } from "@/features/media/MovieIdentity";
import { useMovieEntity } from "@/features/media/useMovieEntity";
import { useGridReset } from "@/features/media/useProgressOwners";
import { ProviderSearchDialog } from "@/features/library-add/ProviderSearchDialog";
import { usePlayer } from "@/features/player/usePlayer";
import { LibraryMoviesDocument, type LibraryMoviesQuery, type MovieOrderByInput, type MovieWhereInput } from "@/graphql/generated/graphql";
import { useContentStatuses, contentStatusMeta } from "@/hooks/useContentStatuses";
import { useServerTable } from "@/hooks/useServerTable";
import { movieBackdrop, moviePoster } from "@/lib/artwork";
import { useIsAdmin } from "@/lib/auth/useSession";
import { formatRuntime, parseTimestamp } from "@/lib/format";
import { LIBRARY_TYPES } from "@/lib/library-types";

import { useBrowserFilters, titleFilter } from "./useBrowserFilters";
import { MediaBrowserTable } from "./MediaBrowserTable";
import { useInfiniteConnection } from "./useInfiniteConnection";
import { useLetterJump } from "./useLetterJump";

type MovieRow = LibraryMoviesQuery["movies"]["edges"][number]["node"];

const getRowId = (row: MovieRow) => row.id;

const column = createDataTableColumns<MovieRow>();

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
    getTitle: (row) => row.sortTitle || row.title,
    hasMore: list.hasMore,
    loading: list.fetching || list.loadingMore,
    queryKey: JSON.stringify(where),
    isTitleSorted: table.sorting.length === 1 && table.sorting[0]?.id === "title" && !table.sorting[0].desc,
    loadMore: list.loadMore,
    ensureTitleSort: () => table.setSorting([{ id: "title", desc: false }]),
  });
  const ids = useMemo(() => list.rows.map((row) => row.id), [list.rows]);
  const statuses = useContentStatuses("MOVIE", ids);
  const reset = useGridReset<MovieRow>("movie");
  const { actions, dialogs } = useMovieEntity<MovieRow>({ reset });
  const columns = useMemo(() => column.columns([
    column.accessor("title", { header: "Movie", width: 30, priority: 1, cardRole: "title", sortable: true, cell: ({ row }) => <MovieIdentity movie={row.original} status={contentStatusMeta(statuses, row.original.id)} /> }),
    column.accessor("year", { header: "Year", width: 6, priority: 2, grow: false, sortable: true, sortDescFirst: true, cell: ({ row }) => <TextCell muted>{row.original.year ?? ""}</TextCell> }),
    column.accessor("runtime", { header: "Length", width: 7, priority: 2, align: "end", grow: false, sortable: true, cell: ({ row }) => <TextCell muted>{row.original.runtime ? formatRuntime(row.original.runtime, "minutes") : ""}</TextCell> }),
    column.display({ id: "certification", header: "Rating", width: 6, priority: 3, grow: false, cell: ({ row }) => <TextCell muted>{row.original.certification ?? ""}</TextCell> }),
    column.display({ id: "quality", header: "Quality", width: 9, priority: 3, cell: ({ row }) => <TextCell muted>{row.original.mediaFile?.resolution ?? ""}</TextCell> }),
    column.accessor("createdAt", { header: "Added", width: 8, priority: 3, sortable: true, sortDescFirst: true, cell: ({ row }) => <RelativeTime value={parseTimestamp(row.original.createdAt)} /> }),
  ]), [statuses]);

  const play = (row: MovieRow) => {
    if (!row.mediaFileId) return;
    player.playVideo({ mediaFileId: row.mediaFileId, title: row.title, subtitle: row.year ? String(row.year) : undefined, artwork: moviePoster(row.id), backdrop: movieBackdrop(row.id), entity: { kind: "movie", id: row.id }, href: `/movies/${row.id}` });
    void navigate({ to: "/watch/$mediaFileId", params: { mediaFileId: row.mediaFileId } });
  };


  const renderPoster = useCallback((row: MovieRow) => (
      <PosterCard
      title={row.title}
      meta={[row.year, row.runtime ? formatRuntime(row.runtime, "minutes") : null].filter(Boolean).join(" · ")}
      image={moviePoster(row.id)}
      tint={LIBRARY_TYPES.movies.tintVar}
      to="/movies/$movieId"
      params={{ movieId: row.id }}
      status={contentStatusMeta(statuses, row.id)}
      onPlay={row.mediaFileId ? () => play(row) : undefined}
      />
  ), [statuses, play]);

  return (
    <>
    <MediaBrowserTable<MovieRow>
      persistKey="movies"
      label="Movies"
      noun={{ one: "movie", other: "movies" }}
      placeholder="Filter movies"
      columns={columns}
      list={list}
      sorting={table.sorting}
      onSortingChange={table.setSorting}
      query={filters.query}
      onQueryChange={filters.setQuery}
      availability={filters.availability}
      onAvailabilityChange={filters.setAvailability}
      getRowId={getRowId}
      getRowLabel={(row) => row.title}
      renderPoster={renderPoster}
      actions={actions}
      onRowOpen={(row) => void navigate({ to: "/movies/$movieId", params: { movieId: row.id } })}
      emptyMessage="No movies yet. Scan the library folder or add a movie from TMDB."
      emptyAction={isAdmin ? <Button variant="primary" onPress={() => setAdding(true)}><IconPlus size="md" /> Add movie</Button> : undefined}
      trailing={isAdmin ? <Button variant="primary" size="sm" onPress={() => setAdding(true)}><IconPlus size="md" /> Add</Button> : null}
      letter={{ active: jump.active, jump: jump.jump, containerRef: jump.containerRef }}
    />
      {dialogs}
      <ProviderSearchDialog kind="movie" libraryId={libraryId} isOpen={adding} onOpenChange={setAdding} onAdded={() => void list.refetch()} />
    </>
  );
}
