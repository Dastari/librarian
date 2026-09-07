
import { useNavigate } from "@tanstack/react-router";
import { IconDeviceTv, IconPlus } from "@tabler/icons-react";
import { useMemo, useState } from "react";

import { Button, DataTable, type DataTableColumn, EmptyState, ErrorState, PosterCard } from "@/components/ui";
import { ProviderSearchDialog } from "@/features/library-add/ProviderSearchDialog";
import { LibraryShowsDocument, type LibraryShowsQuery, type ShowOrderByInput, type ShowWhereInput } from "@/graphql/generated/graphql";
import { useServerTable } from "@/hooks/useServerTable";
import { showPoster } from "@/lib/artwork";
import { useIsAdmin } from "@/lib/auth/useSession";
import { formatDate } from "@/lib/format";
import { LIBRARY_TYPES } from "@/lib/library-types";
import { cn } from "@/lib/utils";

import { BrowserLetterRail, BrowserToolbar } from "./BrowserToolbar";
import { useBrowserFilters, titleFilter } from "./useBrowserFilters";
import { useInfiniteConnection } from "./useInfiniteConnection";
import { useLetterJump } from "./useLetterJump";

type ShowRow = LibraryShowsQuery["shows"]["edges"][number]["node"];
const getRowId = (row: ShowRow) => row.id;

/** "12/24" style progress used for shows, albums and audiobooks. */
export function ProgressBadge({ have, total, className }: { have: number; total: number; className?: string }) {
  if (total === 0) return <span className={cn("text-numeric text-muted", className)}>—</span>;
  const complete = have >= total;
  return (
    <span className={cn("text-numeric", complete ? "text-success" : "text-warning", className)}>
      {have}/{total}
    </span>
  );
}

export function ShowsBrowser({ libraryId }: { libraryId: string }) {
  const navigate = useNavigate();
  const isAdmin = useIsAdmin();
  const filters = useBrowserFilters(`shows.${libraryId}`);
  const table = useServerTable<ShowOrderByInput>({ persistKey: `shows.${libraryId}`, defaultSorting: [{ id: "name", desc: false }], sortMap: { name: "sortName" } });
  const [adding, setAdding] = useState(false);

  const where = useMemo<ShowWhereInput>(() => {
    const clause: ShowWhereInput = { libraryId: { eq: libraryId } };
    const name = titleFilter(filters.debouncedQuery, null);
    if (name) clause.name = name;
    return clause;
  }, [libraryId, filters.debouncedQuery]);

  const list = useInfiniteConnection(LibraryShowsDocument, { where, orderBy: table.orderBy }, (data) => data.shows);
  const jump = useLetterJump({
    rows: list.rows,
    getRowId,
    getTitle: (row) => row.name,
    hasMore: list.hasMore,
    loadMore: list.loadMore,
    ensureTitleSort: () => table.setSorting([{ id: "name", desc: false }]),
  });
  const counts = (row: ShowRow) => ({ have: row.downloaded.pageInfo.totalCount ?? 0, total: row.episodes.pageInfo.totalCount ?? 0 });

  const columns = useMemo<Array<DataTableColumn<ShowRow>>>(
    () => [
      {
        id: "name",
        header: "Show",
        sortable: true,
        cell: (row) => (
          <span className="flex items-center gap-3">
            <img src={showPoster(row)} alt="" className="h-12 w-8 shrink-0 rounded-sm object-cover" loading="lazy" />
            <span className="min-w-0">
              <span className="block truncate text-body-sm text-foreground">{row.name}</span>
              <span className="block truncate text-label-sm text-muted">{[row.year, row.network].filter(Boolean).join(" · ")}</span>
            </span>
          </span>
        ),
      },
      { id: "episodes", header: "Episodes", size: 110, align: "end", cell: (row) => <ProgressBadge {...counts(row)} /> },
      { id: "autoDownload", header: "Auto-download", size: 140, hideBelow: "md", cell: (row) => <span className="text-muted">{row.autoDownloadMode === "NONE" ? "Off" : row.autoDownloadMode === "ALL" ? "All episodes" : "Wanted"}</span> },
      { id: "contentRating", header: "Rating", size: 90, hideBelow: "lg", cell: (row) => <span className="text-muted">{row.contentRating ?? "—"}</span> },
      { id: "genres", header: "Genres", size: 220, hideBelow: "xl", cell: (row) => <span className="text-muted">{row.genres.join(", ")}</span> },
      { id: "createdAt", header: "Added", size: 130, sortable: true, hideBelow: "md", cell: (row) => <span className="text-muted">{formatDate(row.createdAt)}</span> },
    ],
    [],
  );

  const filtered = Boolean(filters.debouncedQuery);
  return (
    <div className="flex min-h-0 flex-1 flex-col gap-4">
      <BrowserToolbar
        query={filters.query}
        onQueryChange={filters.setQuery}
        letter={jump.active}
        onLetterChange={jump.jump}
        view={filters.view}
        onViewChange={filters.setView}
        total={list.totalCount}
        placeholder="Filter shows"
        trailing={isAdmin ? <Button variant="primary" size="sm" onPress={() => setAdding(true)}><IconPlus size={16} /> Add</Button> : null}
      />
      <div className="flex min-h-0 flex-1 gap-4">
        <DataTable<ShowRow>
          className="min-w-0 flex-1"
          view={filters.view}
          columns={columns}
          rows={list.rows}
          getRowId={getRowId}
          isLoading={list.loading}
          totalCount={list.totalCount}
          noun="shows"
          sorting={table.sorting}
          onSortingChange={table.setSorting}
          infinite={{ hasMore: list.hasMore, loadMore: list.loadMore, loadingMore: list.loadingMore }}
          error={list.error && list.rows.length === 0 ? <ErrorState error={list.error} onRetry={() => void list.refetch()} /> : undefined}
          emptyState={
            <EmptyState
              icon={IconDeviceTv}
              title={filtered ? "No shows match" : "No shows yet"}
              description={filtered ? "Try clearing the filters." : "Scan the library folder or add a show from TVmaze."}
              action={isAdmin && !filtered ? <Button variant="primary" onPress={() => setAdding(true)}><IconPlus size={16} /> Add show</Button> : undefined}
            />
          }
          onRowClick={(row) => void navigate({ to: "/shows/$showId", params: { showId: row.id } })}
          renderCard={(row) => {
            const { have, total } = counts(row);
            return (
              <PosterCard
                title={row.name}
                meta={[row.year, row.network].filter(Boolean).join(" · ")}
                image={showPoster(row)}
                tint={LIBRARY_TYPES.tv.tintVar}
                to="/shows/$showId"
                params={{ showId: row.id }}
                badge={total > 0 ? <ProgressBadge have={have} total={total} className="text-foreground" /> : undefined}
              />
            );
          }}
        />
        <BrowserLetterRail letter={jump.active} onLetterChange={jump.jump} />
      </div>
      <ProviderSearchDialog kind="show" libraryId={libraryId} isOpen={adding} onOpenChange={setAdding} onAdded={() => void list.refetch()} />
    </div>
  );
}
