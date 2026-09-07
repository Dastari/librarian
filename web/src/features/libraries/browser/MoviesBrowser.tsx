
import { useNavigate } from "@tanstack/react-router";
import { IconMovie, IconPlus } from "@tabler/icons-react";
import { useMemo, useState } from "react";

import { Button, DataTable, type DataTableColumn, EmptyState, ErrorState, PosterCard, StatusChip } from "@/components/ui";
import { ProviderSearchDialog } from "@/features/library-add/ProviderSearchDialog";
import { usePlayer } from "@/features/player/usePlayer";
import { LibraryMoviesDocument, type LibraryMoviesQuery, type MovieOrderByInput, type MovieWhereInput } from "@/graphql/generated/graphql";
import { useContentStatuses } from "@/hooks/useContentStatuses";
import { useServerTable } from "@/hooks/useServerTable";
import { moviePoster } from "@/lib/artwork";
import { useIsAdmin } from "@/lib/auth/useSession";
import { formatBytes, formatDate, formatRuntime } from "@/lib/format";
import { LIBRARY_TYPES } from "@/lib/library-types";
import { qualityStatus, statusMeta } from "@/lib/status";

import { BrowserLetterRail, BrowserToolbar } from "./BrowserToolbar";
import { useBrowserFilters, titleFilter } from "./useBrowserFilters";
import { useInfiniteConnection } from "./useInfiniteConnection";
import { useLetterJump } from "./useLetterJump";

type MovieRow = LibraryMoviesQuery["movies"]["edges"][number]["node"];

const getRowId = (row: MovieRow) => row.id;

export function MoviesBrowser({ libraryId }: { libraryId: string }) {
  const navigate = useNavigate();
  const player = usePlayer();
  const isAdmin = useIsAdmin();
  const filters = useBrowserFilters(`movies.${libraryId}`);
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

  const columns = useMemo<Array<DataTableColumn<MovieRow>>>(
    () => [
      {
        id: "title",
        header: "Title",
        sortable: true,
        cell: (row) => (
          <span className="flex items-center gap-3">
            <img src={moviePoster(row.id)} alt="" className="h-12 w-8 shrink-0 rounded-sm object-cover" loading="lazy" />
            <span className="min-w-0">
              <span className="block truncate text-body-sm text-foreground">{row.title}</span>
              <span className="block truncate text-label-sm text-muted">{[row.year, row.director].filter(Boolean).join(" · ")}</span>
            </span>
          </span>
        ),
      },
      { id: "status", header: "Status", size: 150, cell: (row) => <StatusChip status={statusMeta(statuses.get(row.id))} /> },
      { id: "quality", header: "Quality", size: 190, hideBelow: "md", cell: (row) => <QualityCell file={row.mediaFile} /> },
      { id: "runtime", header: "Runtime", size: 100, sortable: true, align: "end", numeric: true, hideBelow: "lg", cell: (row) => formatRuntime(row.runtime, "minutes") },
      { id: "year", header: "Year", size: 80, sortable: true, align: "end", numeric: true, hideBelow: "sm", cell: (row) => row.year ?? "—" },
      { id: "genres", header: "Genres", size: 220, hideBelow: "xl", cell: (row) => <span className="text-muted">{row.genres.join(", ")}</span> },
      { id: "createdAt", header: "Added", size: 130, sortable: true, hideBelow: "md", cell: (row) => <span className="text-muted">{formatDate(row.createdAt)}</span> },
    ],
    [statuses],
  );

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
    <div className="flex min-h-0 flex-1 flex-col gap-4">
      <BrowserToolbar
        query={filters.query}
        onQueryChange={filters.setQuery}
        letter={jump.active}
        onLetterChange={jump.jump}
        availability={filters.availability}
        onAvailabilityChange={filters.setAvailability}
        view={filters.view}
        onViewChange={filters.setView}
        total={list.totalCount}
        placeholder="Filter movies"
        trailing={isAdmin ? <Button variant="primary" size="sm" onPress={() => setAdding(true)}><IconPlus size={16} /> Add</Button> : null}
      />
      <div className="flex min-h-0 flex-1 gap-4">
        <DataTable<MovieRow>
          className="min-w-0 flex-1"
          view={filters.view}
          columns={columns}
          rows={list.rows}
          getRowId={getRowId}
          isLoading={list.loading}
          totalCount={list.totalCount}
          noun="movies"
          sorting={table.sorting}
          onSortingChange={table.setSorting}
          infinite={{ hasMore: list.hasMore, loadMore: list.loadMore, loadingMore: list.loadingMore }}
          error={list.error && list.rows.length === 0 ? <ErrorState error={list.error} onRetry={() => void list.refetch()} /> : undefined}
          emptyState={emptyState}
          onRowClick={(row) => void navigate({ to: "/movies/$movieId", params: { movieId: row.id } })}
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

export function QualityCell({ file }: { file: { resolution?: string | null; videoCodec?: string | null; audioCodec?: string | null; isHdr?: boolean; hdrType?: string | null; size?: number | null; qualityStatus?: string | null } | null | undefined }) {
  if (!file) return <span className="text-muted">—</span>;
  const quality = qualityStatus(file.qualityStatus);
  return (
    <span className="flex flex-col leading-tight">
      <span className="text-body-sm text-foreground">{[file.resolution, file.videoCodec?.toUpperCase(), file.isHdr ? (file.hdrType ?? "HDR") : null].filter(Boolean).join(" · ")}</span>
      <span className="text-label-sm text-muted">
        {[file.audioCodec?.toUpperCase(), formatBytes(file.size)].filter(Boolean).join(" · ")}
        {file.qualityStatus === "suboptimal" ? <span className="ml-1.5 text-warning">{quality.label}</span> : null}
      </span>
    </span>
  );
}
