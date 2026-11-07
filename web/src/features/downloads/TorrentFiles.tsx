import { Badge } from "glass-ux/badge";
import { defineActions } from "glass-ux/actions";
import { createDataTableColumns, DataTable, TextCell } from "glass-ux/data-table";
import { IconLink } from "glass-ux/icons";
import { FileIcon } from "glass-ux/file-icon";
import { MeterCell } from "glass-ux/meter-cell";
import { TruncatedText } from "glass-ux/truncated-text";
import { createContext, useContext, useMemo, useState } from "react";

import { EntityChip } from "@/components/entities/EntityChip";
import { refCellEntity } from "@/lib/entities/resolver";
import { Artwork } from "@/components/ui";
import { useIsAdmin } from "@/lib/auth/useSession";
import { TorrentFilesDetailDocument, TorrentMatchPreviewDocument, type TorrentFilesDetailQuery, type TorrentMatchPreviewQuery } from "@/graphql/generated/graphql";
import { entityPath } from "@/lib/entities/kinds";
import { formatBytes } from "@/lib/format";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";

import { MatchFileDialog, type MatchableFile } from "./MatchFileDialog";
import { IDENTIFIED_STATUS, type IdentifiedItem } from "./TorrentIdentification";
import type { DownloadRow } from "./useDownloads";

type Match = NonNullable<TorrentFilesDetailQuery["torrent"]>["pendingMatches"]["edges"][number]["node"];
type Preview = TorrentMatchPreviewQuery["torrentMatchPreview"];
type PreviewFile = Preview["files"][number];

/** What the plan says about each file, by index, for the table's cells. */
const FilePlanContext = createContext<ReadonlyMap<number, PreviewFile>>(new Map());

export interface TorrentFileRow {
  id: string;
  index: number;
  name: string;
  path: string;
  /** Where it is on disk, when the stored list says. */
  filePath?: string;
  size: number;
  progress: number;
  media: boolean;
  match: Match | null;
  /** Set on a heading row: the title these files were identified as, and the files under it. */
  group?: { files: TorrentFileRow[]; item: IdentifiedItem | null };
}

/**
 * The download's files under the titles they were identified as, in the results' order, with
 * anything else (a cover, a text file) after them. With nothing identified, just the files.
 */
export function groupFilesByTitle(files: readonly TorrentFileRow[], items: readonly IdentifiedItem[]): TorrentFileRow[] {
  if (items.length === 0) return [...files];
  const owner = new Map<string, IdentifiedItem>();
  for (const item of items) {
    let paths: unknown = [];
    try { paths = JSON.parse(item.filePaths); } catch { /* nothing to group by */ }
    if (Array.isArray(paths)) for (const path of paths) if (typeof path === "string") owner.set(path, item);
  }
  const ownerOf = (file: TorrentFileRow) => (file.filePath ? owner.get(file.filePath) : undefined) ?? [...owner].find(([path]) => path.endsWith(`/${file.path}`))?.[1];
  const byItem = new Map<string | null, TorrentFileRow[]>();
  for (const file of files) {
    const key = ownerOf(file)?.id ?? null;
    byItem.set(key, [...(byItem.get(key) ?? []), file]);
  }
  const heading = (id: string, name: string, members: TorrentFileRow[], item: IdentifiedItem | null): TorrentFileRow => ({
    id, index: -1, name, path: name, media: true, match: null,
    size: members.reduce((sum, file) => sum + file.size, 0),
    progress: members.reduce((sum, file) => sum + file.progress * file.size, 0) / Math.max(1, members.reduce((sum, file) => sum + file.size, 0)),
    group: { files: members, item },
  });
  const rows = items.flatMap((item) => {
    const members = byItem.get(item.id);
    return members ? [heading(`title:${item.id}`, item.title ?? item.label, members, item)] : [];
  });
  const others = byItem.get(null);
  return others ? [...rows, heading("title:other", "Other files", others, null)] : rows;
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

const PLAN_BADGE = {
  match: { label: "Best match", tone: "info" },
  review: { label: "Needs review", tone: "warning" },
  other: { label: "Other book", tone: "neutral" },
  none: { label: "No match", tone: "warning" },
} as const;

function MatchCell({ file }: { file: TorrentFileRow }) {
  if (file.group) {
    const status = file.group.item ? IDENTIFIED_STATUS[file.group.item.status] : null;
    return status ? <Badge size="sm" marker="dot" tone={status.tone}>{status.label}</Badge> : null;
  }
  const plans = useContext(FilePlanContext);
  const plan = plans.get(file.index);
  const outcome = fileOutcome(file);
  if (!outcome && !plan) return <TextCell muted>Not imported</TextCell>;
  const target = file.match ? matchTarget(file.match) : null;
  // Until a file has an outcome of its own, show what matching expects of it. Once imported, keep
  // the plan's reason, which says why it went where it did.
  const landed = outcome?.label === "Imported" || outcome?.label === "Matched" || outcome?.label === "Copy failed";
  const planBadge = plan && !landed ? PLAN_BADGE[plan.status as keyof typeof PLAN_BADGE] ?? PLAN_BADGE.none : null;
  const detail = outcome?.detail && !plan ? outcome.detail : plan?.detail;
  return (
    <div className="flex min-w-0 flex-col items-start gap-1 py-1">
      <div className="flex min-w-0 max-w-full items-center gap-2">
        <Badge size="sm" marker="dot" tone={planBadge?.tone ?? outcome?.tone ?? "neutral"} className="shrink-0">{planBadge?.label ?? outcome?.label}</Badge>
        {target ? <EntityChip entity={target} className="min-w-0" /> : planBadge && plan?.target ? <TruncatedText className="block min-w-0 text-label-sm text-foreground">{plan.target}</TruncatedText> : null}
      </div>
      {detail ? <TruncatedText className="block min-w-0 max-w-full text-label-sm text-muted">{detail}</TruncatedText> : null}
      {plan?.warning ? <TruncatedText className="block min-w-0 max-w-full text-label-sm text-warning">{plan.warning}</TruncatedText> : null}
    </div>
  );
}

const column = createDataTableColumns<TorrentFileRow>();
const COLUMNS = column.columns([
  column.accessor((row) => row.name, {
    id: "name", header: "File", width: 26, priority: 1, cardRole: "title",
    cell: ({ row }) => (
      <div className="flex min-w-0 items-center gap-2.5">
        {row.original.group ? (row.original.group.item ? <Artwork src={row.original.group.item.coverUrl} alt="" aspect="square" className="w-7 shrink-0" /> : null) : <FileIcon name={row.original.name} size="lg" className="shrink-0" />}
        <TruncatedText className={`block min-w-0 text-body-sm text-foreground ${row.original.group ? "font-medium" : ""}`} text={row.original.path}>{row.original.name}</TruncatedText>
      </div>
    ),
  }),
  column.display({ id: "match", header: "Library match", width: 20, priority: 1, entity: (row) => refCellEntity(row.match ? matchTarget(row.match) : null), cell: ({ row }) => <MatchCell file={row.original} /> }),
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
const GROUP_NOUN = { one: "group", other: "groups" };

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
      ? stored.map((file) => ({ id: file.id, index: file.fileIndex, name: fileName(file.relativePath), path: file.relativePath, filePath: file.filePath, size: file.fileSize, progress: clamp(liveByIndex.get(file.fileIndex)?.progress ?? file.progress), media: MEDIA_FILE.test(file.relativePath), match: byPath.get(file.filePath) ?? null }))
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
  return !file.group && (file.media || Boolean(file.match)) && file.progress >= 1 && !file.match?.copiedAt;
}

const getSubRows = (row: TorrentFileRow) => row.group?.files ?? null;
const TREE = { getSubRows };

export function TorrentFiles({ row, files, items = [] }: { row: DownloadRow; files: ReturnType<typeof useTorrentFiles>; items?: readonly IdentifiedItem[] }) {
  const summary = filesSummary(files.files);
  const rows = useMemo(() => groupFilesByTitle(files.files, items), [files.files, items]);
  const grouped = rows.some((file) => file.group);
  const isAdmin = useIsAdmin();
  const [matching, setMatching] = useState<MatchableFile | null>(null);
  const preview = useQuery(TorrentMatchPreviewDocument, { variables: { infoHash: row.record.infoHash } });
  const plan = preview.data?.torrentMatchPreview;
  const plans = useMemo(() => new Map((plan?.files ?? []).map((file) => [file.fileIndex, file])), [plan]);
  const actions = useMemo(() => defineActions<TorrentFileRow>("torrent-file", [
    {
      id: "match", label: "Match to a title…", icon: IconLink, group: "manage", priority: 1,
      visible: (rows) => rows.length === 1 && canMatchByHand(rows[0]!),
      run: (rows) => { const file = rows[0]!; setMatching({ infoHash: row.record.infoHash, fileIndex: file.index, name: file.name, libraryId: row.record.libraryId }); },
    },
  ]), [row.record.infoHash, row.record.libraryId]);
  return (
    <div className="flex h-full min-h-0 min-w-0 flex-col gap-2">
      {summary ? <p className="shrink-0 text-label-sm text-muted">{summary}</p> : null}
      <FilePlanContext.Provider value={plans}>
      <div className="min-h-0 flex-1">
      <DataTable<TorrentFileRow>
        key={grouped ? "titles" : "files"}
        id="librarian.download-files"
        label={`Files in ${row.record.name}`}
        queryKey={row.record.id}
        columns={COLUMNS}
        rows={rows}
        rowCount={rows.length}
        tree={grouped ? TREE : undefined}
        defaultState={rows.length === 1 && grouped ? { expanded: [rows[0]!.id] } : undefined}
        getRowId={getRowId}
        getRowLabel={getRowLabel}
        loading={files.loading}
        error={files.error ? "Couldn't load this download's files. Retry to reconnect." : undefined}
        onRetry={files.refetch}
        selectable={false}
        actions={isAdmin ? actions : undefined}
        noun={grouped ? GROUP_NOUN : NOUN}
        emptyMessage="The client reports no files."
      />
      </div>
      </FilePlanContext.Provider>
      <MatchFileDialog file={matching} onClose={() => setMatching(null)} onMatched={files.refetch} />
    </div>
  );
}
