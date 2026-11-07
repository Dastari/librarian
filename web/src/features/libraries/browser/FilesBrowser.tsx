import { Badge } from "glass-ux/badge";
import { createDataTableColumns, DataTable, RelativeTime, TextCell, type DataTableSearch, type DataTableState, type DataTableStateChange } from "glass-ux/data-table";
import { useMemo } from "react";

import { EntityChip } from "@/components/entities/EntityChip";
import { ErrorState, GlassSegmented } from "@/components/ui";
import { TableBulkActions } from "@/components/ui/TableBulkActions";
import { openFileDialog } from "@/features/files/fileDialog";
import { FileIdentity } from "@/features/files/FileIdentity";
import { fileFormat, isFileLinked, useFileActions, type FileEntity } from "@/features/files/useFileActions";
import { LibraryMediaFilesDocument, type MediaFileOrderByInput, type MediaFileWhereInput } from "@/graphql/generated/graphql";
import { useServerTable } from "@/hooks/useServerTable";
import { useTableSelection } from "@/hooks/useTableSelection";
import { useSession } from "@/lib/auth/useSession";
import { badgeTone, fileName, linkedItem } from "@/lib/entities/linked";
import { formatBytes, formatRuntime, parseTimestamp } from "@/lib/format";
import { qualityStatus } from "@/lib/status";

import type { AvailabilityFilter } from "./BrowserToolbar";
import { useBrowserFilters } from "./useBrowserFilters";
import { useInfiniteConnection } from "./useInfiniteConnection";

const getRowId = (row: FileEntity) => row.id;
const noun = { one: "file", other: "files" };
const search: DataTableSearch<FileEntity> = { fields: [], placeholder: "Filter by path", toCondition: (query) => ({ path: { contains: query } }) };

const column = createDataTableColumns<FileEntity>();
const columns = column.columns([
  column.accessor("path", { header: "File", width: 28, priority: 1, cardRole: "title", sortable: true, cell: ({ row }) => <FileIdentity file={row.original} /> }),
  column.display({
    id: "linked", header: "Linked to", width: 16, priority: 2, cardRole: "subtitle",
    cell: ({ row }) => { const item = linkedItem(row.original); return item ? <EntityChip entity={item} /> : isFileLinked(row.original) ? <Badge size="sm" tone="success">Linked</Badge> : <Badge size="sm" marker="dot" tone="warning">Unmatched</Badge>; },
  }),
  column.display({ id: "quality", header: "Quality", width: 13, priority: 3, cell: ({ row }) => <TextCell muted>{fileFormat(row.original) || "—"}</TextCell> }),
  column.display({ id: "qualityStatus", header: "Profile", width: 9, priority: 3, cardRole: "status", cell: ({ row }) => { const meta = qualityStatus(row.original.qualityStatus); return <Badge size="sm" marker="dot" tone={badgeTone(meta.tone)}>{meta.label}</Badge>; } }),
  column.accessor("duration", { header: "Length", width: 6, priority: 4, align: "end", grow: false, sortable: true, cell: ({ row }) => <TextCell muted>{formatRuntime(row.original.duration)}</TextCell> }),
  column.accessor("size", { header: "Size", width: 7, priority: 2, align: "end", grow: false, sortable: true, sortDescFirst: true, cell: ({ row }) => <TextCell muted>{formatBytes(row.original.size)}</TextCell> }),
  column.accessor("addedAt", { header: "Added", width: 8, priority: 3, sortable: true, sortDescFirst: true, cell: ({ row }) => <RelativeTime value={parseTimestamp(row.original.addedAt)} /> }),
]);

/** Every media file the scanner knows about, with the unmatched ones one filter away. */
export function FilesBrowser({ libraryId, filterPrefix }: {
  libraryId: string;
  /** Keeps this list's filters in the URL apart from those of a page behind it. */
  filterPrefix?: string;
}) {
  const { user } = useSession();
  const filters = useBrowserFilters(filterPrefix);
  const table = useServerTable<MediaFileOrderByInput>({ persistKey: `files.${libraryId}`, defaultSorting: [{ id: "addedAt", desc: true }] });

  const where = useMemo<MediaFileWhereInput>(() => {
    const clause: MediaFileWhereInput = { libraryId: { eq: libraryId } };
    if (filters.debouncedQuery) clause.path = { contains: filters.debouncedQuery };
    if (filters.availability === "missing" || filters.availability === "wanted") {
      clause.movieId = { isNull: true };
      clause.episodeId = { isNull: true };
      clause.trackId = { isNull: true };
      clause.chapterId = { isNull: true };
    }
    if (filters.availability === "available") clause.qualityStatus = { eq: "suboptimal" };
    return clause;
  }, [libraryId, filters.debouncedQuery, filters.availability]);

  const list = useInfiniteConnection(LibraryMediaFilesDocument, { where, orderBy: table.orderBy }, (data) => data.mediaFiles);

  const { actions, dialogs } = useFileActions(() => void list.refetch());
  const selection = useTableSelection({ scope: [user?.id, libraryId, where, table.sorting], rows: list.rows, ready: !list.fetching });
  const state = useMemo<DataTableState>(() => ({ pagination: { pageIndex: 0, pageSize: 50 }, selection: selection.selection, filters: [], sorting: table.sorting, search: filters.query }), [table.sorting, selection.selection, filters.query]);
  const onStateChange = (next: DataTableState, change: DataTableStateChange) => { if (change === "search") filters.setQuery(next.search ?? ""); else if (change === "sorting") table.setSorting(next.sorting); else if (change === "selection") selection.onSelectionChange(next.selection); };

  const availabilityLabels: Record<AvailabilityFilter, string> = { all: "All files", available: "Below quality target", wanted: "Unmatched", missing: "Unmatched" };

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-3 sm:gap-4">
      <div className="flex min-h-0 min-w-0 flex-1 flex-col">
        <DataTable<FileEntity>
          id={`librarian.${user?.id ?? "session"}.files`}
          label="Files"
          queryKey={list.queryKey}
          height="fill"
          columns={columns}
          rows={list.rows}
          getRowId={getRowId}
          getRowLabel={(row) => fileName(row.path)}
          loading={list.loading}
          refetching={list.fetching && !list.loading}
          rowCount={list.totalCount}
          state={state}
          onStateChange={onStateChange}
          infinite={{ revision: list.loadRevision, hasMore: list.hasMore, onLoadMore: list.loadMore, loadingMore: list.loadingMore }}
          actions={actions}
          selectable
          renderBulkActions={(rows) => <TableBulkActions set={actions} rows={rows} noun={noun} />}
          noun={noun}
          search={search}
          toolbar={
            <GlassSegmented<AvailabilityFilter>
              ariaLabel="File filter"
              size="sm"
              value={filters.availability === "wanted" ? "missing" : filters.availability}
              onChange={filters.setAvailability}
              items={(["all", "missing", "available"] as AvailabilityFilter[]).map((option) => ({ key: option, label: availabilityLabels[option] }))}
            />
          }
          onRowOpen={(row) => openFileDialog(row.id)}
          rowOpen="click"
          error={list.error && list.rows.length === 0 ? <ErrorState error={list.error} onRetry={() => void list.refetch()} /> : undefined}
          emptyMessage={filters.availability === "all" ? "No files yet. Scan the library to discover files." : "No files match this filter."}
        />
      </div>
      {dialogs}
    </div>
  );
}
