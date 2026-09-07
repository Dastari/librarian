import { toast } from "@heroui/react";
import { IconFile, IconFileSearch, IconLinkOff, IconRefresh, IconWand } from "@tabler/icons-react";
import { useMemo, useState } from "react";

import { DataTable, EmptyState, ErrorState, GlassSegmented, StatusChip, type DataTableColumn, type DataTableRowAction } from "@/components/ui";
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
import { useIsAdmin } from "@/lib/auth/useSession";
import { formatBytes, formatDate, formatRuntime } from "@/lib/format";
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
  const filters = useBrowserFilters(`files.${libraryId}`, "table");
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

  const actions = useMemo<Array<DataTableRowAction<FileRow>>>(
    () =>
      isAdmin
        ? [
            {
              key: "automatch",
              label: "Match automatically",
              icon: <IconWand size={16} />,
              hidden: (row) => linkedTo(row) !== null,
              onAction: (row) =>
                run("Match", async () => {
                  const { data } = await matchFile({ variables: { input: { mediaFileId: row.id, libraryId, autoMatch: true, allowProviderFallback: true } } });
                  const result = data?.matchMediaFile;
                  return { ok: Boolean(result?.success), message: result?.reason ?? (result?.matchedType ? `Linked to ${result.matchedType}` : null) };
                }),
            },
            { key: "manual", label: "Match manually…", icon: <IconFileSearch size={16} />, onAction: (row) => setMatching(row) },
            {
              key: "analyze",
              label: "Re-analyze",
              icon: <IconRefresh size={16} />,
              onAction: (row) =>
                run("Analysis", async () => {
                  const { data } = await analyzeFile({ variables: { mediaFileId: row.id, path: row.path } });
                  return { ok: Boolean(data?.analyzeMediaFile.success), message: data?.analyzeMediaFile.message ?? (data?.analyzeMediaFile.queued ? "Analysis queued" : null) };
                }),
            },
            {
              key: "organize",
              label: "Organize",
              icon: <IconFile size={16} />,
              hidden: (row) => linkedTo(row) === null,
              onAction: (row) =>
                run("Organize", async () => {
                  const { data } = await organizeFile({ variables: { input: { mediaFileId: row.id } } });
                  const result = data?.organizeMediaFile;
                  return { ok: Boolean(result?.success), message: result?.reason ?? (result?.newPath ? `Moved to ${result.newPath}` : null) };
                }),
            },
            {
              key: "unmatch",
              label: "Unlink",
              icon: <IconLinkOff size={16} />,
              destructive: true,
              hidden: (row) => linkedTo(row) === null,
              onAction: (row) =>
                run("Unlink", async () => {
                  const { data } = await unmatchFile({ variables: { mediaFileId: row.id } });
                  return { ok: Boolean(data?.unmatchMediaFile.success), message: data?.unmatchMediaFile.reason };
                }),
            },
          ]
        : [],
    [analyzeFile, isAdmin, libraryId, matchFile, organizeFile, unmatchFile],
  );

  const columns = useMemo<Array<DataTableColumn<FileRow>>>(
    () => [
      {
        id: "path",
        header: "File",
        sortable: true,
        cell: (row) => (
          <span className="min-w-0">
            <span className="block truncate text-body-sm text-foreground">{fileName(row.path)}</span>
            <span className="block truncate font-mono text-label-sm text-muted">{row.relativePath ?? row.path}</span>
          </span>
        ),
      },
      {
        id: "linked",
        header: "Linked",
        size: 120,
        cell: (row) => {
          const kind = linkedTo(row);
          return kind ? <StatusChip status={{ label: kind, tone: "success", dot: "bg-success" }} /> : <StatusChip status={{ label: "Unmatched", tone: "warning", dot: "bg-warning" }} />;
        },
      },
      { id: "quality", header: "Quality", size: 190, hideBelow: "md", cell: (row) => <span className="text-muted">{[row.resolution, row.videoCodec?.toUpperCase(), row.audioCodec?.toUpperCase(), row.isHdr ? (row.hdrType ?? "HDR") : null].filter(Boolean).join(" · ") || (row.analyzedAt ? "Audio" : "Not analyzed")}</span> },
      { id: "qualityStatus", header: "Profile", size: 120, hideBelow: "lg", cell: (row) => <StatusChip minimal status={qualityStatus(row.qualityStatus)} /> },
      { id: "duration", header: "Length", size: 90, align: "end", numeric: true, sortable: true, hideBelow: "lg", cell: (row) => formatRuntime(row.duration) },
      { id: "size", header: "Size", size: 100, align: "end", numeric: true, sortable: true, cell: (row) => formatBytes(row.size) },
      { id: "addedAt", header: "Added", size: 130, sortable: true, hideBelow: "md", cell: (row) => <span className="text-muted">{formatDate(row.addedAt)}</span> },
    ],
    [],
  );

  const availabilityLabels: Record<AvailabilityFilter, string> = { all: "All files", available: "Below quality target", wanted: "Unmatched", missing: "Unmatched" };

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-4">
      <BrowserToolbar
        query={filters.query}
        onQueryChange={filters.setQuery}
        letter={null}
        onLetterChange={() => undefined}
        view="table"
        onViewChange={() => undefined}
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
      <DataTable<FileRow>
        columns={columns}
        rows={list.rows}
        getRowId={getRowId}
        isLoading={list.loading}
        totalCount={list.totalCount}
        noun="files"
        density="compact"
        rowActions={actions}
        sorting={table.sorting}
        onSortingChange={table.setSorting}
        infinite={{ hasMore: list.hasMore, loadMore: list.loadMore, loadingMore: list.loadingMore }}
        error={list.error && list.rows.length === 0 ? <ErrorState error={list.error} onRetry={() => void list.refetch()} /> : undefined}
        emptyState={<EmptyState icon={IconFile} title={filters.availability === "all" ? "No files yet" : "Nothing to show"} description={filters.availability === "all" ? "Scan the library to discover files." : "Every file matches this filter's opposite."} />}
      />
      <ManualMatchDialog file={matching} libraryId={libraryId} onClose={() => setMatching(null)} onMatched={() => void list.refetch()} />
    </div>
  );
}
