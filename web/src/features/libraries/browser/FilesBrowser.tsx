import { defineActions } from "glass-ux/actions";
import { Badge } from "glass-ux/badge";
import { createDataTableColumns, DataTable, IdentityCell, RelativeTime, TextCell, type DataTableState, type DataTableStateChange } from "glass-ux/data-table";
import { IconExternalLink, IconRefresh } from "glass-ux/icons";
import { toast } from "glass-ux/toast";
import { IconFile, IconFileSearch, IconLinkOff, IconWand } from "@tabler/icons-react";
import { useMemo, useState } from "react";

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
import { errorMessage } from "@/lib/graphql/errors";
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

  const run = async (label: string, task: () => Promise<{ ok: boolean; message?: string | null }>) => {
    try {
      const result = await task();
      if (result.ok) toast.success(result.message ?? `${label} done`);
      else toast.warning(result.message ?? `${label} did not complete`);
      void list.refetch();
    } catch (error) {
      toast.danger(errorMessage(error, `${label} failed`));
    }
  };

  const actions = useMemo(() => defineActions<FileRow>("media-file", [
    {
      id: "open-item", label: "Open linked item", icon: IconExternalLink, group: "open", priority: 1,
      visible: (rows) => rows.length === 1 && Boolean(linkedItem(rows[0]!)?.href),
      run: (rows, context) => { const href = linkedItem(rows[0]!)?.href; if (href) context.navigate(href); },
    },
    ...(isAdmin ? [
      {
        id: "automatch", label: "Match automatically", icon: MatchIcon, group: "organise" as const, priority: 1,
        visible: (rows: readonly FileRow[]) => rows.length === 1 && linkedTo(rows[0]!) === null,
        run: (rows: readonly FileRow[]) => run("Match", async () => {
          const { data } = await matchFile({ variables: { input: { mediaFileId: rows[0]!.id, libraryId, autoMatch: true, allowProviderFallback: true } } });
          const result = data?.matchMediaFile;
          return { ok: Boolean(result?.success), message: result?.reason ?? (result?.matchedType ? `Linked to ${result.matchedType}` : null) };
        }),
      },
      { id: "manual", label: "Match manually…", icon: ManualIcon, group: "organise" as const, priority: 2, visible: (rows: readonly FileRow[]) => rows.length === 1, run: (rows: readonly FileRow[]) => setMatching(rows[0]!) },
      {
        id: "analyze", label: (rows: readonly FileRow[]) => (rows.length === 1 ? "Re-analyze" : `Re-analyze ${rows.length} files`), icon: IconRefresh, group: "manage" as const, priority: 2, bulk: true,
        run: async (rows: readonly FileRow[]) => {
          for (const row of rows) await run("Analysis", async () => {
            const { data } = await analyzeFile({ variables: { mediaFileId: row.id, path: row.path } });
            return { ok: Boolean(data?.analyzeMediaFile.success), message: data?.analyzeMediaFile.message ?? (data?.analyzeMediaFile.queued ? "Analysis queued" : null) };
          });
        },
      },
      {
        id: "organize", label: "Organize", icon: OrganizeIcon, group: "manage" as const, priority: 3,
        visible: (rows: readonly FileRow[]) => rows.length === 1 && linkedTo(rows[0]!) !== null,
        run: (rows: readonly FileRow[]) => run("Organize", async () => {
          const { data } = await organizeFile({ variables: { input: { mediaFileId: rows[0]!.id } } });
          const result = data?.organizeMediaFile;
          return { ok: Boolean(result?.success), message: result?.reason ?? (result?.newPath ? `Moved to ${result.newPath}` : null) };
        }),
      },
      {
        id: "unmatch", label: "Unlink", icon: UnlinkIcon, group: "danger" as const,
        visible: (rows: readonly FileRow[]) => rows.length === 1 && linkedTo(rows[0]!) !== null,
        confirm: () => ({ title: "Unlink this file?", body: "The file stays on disk and in the library's file list, but is no longer attached to its item.", confirmLabel: "Unlink", tone: "warning" as const }),
        run: (rows: readonly FileRow[]) => run("Unlink", async () => {
          const { data } = await unmatchFile({ variables: { mediaFileId: rows[0]!.id } });
          return { ok: Boolean(data?.unmatchMediaFile.success), message: data?.unmatchMediaFile.reason };
        }),
      },
    ] : []),
  ]), [analyzeFile, isAdmin, libraryId, matchFile, organizeFile, unmatchFile]);
  const state = useMemo<DataTableState>(() => ({ pagination: { pageIndex: 0, pageSize: 50 }, selection: {}, filters: [], sorting: table.sorting }), [table.sorting]);
  const onStateChange = (next: DataTableState, change: DataTableStateChange) => { if (change === "sorting") table.setSorting(next.sorting); };

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
          selectable={isAdmin}
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
