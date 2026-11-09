import { createDataTableColumns, RelativeTime, TextCell } from "glass-ux/data-table";
import { useNavigate } from "@tanstack/react-router";

import { IconPlus } from "glass-ux/icons";

import { useCallback, useMemo, useState } from "react";

import { Button, PosterCard } from "@/components/ui";
import { useShowEntity } from "@/features/media/useShowEntity";
import { useGridReset } from "@/features/media/useProgressOwners";
import { ShowIdentity } from "@/features/media/ShowIdentity";
import { ProviderSearchDialog } from "@/features/library-add/ProviderSearchDialog";
import { LibraryShowsDocument, type LibraryShowsQuery, type ShowOrderByInput, type ShowWhereInput } from "@/graphql/generated/graphql";
import { useServerTable } from "@/hooks/useServerTable";
import { showPoster } from "@/lib/artwork";
import { useIsAdmin } from "@/lib/auth/useSession";
import { LIBRARY_TYPES } from "@/lib/library-types";
import { cn } from "@/lib/utils";

import { useBrowserFilters, titleFilter } from "./useBrowserFilters";
import { parseTimestamp } from "@/lib/format";
import { MediaBrowserTable } from "./MediaBrowserTable";
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

const column = createDataTableColumns<ShowRow>();
const columns = column.columns([
  column.accessor("name", { header: "Show", width: 30, priority: 1, cardRole: "title", sortable: true, cell: ({ row }) => { const have = row.original.downloaded.pageInfo.totalCount ?? 0; const total = row.original.episodes.pageInfo.totalCount ?? 0; return <ShowIdentity show={row.original} have={have} total={total} />; } }),
  column.display({ id: "network", header: "Network", width: 12, priority: 2, cell: ({ row }) => <TextCell muted>{row.original.network ?? ""}</TextCell> }),
  column.display({ id: "year", header: "Year", width: 6, priority: 3, grow: false, cell: ({ row }) => <TextCell muted>{row.original.year ?? ""}</TextCell> }),
  column.display({ id: "episodes", header: "Episodes", width: 8, priority: 2, align: "end", grow: false, cell: ({ row }) => <TextCell muted>{`${row.original.downloaded.pageInfo.totalCount ?? 0} of ${row.original.episodes.pageInfo.totalCount ?? 0}`}</TextCell> }),
  column.accessor("createdAt", { header: "Added", width: 8, priority: 3, sortable: true, sortDescFirst: true, cell: ({ row }) => <RelativeTime value={parseTimestamp(row.original.createdAt)} /> }),
]);

export function ShowsBrowser({ libraryId }: { libraryId: string }) {
  const isAdmin = useIsAdmin();
  const navigate = useNavigate();
  const filters = useBrowserFilters();
  const reset = useGridReset<ShowRow>("show");
  const { actions, dialogs } = useShowEntity<ShowRow>({ reset });
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
    getTitle: (row) => row.sortName || row.name,
    hasMore: list.hasMore,
    loading: list.fetching || list.loadingMore,
    queryKey: JSON.stringify(where),
    isTitleSorted: table.sorting.length === 1 && table.sorting[0]?.id === "name" && !table.sorting[0].desc,
    loadMore: list.loadMore,
    ensureTitleSort: () => table.setSorting([{ id: "name", desc: false }]),
  });

  const renderPoster = useCallback((row: ShowRow) => {
    const have = row.downloaded.pageInfo.totalCount ?? 0;
    const total = row.episodes.pageInfo.totalCount ?? 0;
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
  }, []);

  return (
    <>
    <MediaBrowserTable<ShowRow>
      persistKey="shows"
      label="Shows"
      noun={{ one: "show", other: "shows" }}
      placeholder="Filter shows"
      columns={columns}
      list={list}
      sorting={table.sorting}
      onSortingChange={table.setSorting}
      query={filters.query}
      onQueryChange={filters.setQuery}
      getRowId={getRowId}
      getRowLabel={(row) => row.name}
      renderPoster={renderPoster}
      actions={actions}
      onRowOpen={(row) => void navigate({ to: "/shows/$showId", params: { showId: row.id } })}
      emptyMessage="No shows yet. Scan the library folder or add a show from TVmaze."
      emptyAction={isAdmin ? <Button variant="primary" onPress={() => setAdding(true)}><IconPlus size="md" /> Add show</Button> : undefined}
      trailing={isAdmin ? <Button variant="primary" size="sm" onPress={() => setAdding(true)}><IconPlus size="md" /> Add</Button> : null}
      letter={{ active: jump.active, jump: jump.jump, containerRef: jump.containerRef }}
    />
      {dialogs}
      <ProviderSearchDialog kind="show" libraryId={libraryId} isOpen={adding} onOpenChange={setAdding} onAdded={() => void list.refetch()} />
    </>
  );
}
