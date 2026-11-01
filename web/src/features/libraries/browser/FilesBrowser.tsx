import { copyText } from "glass-ux/copy-button";
import { defineActions } from "glass-ux/actions";
import { Badge } from "glass-ux/badge";
import { createDataTableColumns, DataTable, IdentityCell, RelativeTime, TextCell, type DataTableState, type DataTableStateChange } from "glass-ux/data-table";
import { IconCopy, IconExternalLink, IconRefresh } from "glass-ux/icons";
import { IconFile, IconFileSearch, IconLinkOff, IconWand } from "@tabler/icons-react";
import { useMemo, useRef, useState } from "react";

import { TableBulkActions } from "@/components/ui/TableBulkActions";
import { useTableSelection } from "@/hooks/useTableSelection";
import { runSelectionAction } from "@/lib/actions/runSelectionAction";
import { EntityChip } from "@/components/entities/EntityChip";
import { ErrorState, GlassSegmented } from "@/components/ui";
import { ManualMatchDialog } from "@/features/libraries/ManualMatchDialog";
import { useMutation } from "@apollo/client/react";
import {
  AnalyzeMediaFileDocument,
  LibraryMediaFilesDocument,
  MatchMediaFileDocument,
  OrganizeMediaFileDocument,
  UnmatchMediaFileDocument,
  type LibraryMediaFilesQuery,
  type MediaFileOrderByInput,
  type MediaFileWhereInput,
} from "@/graphql/generated/graphql";
import { useServerTable } from "@/hooks/useServerTable";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { ENTITY_KINDS, glyph } from "@/lib/entities/kinds";
import { badgeTone, linkedItem } from "@/lib/entities/linked";
import { formatBytes, formatRuntime, parseTimestamp } from "@/lib/format";

import { qualityStatus } from "@/lib/status";

import { BrowserToolbar, type AvailabilityFilter } from "./BrowserToolbar";
import { useBrowserFilters } from "./useBrowserFilters";
import { useInfiniteConnection } from "./useInfiniteConnection";

type FileRow = LibraryMediaFilesQuery["mediaFiles"]["edges"][number]["node"];
const getRowId = (row: FileRow) => row.id;

function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

const FileGlyph = ENTITY_KINDS.mediaFile.icon;
const MatchIcon = glyph(IconWand, "wand");
const ManualIcon = glyph(IconFileSearch, "file-search");
const OrganizeIcon = glyph(IconFile, "file");
const UnlinkIcon = glyph(IconLinkOff, "link-off");

const column = createDataTableColumns<FileRow>();
const columns = column.columns([
  column.accessor("path", {
    header: "File", width: 28, priority: 1, cardRole: "title", sortable: true,
    cell: ({ row }) => <IdentityCell icon={<FileGlyph />} title={fileName(row.original.path)} subtitle={row.original.relativePath ?? row.original.path} status={linkedTo(row.original) ? "online" : "warning"} statusLabel={linkedTo(row.original) ? "Linked" : "Unmatched"} />,
  }),
  column.display({
    id: "linked", header: "Linked to", width: 16, priority: 2, cardRole: "subtitle",
    cell: ({ row }) => { const item = linkedItem(row.original); return item ? <EntityChip entity={item} /> : linkedTo(row.original) ? <Badge size="sm" tone="success">{linkedTo(row.original)}</Badge> : <Badge size="sm" marker="dot" tone="warning">Unmatched</Badge>; },
  }),
  column.display({ id: "quality", header: "Quality", width: 13, priority: 3, cell: ({ row }) => { const file = row.original; return <TextCell muted>{[file.resolution, file.videoCodec?.toUpperCase(), file.audioCodec?.toUpperCase(), file.isHdr ? (file.hdrType ?? "HDR") : null].filter(Boolean).join(" · ") || (file.analyzedAt ? "Audio" : "Not analyzed")}</TextCell>; } }),
  column.display({ id: "qualityStatus", header: "Profile", width: 9, priority: 3, cardRole: "status", cell: ({ row }) => { const meta = qualityStatus(row.original.qualityStatus); return <Badge size="sm" marker="dot" tone={badgeTone(meta.tone)}>{meta.label}</Badge>; } }),
  column.accessor("duration", { header: "Length", width: 6, priority: 4, align: "end", grow: false, sortable: true, cell: ({ row }) => <TextCell muted>{formatRuntime(row.original.duration)}</TextCell> }),
  column.accessor("size", { header: "Size", width: 7, priority: 2, align: "end", grow: false, sortable: true, sortDescFirst: true, cell: ({ row }) => <TextCell muted>{formatBytes(row.original.size)}</TextCell> }),
  column.accessor("addedAt", { header: "Added", width: 8, priority: 3, sortable: true, sortDescFirst: true, cell: ({ row }) => <RelativeTime value={parseTimestamp(row.original.addedAt)} /> }),
]);

function linkedTo(row: FileRow): string | null {
  if (row.movieId) return "Movie";
  if (row.episodeId) return "Episode";
  if (row.trackId) return "Track";
  if (row.chapterId) return "Chapter";
  return null;
}

/**
 * Every media file the scanner knows about, with the unmatched ones one filter away. Actions
 * run the same pipeline mutations the scanner uses (match, analyze, organize, unmatch).
 */
export function FilesBrowser({ libraryId }: { libraryId: string }) {
  const isAdmin = useIsAdmin();
  const { user } = useSession();
  const filters = useBrowserFilters();
  const table = useServerTable<MediaFileOrderByInput>({ persistKey: `files.${libraryId}`, defaultSorting: [{ id: "addedAt", desc: true }] });
  const [matching, setMatching] = useState<FileRow | null>(null);

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

  const [matchFile] = useMutation(MatchMediaFileDocument);
  const [analyzeFile] = useMutation(AnalyzeMediaFileDocument);
  const [organizeFile] = useMutation(OrganizeMediaFileDocument);
  const [unmatchFile] = useMutation(UnmatchMediaFileDocument);

  const latestRefetch = useRef(list.refetch);
  latestRefetch.current = list.refetch;
  const actions = useMemo(() => {
    const run = (rows: readonly FileRow[], label: string, task: (row: FileRow) => Promise<{ ok: boolean; message?: string | null }>) =>
      runSelectionAction(rows, label, (row) => fileName(row.path), async (row) => {
        const result = await task(row);
        if (!result.ok) throw new Error(result.message || `${label} did not complete`);
      }, () => { void latestRefetch.current(); });
    return defineActions<FileRow>("media-file", [
      {
        id: "open-item", bulk: true, label: "Open linked item", icon: IconExternalLink, group: "open", priority: 1,
        visible: (rows) => rows.length === 1 && Boolean(linkedItem(rows[0]!)?.href),
        run: (rows, context) => { const href = linkedItem(rows[0]!)?.href; if (href) context.navigate(href); },
      },
      {
        id: "copy-paths", label: "Copy file paths", icon: IconCopy, group: "open", bulk: true,
          run: (rows) => copyText(rows.map((row) => row.path).join("\n")), successMessage: () => "File paths copied",
      },
      ...(isAdmin ? [
        {
          id: "automatch", label: "Match automatically", icon: MatchIcon, group: "organise" as const, priority: 1, bulk: true,
          visible: (rows: readonly FileRow[]) => rows.some((row) => linkedTo(row) === null),
          disabledReason: (rows: readonly FileRow[]) => rows.some((row) => linkedTo(row) !== null) ? "Some selected files are already linked. Deselect them to match." : false,
          run: (rows: readonly FileRow[]) => run(rows, "Match", async (row) => {
            const { data } = await matchFile({ variables: { input: { mediaFileId: row.id, libraryId, autoMatch: true, allowProviderFallback: true } } });
            return { ok: Boolean(data?.matchMediaFile.success), message: data?.matchMediaFile.reason };
          }), successMessage: () => "Matching complete",
        },
        { id: "manual", bulk: true, label: "Match manually…", icon: ManualIcon, group: "organise" as const, priority: 2, visible: (rows: readonly FileRow[]) => rows.length === 1, run: (rows: readonly FileRow[]) => setMatching(rows[0]!) },
        {
          id: "analyze", label: (rows: readonly FileRow[]) => rows.length === 1 ? "Re-analyze" : `Re-analyze ${rows.length} files`, icon: IconRefresh, group: "manage" as const, priority: 2, bulk: true,
          run: (rows: readonly FileRow[]) => run(rows, "Analysis", async (row) => {
            const { data } = await analyzeFile({ variables: { mediaFileId: row.id, path: row.path } });
            return { ok: Boolean(data?.analyzeMediaFile.success), message: data?.analyzeMediaFile.message };
          }), successMessage: () => "Analysis requested",
        },
        {
          id: "organize", label: "Organize", icon: OrganizeIcon, group: "manage" as const, priority: 3, bulk: true,
          visible: (rows: readonly FileRow[]) => rows.some((row) => linkedTo(row) !== null),
          disabledReason: (rows: readonly FileRow[]) => rows.some((row) => linkedTo(row) === null) ? "Some selected files are unmatched. Match them before organizing." : false,
          run: (rows: readonly FileRow[]) => run(rows, "Organize", async (row) => {
            const { data } = await organizeFile({ variables: { input: { mediaFileId: row.id } } });
            return { ok: Boolean(data?.organizeMediaFile.success), message: data?.organizeMediaFile.reason };
          }), successMessage: () => "Files organized",
        },
        {
          id: "unmatch", label: "Unlink", icon: UnlinkIcon, group: "danger" as const, bulk: true,
          visible: (rows: readonly FileRow[]) => rows.some((row) => linkedTo(row) !== null),
          disabledReason: (rows: readonly FileRow[]) => rows.some((row) => linkedTo(row) === null) ? "Some selected files are not linked. Deselect them to unlink." : false,
          confirm: (rows: readonly FileRow[]) => ({ title: rows.length === 1 ? "Unlink this file?" : `Unlink ${rows.length} files?`, body: "Files stay on disk and in the library's file list, but are no longer attached to their items.", confirmLabel: "Unlink", tone: "warning" as const }),
          run: (rows: readonly FileRow[]) => run(rows, "Unlink", async (row) => {
            const { data } = await unmatchFile({ variables: { mediaFileId: row.id } });
            return { ok: Boolean(data?.unmatchMediaFile.success), message: data?.unmatchMediaFile.reason };
          }), successMessage: () => "Files unlinked",
        },
      ] : []),
    ]);
  }, [analyzeFile, isAdmin, libraryId, matchFile, organizeFile, unmatchFile]);
  const selection = useTableSelection({ scope: [user?.id, libraryId, where, table.sorting], rows: list.rows, ready: !list.fetching });
  const state = useMemo<DataTableState>(() => ({ pagination: { pageIndex: 0, pageSize: 50 }, selection: selection.selection, filters: [], sorting: table.sorting }), [table.sorting, selection.selection]);
  const onStateChange = (next: DataTableState, change: DataTableStateChange) => { if (change === "sorting") table.setSorting(next.sorting); else if (change === "selection") selection.onSelectionChange(next.selection); };

  const availabilityLabels: Record<AvailabilityFilter, string> = { all: "All files", available: "Below quality target", wanted: "Unmatched", missing: "Unmatched" };

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-3 sm:gap-4">
      <BrowserToolbar
        tableOnly
        query={filters.query}
        onQueryChange={filters.setQuery}
        letter={null}
        onLetterChange={() => undefined}
        total={list.totalCount}
        placeholder="Filter by path"
        trailing={
          <GlassSegmented<AvailabilityFilter>
            ariaLabel="File filter"
            size="sm"
            value={filters.availability === "wanted" ? "missing" : filters.availability}
            onChange={filters.setAvailability}
            items={(["all", "missing", "available"] as AvailabilityFilter[]).map((option) => ({ key: option, label: availabilityLabels[option] }))}
          />
        }
      />
      <div className="flex min-h-0 min-w-0 flex-1 flex-col">
        <DataTable<FileRow>
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
          renderBulkActions={(rows) => <TableBulkActions set={actions} rows={rows} noun={{ one: "file", other: "files" }} />}
          density="compact"
          noun={{ one: "file", other: "files" }}
          error={list.error && list.rows.length === 0 ? <ErrorState error={list.error} onRetry={() => void list.refetch()} /> : undefined}
          emptyMessage={filters.availability === "all" ? "No files yet. Scan the library to discover files." : "No files match this filter."}
        />
      </div>
      <ManualMatchDialog file={matching} libraryId={libraryId} onClose={() => setMatching(null)} onMatched={() => void list.refetch()} />
    </div>
  );
}
