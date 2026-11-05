import { Badge } from "glass-ux/badge";
import { defineActions } from "glass-ux/actions";
import { createDataTableColumns, DataTable, TextCell } from "glass-ux/data-table";
import { IconLink } from "glass-ux/icons";
import { FileIcon } from "glass-ux/file-icon";
import { MeterCell } from "glass-ux/meter-cell";
import { TruncatedText } from "glass-ux/truncated-text";
import { useMemo, useState } from "react";

import { EntityChip } from "@/components/entities/EntityChip";
import { useIsAdmin } from "@/lib/auth/useSession";
import { TorrentFilesDetailDocument, type TorrentFilesDetailQuery } from "@/graphql/generated/graphql";
import { entityPath } from "@/lib/entities/kinds";
import { formatBytes } from "@/lib/format";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";

import { MatchFileDialog, type MatchableFile } from "./MatchFileDialog";
import type { DownloadRow } from "./useDownloads";

type Match = NonNullable<TorrentFilesDetailQuery["torrent"]>["pendingMatches"]["edges"][number]["node"];

export interface TorrentFileRow {
  id: string;
  index: number;
  name: string;
  path: string;
  size: number;
  progress: number;
  media: boolean;
  match: Match | null;
}

const MEDIA_FILE = /\.(mkv|mp4|m4v|avi|mov|ts|webm|wmv|mpg|mpeg|mp3|flac|m4a|m4b|ogg|opus|wav|aac|wma)$/i;
const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path;
const clamp = (value: number) => (Number.isFinite(value) ? Math.max(0, Math.min(1, value)) : 0);

/** What a matched file was matched to, as a chip. */
function matchTarget(match: Match) {
  const { episode, movie, track, chapter } = match;
  if (episode) {
    const code = `S${String(episode.season).padStart(2, "0")}E${String(episode.episode).padStart(2, "0")}`;
    return { kind: "episode", id: episode.id, label: episode.show ? `${episode.show.name} · ${code}` : code, href: entityPath("episode", episode.id, episode.showId) };
  }
  if (movie) return { kind: "movie", id: movie.id, label: movie.title, href: entityPath("movie", movie.id) };
  if (track) return { kind: "track", id: track.id, label: track.title, href: entityPath("album", track.albumId) };
  if (chapter) return { kind: "chapter", id: chapter.id, label: chapter.title ?? "Chapter", href: entityPath("audiobook", chapter.audiobookId) };
  return null;
}

/** Where a file got to: imported, matched but not copied, not matched, or not something to import. */
export function fileOutcome(file: TorrentFileRow): { label: string; tone: "success" | "info" | "warning" | "danger" | "neutral"; detail?: string | null } | null {
  const { match } = file;
  if (match) {
    if (match.copiedAt) return { label: "Imported", tone: "success" };
    if (!matchTarget(match)) return { label: "No match", tone: "warning", detail: match.unmatchedReason };
    return match.copyError ? { label: "Copy failed", tone: "danger", detail: match.copyError } : { label: "Matched", tone: "info" };
  }
  if (!file.media) return null;
  return file.progress < 1 ? { label: "Downloading", tone: "neutral" } : { label: "Waiting", tone: "neutral" };
}

function MatchCell({ file }: { file: TorrentFileRow }) {
  const outcome = fileOutcome(file);
  if (!outcome) return <TextCell muted>Not imported</TextCell>;
  const target = file.match ? matchTarget(file.match) : null;
  return (
    <div className="flex min-w-0 flex-col items-start gap-1 py-1">
      <div className="flex min-w-0 max-w-full items-center gap-2">
        <Badge size="sm" marker="dot" tone={outcome.tone} className="shrink-0">{outcome.label}</Badge>
        {target ? <EntityChip entity={target} className="min-w-0" /> : null}
      </div>
      {outcome.detail ? <TruncatedText className="block min-w-0 max-w-full text-label-sm text-muted">{outcome.detail}</TruncatedText> : null}
    </div>
  );
}

const column = createDataTableColumns<TorrentFileRow>();
const COLUMNS = column.columns([
  column.accessor((row) => row.name, {
    id: "name", header: "File", width: 26, priority: 1, cardRole: "title",
    cell: ({ row }) => (
      <div className="flex min-w-0 items-center gap-2.5">
        <FileIcon name={row.original.name} size="lg" className="shrink-0" />
        <TruncatedText className="block min-w-0 text-body-sm text-foreground" text={row.original.path}>{row.original.name}</TruncatedText>
      </div>
    ),
  }),
  column.display({ id: "match", header: "Library match", width: 20, priority: 1, cell: ({ row }) => <MatchCell file={row.original} /> }),
  column.accessor((row) => row.size, {
    id: "size", header: "Size", width: 7, priority: 2, align: "end",
    cell: ({ row }) => <TextCell muted>{formatBytes(row.original.size)}</TextCell>,
  }),
  column.accessor((row) => row.progress, {
    id: "progress", header: "Progress", width: 9, priority: 3,
    cell: ({ row }) => <MeterCell label={`Progress of ${row.original.name}`} value={row.original.progress} max={1} warningAt={2} dangerAt={2} tone="data" valueText={`${Math.round(row.original.progress * 100)}%`} />,
  }),
]);

const getRowId = (row: TorrentFileRow) => row.id;
const getRowLabel = (row: TorrentFileRow) => row.name;
const NOUN = { one: "file", other: "files" };

/**
 * Stored files joined with what matching recorded for each one, with live progress over them. The
 * stored list means a download the client hasn't loaded still shows its files. Media comes first,
 * in name order, so a pack reads as its episodes with the screenshots and .nfo after them.
 */
export function useTorrentFiles(row: DownloadRow): { files: TorrentFileRow[]; loading: boolean; error: boolean; refetch: () => void } {
  const { data, loading, error, refetch } = useQuery(TorrentFilesDetailDocument, { variables: { id: row.record.id } });
  const liveFiles = row.live?.files;
  const files = useMemo(() => {
    const matches = (data?.torrent?.pendingMatches.edges ?? []).map((edge) => edge.node);
    const byPath = new Map(matches.map((match) => [match.sourcePath, match]));
    const liveByIndex = new Map((liveFiles ?? []).map((file) => [file.index, file]));
    // Older downloads can hold a stale second row for a file: the live sync updates only one.
    const byIndex = new Map<number, NonNullable<typeof data>["torrent"] extends infer T ? T extends { files: { edges: Array<{ node: infer F }> } } ? F : never : never>();
    for (const { node } of data?.torrent?.files.edges ?? []) {
      const seen = byIndex.get(node.fileIndex);
      if (!seen || node.progress > seen.progress) byIndex.set(node.fileIndex, node);
    }
    const stored = [...byIndex.values()];
    const rows: TorrentFileRow[] = stored.length
      ? stored.map((file) => ({ id: file.id, index: file.fileIndex, name: fileName(file.relativePath), path: file.relativePath, size: file.fileSize, progress: clamp(liveByIndex.get(file.fileIndex)?.progress ?? file.progress), media: MEDIA_FILE.test(file.relativePath), match: byPath.get(file.filePath) ?? null }))
      : (liveFiles ?? []).map((file) => ({ id: String(file.index), index: file.index, name: fileName(file.path), path: file.path, size: file.size, progress: clamp(file.progress), media: MEDIA_FILE.test(file.path), match: matches.find((match) => match.sourcePath.endsWith(file.path)) ?? null }));
    return rows.sort((a, b) => Number(b.media || Boolean(b.match)) - Number(a.media || Boolean(a.match)) || a.name.localeCompare(b.name, undefined, { numeric: true }));
  }, [data, liveFiles]);
  return { files, loading: loading && !data, error: Boolean(error) && !data, refetch: () => void refetch() };
}

/** "18 of 20 media files imported": the one line a pack needs above its files. */
export function filesSummary(files: readonly TorrentFileRow[]): string | null {
  const media = files.filter((file) => file.media || file.match);
  if (media.length === 0) return null;
  const imported = media.filter((file) => file.match?.copiedAt).length;
  const unmatched = media.filter((file) => file.match && !file.match.copiedAt && !matchTarget(file.match)).length;
  const parts = [`${imported} of ${media.length} media ${media.length === 1 ? "file" : "files"} imported`];
  if (unmatched) parts.push(`${unmatched} not matched`);
  return parts.join(" · ");
}

/** Only a finished media file that hasn't landed in a library needs matching by hand. */
function canMatchByHand(file: TorrentFileRow) {
  return (file.media || Boolean(file.match)) && file.progress >= 1 && !file.match?.copiedAt;
}

export function TorrentFiles({ row, files }: { row: DownloadRow; files: ReturnType<typeof useTorrentFiles> }) {
  const summary = filesSummary(files.files);
  const isAdmin = useIsAdmin();
  const [matching, setMatching] = useState<MatchableFile | null>(null);
  const actions = useMemo(() => defineActions<TorrentFileRow>("torrent-file", [
    {
      id: "match", label: "Match to a title…", icon: IconLink, group: "manage", priority: 1,
      visible: (rows) => rows.length === 1 && canMatchByHand(rows[0]!),
      run: (rows) => { const file = rows[0]!; setMatching({ infoHash: row.record.infoHash, fileIndex: file.index, name: file.name, libraryId: row.record.libraryId }); },
    },
  ]), [row.record.infoHash, row.record.libraryId]);
  return (
    <div className="flex min-w-0 flex-col gap-2">
      {summary ? <p className="text-label-sm text-muted">{summary}</p> : null}
      <DataTable<TorrentFileRow>
        id="librarian.download-files"
        label={`Files in ${row.record.name}`}
        queryKey={row.record.id}
        height="auto"
        columns={COLUMNS}
        rows={files.files}
        rowCount={files.files.length}
        getRowId={getRowId}
        getRowLabel={getRowLabel}
        loading={files.loading}
        error={files.error ? "Couldn't load this download's files. Retry to reconnect." : undefined}
        onRetry={files.refetch}
        selectable={false}
        actions={isAdmin ? actions : undefined}
        noun={NOUN}
        emptyMessage="The client reports no files."
      />
      <MatchFileDialog file={matching} onClose={() => setMatching(null)} onMatched={files.refetch} />
    </div>
  );
}
